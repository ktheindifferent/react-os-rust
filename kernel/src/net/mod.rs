// Network Stack Implementation
pub mod ethernet;
pub mod arp;
pub mod ip;
pub mod icmp;
pub mod udp;
pub mod tcp;
pub mod socket;
pub mod dhcp;
pub mod dns;
pub mod interface;
pub mod buffer;
pub mod wireless;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::VecDeque;
use spin::Mutex;
use lazy_static::lazy_static;

// Re-export commonly used types
pub use ethernet::{MacAddress, EthernetFrame};
pub use ip::{IpAddress, Ipv4Address, IpPacket};
pub use socket::{Socket, SocketAddr};

// Network stack initialization
pub fn init() -> Result<(), &'static str> {
    crate::serial_println!("Initializing network stack...");
    
    // Initialize network interfaces
    interface::init();
    
    // Initialize ARP cache
    arp::init();
    
    // Initialize socket layer
    socket::init();
    
    // Start network processing thread (when threading is available)
    // For now, we'll process packets in interrupt context
    
    crate::serial_println!("Network stack initialized");
    Ok(())
}

// Common network types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Ethernet,
    Arp,
    Ipv4,
    Ipv6,
    Icmp,
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkStats {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub errors: u64,
    pub dropped: u64,
}

impl NetworkStats {
    pub fn new() -> Self {
        Self {
            packets_sent: 0,
            packets_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            errors: 0,
            dropped: 0,
        }
    }
}

// Global network statistics
lazy_static! {
    pub static ref NETWORK_STATS: Mutex<NetworkStats> = Mutex::new(NetworkStats::new());
}

// Update statistics
pub fn update_stats_sent(bytes: usize) {
    let mut stats = NETWORK_STATS.lock();
    stats.packets_sent += 1;
    stats.bytes_sent += bytes as u64;
}

pub fn update_stats_received(bytes: usize) {
    let mut stats = NETWORK_STATS.lock();
    stats.packets_received += 1;
    stats.bytes_received += bytes as u64;
}

pub fn update_stats_error() {
    let mut stats = NETWORK_STATS.lock();
    stats.errors += 1;
}

pub fn update_stats_dropped() {
    let mut stats = NETWORK_STATS.lock();
    stats.dropped += 1;
}

// Checksum calculation for network protocols
pub fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    
    // Sum 16-bit words
    while i < data.len() - 1 {
        sum += ((data[i] as u32) << 8) | (data[i + 1] as u32);
        i += 2;
    }
    
    // Add remaining byte if odd length
    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }
    
    // Add carry bits
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    
    // One's complement
    !sum as u16
}