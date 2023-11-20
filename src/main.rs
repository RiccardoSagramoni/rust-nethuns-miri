use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

#[derive(Debug)]
struct RecvPacket<'a> {
    status: &'a AtomicBool,
    #[allow(dead_code)]
    packet: &'a [u8],
}

impl<'a> Drop for RecvPacket<'_> {
    fn drop(&mut self) {
        self.status.store(false, Ordering::Release);
    }
}

#[derive(Debug)]
struct RingSlot {
    status: AtomicBool,
    packet: Vec<u8>,
    timestamp: Instant,
}

#[derive(Debug)]
struct Ring {
    slots: Vec<RingSlot>,
    next: usize,
}

impl Ring {
    fn new() -> Self {
        let mut slots: Vec<RingSlot> = Vec::with_capacity(10);
        for i in 0..slots.capacity() {
            slots.push(RingSlot {
                status: AtomicBool::new(false),
                packet: vec![i as u8; 10],
                timestamp: Instant::now(),
            })
        }
        
        Ring { slots, next: 0 }
    }
    
    fn recv(&mut self) -> Option<RecvPacket> {
        let next_idx = self.next;
        
        if self.slots[next_idx].status.load(Ordering::Acquire) {
            return None;
        }
        
        self.slots[next_idx].status.store(true, Ordering::Release);
        self.slots[next_idx].timestamp = Instant::now();
        self.next = (next_idx + 1) % self.slots.len();
        
        let slot = &self.slots[next_idx];
        
        Some(RecvPacket {
            status: &slot.status,
            packet: &slot.packet,
        })
    }
}


#[derive(Debug)]
#[repr(transparent)]
struct Socket {
    inner: UnsafeCell<Ring>,
}

impl Socket {
    fn new() -> Self {
        Socket {
            inner: UnsafeCell::new(Ring::new()),
        }
    }
    
    fn recv(&self) -> Option<RecvPacket> {
        unsafe { (*self.inner.get()).recv() }
    }
}


fn main() {
    let socket = Socket::new();
    
    let mut v: Vec<RecvPacket> = vec![];
    
    loop {
        match socket.recv() {
            Some(packet) => {
                v.push(packet);
            }
            None => {
                break;
            }
        }
    }
    println!("{:?}", v);
    v.clear();
    
    loop {
        match socket.recv() {
            Some(packet) => {
                v.push(packet);
            }
            None => {
                break;
            }
        }
    }
    println!("{:?}", v);
}
