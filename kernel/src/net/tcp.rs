// TCP (Transmission Control Protocol) Implementation
use super::ip::{IpPacket, Ipv4Address, IP_PROTO_TCP};
use alloc::vec::Vec;
use alloc::collections::{BTreeMap, VecDeque};
use spin::Mutex;
use lazy_static::lazy_static;
use core::cmp::min;

// TCP Header flags
const TCP_FIN: u8 = 0x01;
const TCP_SYN: u8 = 0x02;
const TCP_RST: u8 = 0x04;
const TCP_PSH: u8 = 0x08;
const TCP_ACK: u8 = 0x10;
const TCP_URG: u8 = 0x20;

// TCP states
#[derive(Debug, Clone, Copy, PartialEq)]
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

// TCP Header (20 bytes minimum)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TcpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
    pub data_offset_flags: u16,  // 4 bits data offset, 6 bits reserved, 6 bits flags
    pub window: u16,
    pub checksum: u16,
    pub urgent_ptr: u16,
}

impl TcpHeader {
    pub fn new(src_port: u16, dst_port: u16, seq: u32, ack: u32, flags: u8, window: u16) -> Self {
        let data_offset = 5u8; // 5 * 4 = 20 bytes (no options)
        let data_offset_flags = ((data_offset as u16) << 12) | (flags as u16);
        
        Self {
            src_port: src_port.to_be(),
            dst_port: dst_port.to_be(),
            seq_num: seq.to_be(),
            ack_num: ack.to_be(),
            data_offset_flags: data_offset_flags.to_be(),
            window: window.to_be(),
            checksum: 0,
            urgent_ptr: 0,
        }
    }
    
    pub fn src_port(&self) -> u16 {
        u16::from_be(self.src_port)
    }
    
    pub fn dst_port(&self) -> u16 {
        u16::from_be(self.dst_port)
    }
    
    pub fn seq_num(&self) -> u32 {
        u32::from_be(self.seq_num)
    }
    
    pub fn ack_num(&self) -> u32 {
        u32::from_be(self.ack_num)
    }
    
    pub fn data_offset(&self) -> u8 {
        ((u16::from_be(self.data_offset_flags) >> 12) & 0x0F) as u8
    }
    
    pub fn flags(&self) -> u8 {
        (u16::from_be(self.data_offset_flags) & 0x3F) as u8
    }
    
    pub fn window(&self) -> u16 {
        u16::from_be(self.window)
    }
    
    pub fn has_flag(&self, flag: u8) -> bool {
        self.flags() & flag != 0
    }
}

// TCP Segment
pub struct TcpSegment {
    pub header: TcpHeader,
    pub options: Vec<u8>,
    pub data: Vec<u8>,
}

impl TcpSegment {
    pub fn new(header: TcpHeader, data: Vec<u8>) -> Self {
        Self {
            header,
            options: Vec::new(),
            data,
        }
    }
    
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 20 {
            return Err("TCP segment too small");
        }
        
        let header = unsafe {
            *(data.as_ptr() as *const TcpHeader)
        };
        
        let data_offset = header.data_offset() as usize;
        if data_offset < 5 || data_offset > 15 {
            return Err("Invalid TCP data offset");
        }
        
        let header_len = data_offset * 4;
        if data.len() < header_len {
            return Err("TCP segment truncated");
        }
        
        let options = if header_len > 20 {
            data[20..header_len].to_vec()
        } else {
            Vec::new()
        };
        
        let payload = data[header_len..].to_vec();
        
        Ok(Self {
            header,
            options,
            data: payload,
        })
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut segment = Vec::new();
        
        // Add header
        segment.extend_from_slice(&self.header.src_port.to_be_bytes());
        segment.extend_from_slice(&self.header.dst_port.to_be_bytes());
        segment.extend_from_slice(&self.header.seq_num.to_be_bytes());
        segment.extend_from_slice(&self.header.ack_num.to_be_bytes());
        segment.extend_from_slice(&self.header.data_offset_flags.to_be_bytes());
        segment.extend_from_slice(&self.header.window.to_be_bytes());
        segment.extend_from_slice(&self.header.checksum.to_be_bytes());
        segment.extend_from_slice(&self.header.urgent_ptr.to_be_bytes());
        
