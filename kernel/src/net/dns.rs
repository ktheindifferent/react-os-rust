// DNS (Domain Name System) Resolver Implementation
use super::udp::{self, PORT_DNS};
use super::ip::Ipv4Address;
use alloc::vec::{self, Vec};
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;
use core::convert::TryInto;

// DNS Header Flags
const QR_QUERY: u16 = 0x0000;
const QR_RESPONSE: u16 = 0x8000;
const OPCODE_QUERY: u16 = 0x0000;
const AA_FLAG: u16 = 0x0400;
const TC_FLAG: u16 = 0x0200;
const RD_FLAG: u16 = 0x0100;
const RA_FLAG: u16 = 0x0080;

// DNS Response Codes
const RCODE_NO_ERROR: u16 = 0x0000;
const RCODE_FORMAT_ERROR: u16 = 0x0001;
const RCODE_SERVER_FAILURE: u16 = 0x0002;
const RCODE_NAME_ERROR: u16 = 0x0003;
const RCODE_NOT_IMPLEMENTED: u16 = 0x0004;
const RCODE_REFUSED: u16 = 0x0005;

// DNS Record Types
const TYPE_A: u16 = 1;      // IPv4 address
const TYPE_NS: u16 = 2;     // Name server
const TYPE_CNAME: u16 = 5;  // Canonical name
const TYPE_SOA: u16 = 6;    // Start of authority
const TYPE_PTR: u16 = 12;   // Pointer
const TYPE_MX: u16 = 15;    // Mail exchange
const TYPE_TXT: u16 = 16;   // Text
const TYPE_AAAA: u16 = 28;  // IPv6 address
const TYPE_SRV: u16 = 33;   // Service

// DNS Classes
const CLASS_IN: u16 = 1;    // Internet

// DNS Header Structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DnsHeader {
    pub id: u16,
    pub flags: u16,
    pub qdcount: u16,  // Question count
    pub ancount: u16,  // Answer count
    pub nscount: u16,  // Authority count
    pub arcount: u16,  // Additional count
}

impl DnsHeader {
    pub fn new_query(id: u16, recursion: bool) -> Self {
        let mut flags = QR_QUERY | OPCODE_QUERY;
        if recursion {
            flags |= RD_FLAG;
        }
        
        Self {
            id: id.to_be(),
            flags: flags.to_be(),
            qdcount: 1u16.to_be(),
            ancount: 0,
            nscount: 0,
            arcount: 0,
        }
    }
    
    pub fn id(&self) -> u16 {
        u16::from_be(self.id)
    }
    
    pub fn flags(&self) -> u16 {
        u16::from_be(self.flags)
    }
    
    pub fn is_response(&self) -> bool {
        self.flags() & QR_RESPONSE != 0
    }
    
    pub fn response_code(&self) -> u16 {
        self.flags() & 0x000F
    }
    
    pub fn question_count(&self) -> u16 {
        u16::from_be(self.qdcount)
    }
    
    pub fn answer_count(&self) -> u16 {
        u16::from_be(self.ancount)
    }
}

// DNS Question Structure
pub struct DnsQuestion {
    pub name: String,
    pub qtype: u16,
    pub qclass: u16,
}

impl DnsQuestion {
    pub fn new(name: String, qtype: u16) -> Self {
        Self {
            name,
            qtype,
            qclass: CLASS_IN,
        }
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        // Encode domain name
        for label in self.name.split('.') {
            if label.is_empty() {
                continue;
            }
            bytes.push(label.len() as u8);
            bytes.extend_from_slice(label.as_bytes());
        }
        bytes.push(0); // Null terminator
        
        // Add type and class
        bytes.extend_from_slice(&self.qtype.to_be_bytes());
        bytes.extend_from_slice(&self.qclass.to_be_bytes());
        
        bytes
    }
}

