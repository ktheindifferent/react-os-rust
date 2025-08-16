// ARP (Address Resolution Protocol) Implementation
use super::ethernet::{MacAddress, EthernetFrame, ETHERTYPE_ARP};
use super::ip::Ipv4Address;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

// ARP constants
const ARP_HARDWARE_ETHERNET: u16 = 1;
const ARP_PROTOCOL_IPV4: u16 = 0x0800;
const ARP_OPERATION_REQUEST: u16 = 1;
const ARP_OPERATION_REPLY: u16 = 2;

// ARP packet structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ArpPacket {
    pub hardware_type: u16,
    pub protocol_type: u16,
    pub hardware_len: u8,
    pub protocol_len: u8,
    pub operation: u16,
    pub sender_mac: MacAddress,
    pub sender_ip: Ipv4Address,
    pub target_mac: MacAddress,
    pub target_ip: Ipv4Address,
}

impl ArpPacket {
    pub fn new_request(sender_mac: MacAddress, sender_ip: Ipv4Address, target_ip: Ipv4Address) -> Self {
        Self {
            hardware_type: ARP_HARDWARE_ETHERNET.to_be(),
            protocol_type: ARP_PROTOCOL_IPV4.to_be(),
            hardware_len: 6,
            protocol_len: 4,
            operation: ARP_OPERATION_REQUEST.to_be(),
            sender_mac,
            sender_ip,
            target_mac: MacAddress::ZERO,
            target_ip,
        }
    }
    
    pub fn new_reply(
        sender_mac: MacAddress,
        sender_ip: Ipv4Address,
        target_mac: MacAddress,
        target_ip: Ipv4Address,
    ) -> Self {
        Self {
            hardware_type: ARP_HARDWARE_ETHERNET.to_be(),
            protocol_type: ARP_PROTOCOL_IPV4.to_be(),
            hardware_len: 6,
            protocol_len: 4,
            operation: ARP_OPERATION_REPLY.to_be(),
            sender_mac,
            sender_ip,
            target_mac,
            target_ip,
        }
    }
    
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 28 {
            return None;
        }
        
        let hardware_type = u16::from_be_bytes([data[0], data[1]]);
        let protocol_type = u16::from_be_bytes([data[2], data[3]]);
        let hardware_len = data[4];
        let protocol_len = data[5];
        let operation = u16::from_be_bytes([data[6], data[7]]);
        
        if hardware_type != ARP_HARDWARE_ETHERNET || 
           protocol_type != ARP_PROTOCOL_IPV4 ||
           hardware_len != 6 || 
           protocol_len != 4 {
            return None;
        }
        
        let sender_mac = MacAddress::from_bytes(&data[8..14])?;
        let sender_ip = Ipv4Address::from_bytes(&data[14..18])?;
        let target_mac = MacAddress::from_bytes(&data[18..24])?;
        let target_ip = Ipv4Address::from_bytes(&data[24..28])?;
        
        Some(Self {
            hardware_type: hardware_type.to_be(),
            protocol_type: protocol_type.to_be(),
            hardware_len,
            protocol_len,
            operation: operation.to_be(),
            sender_mac,
            sender_ip,
            target_mac,
            target_ip,
        })
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut packet = Vec::with_capacity(28);
        
        packet.extend_from_slice(&self.hardware_type.to_be_bytes());
        packet.extend_from_slice(&self.protocol_type.to_be_bytes());
        packet.push(self.hardware_len);
        packet.push(self.protocol_len);
        packet.extend_from_slice(&self.operation.to_be_bytes());
        packet.extend_from_slice(self.sender_mac.as_bytes());
        packet.extend_from_slice(self.sender_ip.as_bytes());
        packet.extend_from_slice(self.target_mac.as_bytes());
        packet.extend_from_slice(self.target_ip.as_bytes());
        
        packet
    }
    
    pub fn operation(&self) -> u16 {
        u16::from_be(self.operation)
    }
}

// ARP cache entry
#[derive(Debug, Clone)]
pub struct ArpEntry {
    pub mac_address: MacAddress,
    pub timestamp: u64,
    pub permanent: bool,
}

// ARP cache
pub struct ArpCache {
    entries: BTreeMap<Ipv4Address, ArpEntry>,
    pending: BTreeMap<Ipv4Address, Vec<Vec<u8>>>, // Packets waiting for ARP resolution
}

