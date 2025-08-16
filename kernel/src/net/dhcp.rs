// DHCP (Dynamic Host Configuration Protocol) Client Implementation
use super::udp::{UdpSocket, PORT_DHCP_CLIENT, PORT_DHCP_SERVER};
use super::ip::Ipv4Address;
use super::ethernet::MacAddress;
use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;
use core::convert::TryInto;

// DHCP Message Types
const DHCP_DISCOVER: u8 = 1;
const DHCP_OFFER: u8 = 2;
const DHCP_REQUEST: u8 = 3;
const DHCP_DECLINE: u8 = 4;
const DHCP_ACK: u8 = 5;
const DHCP_NAK: u8 = 6;
const DHCP_RELEASE: u8 = 7;
const DHCP_INFORM: u8 = 8;

// DHCP Options
const DHCP_OPT_PAD: u8 = 0;
const DHCP_OPT_SUBNET_MASK: u8 = 1;
const DHCP_OPT_ROUTER: u8 = 3;
const DHCP_OPT_DNS_SERVER: u8 = 6;
const DHCP_OPT_HOSTNAME: u8 = 12;
const DHCP_OPT_DOMAIN_NAME: u8 = 15;
const DHCP_OPT_REQUESTED_IP: u8 = 50;
const DHCP_OPT_LEASE_TIME: u8 = 51;
const DHCP_OPT_MESSAGE_TYPE: u8 = 53;
const DHCP_OPT_SERVER_ID: u8 = 54;
const DHCP_OPT_PARAM_REQUEST: u8 = 55;
const DHCP_OPT_MAX_MESSAGE_SIZE: u8 = 57;
const DHCP_OPT_RENEWAL_TIME: u8 = 58;
const DHCP_OPT_REBINDING_TIME: u8 = 59;
const DHCP_OPT_CLIENT_ID: u8 = 61;
const DHCP_OPT_END: u8 = 255;

// DHCP Magic Cookie
const DHCP_MAGIC_COOKIE: [u8; 4] = [99, 130, 83, 99];

// DHCP Operation Codes
const BOOTREQUEST: u8 = 1;
const BOOTREPLY: u8 = 2;

// DHCP Hardware Types
const HTYPE_ETHERNET: u8 = 1;

// DHCP Flags
const FLAG_BROADCAST: u16 = 0x8000;

// DHCP Message Structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DhcpMessage {
    pub op: u8,           // Operation (BOOTREQUEST or BOOTREPLY)
    pub htype: u8,        // Hardware type
    pub hlen: u8,         // Hardware address length
    pub hops: u8,         // Hops
    pub xid: u32,         // Transaction ID
    pub secs: u16,        // Seconds elapsed
    pub flags: u16,       // Flags
    pub ciaddr: [u8; 4],  // Client IP address
    pub yiaddr: [u8; 4],  // Your (client) IP address
    pub siaddr: [u8; 4],  // Next server IP address
    pub giaddr: [u8; 4],  // Relay agent IP address
    pub chaddr: [u8; 16], // Client hardware address
    pub sname: [u8; 64],  // Server host name
    pub file: [u8; 128],  // Boot file name
    pub options: [u8; 312], // Options (including magic cookie)
}

impl DhcpMessage {
    pub fn new_discover(mac_addr: MacAddress, xid: u32) -> Self {
        let mut msg = Self::new_base(mac_addr, xid);
        msg.op = BOOTREQUEST;
        msg.flags = FLAG_BROADCAST.to_be();
        
        // Add options
        let mut options = Vec::new();
        options.extend_from_slice(&DHCP_MAGIC_COOKIE);
        
        // Message type: DISCOVER
        options.push(DHCP_OPT_MESSAGE_TYPE);
        options.push(1);
        options.push(DHCP_DISCOVER);
        
        // Client ID
        options.push(DHCP_OPT_CLIENT_ID);
        options.push(7);
        options.push(HTYPE_ETHERNET);
        options.extend_from_slice(mac_addr.as_bytes());
        
        // Parameter request list
        options.push(DHCP_OPT_PARAM_REQUEST);
        options.push(4);
        options.push(DHCP_OPT_SUBNET_MASK);
        options.push(DHCP_OPT_ROUTER);
        options.push(DHCP_OPT_DNS_SERVER);
        options.push(DHCP_OPT_DOMAIN_NAME);
        
        // End option
        options.push(DHCP_OPT_END);
        
        // Copy options to message
        for (i, byte) in options.iter().enumerate() {
            if i < msg.options.len() {
                msg.options[i] = *byte;
            }
        }
        
        msg
    }
    