        // Add options
        segment.extend_from_slice(&self.options);
        
        // Add data
        segment.extend_from_slice(&self.data);
        
        segment
    }
    
    // Calculate TCP checksum
    pub fn calculate_checksum(&self, src_ip: Ipv4Address, dst_ip: Ipv4Address) -> u16 {
        let mut sum: u32 = 0;
        
        // Pseudo header
        for byte in src_ip.as_bytes() {
            sum += (*byte as u32) << 8;
        }
        for byte in dst_ip.as_bytes() {
            sum += (*byte as u32) << 8;
        }
        sum += IP_PROTO_TCP as u32;
        let total_len = 20 + self.options.len() + self.data.len();
        sum += total_len as u32;
        
        // TCP header (with checksum field as 0)
        let header_bytes = self.to_bytes();
        let mut i = 0;
        while i < header_bytes.len() - 1 {
            if i == 16 {
                // Skip checksum field
                i += 2;
                continue;
            }
            sum += ((header_bytes[i] as u32) << 8) | (header_bytes[i + 1] as u32);
            i += 2;
        }
        
        // Add remaining byte if odd length
        if i < header_bytes.len() {
            sum += (header_bytes[i] as u32) << 8;
        }
        
        // Add carry bits
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        // One's complement
        !sum as u16
    }
}

// TCP Control Block (TCB)
pub struct TcpControlBlock {
    pub state: TcpState,
    pub local_addr: Ipv4Address,
    pub local_port: u16,
    pub remote_addr: Ipv4Address,
    pub remote_port: u16,
    
    // Sequence numbers
    pub snd_una: u32,  // Send unacknowledged
    pub snd_nxt: u32,  // Send next
    pub snd_wnd: u16,  // Send window
    pub rcv_nxt: u32,  // Receive next
    pub rcv_wnd: u16,  // Receive window
    pub iss: u32,      // Initial send sequence
    pub irs: u32,      // Initial receive sequence
    
    // Buffers
    pub send_buffer: VecDeque<u8>,
    pub recv_buffer: VecDeque<u8>,
    pub retransmit_queue: VecDeque<(u32, Vec<u8>)>, // (seq_num, data)
    
    // Timers (simplified)
    pub retransmit_timeout: u64,
    pub time_wait_timeout: u64,
    
    // Options
    pub mss: u16,  // Maximum segment size
}

impl TcpControlBlock {
    pub fn new(local_addr: Ipv4Address, local_port: u16) -> Self {
        let iss = Self::generate_isn();
        
        Self {
            state: TcpState::Closed,
            local_addr,
            local_port,
            remote_addr: Ipv4Address::new(0, 0, 0, 0),
            remote_port: 0,
            snd_una: iss,
            snd_nxt: iss,
            snd_wnd: 8192,
            rcv_nxt: 0,
            rcv_wnd: 8192,
            iss,
            irs: 0,
            send_buffer: VecDeque::new(),
            recv_buffer: VecDeque::new(),
            retransmit_queue: VecDeque::new(),
            retransmit_timeout: 1000,
            time_wait_timeout: 120000,
            mss: 1460,  // Default MSS for Ethernet
        }
    }
    
    fn generate_isn() -> u32 {
        // Simple ISN generation (should use secure random in production)
        static mut COUNTER: u32 = 0x1000;
        unsafe {
            COUNTER += 64000;
            COUNTER
        }
    }
    
    pub fn send_syn(&mut self) -> TcpSegment {
        self.state = TcpState::SynSent;
        let header = TcpHeader::new(
            self.local_port,
            self.remote_port,
            self.snd_nxt,
            0,
            TCP_SYN,
            self.rcv_wnd,
        );
        self.snd_nxt = self.snd_nxt.wrapping_add(1);
        TcpSegment::new(header, Vec::new())
    }
    
