use crate::ffi;
use std::{
    io::{self, Result},
    net::TcpStream,
    os::fd::AsRawFd,
};

// We can be interested in multiple events
type Events = Vec<ffi::Event>;

// The file descriptor of our target (could be a TCP socket or a TcpStream in our case)
pub struct Registry {
    raw_fd: i32,
}

impl Registry {
    // Register interest by adding it
    // TcpStream is a high level representation of a TCP socket file descriptor
    // token is too differentiate from different file descriptor, as a label
    pub fn register(&self, source: &TcpStream, token: usize, interests: i32) -> Result<()> {
        match unsafe {
            ffi::epoll_ctl(
                self.raw_fd,
                ffi::EPOLL_CTL_ADD,
                source.as_raw_fd(),
                &mut ffi::Event {
                    events: interests as u32,
                    epoll_data: token,
                },
            )
        } {
            exit_code if exit_code < 0 => Err(io::Error::last_os_error()),
            _ => Ok(()),
        }
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        let res = unsafe { ffi::close(self.raw_fd) };

        if res < 0 {
            let err = io::Error::last_os_error();
            eprintln!("ERROR: {err:?}");
        }
    }
}

pub struct Poll {
    registry: Registry,
}

impl Poll {
    pub fn new() -> Result<Self> {
        let res = unsafe { ffi::epoll_create(1) };
        if res < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            registry: Registry { raw_fd: res },
        })
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn poll(&mut self, events: &mut Events, timeout: Option<i32>) -> Result<()> {
        let fd = self.registry.raw_fd;

        let timeout = timeout.unwrap_or(-1);

        let max_events = events.capacity() as i32;

        let res = unsafe { ffi::epoll_wait(fd, events.as_mut_ptr(), max_events, timeout) };

        if res < 0 {
            return Err(io::Error::last_os_error());
        };

        // when epoll_wait success, number of file descriptors
        // ready for the requested I/O operation, or zero if no file
        // descriptor became ready during the requested timeout
        // milliseconds
        unsafe { events.set_len(res as usize) };

        Ok(())
    }
}