    pub fn new_request(
        mac_addr: MacAddress,
        xid: u32,
        requested_ip: Ipv4Address,
        server_ip: Ipv4Address,
    ) -> Self {
        let mut msg = Self::new_base(mac_addr, xid);
        msg.op = BOOTREQUEST;
        msg.flags = FLAG_BROADCAST.to_be();
        
        // Add options
        let mut options = Vec::new();
        options.extend_from_slice(&DHCP_MAGIC_COOKIE);
        
        // Message type: REQUEST
        options.push(DHCP_OPT_MESSAGE_TYPE);
        options.push(1);
        options.push(DHCP_REQUEST);
        
        // Requested IP address
        options.push(DHCP_OPT_REQUESTED_IP);
        options.push(4);
        options.extend_from_slice(requested_ip.as_bytes());
        
        // Server identifier
        options.push(DHCP_OPT_SERVER_ID);
        options.push(4);
        options.extend_from_slice(server_ip.as_bytes());
        
        // Client ID
        options.push(DHCP_OPT_CLIENT_ID);
        options.push(7);
        options.push(HTYPE_ETHERNET);
        options.extend_from_slice(mac_addr.as_bytes());
        
        // Parameter request list
        options.push(DHCP_OPT_PARAM_REQUEST);
        options.push(4);
        options.push(DHCP_OPT_SUBNET_MASK);
        options.push(DHCP_OPT_ROUTER);
        options.push(DHCP_OPT_DNS_SERVER);
        options.push(DHCP_OPT_DOMAIN_NAME);
        
        // End option
        options.push(DHCP_OPT_END);
        
        // Copy options to message
        for (i, byte) in options.iter().enumerate() {
            if i < msg.options.len() {
                msg.options[i] = *byte;
            }
        }
        
        msg
    }
    
    fn new_base(mac_addr: MacAddress, xid: u32) -> Self {
        let mut msg = Self {
            op: 0,
            htype: HTYPE_ETHERNET,
            hlen: 6,
            hops: 0,
            xid: xid.to_be(),
            secs: 0,
            flags: 0,
            ciaddr: [0; 4],
            yiaddr: [0; 4],
            siaddr: [0; 4],
            giaddr: [0; 4],
            chaddr: [0; 16],
            sname: [0; 64],
            file: [0; 128],
            options: [0; 312],
        };
        
        // Set client hardware address
        let mac_bytes = mac_addr.as_bytes();
        for i in 0..6 {
            msg.chaddr[i] = mac_bytes[i];
        }
        
        msg
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(548);
        
        bytes.push(self.op);
        bytes.push(self.htype);
        bytes.push(self.hlen);
        bytes.push(self.hops);
        bytes.extend_from_slice(&self.xid.to_be_bytes());
        bytes.extend_from_slice(&self.secs.to_be_bytes());
        bytes.extend_from_slice(&self.flags.to_be_bytes());
        bytes.extend_from_slice(&self.ciaddr);
        bytes.extend_from_slice(&self.yiaddr);
        bytes.extend_from_slice(&self.siaddr);
        bytes.extend_from_slice(&self.giaddr);
        bytes.extend_from_slice(&self.chaddr);
        bytes.extend_from_slice(&self.sname);
        bytes.extend_from_slice(&self.file);
        bytes.extend_from_slice(&self.options);
        
        bytes
    }
    
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 240 {
            return None;
        }
        
