// UDP (User Datagram Protocol) Implementation
use super::ip::{IpPacket, Ipv4Address, IP_PROTO_UDP};
use alloc::vec::Vec;
use alloc::collections::{BTreeMap, VecDeque};
use spin::Mutex;
use lazy_static::lazy_static;

// UDP Header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct UdpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
    pub checksum: u16,
}

impl UdpHeader {
    pub fn new(src_port: u16, dst_port: u16, payload_len: usize) -> Self {
        Self {
            src_port: src_port.to_be(),
            dst_port: dst_port.to_be(),
            length: ((8 + payload_len) as u16).to_be(),
            checksum: 0, // Optional for IPv4
        }
    }
    
    pub fn src_port(&self) -> u16 {
        u16::from_be(self.src_port)
    }
    
    pub fn dst_port(&self) -> u16 {
        u16::from_be(self.dst_port)
    }
    
    pub fn length(&self) -> u16 {
        u16::from_be(self.length)
    }
}

// UDP Packet
pub struct UdpPacket {
    pub header: UdpHeader,
    pub data: Vec<u8>,
}

impl UdpPacket {
    pub fn new(src_port: u16, dst_port: u16, data: Vec<u8>) -> Self {
        let header = UdpHeader::new(src_port, dst_port, data.len());
        Self { header, data }
    }
    
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 8 {
            return Err("UDP packet too small");
        }
        
        let header = unsafe {
            *(data.as_ptr() as *const UdpHeader)
        };
        
        let payload_len = header.length() as usize - 8;
        if data.len() < 8 + payload_len {
            return Err("UDP packet truncated");
        }
        
        let payload = data[8..8 + payload_len].to_vec();
        
        Ok(Self {
            header,
            data: payload,
        })
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut packet = Vec::new();
        
        // Add header
        packet.extend_from_slice(&self.header.src_port.to_be_bytes());
        packet.extend_from_slice(&self.header.dst_port.to_be_bytes());
        packet.extend_from_slice(&self.header.length.to_be_bytes());
        packet.extend_from_slice(&self.header.checksum.to_be_bytes());
        
        // Add data
        packet.extend_from_slice(&self.data);
        
        packet
    }
    
    // Calculate UDP checksum (optional for IPv4)
    pub fn calculate_checksum(&self, src_ip: Ipv4Address, dst_ip: Ipv4Address) -> u16 {
        let mut sum: u32 = 0;
        
        // Pseudo header
        for byte in src_ip.as_bytes() {
            sum += (*byte as u32) << 8;
        }
        for byte in dst_ip.as_bytes() {
            sum += (*byte as u32) << 8;
        }
        sum += IP_PROTO_UDP as u32;
        sum += self.header.length() as u32;
        
        // UDP header
        sum += self.header.src_port() as u32;
        sum += self.header.dst_port() as u32;
        sum += self.header.length() as u32;
        // Skip checksum field
        
        // UDP data
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
        let checksum = !sum as u16;
        if checksum == 0 {
            0xFFFF // 0 means no checksum, so use 0xFFFF
        } else {
            checksum
        }
    }
}

// UDP Socket
pub struct UdpSocket {
    pub local_port: u16,
    pub remote_addr: Option<(Ipv4Address, u16)>,
    pub receive_buffer: VecDeque<(Ipv4Address, u16, Vec<u8>)>,
}

impl UdpSocket {
    pub fn new(local_port: u16) -> Self {
        Self {
            local_port,
            remote_addr: None,
            receive_buffer: VecDeque::new(),
        }
    }
    
