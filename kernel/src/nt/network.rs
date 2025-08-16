use super::NtStatus;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::format;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU32, AtomicU16, Ordering};

// Ethernet frame structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct EthernetFrame {
    pub dest_mac: [u8; 6],
    pub src_mac: [u8; 6],
    pub ether_type: u16,
}

impl EthernetFrame {
    pub const TYPE_IPV4: u16 = 0x0800;
    pub const TYPE_ARP: u16 = 0x0806;
    pub const TYPE_IPV6: u16 = 0x86DD;
    
    pub fn new(dest_mac: [u8; 6], src_mac: [u8; 6], ether_type: u16) -> Self {
        Self {
            dest_mac,
            src_mac,
            ether_type: ether_type.to_be(),
        }
    }
}

// IPv4 header structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ipv4Header {
    pub version_ihl: u8,     // Version (4 bits) + IHL (4 bits)
    pub dscp_ecn: u8,        // DSCP (6 bits) + ECN (2 bits)
    pub total_length: u16,
    pub identification: u16,
    pub flags_fragment: u16, // Flags (3 bits) + Fragment offset (13 bits)
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: u16,
    pub src_addr: u32,
    pub dest_addr: u32,
}

impl Ipv4Header {
    pub const PROTOCOL_ICMP: u8 = 1;
    pub const PROTOCOL_TCP: u8 = 6;
    pub const PROTOCOL_UDP: u8 = 17;
    
    pub fn new(src_addr: Ipv4Address, dest_addr: Ipv4Address, protocol: u8, payload_len: u16) -> Self {
        let mut header = Self {
            version_ihl: 0x45, // Version 4, IHL 5 (20 bytes)
            dscp_ecn: 0,
            total_length: (20 + payload_len).to_be(),
            identification: 0,
            flags_fragment: 0x4000u16.to_be(), // Don't fragment
            ttl: 64,
            protocol,
            checksum: 0,
            src_addr: src_addr.to_u32().to_be(),
            dest_addr: dest_addr.to_u32().to_be(),
        };
        
        header.checksum = header.calculate_checksum();
        header
    }
    
    fn calculate_checksum(&self) -> u16 {
        let mut sum = 0u32;
        
        // Calculate checksum over header (excluding checksum field)
        let header_words = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u16,
                10 // 20 bytes / 2
            )
        };
        
        for (i, &word) in header_words.iter().enumerate() {
            if i != 5 { // Skip checksum field
                sum += u16::from_be(word) as u32;
            }
        }
        
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        (!sum as u16).to_be()
    }
}

// IPv4 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ipv4Address {
    octets: [u8; 4],
}

impl Ipv4Address {
    pub const LOCALHOST: Self = Self::new(127, 0, 0, 1);
    pub const ANY: Self = Self::new(0, 0, 0, 0);
    pub const BROADCAST: Self = Self::new(255, 255, 255, 255);
    
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self {
            octets: [a, b, c, d],
        }
    }
    
    pub fn from_u32(value: u32) -> Self {
        Self {
            octets: [
                ((value >> 24) & 0xFF) as u8,
                ((value >> 16) & 0xFF) as u8,
                ((value >> 8) & 0xFF) as u8,
                (value & 0xFF) as u8,
            ],
        }
    }
    
    pub fn to_u32(&self) -> u32 {
        ((self.octets[0] as u32) << 24) |
        ((self.octets[1] as u32) << 16) |
        ((self.octets[2] as u32) << 8) |
        (self.octets[3] as u32)
    }
    
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}.{}", self.octets[0], self.octets[1], self.octets[2], self.octets[3])
    }
}

// TCP header structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TcpHeader {
    pub src_port: u16,
    pub dest_port: u16,
    pub seq_number: u32,
    pub ack_number: u32,
    pub data_offset_flags: u16, // Data offset (4 bits) + Reserved (3 bits) + Flags (9 bits)
    pub window_size: u16,
    pub checksum: u16,
    pub urgent_pointer: u16,
}

