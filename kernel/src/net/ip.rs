// IP (Internet Protocol) Layer Implementation
use alloc::vec::Vec;
use core::fmt;

// IP protocol numbers
pub const IP_PROTO_ICMP: u8 = 1;
pub const IP_PROTO_TCP: u8 = 6;
pub const IP_PROTO_UDP: u8 = 17;

// IP version constants
pub const IPV4_VERSION: u8 = 4;
pub const IPV4_HEADER_MIN_SIZE: usize = 20;
pub const IPV4_TTL_DEFAULT: u8 = 64;

// IP Address trait
pub trait IpAddress: Copy + Clone + PartialEq + Eq {
    fn is_loopback(&self) -> bool;
    fn is_multicast(&self) -> bool;
    fn is_broadcast(&self) -> bool;
}

// IPv4 Address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ipv4Address([u8; 4]);

impl Ipv4Address {
    pub const UNSPECIFIED: Ipv4Address = Ipv4Address([0; 4]);
    pub const LOOPBACK: Ipv4Address = Ipv4Address([127, 0, 0, 1]);
    pub const BROADCAST: Ipv4Address = Ipv4Address([255, 255, 255, 255]);
    
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Ipv4Address([a, b, c, d])
    }
    
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 4 {
            return None;
        }
        let mut addr = [0u8; 4];
        addr.copy_from_slice(&bytes[0..4]);
        Some(Ipv4Address(addr))
    }
    
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }
    
    pub fn to_u32(&self) -> u32 {
        u32::from_be_bytes(self.0)
    }
    
    pub fn from_u32(value: u32) -> Self {
        Ipv4Address(value.to_be_bytes())
    }
    
    pub fn octets(&self) -> [u8; 4] {
        self.0
    }
    
    pub fn is_private(&self) -> bool {
        match self.0[0] {
            10 => true,                                    // 10.0.0.0/8
            172 => self.0[1] >= 16 && self.0[1] <= 31,    // 172.16.0.0/12
            192 => self.0[1] == 168,                       // 192.168.0.0/16
            _ => false,
        }
    }
}

impl IpAddress for Ipv4Address {
    fn is_loopback(&self) -> bool {
        self.0[0] == 127
    }
    
    fn is_multicast(&self) -> bool {
        self.0[0] >= 224 && self.0[0] <= 239
    }
    
    fn is_broadcast(&self) -> bool {
        *self == Self::BROADCAST
    }
}

impl fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

// IPv4 Header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ipv4Header {
    pub version_ihl: u8,        // Version (4 bits) + IHL (4 bits)
    pub dscp_ecn: u8,           // DSCP (6 bits) + ECN (2 bits)
    pub total_length: u16,      // Total length in bytes
    pub identification: u16,    // Identification for fragments
    pub flags_fragment: u16,    // Flags (3 bits) + Fragment offset (13 bits)
    pub ttl: u8,               // Time to live
    pub protocol: u8,          // Protocol number
    pub checksum: u16,         // Header checksum
    pub src_addr: Ipv4Address, // Source IP address
    pub dst_addr: Ipv4Address, // Destination IP address
}

impl Ipv4Header {
    pub fn new(
        src_addr: Ipv4Address,
        dst_addr: Ipv4Address,
        protocol: u8,
        payload_len: usize,
    ) -> Self {
        let mut header = Self {
            version_ihl: (IPV4_VERSION << 4) | 5, // Version 4, IHL 5 (20 bytes)
            dscp_ecn: 0,
            total_length: ((IPV4_HEADER_MIN_SIZE + payload_len) as u16).to_be(),
            identification: 0,
            flags_fragment: 0x4000u16.to_be(), // Don't fragment flag
            ttl: IPV4_TTL_DEFAULT,
            protocol,
            checksum: 0,
            src_addr,
            dst_addr,
        };
        
        header.checksum = header.calculate_checksum();
        header
    }
    
    pub fn version(&self) -> u8 {
        self.version_ihl >> 4
    }
    
    pub fn ihl(&self) -> u8 {
        self.version_ihl & 0x0F
    }
    
    pub fn header_len(&self) -> usize {
        (self.ihl() as usize) * 4
    }
    
    pub fn total_length(&self) -> u16 {
        u16::from_be(self.total_length)
    }
    
    pub fn payload_length(&self) -> usize {
        (self.total_length() as usize) - self.header_len()
    }
    
    pub fn dont_fragment(&self) -> bool {
        u16::from_be(self.flags_fragment) & 0x4000 != 0
    }
    
    pub fn more_fragments(&self) -> bool {
        u16::from_be(self.flags_fragment) & 0x2000 != 0
    }
    
    pub fn fragment_offset(&self) -> u16 {
        (u16::from_be(self.flags_fragment) & 0x1FFF) * 8
    }
    