// DNS Resource Record
#[derive(Debug, Clone)]
pub struct DnsRecord {
    pub name: String,
    pub rtype: u16,
    pub rclass: u16,
    pub ttl: u32,
    pub data: Vec<u8>,
}

impl DnsRecord {
    pub fn parse_a_record(&self) -> Option<Ipv4Address> {
        if self.rtype == TYPE_A && self.data.len() == 4 {
            Some(Ipv4Address::new(
                self.data[0],
                self.data[1],
                self.data[2],
                self.data[3],
            ))
        } else {
            None
        }
    }
    
    pub fn parse_cname(&self) -> Option<String> {
        if self.rtype == TYPE_CNAME {
            parse_domain_name(&self.data, 0).ok().map(|(name, _)| name)
        } else {
            None
        }
    }
}

// DNS Message
pub struct DnsMessage {
    pub header: DnsHeader,
    pub questions: Vec<DnsQuestion>,
    pub answers: Vec<DnsRecord>,
    pub authorities: Vec<DnsRecord>,
    pub additionals: Vec<DnsRecord>,
}

impl DnsMessage {
    pub fn new_query(id: u16, hostname: &str, qtype: u16) -> Self {
        let header = DnsHeader::new_query(id, true);
        let question = DnsQuestion::new(hostname.to_string(), qtype);
        
        let mut questions = Vec::new();
        questions.push(question);
        
        Self {
            header,
            questions,
            answers: Vec::new(),
            authorities: Vec::new(),
            additionals: Vec::new(),
        }
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        // Add header
        bytes.extend_from_slice(&self.header.id.to_be_bytes());
        bytes.extend_from_slice(&self.header.flags.to_be_bytes());
        bytes.extend_from_slice(&self.header.qdcount.to_be_bytes());
        bytes.extend_from_slice(&self.header.ancount.to_be_bytes());
        bytes.extend_from_slice(&self.header.nscount.to_be_bytes());
        bytes.extend_from_slice(&self.header.arcount.to_be_bytes());
        
        // Add questions
        for question in &self.questions {
            bytes.extend_from_slice(&question.to_bytes());
        }
        
        // Add answers, authorities, and additionals would go here
        
        bytes
    }
    
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 12 {
            return Err("DNS message too short");
        }
        
        // Parse header
        let header = DnsHeader {
            id: u16::from_be_bytes([data[0], data[1]]),
            flags: u16::from_be_bytes([data[2], data[3]]),
            qdcount: u16::from_be_bytes([data[4], data[5]]),
            ancount: u16::from_be_bytes([data[6], data[7]]),
            nscount: u16::from_be_bytes([data[8], data[9]]),
            arcount: u16::from_be_bytes([data[10], data[11]]),
        };
        
        let mut offset = 12;
        let mut questions = Vec::new();
        let mut answers = Vec::new();
        
        // Parse questions
        for _ in 0..header.question_count() {
            let (name, new_offset) = parse_domain_name(data, offset)?;
            offset = new_offset;
            
            if offset + 4 > data.len() {
                return Err("DNS question truncated");
            }
            
            let qtype = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let qclass = u16::from_be_bytes([data[offset + 2], data[offset + 3]]);
            offset += 4;
            
            questions.push(DnsQuestion {
                name,
                qtype,
                qclass,
            });
        }
        
        // Parse answers
        for _ in 0..header.answer_count() {
            let (record, new_offset) = parse_resource_record(data, offset)?;
            offset = new_offset;
            answers.push(record);
        }
        
        Ok(Self {
            header,
            questions,
            answers,
            authorities: Vec::new(),
            additionals: Vec::new(),
        })
    }
}