impl TcpHeader {
    pub const FLAG_FIN: u16 = 0x0001;
    pub const FLAG_SYN: u16 = 0x0002;
    pub const FLAG_RST: u16 = 0x0004;
    pub const FLAG_PSH: u16 = 0x0008;
    pub const FLAG_ACK: u16 = 0x0010;
    pub const FLAG_URG: u16 = 0x0020;
    
    pub fn new(
        src_port: u16,
        dest_port: u16,
        seq_number: u32,
        ack_number: u32,
        flags: u16,
        window_size: u16,
    ) -> Self {
        Self {
            src_port: src_port.to_be(),
            dest_port: dest_port.to_be(),
            seq_number: seq_number.to_be(),
            ack_number: ack_number.to_be(),
            data_offset_flags: ((5 << 12) | flags).to_be(), // 5 * 4 = 20 bytes header
            window_size: window_size.to_be(),
            checksum: 0,
            urgent_pointer: 0,
        }
    }
    
    pub fn get_flags(&self) -> u16 {
        u16::from_be(self.data_offset_flags) & 0x01FF
    }
    
    pub fn set_flags(&mut self, flags: u16) {
        let data_offset = (u16::from_be(self.data_offset_flags) >> 12) << 12;
        self.data_offset_flags = (data_offset | (flags & 0x01FF)).to_be();
    }
}

// UDP header structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct UdpHeader {
    pub src_port: u16,
    pub dest_port: u16,
    pub length: u16,
    pub checksum: u16,
}

impl UdpHeader {
    pub fn new(src_port: u16, dest_port: u16, payload_len: u16) -> Self {
        Self {
            src_port: src_port.to_be(),
            dest_port: dest_port.to_be(),
            length: (8 + payload_len).to_be(), // UDP header is 8 bytes
            checksum: 0,
        }
    }
}

// Socket types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    Stream,    // TCP
    Datagram,  // UDP
    Raw,       // Raw IP
}

// Socket address family
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressFamily {
    Unspec,    // AF_UNSPEC
    Unix,      // AF_UNIX
    Inet,      // AF_INET (IPv4)
    Inet6,     // AF_INET6 (IPv6)
}

// Socket address
#[derive(Debug, Clone)]
pub enum SocketAddress {
    Ipv4(Ipv4Address, u16), // Address and port
    Ipv6([u16; 8], u16),     // Address and port
    Unix(String),            // Unix domain socket path
}

// TCP connection states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

// TCP connection
#[derive(Debug)]
pub struct TcpConnection {
    pub local_addr: Ipv4Address,
    pub local_port: u16,
    pub remote_addr: Ipv4Address,
    pub remote_port: u16,
    pub state: TcpState,
    pub seq_number: u32,
    pub ack_number: u32,
    pub window_size: u16,
    pub send_buffer: Vec<u8>,
    pub recv_buffer: Vec<u8>,
    pub mss: u16, // Maximum segment size
}

impl TcpConnection {
    pub fn new(
        local_addr: Ipv4Address,
        local_port: u16,
        remote_addr: Ipv4Address,
        remote_port: u16,
    ) -> Self {
        Self {
            local_addr,
            local_port,
            remote_addr,
            remote_port,
            state: TcpState::Closed,
            seq_number: 0,
            ack_number: 0,
            window_size: 8192,
            send_buffer: Vec::new(),
            recv_buffer: Vec::new(),
            mss: 1460, // Standard MSS for Ethernet
        }
    }
    
