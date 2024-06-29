// Register interest
pub const EPOLL_CTL_ADD: i32 = 1;

// Bit mask so express that
// we are interest when the data is available to READ
pub const EPOLLIN: i32 = 0x1;

// Bit mask for requests
// edge-triggered notification
// for the associated file descriptor.
// The default behavior for epoll is level-triggered.
pub const EPOLLET: i32 = 1 << 31;

// Here we have the syscalls
// Unsafe !!!
#[link(name = "c")]
extern "C" {
    pub fn epoll_create(size: i32) -> i32;
    pub fn close(fd: i32) -> i32;
    pub fn epoll_ctl(epfd: i32, op: i32, fd: i32, event: *mut Event) -> i32;
    pub fn epoll_wait(epfd: i32, events: *mut Event, maxevents: i32, timeout: i32) -> i32;
}

// Avoid padding by using repr(packed)
// Data struct is different in Rust compared to C
#[derive(Debug)]
#[repr(C)]
#[cfg_attr(target_arch = "x86_64", repr(packed))]
pub struct Event {
    pub(crate) events: u32,
    // Using `Token` a.k.a `epoll_data` to track which socket generated the event
    pub(crate) epoll_data: usize,
}

impl Event {
    pub fn token(&self) -> usize {
        self.epoll_data
    }
}