// Parse domain name from DNS message
fn parse_domain_name(data: &[u8], mut offset: usize) -> Result<(String, usize), &'static str> {
    let mut name = String::new();
    let mut jumped = false;
    let mut jump_offset = 0;
    let max_jumps = 5;
    let mut jump_count = 0;
    
    loop {
        if offset >= data.len() {
            return Err("DNS name out of bounds");
        }
        
        let len = data[offset] as usize;
        
        if len == 0 {
            // End of name
            offset += 1;
            break;
        } else if len & 0xC0 == 0xC0 {
            // Compression pointer
            if !jumped {
                jump_offset = offset + 2;
            }
            
            if offset + 1 >= data.len() {
                return Err("DNS compression pointer truncated");
            }
            
            let pointer = (((len & 0x3F) as usize) << 8) | (data[offset + 1] as usize);
            offset = pointer;
            jumped = true;
            
            jump_count += 1;
            if jump_count > max_jumps {
                return Err("DNS compression loop detected");
            }
        } else if len & 0xC0 != 0 {
            return Err("Invalid DNS label length");
        } else {
            // Normal label
            if offset + len + 1 > data.len() {
                return Err("DNS label truncated");
            }
            
            if !name.is_empty() {
                name.push('.');
            }
            
            for i in 1..=len {
                name.push(data[offset + i] as char);
            }
            
            offset += len + 1;
        }
    }
    
    if jumped {
        Ok((name, jump_offset))
    } else {
        Ok((name, offset))
    }
}

// Parse resource record
fn parse_resource_record(data: &[u8], offset: usize) -> Result<(DnsRecord, usize), &'static str> {
    let (name, mut offset) = parse_domain_name(data, offset)?;
    
    if offset + 10 > data.len() {
        return Err("DNS record header truncated");
    }
    
    let rtype = u16::from_be_bytes([data[offset], data[offset + 1]]);
    let rclass = u16::from_be_bytes([data[offset + 2], data[offset + 3]]);
    let ttl = u32::from_be_bytes([
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ]);
    let rdlength = u16::from_be_bytes([data[offset + 8], data[offset + 9]]) as usize;
    offset += 10;
    
    if offset + rdlength > data.len() {
        return Err("DNS record data truncated");
    }
    
    let rdata = data[offset..offset + rdlength].to_vec();
    offset += rdlength;
    
    Ok((
        DnsRecord {
            name,
            rtype,
            rclass,
            ttl,
            data: rdata,
        },
        offset,
    ))
}

// DNS Cache Entry
#[derive(Debug, Clone)]
struct DnsCacheEntry {
    ip_address: Ipv4Address,
    ttl: u32,
    timestamp: u64,
}

// DNS Resolver
pub struct DnsResolver {
    dns_servers: Vec<Ipv4Address>,
    cache: BTreeMap<String, DnsCacheEntry>,
    query_id: u16,
}

impl DnsResolver {
    pub fn new() -> Self {
        let mut dns_servers = Vec::new();
        dns_servers.push(Ipv4Address::new(8, 8, 8, 8));      // Google DNS
        dns_servers.push(Ipv4Address::new(8, 8, 4, 4));      // Google DNS secondary
        dns_servers.push(Ipv4Address::new(1, 1, 1, 1));      // Cloudflare DNS
        
        Self {
            dns_servers,
            cache: BTreeMap::new(),
            query_id: 1,
        }
    }
    
    pub fn set_dns_servers(&mut self, servers: Vec<Ipv4Address>) {
        if !servers.is_empty() {
            self.dns_servers = servers;
        }
    }
    
    pub fn resolve(&mut self, hostname: &str) -> Result<Ipv4Address, &'static str> {
        // Check if it's already an IP address
        if let Some(ip) = parse_ip_address(hostname) {
            return Ok(ip);
        }
        
        // Check cache
        if let Some(entry) = self.cache.get(hostname) {
            // Simple cache without TTL checking for now
            return Ok(entry.ip_address);
        }
        
        // Create DNS query
        let query = DnsMessage::new_query(self.query_id, hostname, TYPE_A);
        self.query_id = self.query_id.wrapping_add(1);
        
