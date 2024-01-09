use std::cell::UnsafeCell;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;


/// Structure which emulates a received packet in Nethuns.
#[derive(Debug)]
struct RecvPacket<'a> {
    idx: usize,
    status: &'a AtomicBool,
    packet: &'a [u8],
}

impl Display for RecvPacket<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "idx: {:?}, status: {:?}, packet: {:?}",
            self.idx,
            self.status.load(Ordering::Acquire),
            self.packet
        )
    }
}

impl Drop for RecvPacket<'_> {
    fn drop(&mut self) {
        println!("drop packet {}", self.idx);
        // Set the slot as FREE, since the corresponding RecvPacket
        // will be destroyed
        self.status.store(false, Ordering::Release);
    }
}


/// Structure which emulates a Nethuns ring slot.
#[derive(Debug)]
struct RingSlot {
    /// Slot status: false if free, true otherwise
    status: AtomicBool,
    /// Packet buffer
    packet: Vec<u8>,
    /// Timestamp when the packet was received
    timestamp: Instant,
}


/// Structure which emulates a Nethuns ring in RX mode.
/// 
/// The ring is implemented as a vector of ring slots
/// and a index pointing to the next available slot
/// (slot is free <==> status is false).
/// 
/// The `next` index wrap around when reaching the end
/// of the vector, in order to simulate a circular queue.
#[derive(Debug)]
struct Ring {
    slots: Vec<RingSlot>,
    next: usize,
}

impl Ring {
    /// Create a new ring with 5 ring slots.
    /// Each slot is initialized with a new packet to be received.
    fn new() -> Self {
        let mut slots: Vec<RingSlot> = Vec::with_capacity(5);
        for i in 0..slots.capacity() {
            slots.push(RingSlot {
                status: AtomicBool::new(false),
                packet: vec![i as u8; 5],
                timestamp: Instant::now(),
            })
        }
        
        Ring { slots, next: 0 }
    }
    
    /// Receive a packet
    fn recv(&mut self) -> Option<RecvPacket> {
        let next_idx = self.next;
        
        // Check if `next` slot is free, i.e. no RecvPacket object
        // is currently pointing to its internal buffer
        if self.slots[next_idx].status.load(Ordering::Acquire) {
            return None;
        }
        
        // Set the slot as IN-USE
        self.slots[next_idx].status.store(true, Ordering::Release);
        // Mutate the ring slot to test if it's safe to mutate
        // the socket structure while RecvPacket objects exist
        self.slots[next_idx].timestamp = Instant::now();
        // Circularly increase the `next` index
        self.next = (next_idx + 1) % self.slots.len();
        
        // Mutate the packet buffer to test if it's safe to mutate
        self.slots[next_idx].packet.clear();
        for i in 0..self.slots[next_idx].packet.capacity() {
            self.slots[next_idx].packet.push(i as u8);
        }
        
        let slot = &self.slots[next_idx];
        
        // Return the received packet
        Some(RecvPacket {
            idx: next_idx,
            status: &slot.status,
            packet: &slot.packet,
        })
    }
}


/// Socket which emulates the behavior of a 
/// Nethuns socket in RX mode.
/// 
/// The socket wraps a unique inner ring, 
/// which is responsible for receiving packets.
#[derive(Debug)]
#[repr(transparent)]
struct Socket {
    /// rx ring
    inner: UnsafeCell<Ring>,
}

impl Socket {
    /// Create a new socket
    fn new() -> Self {
        Socket {
            inner: UnsafeCell::new(Ring::new()),
        }
    }
    
    /// Receive a packet
    fn recv(&self) -> Option<RecvPacket> {
        // Call `recv` on the inner ring
        // by exploiting the "inner mutability pattern"
        unsafe { (*self.inner.get()).recv() }
    }
}


fn main() {
    let socket = Socket::new();
    
    let mut v: Vec<RecvPacket> = vec![];
    
    // Receive as many packets as possible,
    // so that every slot is set as NOT FREE
    while let Some(packet) = socket.recv() {
        v.push(packet);
    }
    for pkt in &v {
        println!("{}", pkt);
    }
    
    // Drop all received packets
    v.clear();
    
    // Receive as many packets as possible,
    // so that every slot is set as NOT FREE.
    // If the Drop trait was correctly implemented,
    // this loop should have the same output as the previous.
    while let Some(packet) = socket.recv() {
        v.push(packet);
    }
    for pkt in &v {
        println!("{}", pkt);
    }
}
