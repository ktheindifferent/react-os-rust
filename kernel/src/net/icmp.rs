// ICMP (Internet Control Message Protocol) Implementation
use super::ip::{IpPacket, Ipv4Address, IP_PROTO_ICMP};
use alloc::vec::Vec;

// ICMP message types
pub const ICMP_TYPE_ECHO_REPLY: u8 = 0;
pub const ICMP_TYPE_DEST_UNREACHABLE: u8 = 3;
pub const ICMP_TYPE_SOURCE_QUENCH: u8 = 4;
pub const ICMP_TYPE_REDIRECT: u8 = 5;
pub const ICMP_TYPE_ECHO_REQUEST: u8 = 8;
pub const ICMP_TYPE_TIME_EXCEEDED: u8 = 11;
pub const ICMP_TYPE_PARAMETER_PROBLEM: u8 = 12;
pub const ICMP_TYPE_TIMESTAMP_REQUEST: u8 = 13;
pub const ICMP_TYPE_TIMESTAMP_REPLY: u8 = 14;

// ICMP destination unreachable codes
pub const ICMP_CODE_NET_UNREACHABLE: u8 = 0;
pub const ICMP_CODE_HOST_UNREACHABLE: u8 = 1;
pub const ICMP_CODE_PROTOCOL_UNREACHABLE: u8 = 2;
pub const ICMP_CODE_PORT_UNREACHABLE: u8 = 3;
pub const ICMP_CODE_FRAGMENTATION_NEEDED: u8 = 4;

// ICMP Header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct IcmpHeader {
    pub typ: u8,
    pub code: u8,
    pub checksum: u16,
    pub rest: u32, // Contents depend on type/code
}

impl IcmpHeader {
    pub fn new(typ: u8, code: u8, rest: u32) -> Self {
        let mut header = Self {
            typ,
            code,
            checksum: 0,
            rest: rest.to_be(),
        };
        header.checksum = 0; // Will be calculated with payload
        header
    }
    
    pub fn echo_request(identifier: u16, sequence: u16) -> Self {
        let rest = ((identifier as u32) << 16) | (sequence as u32);
        Self::new(ICMP_TYPE_ECHO_REQUEST, 0, rest)
    }
    
    pub fn echo_reply(identifier: u16, sequence: u16) -> Self {
        let rest = ((identifier as u32) << 16) | (sequence as u32);
        Self::new(ICMP_TYPE_ECHO_REPLY, 0, rest)
    }
    
    pub fn get_identifier(&self) -> u16 {
        (u32::from_be(self.rest) >> 16) as u16
    }
    
    pub fn get_sequence(&self) -> u16 {
        u32::from_be(self.rest) as u16
    }
}

// ICMP Packet
pub struct IcmpPacket {
    pub header: IcmpHeader,
    pub data: Vec<u8>,
}

impl IcmpPacket {
    pub fn new(header: IcmpHeader, data: Vec<u8>) -> Self {
        let mut packet = Self { header, data };
        packet.update_checksum();
        packet
    }
    
    pub fn echo_request(identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        let header = IcmpHeader::echo_request(identifier, sequence);
        Self::new(header, data)
    }
    
    pub fn echo_reply(identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        let header = IcmpHeader::echo_reply(identifier, sequence);
        Self::new(header, data)
    }
    
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 8 {
            return Err("ICMP packet too small");
        }
        
        let header = unsafe {
            *(data.as_ptr() as *const IcmpHeader)
        };
        
        let payload = data[8..].to_vec();
        
        let packet = Self {
            header,
            data: payload,
        };
        
        // Verify checksum
        if !packet.verify_checksum(data) {
            return Err("Invalid ICMP checksum");
        }
        
        Ok(packet)
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut packet = Vec::new();
        
        // Add header
        packet.push(self.header.typ);
        packet.push(self.header.code);
        packet.extend_from_slice(&self.header.checksum.to_be_bytes());
        packet.extend_from_slice(&self.header.rest.to_be_bytes());
        
        // Add data
        packet.extend_from_slice(&self.data);
        
        packet
    }
    
    fn calculate_checksum(&self) -> u16 {
        let mut sum: u32 = 0;
        
        // Add header fields (except checksum)
        sum += (self.header.typ as u32) << 8 | (self.header.code as u32);
        sum += (self.header.rest >> 16) as u32;
        sum += (self.header.rest & 0xFFFF) as u32;
        
        // Add data
        let mut i = 0;
        while i < self.data.len() - 1 {
            sum += ((self.data[i] as u32) << 8) | (self.data[i + 1] as u32);
            i += 2;
        }
        
        // Add remaining byte if odd length
        if i < self.data.len() {
            sum += (self.data[i] as u32) << 8;
        }
        
        // Add carry bits
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        // One's complement
        !sum as u16
    }
    
    fn update_checksum(&mut self) {
        self.header.checksum = self.calculate_checksum().to_be();
    }
    
    fn verify_checksum(&self, raw_data: &[u8]) -> bool {
        let mut sum: u32 = 0;
        
        // Sum all 16-bit words
        let mut i = 0;
        while i < raw_data.len() - 1 {
            sum += ((raw_data[i] as u32) << 8) | (raw_data[i + 1] as u32);
            i += 2;
        }
        
        // Add remaining byte if odd length
        if i < raw_data.len() {
            sum += (raw_data[i] as u32) << 8;
        }
        
        // Add carry bits
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        // Should be 0xFFFF if valid
        sum == 0xFFFF
    }
}

