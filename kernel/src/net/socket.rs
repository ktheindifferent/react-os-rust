// Socket API Implementation
use super::ip::Ipv4Address;
use super::tcp::{TcpSocket, TcpControlBlock};
use super::udp::UdpSocket;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddr {
    pub ip: Ipv4Address,
    pub port: u16,
}

impl SocketAddr {
    pub fn new(ip: Ipv4Address, port: u16) -> Self {
        Self { ip, port }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    Stream,    // TCP
    Datagram,  // UDP
    Raw,       // Raw IP
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    Unbound,
    Bound,
    Listening,
    Connected,
    Closing,
    Closed,
}

#[derive(Debug, Clone, Copy)]
pub struct SocketOptions {
    pub reuse_addr: bool,
    pub reuse_port: bool,
    pub no_delay: bool,      // TCP_NODELAY
    pub keep_alive: bool,
    pub linger: Option<u32>,  // Linger time in seconds
    pub recv_buffer: usize,
    pub send_buffer: usize,
    pub recv_timeout: Option<u64>,  // Timeout in milliseconds
    pub send_timeout: Option<u64>,
}

impl Default for SocketOptions {
    fn default() -> Self {
        Self {
            reuse_addr: false,
            reuse_port: false,
            no_delay: false,
            keep_alive: false,
            linger: None,
            recv_buffer: 65536,
            send_buffer: 65536,
            recv_timeout: None,
            send_timeout: None,
        }
    }
}

pub enum SocketImpl {
    Tcp(TcpSocket),
    Udp(UdpSocket),
}

pub struct Socket {
    pub id: u32,
    pub socket_type: SocketType,
    pub state: SocketState,
    pub local_addr: Option<SocketAddr>,
    pub remote_addr: Option<SocketAddr>,
    pub options: SocketOptions,
    pub implementation: Option<SocketImpl>,
}

impl Socket {
    pub fn new(socket_type: SocketType) -> Self {
        static SOCKET_ID: AtomicU32 = AtomicU32::new(1);
        
        Self {
            id: SOCKET_ID.fetch_add(1, Ordering::SeqCst),
            socket_type,
            state: SocketState::Unbound,
            local_addr: None,
            remote_addr: None,
            options: SocketOptions::default(),
            implementation: None,
        }
    }
    
    pub fn bind(&mut self, addr: SocketAddr) -> Result<(), &'static str> {
        if self.state != SocketState::Unbound {
            return Err("Socket already bound");
        }
        
        // Check if address is already in use
        if !self.options.reuse_addr && is_port_in_use(addr.port, self.socket_type) {
            return Err("Address already in use");
        }
        
        self.local_addr = Some(addr);
        self.state = SocketState::Bound;
        
        // Create implementation
        match self.socket_type {
            SocketType::Stream => {
                let tcp_socket = TcpSocket::new(addr.ip, addr.port);
                self.implementation = Some(SocketImpl::Tcp(tcp_socket));
            }
            SocketType::Datagram => {
                let udp_socket = UdpSocket::new(addr.port);
                self.implementation = Some(SocketImpl::Udp(udp_socket));
            }
            SocketType::Raw => {
                return Err("Raw sockets not yet implemented");
            }
        }
        
        // Register socket
        register_socket(self.id, addr.port, self.socket_type);
        
        Ok(())
    }
    
    pub fn listen(&mut self, backlog: u32) -> Result<(), &'static str> {
        if self.state != SocketState::Bound {
            return Err("Socket must be bound before listening");
        }
        
        if self.socket_type != SocketType::Stream {
            return Err("Only stream sockets can listen");
        }
        