    pub fn calculate_checksum(&self) -> u16 {
        let mut sum: u32 = 0;
        let header_bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                self.header_len()
            )
        };
        
        // Sum 16-bit words, skipping checksum field
        for i in (0..self.header_len()).step_by(2) {
            if i == 10 { // Skip checksum field at offset 10-11
                continue;
            }
            let word = if i + 1 < self.header_len() {
                ((header_bytes[i] as u32) << 8) | (header_bytes[i + 1] as u32)
            } else {
                (header_bytes[i] as u32) << 8
            };
            sum += word;
        }
        
        // Add carry bits
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        // One's complement
        (!sum as u16).to_be()
    }
    
    pub fn verify_checksum(&self) -> bool {
        let mut sum: u32 = 0;
        let header_bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                self.header_len()
            )
        };
        
        // Sum all 16-bit words including checksum
        for i in (0..self.header_len()).step_by(2) {
            let word = if i + 1 < self.header_len() {
                ((header_bytes[i] as u32) << 8) | (header_bytes[i + 1] as u32)
            } else {
                (header_bytes[i] as u32) << 8
            };
            sum += word;
        }
        
        // Add carry bits
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        // Should be 0xFFFF if valid
        sum == 0xFFFF
    }
}

// IP Packet
pub struct IpPacket {
    pub header: Ipv4Header,
    pub payload: Vec<u8>,
}

impl IpPacket {
    pub fn new(
        src_addr: Ipv4Address,
        dst_addr: Ipv4Address,
        protocol: u8,
        payload: Vec<u8>,
    ) -> Self {
        let header = Ipv4Header::new(src_addr, dst_addr, protocol, payload.len());
        Self { header, payload }
    }
    
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < IPV4_HEADER_MIN_SIZE {
            return Err("Packet too small");
        }
        
        // Parse header
        let header = unsafe {
            *(data.as_ptr() as *const Ipv4Header)
        };
        
        // Verify version
        if header.version() != IPV4_VERSION {
            return Err("Not IPv4");
        }
        
        // Verify header checksum
        if !header.verify_checksum() {
            return Err("Invalid checksum");
        }
        
        // Extract payload
        let header_len = header.header_len();
        if data.len() < header_len {
            return Err("Invalid header length");
        }
        
        let total_len = header.total_length() as usize;
        if data.len() < total_len {
            return Err("Packet truncated");
        }
        
        let payload = data[header_len..total_len].to_vec();
        
        Ok(Self { header, payload })
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut packet = Vec::new();
        let header_bytes = unsafe {
            core::slice::from_raw_parts(
                &self.header as *const _ as *const u8,
                self.header.header_len()
            )
        };
        
        packet.extend_from_slice(header_bytes);
        packet.extend_from_slice(&self.payload);
        packet
    }
}

// Process incoming IP packet
pub fn process_ip_packet(data: &[u8]) {
    let packet = match IpPacket::from_bytes(data) {
        Ok(p) => p,
        Err(e) => {
            crate::serial_println!("Invalid IP packet: {}", e);
            super::update_stats_error();
            return;
        }
    };
    
    crate::serial_println!("IP packet: {} -> {} (proto {})",
        packet.header.src_addr,
        packet.header.dst_addr,
        packet.header.protocol
    );
    
    // Check if packet is for us
    if !is_our_ip(&packet.header.dst_addr) && !packet.header.dst_addr.is_broadcast() {
        // Forward packet if we're a router (not implemented)
        crate::serial_println!("Packet not for us");
        return;
    }
    
    // Process based on protocol
    match packet.header.protocol {
        IP_PROTO_ICMP => {
            super::icmp::process_icmp_packet(&packet);
        }
        IP_PROTO_TCP => {
            super::tcp::process_tcp_packet(&packet);
        }
        IP_PROTO_UDP => {
            super::udp::process_udp_packet(&packet);
        }
        _ => {
            crate::serial_println!("Unknown IP protocol: {}", packet.header.protocol);
        }
    }
}

// Send IP packet
pub fn send_ip_packet(packet: IpPacket) -> Result<(), &'static str> {
    use super::ethernet::{EthernetFrame, ETHERTYPE_IPV4};
    use super::arp;
    
    // Resolve destination MAC address
    let dst_mac = if packet.header.dst_addr.is_broadcast() {
        super::ethernet::MacAddress::BROADCAST
    } else if let Some(mac) = arp::resolve(packet.header.dst_addr) {
        mac
    } else {
        return Err("ARP resolution failed");
    };
    
    let src_mac = get_our_mac();
    
    // Create Ethernet frame
    let frame = EthernetFrame::new(
        dst_mac,
        src_mac,
        ETHERTYPE_IPV4,
        packet.to_bytes(),
    );
    
    // Send through network interface
    crate::serial_println!("Sending IP packet to {}", packet.header.dst_addr);
    super::update_stats_sent(frame.len());
    
    // interface::send_frame(&frame)?;
    Ok(())
}

// Helper functions
fn is_our_ip(ip: &Ipv4Address) -> bool {
    // Check against our configured IPs
    *ip == Ipv4Address::new(192, 168, 1, 100) || ip.is_loopback()
}

fn get_our_mac() -> super::ethernet::MacAddress {
    super::ethernet::MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56])
}