// Process incoming ICMP packet
pub fn process_icmp_packet(ip_packet: &IpPacket) {
    let icmp_packet = match IcmpPacket::from_bytes(&ip_packet.payload) {
        Ok(p) => p,
        Err(e) => {
            crate::serial_println!("Invalid ICMP packet: {}", e);
            return;
        }
    };
    
    match icmp_packet.header.typ {
        ICMP_TYPE_ECHO_REQUEST => {
            crate::serial_println!("ICMP Echo Request from {} (id={}, seq={})",
                ip_packet.header.src_addr,
                icmp_packet.header.get_identifier(),
                icmp_packet.header.get_sequence()
            );
            
            // Send echo reply
            send_echo_reply(
                ip_packet.header.src_addr,
                icmp_packet.header.get_identifier(),
                icmp_packet.header.get_sequence(),
                icmp_packet.data,
            );
        }
        ICMP_TYPE_ECHO_REPLY => {
            crate::serial_println!("ICMP Echo Reply from {} (id={}, seq={})",
                ip_packet.header.src_addr,
                icmp_packet.header.get_identifier(),
                icmp_packet.header.get_sequence()
            );
        }
        ICMP_TYPE_DEST_UNREACHABLE => {
            crate::serial_println!("ICMP Destination Unreachable from {} (code={})",
                ip_packet.header.src_addr,
                icmp_packet.header.code
            );
        }
        ICMP_TYPE_TIME_EXCEEDED => {
            crate::serial_println!("ICMP Time Exceeded from {}",
                ip_packet.header.src_addr
            );
        }
        _ => {
            crate::serial_println!("ICMP type {} from {}",
                icmp_packet.header.typ,
                ip_packet.header.src_addr
            );
        }
    }
}

// Send ICMP echo reply
fn send_echo_reply(dst_addr: Ipv4Address, identifier: u16, sequence: u16, data: Vec<u8>) {
    let icmp_packet = IcmpPacket::echo_reply(identifier, sequence, data);
    let our_ip = Ipv4Address::new(192, 168, 1, 100); // Static IP for now
    
    let ip_packet = IpPacket::new(
        our_ip,
        dst_addr,
        IP_PROTO_ICMP,
        icmp_packet.to_bytes(),
    );
    
    if let Err(e) = super::ip::send_ip_packet(ip_packet) {
        crate::serial_println!("Failed to send ICMP reply: {}", e);
    }
}

// Send ICMP echo request (ping)
pub fn send_ping(dst_addr: Ipv4Address, identifier: u16, sequence: u16) {
    let data = b"RustOS ping!".to_vec();
    let icmp_packet = IcmpPacket::echo_request(identifier, sequence, data);
    let our_ip = Ipv4Address::new(192, 168, 1, 100);
    
    let ip_packet = IpPacket::new(
        our_ip,
        dst_addr,
        IP_PROTO_ICMP,
        icmp_packet.to_bytes(),
    );
    
    crate::serial_println!("Sending ping to {} (id={}, seq={})",
        dst_addr, identifier, sequence
    );
    
    if let Err(e) = super::ip::send_ip_packet(ip_packet) {
        crate::serial_println!("Failed to send ping: {}", e);
    }
}

// Send ICMP destination unreachable
pub fn send_dest_unreachable(
    dst_addr: Ipv4Address,
    code: u8,
    original_packet: &[u8],
) {
    let mut data = Vec::new();
    // Include first 64 bits of original packet
    data.extend_from_slice(&original_packet[..original_packet.len().min(8)]);
    
    let header = IcmpHeader::new(ICMP_TYPE_DEST_UNREACHABLE, code, 0);
    let icmp_packet = IcmpPacket::new(header, data);
    let our_ip = Ipv4Address::new(192, 168, 1, 100);
    
    let ip_packet = IpPacket::new(
        our_ip,
        dst_addr,
        IP_PROTO_ICMP,
        icmp_packet.to_bytes(),
    );
    
    if let Err(e) = super::ip::send_ip_packet(ip_packet) {
        crate::serial_println!("Failed to send ICMP unreachable: {}", e);
    }
}