    pub fn send_syn_ack(&mut self) -> TcpSegment {
        self.state = TcpState::SynReceived;
        let header = TcpHeader::new(
            self.local_port,
            self.remote_port,
            self.snd_nxt,
            self.rcv_nxt,
            TCP_SYN | TCP_ACK,
            self.rcv_wnd,
        );
        self.snd_nxt = self.snd_nxt.wrapping_add(1);
        TcpSegment::new(header, Vec::new())
    }
    
    pub fn send_ack(&mut self) -> TcpSegment {
        let header = TcpHeader::new(
            self.local_port,
            self.remote_port,
            self.snd_nxt,
            self.rcv_nxt,
            TCP_ACK,
            self.rcv_wnd,
        );
        TcpSegment::new(header, Vec::new())
    }
    
    pub fn send_data(&mut self, data: &[u8]) -> Vec<TcpSegment> {
        let mut segments = Vec::new();
        let mut offset = 0;
        
        while offset < data.len() {
            let chunk_size = min(self.mss as usize, data.len() - offset);
            let chunk = data[offset..offset + chunk_size].to_vec();
            
            let header = TcpHeader::new(
                self.local_port,
                self.remote_port,
                self.snd_nxt,
                self.rcv_nxt,
                TCP_ACK | TCP_PSH,
                self.rcv_wnd,
            );
            
            // Add to retransmit queue
            self.retransmit_queue.push_back((self.snd_nxt, chunk.clone()));
            
            self.snd_nxt = self.snd_nxt.wrapping_add(chunk_size as u32);
            segments.push(TcpSegment::new(header, chunk));
            
            offset += chunk_size;
        }
        
        segments
    }
    
    pub fn send_fin(&mut self) -> TcpSegment {
        let header = TcpHeader::new(
            self.local_port,
            self.remote_port,
            self.snd_nxt,
            self.rcv_nxt,
            TCP_FIN | TCP_ACK,
            self.rcv_wnd,
        );
        self.snd_nxt = self.snd_nxt.wrapping_add(1);
        
        match self.state {
            TcpState::Established => self.state = TcpState::FinWait1,
            TcpState::CloseWait => self.state = TcpState::LastAck,
            _ => {}
        }
        
        TcpSegment::new(header, Vec::new())
    }
    
    pub fn send_rst(&mut self) -> TcpSegment {
        let header = TcpHeader::new(
            self.local_port,
            self.remote_port,
            self.snd_nxt,
            0,
            TCP_RST,
            0,
        );
        self.state = TcpState::Closed;
        TcpSegment::new(header, Vec::new())
    }
    
