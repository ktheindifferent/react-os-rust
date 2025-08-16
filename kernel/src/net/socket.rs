// Socket API Implementation
use super::ip::Ipv4Address;

#[derive(Debug, Clone, Copy)]
pub struct SocketAddr {
    pub ip: Ipv4Address,
    pub port: u16,
}

pub struct Socket {
    pub addr: SocketAddr,
}

pub fn init() {
    crate::serial_println!("Socket layer initialized");
}