// Ethernet Layer Implementation
use alloc::vec::Vec;
use core::fmt;

// Ethernet constants
pub const ETH_HEADER_SIZE: usize = 14;
pub const ETH_MIN_FRAME_SIZE: usize = 64;
pub const ETH_MAX_FRAME_SIZE: usize = 1518;
pub const ETH_MTU: usize = 1500;

// EtherType values
pub const ETHERTYPE_IPV4: u16 = 0x0800;
pub const ETHERTYPE_ARP: u16 = 0x0806;
pub const ETHERTYPE_IPV6: u16 = 0x86DD;
pub const ETHERTYPE_VLAN: u16 = 0x8100;

// MAC Address
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    pub const ZERO: MacAddress = MacAddress([0; 6]);
    pub const BROADCAST: MacAddress = MacAddress([0xFF; 6]);
    
    pub fn new(bytes: [u8; 6]) -> Self {
        MacAddress(bytes)
    }
    
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 6 {
            return None;
        }
        let mut addr = [0u8; 6];
        addr.copy_from_slice(&bytes[0..6]);
        Some(MacAddress(addr))
    }
    
    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }
    
    pub fn is_broadcast(&self) -> bool {
        *self == Self::BROADCAST
    }
    
    pub fn is_multicast(&self) -> bool {
        self.0[0] & 0x01 != 0
    }
    
    pub fn is_unicast(&self) -> bool {
        !self.is_multicast()
    }
    
    pub fn is_local(&self) -> bool {
        self.0[0] & 0x02 != 0
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2],
            self.0[3], self.0[4], self.0[5])
    }
}

impl From<[u8; 6]> for MacAddress {
    fn from(bytes: [u8; 6]) -> Self {
        MacAddress(bytes)
    }
}

// Ethernet Frame Header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct EthernetHeader {
    pub dest_mac: MacAddress,
    pub src_mac: MacAddress,
    pub ethertype: u16,
}

impl EthernetHeader {
    pub fn new(dest_mac: MacAddress, src_mac: MacAddress, ethertype: u16) -> Self {
        Self {
            dest_mac,
            src_mac,
            ethertype: ethertype.to_be(),
        }
    }
    
    pub fn ethertype(&self) -> u16 {
        u16::from_be(self.ethertype)
    }
}

// Ethernet Frame
pub struct EthernetFrame {
    pub header: EthernetHeader,
    pub payload: Vec<u8>,
}

impl EthernetFrame {
    pub fn new(dest_mac: MacAddress, src_mac: MacAddress, ethertype: u16, payload: Vec<u8>) -> Self {
        Self {
            header: EthernetHeader::new(dest_mac, src_mac, ethertype),
            payload,
        }
    }
    
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < ETH_HEADER_SIZE {
            return Err("Frame too small");
        }
        
        let dest_mac = MacAddress::from_bytes(&data[0..6])
            .ok_or("Invalid destination MAC")?;
        let src_mac = MacAddress::from_bytes(&data[6..12])
            .ok_or("Invalid source MAC")?;
        let ethertype = u16::from_be_bytes([data[12], data[13]]);
        
        let header = EthernetHeader {
            dest_mac,
            src_mac,
            ethertype,
        };
        
        let payload = data[ETH_HEADER_SIZE..].to_vec();
        
        Ok(Self { header, payload })
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut frame = Vec::new();
        
        // Add header
        frame.extend_from_slice(self.header.dest_mac.as_bytes());
        frame.extend_from_slice(self.header.src_mac.as_bytes());
        frame.extend_from_slice(&self.header.ethertype.to_be_bytes());
        
        // Add payload
        frame.extend_from_slice(&self.payload);
        
        // Pad to minimum frame size if needed
        while frame.len() < ETH_MIN_FRAME_SIZE {
            frame.push(0);
        }
        
        frame
    }
    
    pub fn len(&self) -> usize {
        ETH_HEADER_SIZE + self.payload.len()
    }
}

// Ethernet controller trait
pub trait EthernetController {
    fn get_mac_address(&self) -> MacAddress;
    fn send_frame(&mut self, frame: &EthernetFrame) -> Result<(), &'static str>;
    fn receive_frame(&mut self) -> Option<EthernetFrame>;
    fn set_promiscuous(&mut self, enabled: bool);
    fn get_link_status(&self) -> bool;
}

// Get our MAC address
pub fn get_mac_address() -> MacAddress {
    // For now, return a static MAC address
    // In a real implementation, this would query the network interface
    MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]) // QEMU default
}

// Process incoming Ethernet frame
pub fn process_frame(frame: EthernetFrame) {
    use super::arp;
    use super::ip;
    
    match frame.header.ethertype() {
        ETHERTYPE_ARP => {
            crate::serial_println!("Received ARP frame");
            arp::process_arp_packet(&frame.payload);
        }
        ETHERTYPE_IPV4 => {
            crate::serial_println!("Received IPv4 frame");
            ip::process_ip_packet(&frame.payload);
        }
        ETHERTYPE_IPV6 => {
            crate::serial_println!("Received IPv6 frame (not supported)");
        }
        _ => {
            crate::serial_println!("Unknown EtherType: 0x{:04x}", frame.header.ethertype());
        }
    }
    
    super::update_stats_received(frame.len());
}