    pub fn process_segment(&mut self, segment: &TcpSegment) -> Option<TcpSegment> {
        let seq = segment.header.seq_num();
        let ack = segment.header.ack_num();
        let flags = segment.header.flags();
        
        match self.state {
            TcpState::Listen => {
                if flags & TCP_SYN != 0 {
                    // Received SYN, send SYN-ACK
                    self.irs = seq;
                    self.rcv_nxt = seq.wrapping_add(1);
                    self.remote_addr = Ipv4Address::new(0, 0, 0, 0); // Would get from IP packet
                    self.remote_port = segment.header.src_port();
                    return Some(self.send_syn_ack());
                }
            }
            
            TcpState::SynSent => {
                if flags & TCP_SYN != 0 {
                    self.irs = seq;
                    self.rcv_nxt = seq.wrapping_add(1);
                    
                    if flags & TCP_ACK != 0 {
                        // Received SYN-ACK
                        if ack == self.snd_nxt {
                            self.snd_una = ack;
                            self.state = TcpState::Established;
                            return Some(self.send_ack());
                        }
                    } else {
                        // Simultaneous open - received SYN
                        self.state = TcpState::SynReceived;
                        return Some(self.send_syn_ack());
                    }
                }
            }
            
            TcpState::SynReceived => {
                if flags & TCP_ACK != 0 && ack == self.snd_nxt {
                    self.snd_una = ack;
                    self.state = TcpState::Established;
                    crate::serial_println!("TCP connection established");
                }
            }
            
            TcpState::Established => {
                // Process ACK
                if flags & TCP_ACK != 0 {
                    if ack > self.snd_una && ack <= self.snd_nxt {
                        self.snd_una = ack;
                        
                        // Remove acknowledged data from retransmit queue
                        while let Some((seq_num, _)) = self.retransmit_queue.front() {
                            if *seq_num < self.snd_una {
                                self.retransmit_queue.pop_front();
                            } else {
                                break;
                            }
                        }
                    }
                }
                
                // Process data
                if !segment.data.is_empty() {
                    if seq == self.rcv_nxt {
                        // In-order data
                        self.recv_buffer.extend(&segment.data);
                        self.rcv_nxt = self.rcv_nxt.wrapping_add(segment.data.len() as u32);
                        return Some(self.send_ack());
                    }
                    // Out-of-order data would be buffered in a real implementation
                }
                
                // Process FIN
                if flags & TCP_FIN != 0 {
                    self.rcv_nxt = self.rcv_nxt.wrapping_add(1);
                    self.state = TcpState::CloseWait;
                    return Some(self.send_ack());
                }
            }
            
            TcpState::FinWait1 => {
                if flags & TCP_ACK != 0 && ack == self.snd_nxt {
                    self.state = TcpState::FinWait2;
                }
                if flags & TCP_FIN != 0 {
                    self.rcv_nxt = self.rcv_nxt.wrapping_add(1);
                    if self.state == TcpState::FinWait2 {
                        self.state = TcpState::TimeWait;
                    } else {
                        self.state = TcpState::Closing;
                    }
                    return Some(self.send_ack());
                }
            }
            
            TcpState::FinWait2 => {
                if flags & TCP_FIN != 0 {
                    self.rcv_nxt = self.rcv_nxt.wrapping_add(1);
                    self.state = TcpState::TimeWait;
                    return Some(self.send_ack());
                }
            }
            
            TcpState::CloseWait => {
                // Application should close
            }
            
            TcpState::Closing => {
                if flags & TCP_ACK != 0 && ack == self.snd_nxt {
                    self.state = TcpState::TimeWait;
                }
            }
            
            TcpState::LastAck => {
                if flags & TCP_ACK != 0 && ack == self.snd_nxt {
                    self.state = TcpState::Closed;
                    crate::serial_println!("TCP connection closed");
                }
            }
            
            TcpState::TimeWait => {
                // Wait for 2*MSL before closing
            }
            
            TcpState::Closed => {
                if flags & TCP_RST == 0 {
                    return Some(self.send_rst());
                }
            }
        }
        
        None
    }
}

// TCP Socket
pub struct TcpSocket {
    pub tcb: TcpControlBlock,
    pub id: u32,
}

impl TcpSocket {
    pub fn new(local_addr: Ipv4Address, local_port: u16) -> Self {
        static mut SOCKET_ID: u32 = 0;
        let id = unsafe {
            SOCKET_ID += 1;
            SOCKET_ID
        };
        
        Self {
            tcb: TcpControlBlock::new(local_addr, local_port),
            id,
        }
    }
    
    pub fn listen(&mut self) {
        self.tcb.state = TcpState::Listen;
        crate::serial_println!("TCP socket listening on port {}", self.tcb.local_port);
    }
    
    pub fn connect(&mut self, remote_addr: Ipv4Address, remote_port: u16) -> TcpSegment {
        self.tcb.remote_addr = remote_addr;
        self.tcb.remote_port = remote_port;
        self.tcb.send_syn()
    }
    
    pub fn send(&mut self, data: &[u8]) -> Result<Vec<TcpSegment>, &'static str> {
        if self.tcb.state != TcpState::Established {
            return Err("Connection not established");
        }
        
        // Add to send buffer
        self.tcb.send_buffer.extend(data);
        
