# Epore - Learning how to use epoll to Event Queue for non-blocking I/O

## Files Structure

- `ffi.rs`: This module will contain the code related to the syscalls we need to communicate with the host operating system.
- `main.rs`: This is the example program itself
- `poll.rs`: This module contains the main abstraction, which is a thin layer over epoll

## Overview

- `Poll`: Struct to interface with the OS's event notification system aka event queue (`io_uring`, `epoll`, `kqueue`, `IOCP`).

  - `new()`: To create a new interface to OS's event queue.
    Similar to [`epoll_create`](https://man7.org/linux/man-pages/man2/epoll_create.2.html)
  - `registry()`: Returns a reference to the registry that we can use to register interest to be notified about new events.
    Similar to [`int epoll_ctl(int epfd, int op, int fd, struct epoll_event *_Nullable event);`](https://man7.org/linux/man-pages/man2/epoll_ctl.2.html)
  - `poll()`: blocks the thread it's called on until an event is ready or its times out, whichever occurs first.

- `Registry`: Struct to register interest in a certain `Event`.

- `Token`: Using `Token` to track which `TcpStream` socket generated the event.

### Sample Usage

```rust
let queue = Poll::new().unwrap();
let id = 1;

// register interest in events on a TcpStream
queue.registry().register(&stream, id, ...).unwrap();

// store the to be tracked events
let mut events = Vec::with_capacity(1);

// This will block the curren thread
queue.poll(&mut events, None).unwrap();
//...data is ready on one of the tracked streams
```

## Notes

### `Registry` and `Poll` Relationship

We can see that the struct `Poll` has an internal struct `Registry` inside of it. By moving the struct `Registry` inside of the `Poll` struct, we can call `Registry::try_clone()` to get an owned Registry instance.

Therefore, we can pass the `Registry` to other threads with `Arc`, allowing multiple threads to register their interest to the same `Poll` instance even when `Poll` is blocking another thread while waiting for new events to happen in `Poll::poll`

`Poll::poll()` requires exclusive access since it takes a `&mut self`, so when we're waiting for events in `Poll::poll()`, there is no way to register interest from a different thread at the same time if we rely on using `Poll` to register.