    pub fn bind_remote(&mut self, addr: Ipv4Address, port: u16) {
        self.remote_addr = Some((addr, port));
    }
    
    
    pub fn send(&self, data: Vec<u8>) -> Result<(), &'static str> {
        if let Some((addr, port)) = self.remote_addr {
            self.send_to(data, addr, port)
        } else {
            Err("No remote address bound")
        }
    }
    
    pub fn recv(&mut self) -> Option<(Ipv4Address, u16, Vec<u8>)> {
        self.receive_buffer.pop_front()
    }
    
    pub fn recv_from(&mut self) -> Result<(Vec<u8>, super::socket::SocketAddr), &'static str> {
        if let Some((addr, port, data)) = self.receive_buffer.pop_front() {
            Ok((data, super::socket::SocketAddr::new(addr, port)))
        } else {
            Err("No data available")
        }
    }
    
    pub fn send_to(&self, data: &[u8], addr: Ipv4Address, port: u16) -> Result<(), &'static str> {
        let udp_packet = UdpPacket::new(self.local_port, port, data.to_vec());
        let our_ip = Ipv4Address::new(192, 168, 1, 100);
        
        let ip_packet = IpPacket::new(
            our_ip,
            addr,
            IP_PROTO_UDP,
            udp_packet.to_bytes(),
        );
        
        super::ip::send_ip_packet(ip_packet)?;
        Ok(())
    }
}

// UDP socket table
lazy_static! {
    static ref UDP_SOCKETS: Mutex<BTreeMap<u16, UdpSocket>> = Mutex::new(BTreeMap::new());
}

// Process incoming UDP packet
pub fn process_udp_packet(ip_packet: &IpPacket) {
    let udp_packet = match UdpPacket::from_bytes(&ip_packet.payload) {
        Ok(p) => p,
        Err(e) => {
            crate::serial_println!("Invalid UDP packet: {}", e);
            return;
        }
    };
    
    crate::serial_println!("UDP packet from {}:{} to port {}",
        ip_packet.header.src_addr,
        udp_packet.header.src_port(),
        udp_packet.header.dst_port()
    );
    
    // Find socket listening on this port
    let mut sockets = UDP_SOCKETS.lock();
    if let Some(socket) = sockets.get_mut(&udp_packet.header.dst_port()) {
        // Add to socket's receive buffer
        socket.receive_buffer.push_back((
            ip_packet.header.src_addr,
            udp_packet.header.src_port(),
            udp_packet.data,
        ));
        
        crate::serial_println!("Delivered to UDP socket on port {}",
            udp_packet.header.dst_port()
        );
    } else {
        // Send ICMP port unreachable
        super::icmp::send_dest_unreachable(
            ip_packet.header.src_addr,
            super::icmp::ICMP_CODE_PORT_UNREACHABLE,
            &ip_packet.to_bytes(),
        );
    }
}

// UDP socket API functions
pub fn bind(port: u16) -> Result<(), &'static str> {
    let mut sockets = UDP_SOCKETS.lock();
    
    if sockets.contains_key(&port) {
        return Err("Port already in use");
    }
    
    sockets.insert(port, UdpSocket::new(port));
    crate::serial_println!("UDP socket bound to port {}", port);
    Ok(())
}

pub fn unbind(port: u16) -> Result<(), &'static str> {
    let mut sockets = UDP_SOCKETS.lock();
    
    if sockets.remove(&port).is_some() {
        crate::serial_println!("UDP socket on port {} closed", port);
        Ok(())
    } else {
        Err("Socket not found")
    }
}

pub fn send_to(
    local_port: u16,
    data: Vec<u8>,
    dst_addr: Ipv4Address,
    dst_port: u16,
) -> Result<(), &'static str> {
    let sockets = UDP_SOCKETS.lock();
    
    if let Some(socket) = sockets.get(&local_port) {
        socket.send_to(data, dst_addr, dst_port)
    } else {
        Err("Socket not found")
    }
}

pub fn recv_from(local_port: u16) -> Result<Option<(Ipv4Address, u16, Vec<u8>)>, &'static str> {
    let mut sockets = UDP_SOCKETS.lock();
    
    if let Some(socket) = sockets.get_mut(&local_port) {
        Ok(socket.recv())
    } else {
        Err("Socket not found")
    }
}

// Well-known UDP ports
pub const PORT_DNS: u16 = 53;
pub const PORT_DHCP_SERVER: u16 = 67;
pub const PORT_DHCP_CLIENT: u16 = 68;
pub const PORT_TFTP: u16 = 69;
pub const PORT_NTP: u16 = 123;
pub const PORT_NETBIOS_NS: u16 = 137;
pub const PORT_NETBIOS_DGM: u16 = 138;
pub const PORT_SNMP: u16 = 161;