    pub fn handle_packet(&mut self, header: &TcpHeader, data: &[u8]) -> Option<Vec<u8>> {
        let flags = header.get_flags();
        
        match self.state {
            TcpState::Closed => {
                if flags & TcpHeader::FLAG_SYN != 0 {
                    // Received SYN, send SYN-ACK
                    self.state = TcpState::SynReceived;
                    self.ack_number = u32::from_be(header.seq_number) + 1;
                    
                    // Create SYN-ACK response
                    let response = TcpHeader::new(
                        self.local_port,
                        self.remote_port,
                        self.seq_number,
                        self.ack_number,
                        TcpHeader::FLAG_SYN | TcpHeader::FLAG_ACK,
                        self.window_size,
                    );
                    
                    self.seq_number += 1;
                    
                    // Return serialized response
                    Some(self.serialize_tcp_header(&response))
                } else {
                    None
                }
            }
            TcpState::SynSent => {
                if flags & TcpHeader::FLAG_SYN != 0 && flags & TcpHeader::FLAG_ACK != 0 {
                    // Received SYN-ACK, send ACK
                    self.state = TcpState::Established;
                    self.ack_number = u32::from_be(header.seq_number) + 1;
                    
                    let response = TcpHeader::new(
                        self.local_port,
                        self.remote_port,
                        self.seq_number,
                        self.ack_number,
                        TcpHeader::FLAG_ACK,
                        self.window_size,
                    );
                    
                    Some(self.serialize_tcp_header(&response))
                } else {
                    None
                }
            }
            TcpState::Established => {
                if flags & TcpHeader::FLAG_FIN != 0 {
                    // Received FIN, send ACK and go to CloseWait
                    self.state = TcpState::CloseWait;
                    self.ack_number = u32::from_be(header.seq_number) + 1;
                    
                    let response = TcpHeader::new(
                        self.local_port,
                        self.remote_port,
                        self.seq_number,
                        self.ack_number,
                        TcpHeader::FLAG_ACK,
                        self.window_size,
                    );
                    
                    Some(self.serialize_tcp_header(&response))
                } else if !data.is_empty() {
                    // Received data, add to buffer and send ACK
                    self.recv_buffer.extend_from_slice(data);
                    self.ack_number = u32::from_be(header.seq_number) + data.len() as u32;
                    
                    let response = TcpHeader::new(
                        self.local_port,
                        self.remote_port,
                        self.seq_number,
                        self.ack_number,
                        TcpHeader::FLAG_ACK,
                        self.window_size,
                    );
                    
                    Some(self.serialize_tcp_header(&response))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    
    fn serialize_tcp_header(&self, header: &TcpHeader) -> Vec<u8> {
        let header_bytes = unsafe {
            core::slice::from_raw_parts(
                header as *const _ as *const u8,
                core::mem::size_of::<TcpHeader>(),
            )
        };
        header_bytes.to_vec()
    }
}

// Socket structure
#[derive(Debug)]
pub struct Socket {
    pub socket_type: SocketType,
    pub address_family: AddressFamily,
    pub local_address: Option<SocketAddress>,
    pub remote_address: Option<SocketAddress>,
    pub tcp_connection: Option<TcpConnection>,
    pub udp_buffer: Vec<u8>,
    pub is_listening: bool,
    pub backlog: Vec<TcpConnection>,
}

impl Socket {
    pub fn new(socket_type: SocketType, address_family: AddressFamily) -> Self {
        Self {
            socket_type,
            address_family,
            local_address: None,
            remote_address: None,
            tcp_connection: None,
            udp_buffer: Vec::new(),
            is_listening: false,
            backlog: Vec::new(),
        }
    }
    
    pub fn bind(&mut self, address: SocketAddress) -> NtStatus {
        self.local_address = Some(address);
        NtStatus::Success
    }
    
    pub fn listen(&mut self, backlog: usize) -> NtStatus {
        if self.socket_type != SocketType::Stream {
            return NtStatus::InvalidParameter;
        }
        
        self.is_listening = true;
        self.backlog.reserve(backlog);
        NtStatus::Success
    }
    
    pub fn connect(&mut self, address: SocketAddress) -> NtStatus {
        if self.socket_type != SocketType::Stream {
            return NtStatus::InvalidParameter;
        }
        
        self.remote_address = Some(address.clone());
        
        // Create TCP connection
        if let (Some(SocketAddress::Ipv4(local_ip, local_port)), SocketAddress::Ipv4(remote_ip, remote_port)) = 
            (&self.local_address, &address) {
            let mut connection = TcpConnection::new(*local_ip, *local_port, *remote_ip, *remote_port);
            connection.state = TcpState::SynSent;
            self.tcp_connection = Some(connection);
        }
        
        NtStatus::Success
    }
    
    pub fn send(&mut self, data: &[u8]) -> Result<usize, NtStatus> {
        match self.socket_type {
            SocketType::Stream => {
                if let Some(ref mut conn) = self.tcp_connection {
                    if conn.state == TcpState::Established {
                        conn.send_buffer.extend_from_slice(data);
                        Ok(data.len())
                    } else {
                        Err(NtStatus::InvalidDeviceState)
                    }
                } else {
                    Err(NtStatus::InvalidHandle)
                }
            }
            SocketType::Datagram => {
                self.udp_buffer.extend_from_slice(data);
                Ok(data.len())
            }
            _ => Err(NtStatus::NotImplemented),
        }
    }
    
    pub fn recv(&mut self, buffer: &mut [u8]) -> Result<usize, NtStatus> {
        match self.socket_type {
            SocketType::Stream => {
                if let Some(ref mut conn) = self.tcp_connection {
                    if conn.state == TcpState::Established {
                        let available = conn.recv_buffer.len().min(buffer.len());
                        if available > 0 {
                            buffer[..available].copy_from_slice(&conn.recv_buffer[..available]);
                            conn.recv_buffer.drain(..available);
                            Ok(available)
                        } else {
                            Ok(0) // No data available
                        }
                    } else {
                        Err(NtStatus::InvalidDeviceState)
                    }
                } else {
                    Err(NtStatus::InvalidHandle)
                }
            }
            SocketType::Datagram => {
                let available = self.udp_buffer.len().min(buffer.len());
                if available > 0 {
                    buffer[..available].copy_from_slice(&self.udp_buffer[..available]);
                    self.udp_buffer.drain(..available);
                    Ok(available)
                } else {
                    Ok(0)
                }
            }
            _ => Err(NtStatus::NotImplemented),
        }
    }
}

// Network interface
#[derive(Debug)]
pub struct NetworkInterface {
    pub name: String,
    pub mac_address: [u8; 6],
    pub ipv4_address: Option<Ipv4Address>,
    pub ipv4_netmask: Option<Ipv4Address>,
    pub ipv4_gateway: Option<Ipv4Address>,
    pub mtu: u16,
    pub is_up: bool,
}

impl NetworkInterface {
    pub fn new(name: String, mac_address: [u8; 6]) -> Self {
        Self {
            name,
            mac_address,
            ipv4_address: None,
            ipv4_netmask: None,
            ipv4_gateway: None,
            mtu: 1500,
            is_up: false,
        }
    }
    
    pub fn configure_ipv4(&mut self, address: Ipv4Address, netmask: Ipv4Address, gateway: Option<Ipv4Address>) {
        self.ipv4_address = Some(address);
        self.ipv4_netmask = Some(netmask);
        self.ipv4_gateway = gateway;
    }
    
    pub fn bring_up(&mut self) {
        self.is_up = true;
    }
    
    pub fn bring_down(&mut self) {
        self.is_up = false;
    }
}

// ARP cache entry
#[derive(Debug, Clone)]
pub struct ArpEntry {
    pub ip_address: Ipv4Address,
    pub mac_address: [u8; 6],
    pub timestamp: u64,
}

// Routing table entry
#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub destination: Ipv4Address,
    pub netmask: Ipv4Address,
    pub gateway: Ipv4Address,
    pub interface: String,
    pub metric: u32,
}

// Network stack manager
pub struct NetworkStack {
    interfaces: BTreeMap<String, NetworkInterface>,
    sockets: BTreeMap<u32, Socket>,
    arp_cache: Vec<ArpEntry>,
    routing_table: Vec<RouteEntry>,
    next_socket_id: AtomicU32,
    next_ephemeral_port: AtomicU16,
}

impl NetworkStack {
    pub fn new() -> Self {
        Self {
            interfaces: BTreeMap::new(),
            sockets: BTreeMap::new(),
            arp_cache: Vec::new(),
            routing_table: Vec::new(),
            next_socket_id: AtomicU32::new(1),
            next_ephemeral_port: AtomicU16::new(49152), // Start of dynamic port range
        }
    }
    
    pub fn initialize(&mut self) -> NtStatus {
        use crate::serial_println;
        
        serial_println!("Network: Initializing TCP/IP stack");
        
        // Create loopback interface
        let mut lo = NetworkInterface::new("lo".to_string(), [0, 0, 0, 0, 0, 0]);
        lo.configure_ipv4(
            Ipv4Address::LOCALHOST,
            Ipv4Address::new(255, 0, 0, 0),
            None,
        );
        lo.bring_up();
        self.interfaces.insert("lo".to_string(), lo);
        
        // Add default loopback route
        self.routing_table.push(RouteEntry {
            destination: Ipv4Address::new(127, 0, 0, 0),
            netmask: Ipv4Address::new(255, 0, 0, 0),
            gateway: Ipv4Address::LOCALHOST,
            interface: "lo".to_string(),
            metric: 0,
        });
        
        serial_println!("Network: Created loopback interface");
        
        // Create default Ethernet interface (if available)
        let mut eth0 = NetworkInterface::new("eth0".to_string(), [0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);
        eth0.configure_ipv4(
            Ipv4Address::new(10, 0, 2, 15),
            Ipv4Address::new(255, 255, 255, 0),
            Some(Ipv4Address::new(10, 0, 2, 2)),
        );
        self.interfaces.insert("eth0".to_string(), eth0);
        
        serial_println!("Network: Created eth0 interface");
        serial_println!("Network: TCP/IP stack initialized");
        
        NtStatus::Success
    }
    
    pub fn create_socket(&mut self, socket_type: SocketType, address_family: AddressFamily) -> Result<u32, NtStatus> {
        let socket_id = self.next_socket_id.fetch_add(1, Ordering::SeqCst);
        let socket = Socket::new(socket_type, address_family);
        self.sockets.insert(socket_id, socket);
        Ok(socket_id)
    }
    
    pub fn bind_socket(&mut self, socket_id: u32, address: SocketAddress) -> NtStatus {
        if let Some(socket) = self.sockets.get_mut(&socket_id) {
            socket.bind(address)
        } else {
            NtStatus::InvalidHandle
        }
    }
    
    pub fn listen_socket(&mut self, socket_id: u32, backlog: usize) -> NtStatus {
        if let Some(socket) = self.sockets.get_mut(&socket_id) {
            socket.listen(backlog)
        } else {
            NtStatus::InvalidHandle
        }
    }
    
    pub fn connect_socket(&mut self, socket_id: u32, address: SocketAddress) -> NtStatus {
        // Allocate ephemeral port if needed
        if let Some(socket) = self.sockets.get_mut(&socket_id) {
            if socket.local_address.is_none() {
                let port = self.next_ephemeral_port.fetch_add(1, Ordering::SeqCst);
                socket.local_address = Some(SocketAddress::Ipv4(
                    Ipv4Address::ANY,
                    port,
                ));
            }
            socket.connect(address)
        } else {
            NtStatus::InvalidHandle
        }
    }
    
    pub fn send_socket(&mut self, socket_id: u32, data: &[u8]) -> Result<usize, NtStatus> {
        if let Some(socket) = self.sockets.get_mut(&socket_id) {
            socket.send(data)
        } else {
            Err(NtStatus::InvalidHandle)
        }
    }
    
    pub fn recv_socket(&mut self, socket_id: u32, buffer: &mut [u8]) -> Result<usize, NtStatus> {
        if let Some(socket) = self.sockets.get_mut(&socket_id) {
            socket.recv(buffer)
        } else {
            Err(NtStatus::InvalidHandle)
        }
    }
    
    pub fn close_socket(&mut self, socket_id: u32) -> NtStatus {
        if self.sockets.remove(&socket_id).is_some() {
            NtStatus::Success
        } else {
            NtStatus::InvalidHandle
        }
    }
    
    pub fn add_interface(&mut self, interface: NetworkInterface) {
        self.interfaces.insert(interface.name.clone(), interface);
    }
    
    pub fn get_interface(&self, name: &str) -> Option<&NetworkInterface> {
        self.interfaces.get(name)
    }
    
    pub fn get_interface_mut(&mut self, name: &str) -> Option<&mut NetworkInterface> {
        self.interfaces.get_mut(name)
    }
    
    pub fn add_route(&mut self, route: RouteEntry) {
        self.routing_table.push(route);
        self.routing_table.sort_by_key(|r| r.metric);
    }
    
    pub fn lookup_route(&self, destination: Ipv4Address) -> Option<&RouteEntry> {
        for route in &self.routing_table {
            let dest_masked = destination.to_u32() & route.netmask.to_u32();
            let route_masked = route.destination.to_u32() & route.netmask.to_u32();
            
            if dest_masked == route_masked {
                return Some(route);
            }
        }
        None
    }
    
    pub fn add_arp_entry(&mut self, ip: Ipv4Address, mac: [u8; 6]) {
        self.arp_cache.push(ArpEntry {
            ip_address: ip,
            mac_address: mac,
            timestamp: 0, // Would be current time
        });
    }
    
    pub fn lookup_arp(&self, ip: Ipv4Address) -> Option<[u8; 6]> {
        self.arp_cache.iter()
            .find(|entry| entry.ip_address == ip)
            .map(|entry| entry.mac_address)
    }
}

// Global network stack
lazy_static! {
    pub static ref NETWORK_STACK: Mutex<NetworkStack> = Mutex::new(NetworkStack::new());
}

// Public API functions
pub fn initialize_network() -> NtStatus {
    let mut stack = NETWORK_STACK.lock();
    stack.initialize()
}

// Windows Sockets (Winsock) API functions
pub fn ws2_socket(af: i32, socket_type: i32, protocol: i32) -> Result<u32, NtStatus> {
    let mut stack = NETWORK_STACK.lock();
    
    let address_family = match af {
        0 => AddressFamily::Unspec,
        1 => AddressFamily::Unix,
        2 => AddressFamily::Inet,
        23 => AddressFamily::Inet6,
        _ => return Err(NtStatus::InvalidParameter),
    };
    
    let sock_type = match socket_type {
        1 => SocketType::Stream,
        2 => SocketType::Datagram,
        3 => SocketType::Raw,
        _ => return Err(NtStatus::InvalidParameter),
    };
    
    stack.create_socket(sock_type, address_family)
}

pub fn ws2_bind(socket: u32, addr: &SocketAddress) -> NtStatus {
    let mut stack = NETWORK_STACK.lock();
    stack.bind_socket(socket, addr.clone())
}

pub fn ws2_listen(socket: u32, backlog: i32) -> NtStatus {
    let mut stack = NETWORK_STACK.lock();
    stack.listen_socket(socket, backlog as usize)
}

pub fn ws2_connect(socket: u32, addr: &SocketAddress) -> NtStatus {
    let mut stack = NETWORK_STACK.lock();
    stack.connect_socket(socket, addr.clone())
}

pub fn ws2_send(socket: u32, buf: &[u8], flags: i32) -> Result<usize, NtStatus> {
    let mut stack = NETWORK_STACK.lock();
    stack.send_socket(socket, buf)
}

pub fn ws2_recv(socket: u32, buf: &mut [u8], flags: i32) -> Result<usize, NtStatus> {
    let mut stack = NETWORK_STACK.lock();
    stack.recv_socket(socket, buf)
}

pub fn ws2_closesocket(socket: u32) -> NtStatus {
    let mut stack = NETWORK_STACK.lock();
    stack.close_socket(socket)
}