        let msg = Self {
            op: data[0],
            htype: data[1],
            hlen: data[2],
            hops: data[3],
            xid: u32::from_be_bytes(data[4..8].try_into().ok()?),
            secs: u16::from_be_bytes(data[8..10].try_into().ok()?),
            flags: u16::from_be_bytes(data[10..12].try_into().ok()?),
            ciaddr: data[12..16].try_into().ok()?,
            yiaddr: data[16..20].try_into().ok()?,
            siaddr: data[20..24].try_into().ok()?,
            giaddr: data[24..28].try_into().ok()?,
            chaddr: data[28..44].try_into().ok()?,
            sname: data[44..108].try_into().ok()?,
            file: data[108..236].try_into().ok()?,
            options: if data.len() >= 548 {
                data[236..548].try_into().ok()?
            } else {
                let mut opts = [0u8; 312];
                let copy_len = core::cmp::min(data.len() - 236, 312);
                opts[..copy_len].copy_from_slice(&data[236..236 + copy_len]);
                opts
            },
        };
        
        Some(msg)
    }
    
    pub fn parse_options(&self) -> DhcpOptions {
        let mut opts = DhcpOptions::default();
        
        // Check magic cookie
        if self.options[0..4] != DHCP_MAGIC_COOKIE {
            return opts;
        }
        
        let mut i = 4;
        while i < self.options.len() {
            let opt_type = self.options[i];
            if opt_type == DHCP_OPT_END {
                break;
            }
            if opt_type == DHCP_OPT_PAD {
                i += 1;
                continue;
            }
            
            if i + 1 >= self.options.len() {
                break;
            }
            
            let opt_len = self.options[i + 1] as usize;
            if i + 2 + opt_len > self.options.len() {
                break;
            }
            
            let opt_data = &self.options[i + 2..i + 2 + opt_len];
            
            match opt_type {
                DHCP_OPT_MESSAGE_TYPE => {
                    if opt_len == 1 {
                        opts.message_type = Some(opt_data[0]);
                    }
                }
                DHCP_OPT_SUBNET_MASK => {
                    if opt_len == 4 {
                        opts.subnet_mask = Ipv4Address::from_bytes(opt_data);
                    }
                }
                DHCP_OPT_ROUTER => {
                    if opt_len >= 4 {
                        opts.router = Ipv4Address::from_bytes(&opt_data[0..4]);
                    }
                }
                DHCP_OPT_DNS_SERVER => {
                    if opt_len >= 4 {
                        let mut servers = Vec::new();
                        for j in (0..opt_len).step_by(4) {
                            if let Some(addr) = Ipv4Address::from_bytes(&opt_data[j..j + 4]) {
                                servers.push(addr);
                            }
                        }
                        opts.dns_servers = servers;
                    }
                }
                DHCP_OPT_DOMAIN_NAME => {
                    opts.domain_name = String::from_utf8_lossy(opt_data).into_owned();
                }
                DHCP_OPT_LEASE_TIME => {
                    if opt_len == 4 {
                        opts.lease_time = Some(u32::from_be_bytes(opt_data.try_into().unwrap()));
                    }
                }
                DHCP_OPT_SERVER_ID => {
                    if opt_len == 4 {
                        opts.server_id = Ipv4Address::from_bytes(opt_data);
                    }
                }
                DHCP_OPT_RENEWAL_TIME => {
                    if opt_len == 4 {
                        opts.renewal_time = Some(u32::from_be_bytes(opt_data.try_into().unwrap()));
                    }
                }
                DHCP_OPT_REBINDING_TIME => {
                    if opt_len == 4 {
                        opts.rebinding_time = Some(u32::from_be_bytes(opt_data.try_into().unwrap()));
                    }
                }
                _ => {}
            }
            
            i += 2 + opt_len;
        }
        
        opts
    }
}

// DHCP Options Structure
#[derive(Debug, Default)]
pub struct DhcpOptions {
    pub message_type: Option<u8>,
    pub subnet_mask: Option<Ipv4Address>,
    pub router: Option<Ipv4Address>,
    pub dns_servers: Vec<Ipv4Address>,
    pub domain_name: String,
    pub lease_time: Option<u32>,
    pub server_id: Option<Ipv4Address>,
    pub renewal_time: Option<u32>,
    pub rebinding_time: Option<u32>,
}

// DHCP Client State
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DhcpState {
    Init,
    Selecting,
    Requesting,
    Bound,
    Renewing,
    Rebinding,
}

// DHCP Client Configuration
#[derive(Debug, Clone)]
pub struct DhcpConfig {
    pub ip_address: Option<Ipv4Address>,
    pub subnet_mask: Option<Ipv4Address>,
    pub gateway: Option<Ipv4Address>,
    pub dns_servers: Vec<Ipv4Address>,
    pub domain_name: String,
    pub lease_time: u32,
    pub renewal_time: u32,
    pub rebinding_time: u32,
    pub server_ip: Option<Ipv4Address>,
}