        let query_bytes = query.to_bytes();
        
        // Try each DNS server
        for &dns_server in &self.dns_servers {
            // Send query
            if let Ok(()) = udp::send_to(
                PORT_DNS + 1000, // Use a high port for client
                query_bytes.clone(),
                dns_server,
                PORT_DNS,
            ) {
                // Wait for response (simplified - would need timeout)
                if let Ok(Some((_, _, response_data))) = 
                    udp::recv_from(PORT_DNS + 1000) {
                    
                    // Parse response
                    if let Ok(response) = DnsMessage::from_bytes(&response_data) {
                        if response.header.id() == query.header.id() &&
                           response.header.is_response() &&
                           response.header.response_code() == 0 {
                            
                            // Look for A record in answers
                            for answer in &response.answers {
                                if let Some(ip) = answer.parse_a_record() {
                                    // Add to cache
                                    self.cache.insert(
                                        hostname.to_string(),
                                        DnsCacheEntry {
                                            ip_address: ip,
                                            ttl: answer.ttl,
                                            timestamp: 0, // Would use actual time
                                        },
                                    );
                                    
                                    crate::serial_println!("DNS: Resolved {} to {}", 
                                        hostname, ip);
                                    return Ok(ip);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Err("DNS resolution failed")
    }
    
    pub fn resolve_reverse(&mut self, ip: Ipv4Address) -> Result<String, &'static str> {
        // Create reverse DNS query (in-addr.arpa)
        let octets = ip.octets();
        let reverse_name = alloc::format!(
            "{}.{}.{}.{}.in-addr.arpa",
            octets[3], octets[2], octets[1], octets[0]
        );
        
        let query = DnsMessage::new_query(self.query_id, &reverse_name, TYPE_PTR);
        self.query_id = self.query_id.wrapping_add(1);
        
        // Similar query process as forward resolution
        // Returns the PTR record if found
        
        Err("Reverse DNS not fully implemented")
    }
    
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

// Parse IP address string
fn parse_ip_address(s: &str) -> Option<Ipv4Address> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    
    let mut octets = [0u8; 4];
    for (i, part) in parts.iter().enumerate() {
        match part.parse::<u8>() {
            Ok(n) => octets[i] = n,
            Err(_) => return None,
        }
    }
    
    Some(Ipv4Address::new(octets[0], octets[1], octets[2], octets[3]))
}

// Global DNS resolver
lazy_static! {
    static ref DNS_RESOLVER: Mutex<DnsResolver> = Mutex::new(DnsResolver::new());
}

// Public API
pub fn resolve_hostname(hostname: &str) -> Option<Ipv4Address> {
    match DNS_RESOLVER.lock().resolve(hostname) {
        Ok(ip) => Some(ip),
        Err(e) => {
            crate::serial_println!("DNS resolution failed: {}", e);
            None
        }
    }
}

pub fn set_dns_servers(servers: Vec<Ipv4Address>) {
    DNS_RESOLVER.lock().set_dns_servers(servers);
}

pub fn resolve_hostname_with_type(hostname: &str, record_type: u16) -> Result<Vec<DnsRecord>, &'static str> {
    // Extended resolution for different record types
    // Would implement similar to resolve() but return different record types
    Err("Extended DNS resolution not implemented")
}

// Well-known DNS record types for export
pub const DNS_TYPE_A: u16 = TYPE_A;
pub const DNS_TYPE_AAAA: u16 = TYPE_AAAA;
pub const DNS_TYPE_CNAME: u16 = TYPE_CNAME;
pub const DNS_TYPE_MX: u16 = TYPE_MX;
pub const DNS_TYPE_TXT: u16 = TYPE_TXT;
pub const DNS_TYPE_NS: u16 = TYPE_NS;
pub const DNS_TYPE_PTR: u16 = TYPE_PTR;
pub const DNS_TYPE_SRV: u16 = TYPE_SRV;