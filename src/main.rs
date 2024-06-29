use core::panic;
use std::{
    collections::HashSet,
    io::{self, Read, Result, Write},
    net::TcpStream,
    usize,
};

use bytes::{BufMut, BytesMut};
use ffi::Event;
use poll::Poll;
use rand::Rng;

mod ffi;
mod poll;

// build our request as a buffer of bytes (&[u8])
//
// NOTE: BytesMut does implement AsRef so that
// it can be easily converted into &[u8] for write_all()
//
fn get_req(path: &str) -> BytesMut {
    let mut buffer = BytesMut::new();
    buffer.put(&b"GET "[..]);
    buffer.put(path.as_bytes());
    buffer.put(&b" HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"[..]);
    buffer
}

// Function call happens when an event is ready to be processed
fn handle_event(
    events: &[Event],
    tcp_streams: &mut [TcpStream],
    handled_tokens: &mut HashSet<usize>,
) -> Result<usize> {
    let mut handled_events_curr = 0;

    for event in events {
        // The token helps differentiate each I/O resource
        // in our case, there are
        // 200 TcpStreams aka 200 TCP socket file descriptors
        let resource_index = event.token();

        // This buffer can hold 4096 characters/bytes
        let mut data_buffer_read_from_tcp_stream = vec![0u8; 4096];

        loop {
            match tcp_streams[resource_index].read(&mut data_buffer_read_from_tcp_stream) {
                // data_buffer_read_from_tcp_stream is completely drained
                // we can consider the event successfully handled
                Ok(0) => {
                    if !handled_tokens.insert(resource_index) {
                        break;
                    };
                    handled_events_curr += 1;
                    break;
                }

                // data_buffer_read_from_tcp_stream is not completely drained
                // we still have some data left in the buffer
                Ok(n) => {
                    let txt = String::from_utf8_lossy(&data_buffer_read_from_tcp_stream[..n]);
                    println!("RECEIVED: {:?}", event);
                    println!("{txt}\n------\n");
                }

                // WouldBlock indicates that the data transfer is not complete,
                // but there is no data ready right now.
                // the transfer must be retried
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => break,
                // return the error and break the loop
                Err(error) => return Err(error),
            }
        }
    }

    Ok(handled_events_curr)
}

fn main() -> Result<()> {
    // The Event "queue":
    // not really,
    // just the interface to Linux's epoll_queue
    let mut epoll_interface_registry = Poll::new().expect("Can't run epoll_create.");

    // aka how many requests do we want to send
    // this is also how many TcpStreams we will create
    // also the number of TCP socket file descriptors
    let number_of_events: usize = 200;

    let mut tcp_streams: Vec<TcpStream> = vec![];

    let packet_addr: &str = "localhost:8080";

    for request_id in 0..number_of_events {
        let random_delay = rand::thread_rng().gen_range(1..=number_of_events / 5) * 1000;

        // the TcpServer should return our GET request after random_delay seconds
        let url_path = format!("/{random_delay}/request-{request_id}");

        let request_buffer = get_req(&url_path);

        let mut tcp_stream = TcpStream::connect(packet_addr).expect("Failed to create TcpStream.");

        // nonblocking: Moves this TCP stream into or out of nonblocking mode.
        // This will result in read, write, recv and send operations
        // becoming nonblocking, i.e.,
        // immediately returning from their calls.
        // If the IO operation is successful,
        // Ok is returned and no further action is required.
        // If the IO operation could not be completed and needs to be retried,
        // an error with kind io::ErrorKind::WouldBlock is returned.
        //
        // nodelay: If set, this option disables the Nagle algorithm.
        // This means that segments are always sent as soon as possible,
        // even if there is only a small amount of data.
        // When not set, data is buffered until there is a sufficient amount to send out,
        // thereby avoiding the frequent sending of small packets.
        tcp_stream
            .set_nonblocking(true)
            .and_then(|_| tcp_stream.set_nodelay(true))
            .expect("Failed to set TcpStream to nonblocking and nodelay");

        tcp_stream
            .write_all(request_buffer.as_ref())
            .expect("Failed to write to TcpStream.");

        // register interests
        // for when data is ready to be READ
        // from this TcpStream,
        // edge-triggered by EPOLLET
        epoll_interface_registry
            .registry()
            // the request_id is also the token for file descriptor ID purposes
            .register(&tcp_stream, request_id, ffi::EPOLLET | ffi::EPOLLIN)
            .unwrap_or_else(|_| {
                panic!("Failed to register interests in the event queue for {request_id}")
            });

        tcp_streams.push(tcp_stream);
    }

    println!("Finished sending requests");

    let mut handled_events = 0;

    // track which resources has been handled
    //
    // while a vector is simpler,
    // a hashset is much more efficient
    // as it only tracks the resources that are handled
    // avoid empty allocations
    let mut handled_tokens: HashSet<usize> = HashSet::new();

    // this loop will run for a while
    while handled_events < number_of_events {
        // too low of a number would limit
        // how many events the OS could notify us
        // on each wake up (see: EPOLLET)
        let mut events_buffer: Vec<Event> = Vec::with_capacity(20);

        // when epoll_wait success, number of file descriptors
        // ready for the requested I/O operation, or zero if no file
        // descriptor became ready during the requested timeout
        // milliseconds
        //
        // timeout equal to zero (aka None) causes epoll_wait() to return
        // immediately, even if no events are available.
        //
        // this should speed up our loop a lil bit
        epoll_interface_registry.poll(&mut events_buffer, None)?;

        if events_buffer.is_empty() {
            println!("Timed out");
            continue;
        }

        // the loop only reaches here when an event is handled
        handled_events += handle_event(&events_buffer, &mut tcp_streams, &mut handled_tokens)?;
    }

    println!("Finished receiving all responses");

    Ok(())
}