impl Default for DhcpConfig {
    fn default() -> Self {
        Self {
            ip_address: None,
            subnet_mask: None,
            gateway: None,
            dns_servers: Vec::new(),
            domain_name: String::new(),
            lease_time: 0,
            renewal_time: 0,
            rebinding_time: 0,
            server_ip: None,
        }
    }
}

// DHCP Client
pub struct DhcpClient {
    state: DhcpState,
    mac_addr: MacAddress,
    xid: u32,
    config: DhcpConfig,
    socket: Option<UdpSocket>,
}

impl DhcpClient {
    pub fn new(mac_addr: MacAddress) -> Self {
        Self {
            state: DhcpState::Init,
            mac_addr,
            xid: Self::generate_xid(),
            config: DhcpConfig::default(),
            socket: None,
        }
    }
    
    fn generate_xid() -> u32 {
        // Simple XID generation (should use random in production)
        static mut COUNTER: u32 = 0x12345678;
        unsafe {
            COUNTER = COUNTER.wrapping_add(0x1000);
            COUNTER
        }
    }
    
    pub fn start(&mut self) -> Result<(), &'static str> {
        // Bind UDP socket
        super::udp::bind(PORT_DHCP_CLIENT)?;
        
        // Send DHCP DISCOVER
        self.send_discover()?;
        self.state = DhcpState::Selecting;
        