        // Create segments
        Ok(self.tcb.send_data(data))
    }
    
    pub fn recv(&mut self, max_len: usize) -> Vec<u8> {
        let mut data = Vec::new();
        let len = min(max_len, self.tcb.recv_buffer.len());
        
        for _ in 0..len {
            if let Some(byte) = self.tcb.recv_buffer.pop_front() {
                data.push(byte);
            }
        }
        
        data
    }
    
    pub fn close(&mut self) -> TcpSegment {
        self.tcb.send_fin()
    }
}

// TCP connection table
pub struct TcpConnection {
    pub local_addr: Ipv4Address,
    pub local_port: u16,
    pub remote_addr: Ipv4Address,
    pub remote_port: u16,
}

impl TcpConnection {
    pub fn new(local_addr: Ipv4Address, local_port: u16, 
               remote_addr: Ipv4Address, remote_port: u16) -> Self {
        Self {
            local_addr,
            local_port,
            remote_addr,
            remote_port,
        }
    }
    
    pub fn to_key(&self) -> u64 {
        let local = ((self.local_addr.to_u32() as u64) << 16) | (self.local_port as u64);
        let remote = ((self.remote_addr.to_u32() as u64) << 16) | (self.remote_port as u64);
        local ^ remote
    }
}

// Global TCP state
lazy_static! {
    static ref TCP_SOCKETS: Mutex<BTreeMap<u16, TcpSocket>> = Mutex::new(BTreeMap::new());
    static ref TCP_CONNECTIONS: Mutex<BTreeMap<u64, TcpSocket>> = Mutex::new(BTreeMap::new());
}

// Process incoming TCP segment
pub fn process_tcp_packet(ip_packet: &IpPacket) {
    let segment = match TcpSegment::from_bytes(&ip_packet.payload) {
        Ok(s) => s,
        Err(e) => {
            crate::serial_println!("Invalid TCP segment: {}", e);
            return;
        }
    };
    
    let src_addr = ip_packet.header.src_addr;
    let dst_addr = ip_packet.header.dst_addr;
    let src_port = segment.header.src_port();
    let dst_port = segment.header.dst_port();
    
    crate::serial_println!("TCP segment from {}:{} to {}:{}, flags: {:02X}",
        src_addr, src_port, dst_addr, dst_port, segment.header.flags());
    
    // Look for existing connection
    let conn = TcpConnection::new(dst_addr, dst_port, src_addr, src_port);
    let conn_key = conn.to_key();
    
    let mut connections = TCP_CONNECTIONS.lock();
    if let Some(socket) = connections.get_mut(&conn_key) {
        // Process segment in existing connection
        if let Some(response) = socket.tcb.process_segment(&segment) {
            send_tcp_segment(response, dst_addr, src_addr);
        }
        return;
    }
    drop(connections);
    
    // Check for listening socket
    let mut sockets = TCP_SOCKETS.lock();
    if let Some(socket) = sockets.get_mut(&dst_port) {
        if socket.tcb.state == TcpState::Listen {
            // Create new connection
            let mut new_socket = TcpSocket::new(dst_addr, dst_port);
            new_socket.tcb.remote_addr = src_addr;
            new_socket.tcb.remote_port = src_port;
            new_socket.tcb.state = TcpState::Listen;
            
            if let Some(response) = new_socket.tcb.process_segment(&segment) {
                send_tcp_segment(response, dst_addr, src_addr);
                
                // Add to connections table
                let mut connections = TCP_CONNECTIONS.lock();
                connections.insert(conn_key, new_socket);
            }
        }
    } else {
        // No listening socket, send RST
        send_tcp_rst(dst_port, src_port, segment.header.seq_num(), dst_addr, src_addr);
    }
}