        if let Some(SocketImpl::Tcp(ref mut tcp_socket)) = self.implementation {
            tcp_socket.listen();
            self.state = SocketState::Listening;
            Ok(())
        } else {
            Err("Invalid socket implementation")
        }
    }
    
    pub fn connect(&mut self, addr: SocketAddr) -> Result<(), &'static str> {
        if self.socket_type != SocketType::Stream {
            return Err("Only stream sockets can connect");
        }
        
        // Auto-bind if not bound
        if self.state == SocketState::Unbound {
            let local_port = allocate_ephemeral_port();
            let local_addr = SocketAddr::new(get_local_ip(), local_port);
            self.bind(local_addr)?;
        }
        
        if let Some(SocketImpl::Tcp(ref mut tcp_socket)) = self.implementation {
            let syn = tcp_socket.connect(addr.ip, addr.port);
            // In real implementation, would send SYN and wait for response
            self.remote_addr = Some(addr);
            self.state = SocketState::Connected;
            Ok(())
        } else {
            Err("Invalid socket implementation")
        }
    }
    
    pub fn accept(&mut self) -> Result<Socket, &'static str> {
        if self.state != SocketState::Listening {
            return Err("Socket is not listening");
        }
        
        // In real implementation, would block until connection available
        Err("No pending connections")
    }
    
    pub fn send(&mut self, data: &[u8]) -> Result<usize, &'static str> {
        if self.state != SocketState::Connected {
            return Err("Socket is not connected");
        }
        
        match &mut self.implementation {
            Some(SocketImpl::Tcp(tcp_socket)) => {
                let segments = tcp_socket.send(data)?;
                // In real implementation, would send segments
                Ok(data.len())
            }
            Some(SocketImpl::Udp(udp_socket)) => {
                if let Some(remote) = self.remote_addr {
                    udp_socket.send_to(data, remote.ip, remote.port)?;
                    Ok(data.len())
                } else {
                    Err("No remote address specified")
                }
            }
            None => Err("Socket not initialized"),
        }
    }
    
    pub fn recv(&mut self, buffer: &mut [u8]) -> Result<usize, &'static str> {
        match &mut self.implementation {
            Some(SocketImpl::Tcp(tcp_socket)) => {
                let data = tcp_socket.recv(buffer.len());
                let len = core::cmp::min(data.len(), buffer.len());
                buffer[..len].copy_from_slice(&data[..len]);
                Ok(len)
            }
            Some(SocketImpl::Udp(udp_socket)) => {
                let (data, _addr) = udp_socket.recv_from()?;
                let len = core::cmp::min(data.len(), buffer.len());
                buffer[..len].copy_from_slice(&data[..len]);
                Ok(len)
            }
            None => Err("Socket not initialized"),
        }
    }
    
    pub fn send_to(&mut self, data: &[u8], addr: SocketAddr) -> Result<usize, &'static str> {
        if self.socket_type != SocketType::Datagram {
            return Err("send_to only works with datagram sockets");
        }
        
        // Auto-bind if not bound
        if self.state == SocketState::Unbound {
            let local_port = allocate_ephemeral_port();
            let local_addr = SocketAddr::new(get_local_ip(), local_port);
            self.bind(local_addr)?;
        }
        
        if let Some(SocketImpl::Udp(ref mut udp_socket)) = self.implementation {
            udp_socket.send_to(data, addr.ip, addr.port)?;
            Ok(data.len())
        } else {
            Err("Invalid socket implementation")
        }
    }
    
    pub fn recv_from(&mut self, buffer: &mut [u8]) -> Result<(usize, SocketAddr), &'static str> {
        if self.socket_type != SocketType::Datagram {
            return Err("recv_from only works with datagram sockets");
        }
        
        if let Some(SocketImpl::Udp(ref mut udp_socket)) = self.implementation {
            let (data, addr) = udp_socket.recv_from()?;
            let len = core::cmp::min(data.len(), buffer.len());
            buffer[..len].copy_from_slice(&data[..len]);
            Ok((len, addr))
        } else {
            Err("Socket not initialized")
        }
    }
    
    pub fn close(&mut self) -> Result<(), &'static str> {
        match &mut self.implementation {
            Some(SocketImpl::Tcp(tcp_socket)) => {
                let _fin = tcp_socket.close();
                // Would send FIN segment
                self.state = SocketState::Closing;
            }
            Some(SocketImpl::Udp(_)) => {
                self.state = SocketState::Closed;
            }
            None => {}
        }
        
        // Unregister socket
        if let Some(addr) = self.local_addr {
            unregister_socket(self.id, addr.port);
        }
        
        Ok(())
    }
    
    pub fn set_option(&mut self, option: SocketOption) -> Result<(), &'static str> {
        match option {
            SocketOption::ReuseAddr(val) => self.options.reuse_addr = val,
            SocketOption::ReusePort(val) => self.options.reuse_port = val,
            SocketOption::NoDelay(val) => self.options.no_delay = val,
            SocketOption::KeepAlive(val) => {
                self.options.keep_alive = val;
                if let Some(SocketImpl::Tcp(ref mut tcp)) = self.implementation {
                    tcp.tcb.keepalive_enabled = val;
                }
            }
            SocketOption::Linger(val) => self.options.linger = val,
            SocketOption::RecvBuffer(size) => self.options.recv_buffer = size,
            SocketOption::SendBuffer(size) => self.options.send_buffer = size,
            SocketOption::RecvTimeout(timeout) => self.options.recv_timeout = timeout,
            SocketOption::SendTimeout(timeout) => self.options.send_timeout = timeout,
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum SocketOption {
    ReuseAddr(bool),
    ReusePort(bool),
    NoDelay(bool),
    KeepAlive(bool),
    Linger(Option<u32>),
    RecvBuffer(usize),
    SendBuffer(usize),
    RecvTimeout(Option<u64>),
    SendTimeout(Option<u64>),
}

// Socket registry
lazy_static! {
    static ref SOCKET_REGISTRY: Mutex<BTreeMap<u16, Vec<(u32, SocketType)>>> = 
        Mutex::new(BTreeMap::new());
    static ref EPHEMERAL_PORT_COUNTER: AtomicU32 = AtomicU32::new(49152);
}

fn is_port_in_use(port: u16, socket_type: SocketType) -> bool {
    let registry = SOCKET_REGISTRY.lock();
    if let Some(sockets) = registry.get(&port) {
        for (_, stype) in sockets {
            if *stype == socket_type || socket_type == SocketType::Stream {
                return true;
            }
        }
    }
    false
}

fn register_socket(id: u32, port: u16, socket_type: SocketType) {
    let mut registry = SOCKET_REGISTRY.lock();
    registry.entry(port).or_insert_with(Vec::new).push((id, socket_type));
}

fn unregister_socket(id: u32, port: u16) {
    let mut registry = SOCKET_REGISTRY.lock();
    if let Some(sockets) = registry.get_mut(&port) {
        sockets.retain(|(sid, _)| *sid != id);
        if sockets.is_empty() {
            registry.remove(&port);
        }
    }
}

fn allocate_ephemeral_port() -> u16 {
    // Ephemeral port range: 49152-65535
    loop {
        let port = EPHEMERAL_PORT_COUNTER.fetch_add(1, Ordering::SeqCst) as u16;
        if port < 49152 {
            EPHEMERAL_PORT_COUNTER.store(49152, Ordering::SeqCst);
            continue;
        }
        
        if !is_port_in_use(port, SocketType::Stream) {
            return port;
        }
        
        if port >= 65535 {
            EPHEMERAL_PORT_COUNTER.store(49152, Ordering::SeqCst);
        }
    }
}

fn get_local_ip() -> Ipv4Address {
    // In real implementation, would get from network interface
    Ipv4Address::new(192, 168, 1, 100)
}

pub fn init() {
    crate::serial_println!("Socket layer initialized");
}