        crate::serial_println!("DHCP: Discovering DHCP servers...");
        Ok(())
    }
    
    fn send_discover(&mut self) -> Result<(), &'static str> {
        let msg = DhcpMessage::new_discover(self.mac_addr, self.xid);
        let data = msg.to_bytes();
        
        // Send to broadcast address
        super::udp::send_to(
            PORT_DHCP_CLIENT,
            data,
            Ipv4Address::BROADCAST,
            PORT_DHCP_SERVER,
        )?;
        
        Ok(())
    }
    
    fn send_request(&mut self, offered_ip: Ipv4Address, server_ip: Ipv4Address) -> Result<(), &'static str> {
        let msg = DhcpMessage::new_request(self.mac_addr, self.xid, offered_ip, server_ip);
        let data = msg.to_bytes();
        
        // Send to broadcast address
        super::udp::send_to(
            PORT_DHCP_CLIENT,
            data,
            Ipv4Address::BROADCAST,
            PORT_DHCP_SERVER,
        )?;
        
        self.state = DhcpState::Requesting;
        Ok(())
    }
    
    pub fn process_message(&mut self, data: &[u8]) -> Result<(), &'static str> {
        let msg = DhcpMessage::from_bytes(data).ok_or("Invalid DHCP message")?;
        
        // Check transaction ID
        if msg.xid != self.xid.to_be() {
            return Ok(()); // Not our message
        }
        
        let options = msg.parse_options();
        
        match self.state {
            DhcpState::Selecting => {
                if options.message_type == Some(DHCP_OFFER) {
                    crate::serial_println!("DHCP: Received OFFER");
                    
                    // Extract offered IP
                    let offered_ip = Ipv4Address::from_bytes(&msg.yiaddr)
                        .ok_or("Invalid offered IP")?;
                    
                    let server_ip = options.server_id
                        .ok_or("No server ID in OFFER")?;
                    
                    crate::serial_println!("DHCP: Offered IP: {}, Server: {}", 
                        offered_ip, server_ip);
                    
                    // Send REQUEST
                    self.send_request(offered_ip, server_ip)?;
                }
            }
            
            DhcpState::Requesting => {
                if options.message_type == Some(DHCP_ACK) {
                    crate::serial_println!("DHCP: Received ACK");
                    
                    // Update configuration
                    self.config.ip_address = Ipv4Address::from_bytes(&msg.yiaddr);
                    self.config.subnet_mask = options.subnet_mask;
                    self.config.gateway = options.router;
                    self.config.dns_servers = options.dns_servers;
                    self.config.domain_name = options.domain_name;
                    self.config.lease_time = options.lease_time.unwrap_or(86400);
                    self.config.renewal_time = options.renewal_time
                        .unwrap_or(self.config.lease_time / 2);
                    self.config.rebinding_time = options.rebinding_time
                        .unwrap_or(self.config.lease_time * 7 / 8);
                    self.config.server_ip = options.server_id;
                    
                    self.state = DhcpState::Bound;
                    
                    // Apply configuration
                    self.apply_configuration()?;
                    
                    crate::serial_println!("DHCP: Configuration complete");
                    if let Some(ip) = self.config.ip_address {
                        crate::serial_println!("  IP Address: {}", ip);
                    }
                    if let Some(mask) = self.config.subnet_mask {
                        crate::serial_println!("  Subnet Mask: {}", mask);
                    }
                    if let Some(gw) = self.config.gateway {
                        crate::serial_println!("  Gateway: {}", gw);
                    }
                    for dns in &self.config.dns_servers {
                        crate::serial_println!("  DNS Server: {}", dns);
                    }
                    if !self.config.domain_name.is_empty() {
                        crate::serial_println!("  Domain: {}", self.config.domain_name);
                    }
                    crate::serial_println!("  Lease Time: {} seconds", self.config.lease_time);
                    
                } else if options.message_type == Some(DHCP_NAK) {
                    crate::serial_println!("DHCP: Received NAK, restarting");
                    self.state = DhcpState::Init;
                    self.start()?;
                }
            }
            
            _ => {}
        }
        
        Ok(())
    }
    
    fn apply_configuration(&self) -> Result<(), &'static str> {
        // In a real implementation, this would:
        // 1. Configure the network interface with the IP address
        // 2. Set up routing table with the gateway
        // 3. Configure DNS resolver with DNS servers
        
        // For now, just store in global state
        if let Some(ip) = self.config.ip_address {
            DHCP_CLIENT_CONFIG.lock().ip_address = Some(ip);
        }
        if let Some(gw) = self.config.gateway {
            DHCP_CLIENT_CONFIG.lock().gateway = Some(gw);
        }
        DHCP_CLIENT_CONFIG.lock().dns_servers = self.config.dns_servers.clone();
        
        Ok(())
    }
    
    pub fn renew(&mut self) -> Result<(), &'static str> {
        if self.state != DhcpState::Bound {
            return Err("Not in bound state");
        }
        
        // Send REQUEST to renew lease
        if let (Some(ip), Some(server)) = (self.config.ip_address, self.config.server_ip) {
            self.send_request(ip, server)?;
            self.state = DhcpState::Renewing;
            crate::serial_println!("DHCP: Renewing lease");
        }
        
        Ok(())
    }
    
    pub fn release(&mut self) -> Result<(), &'static str> {
        // Send DHCP RELEASE message
        // Not implemented for simplicity
        self.state = DhcpState::Init;
        self.config = DhcpConfig::default();
        Ok(())
    }
}

// Global DHCP client state
lazy_static! {
    static ref DHCP_CLIENT: Mutex<Option<DhcpClient>> = Mutex::new(None);
    static ref DHCP_CLIENT_CONFIG: Mutex<DhcpConfig> = Mutex::new(DhcpConfig::default());
}

// Public API
pub fn start_dhcp_client() -> Result<(), &'static str> {
    let mac_addr = super::ethernet::get_mac_address();
    let mut client = DhcpClient::new(mac_addr);
    client.start()?;
    
    *DHCP_CLIENT.lock() = Some(client);
    Ok(())
}

pub fn process_dhcp_reply(data: &[u8]) -> Result<(), &'static str> {
    if let Some(client) = DHCP_CLIENT.lock().as_mut() {
        client.process_message(data)?;
    }
    Ok(())
}

pub fn get_dhcp_config() -> DhcpConfig {
    DHCP_CLIENT_CONFIG.lock().clone()
}

pub fn get_assigned_ip() -> Option<Ipv4Address> {
    DHCP_CLIENT_CONFIG.lock().ip_address
}

pub fn get_gateway() -> Option<Ipv4Address> {
    DHCP_CLIENT_CONFIG.lock().gateway
}

pub fn get_dns_servers() -> Vec<Ipv4Address> {
    DHCP_CLIENT_CONFIG.lock().dns_servers.clone()
}