// Send TCP segment
fn send_tcp_segment(mut segment: TcpSegment, src_addr: Ipv4Address, dst_addr: Ipv4Address) {
    // Calculate checksum
    segment.header.checksum = segment.calculate_checksum(src_addr, dst_addr).to_be();
    
    // Create IP packet
    let ip_packet = IpPacket::new(
        src_addr,
        dst_addr,
        IP_PROTO_TCP,
        segment.to_bytes(),
    );
    
    // Send through IP layer
    if let Err(e) = super::ip::send_ip_packet(ip_packet) {
        crate::serial_println!("Failed to send TCP segment: {}", e);
    }
}

// Send TCP RST
fn send_tcp_rst(src_port: u16, dst_port: u16, ack_num: u32, 
                src_addr: Ipv4Address, dst_addr: Ipv4Address) {
    let header = TcpHeader::new(
        src_port,
        dst_port,
        0,
        ack_num.wrapping_add(1),
        TCP_RST | TCP_ACK,
        0,
    );
    
    let segment = TcpSegment::new(header, Vec::new());
    send_tcp_segment(segment, src_addr, dst_addr);
    
    crate::serial_println!("Sent TCP RST to {}:{}", dst_addr, dst_port);
}

// TCP API functions
pub fn listen(port: u16) -> Result<(), &'static str> {
    let mut sockets = TCP_SOCKETS.lock();
    
    if sockets.contains_key(&port) {
        return Err("Port already in use");
    }
    
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut socket = TcpSocket::new(local_addr, port);
    socket.listen();
    
    sockets.insert(port, socket);
    Ok(())
}

pub fn connect(local_port: u16, remote_addr: Ipv4Address, remote_port: u16) -> Result<u64, &'static str> {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut socket = TcpSocket::new(local_addr, local_port);
    
    let syn_segment = socket.connect(remote_addr, remote_port);
    send_tcp_segment(syn_segment, local_addr, remote_addr);
    
    let conn = TcpConnection::new(local_addr, local_port, remote_addr, remote_port);
    let conn_key = conn.to_key();
    
    let mut connections = TCP_CONNECTIONS.lock();
    connections.insert(conn_key, socket);
    
    crate::serial_println!("TCP connection initiated to {}:{}", remote_addr, remote_port);
    Ok(conn_key)
}

pub fn send(conn_key: u64, data: &[u8]) -> Result<(), &'static str> {
    let mut connections = TCP_CONNECTIONS.lock();
    
    if let Some(socket) = connections.get_mut(&conn_key) {
        let segments = socket.send(data)?;
        let src_addr = socket.tcb.local_addr;
        let dst_addr = socket.tcb.remote_addr;
        
        for segment in segments {
            send_tcp_segment(segment, src_addr, dst_addr);
        }
        
        Ok(())
    } else {
        Err("Connection not found")
    }
}

pub fn recv(conn_key: u64, max_len: usize) -> Result<Vec<u8>, &'static str> {
    let mut connections = TCP_CONNECTIONS.lock();
    
    if let Some(socket) = connections.get_mut(&conn_key) {
        Ok(socket.recv(max_len))
    } else {
        Err("Connection not found")
    }
}

pub fn close(conn_key: u64) -> Result<(), &'static str> {
    let mut connections = TCP_CONNECTIONS.lock();
    
    if let Some(socket) = connections.get_mut(&conn_key) {
        let fin_segment = socket.close();
        let src_addr = socket.tcb.local_addr;
        let dst_addr = socket.tcb.remote_addr;
        
        send_tcp_segment(fin_segment, src_addr, dst_addr);
        
        crate::serial_println!("TCP connection closing");
        Ok(())
    } else {
        Err("Connection not found")
    }
}

// Well-known TCP ports
pub const PORT_FTP_DATA: u16 = 20;
pub const PORT_FTP_CONTROL: u16 = 21;
pub const PORT_SSH: u16 = 22;
pub const PORT_TELNET: u16 = 23;
pub const PORT_SMTP: u16 = 25;
pub const PORT_HTTP: u16 = 80;
pub const PORT_POP3: u16 = 110;
pub const PORT_IMAP: u16 = 143;
pub const PORT_HTTPS: u16 = 443;
pub const PORT_SMB: u16 = 445;
pub const PORT_RDP: u16 = 3389;