impl ArpCache {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            pending: BTreeMap::new(),
        }
    }
    
    pub fn insert(&mut self, ip: Ipv4Address, mac: MacAddress) {
        let entry = ArpEntry {
            mac_address: mac,
            timestamp: 0, // Would use actual timestamp
            permanent: false,
        };
        self.entries.insert(ip, entry);
        
        // Send any pending packets for this IP
        if let Some(packets) = self.pending.remove(&ip) {
            for packet in packets {
                // Send packet now that we have the MAC address
                crate::serial_println!("Sending pending packet to {}", ip);
            }
        }
    }
    
    pub fn lookup(&self, ip: &Ipv4Address) -> Option<MacAddress> {
        self.entries.get(ip).map(|entry| entry.mac_address)
    }
    
    pub fn add_pending(&mut self, ip: Ipv4Address, packet: Vec<u8>) {
        self.pending.entry(ip).or_insert_with(Vec::new).push(packet);
    }
    
    pub fn clear(&mut self) {
        self.entries.retain(|_, entry| entry.permanent);
        self.pending.clear();
    }
}

// Global ARP cache
lazy_static! {
    static ref ARP_CACHE: Mutex<ArpCache> = Mutex::new(ArpCache::new());
}

pub fn init() {
    // Add some static ARP entries if needed
    let mut cache = ARP_CACHE.lock();
    
    // Add gateway
    cache.insert(
        Ipv4Address::new(192, 168, 1, 1),
        MacAddress::new([0x00, 0x00, 0x00, 0x00, 0x00, 0x01]),
    );
    
    crate::serial_println!("ARP cache initialized");
}

// Process incoming ARP packet
pub fn process_arp_packet(data: &[u8]) {
    let packet = match ArpPacket::from_bytes(data) {
        Some(p) => p,
        None => {
            crate::serial_println!("Invalid ARP packet");
            return;
        }
    };
    
    // Update ARP cache with sender's information
    let mut cache = ARP_CACHE.lock();
    cache.insert(packet.sender_ip, packet.sender_mac);
    
    match packet.operation() {
        ARP_OPERATION_REQUEST => {
            crate::serial_println!("ARP request from {} for {}", 
                packet.sender_ip, packet.target_ip);
            
            // Check if the request is for our IP
            if let Some(our_ip) = get_our_ip() {
                if packet.target_ip == our_ip {
                    // Send ARP reply
                    send_arp_reply(packet.sender_mac, packet.sender_ip);
                }
            }
        }
        ARP_OPERATION_REPLY => {
            crate::serial_println!("ARP reply from {} ({})", 
                packet.sender_ip, packet.sender_mac);
        }
        _ => {
            crate::serial_println!("Unknown ARP operation: {}", packet.operation());
        }
    }
}

// Send ARP request
pub fn send_arp_request(target_ip: Ipv4Address) {
    let our_mac = get_our_mac();
    let our_ip = get_our_ip().unwrap_or(Ipv4Address::new(0, 0, 0, 0));
    
    let arp_packet = ArpPacket::new_request(our_mac, our_ip, target_ip);
    let frame = EthernetFrame::new(
        MacAddress::BROADCAST,
        our_mac,
        ETHERTYPE_ARP,
        arp_packet.to_bytes(),
    );
    
    // Send through network interface
    crate::serial_println!("Sending ARP request for {}", target_ip);
    // interface::send_frame(&frame);
}

// Send ARP reply
pub fn send_arp_reply(target_mac: MacAddress, target_ip: Ipv4Address) {
    let our_mac = get_our_mac();
    let our_ip = get_our_ip().unwrap_or(Ipv4Address::new(0, 0, 0, 0));
    
    let arp_packet = ArpPacket::new_reply(our_mac, our_ip, target_mac, target_ip);
    let frame = EthernetFrame::new(
        target_mac,
        our_mac,
        ETHERTYPE_ARP,
        arp_packet.to_bytes(),
    );
    
    crate::serial_println!("Sending ARP reply to {}", target_ip);
    // interface::send_frame(&frame);
}

// Resolve IP to MAC address
pub fn resolve(ip: Ipv4Address) -> Option<MacAddress> {
    let cache = ARP_CACHE.lock();
    
    if let Some(mac) = cache.lookup(&ip) {
        return Some(mac);
    }
    
    // Drop lock before sending request
    drop(cache);
    
    // Send ARP request and wait (in real implementation)
    send_arp_request(ip);
    
    // For now, return None (would wait for reply)
    None
}

// Helper functions (would be provided by network interface)
fn get_our_mac() -> MacAddress {
    MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]) // QEMU default
}

fn get_our_ip() -> Option<Ipv4Address> {
    Some(Ipv4Address::new(192, 168, 1, 100)) // Static IP for now
}