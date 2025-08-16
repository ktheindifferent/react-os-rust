// TCP (Transmission Control Protocol) Implementation
use super::ip::{IpPacket, Ipv4Address, IP_PROTO_TCP};
use alloc::vec::Vec;
use alloc::collections::{BTreeMap, VecDeque};
use spin::Mutex;
use lazy_static::lazy_static;
use core::cmp::{min, max};
use core::time::Duration;
use alloc::boxed::Box;

// TCP Header flags
const TCP_FIN: u8 = 0x01;
const TCP_SYN: u8 = 0x02;
const TCP_RST: u8 = 0x04;
const TCP_PSH: u8 = 0x08;
const TCP_ACK: u8 = 0x10;
const TCP_URG: u8 = 0x20;
const TCP_ECE: u8 = 0x40;
const TCP_CWR: u8 = 0x80;

// TCP Options
const TCP_OPT_END: u8 = 0;
const TCP_OPT_NOP: u8 = 1;
const TCP_OPT_MSS: u8 = 2;
const TCP_OPT_WINDOW_SCALE: u8 = 3;
const TCP_OPT_SACK_PERMITTED: u8 = 4;
const TCP_OPT_SACK: u8 = 5;
const TCP_OPT_TIMESTAMP: u8 = 8;

// TCP Constants
const TCP_MSS_DEFAULT: u16 = 536;
const TCP_MSS_ETHERNET: u16 = 1460;
const TCP_WINDOW_DEFAULT: u16 = 65535;
const TCP_WINDOW_SCALE_MAX: u8 = 14;
const TCP_MAX_RETRIES: u32 = 15;
const TCP_RTO_MIN: u64 = 200;  // 200ms
const TCP_RTO_MAX: u64 = 120000;  // 120s
const TCP_RTO_INITIAL: u64 = 1000;  // 1s
const TCP_TIME_WAIT_DURATION: u64 = 120000;  // 2*MSL = 120s
const TCP_KEEPALIVE_INTERVAL: u64 = 7200000;  // 2 hours
const TCP_KEEPALIVE_PROBES: u32 = 9;
const TCP_PERSIST_MIN: u64 = 5000;  // 5s
const TCP_PERSIST_MAX: u64 = 60000;  // 60s

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
    pub fn to_bytes(&self) -> [u8; 20] {
        let mut bytes = [0u8; 20];
        bytes[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        bytes[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.seq_num.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.ack_num.to_be_bytes());
        bytes[12..14].copy_from_slice(&self.data_offset_flags.to_be_bytes());
        bytes[14..16].copy_from_slice(&self.window.to_be_bytes());
        bytes[16..18].copy_from_slice(&self.checksum.to_be_bytes());
        bytes[18..20].copy_from_slice(&self.urgent_ptr.to_be_bytes());
        bytes
    }
    
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

// Congestion Control Implementation
impl CongestionControl {
    pub fn new(algorithm: CongestionAlgorithm) -> Self {
        Self {
            algorithm,
            state: CongestionState::SlowStart,
            cwnd: 10 * TCP_MSS_ETHERNET as u32,  // Initial window (RFC 6928)
            ssthresh: u32::MAX,
            rwnd: TCP_WINDOW_DEFAULT as u32,
            snd_wnd: 10 * TCP_MSS_ETHERNET as u32,
            recovery_point: 0,
            dup_ack_count: 0,
            bytes_in_flight: 0,
            srtt: 0,
            rttvar: 0,
            rto: TCP_RTO_INITIAL,
            rtt_seq: 0,
            rtt_time: 0,
            cubic_last_max: 0,
            cubic_epoch_start: 0,
            cubic_k: 0,
            cubic_c: 0.4,
            cubic_beta: 0.7,
        }
    }
    
    // On ACK received - increase congestion window
    pub fn on_ack(&mut self, acked_bytes: u32, mss: u16) {
        self.bytes_in_flight = self.bytes_in_flight.saturating_sub(acked_bytes);
        
        match self.algorithm {
            CongestionAlgorithm::Reno => self.reno_on_ack(acked_bytes, mss),
            CongestionAlgorithm::Cubic => self.cubic_on_ack(acked_bytes, mss),
            CongestionAlgorithm::Bbr => {} // BBR implementation would go here
        }
        
        self.update_send_window();
    }
    
    // Reno congestion control
    fn reno_on_ack(&mut self, acked_bytes: u32, mss: u16) {
        match self.state {
            CongestionState::SlowStart => {
                // Increase by one MSS per ACK
                self.cwnd += acked_bytes;
                if self.cwnd >= self.ssthresh {
                    self.state = CongestionState::CongestionAvoidance;
                }
            }
            CongestionState::CongestionAvoidance => {
                // Increase by MSS^2/cwnd per ACK (approximately 1 MSS per RTT)
                let increment = (mss as u32 * acked_bytes) / self.cwnd;
                self.cwnd += max(1, increment);
            }
            CongestionState::FastRecovery => {
                // Increase by MSS for each duplicate ACK
                self.cwnd += mss as u32;
            }
            CongestionState::Loss => {
                // Exit loss recovery
                self.state = CongestionState::SlowStart;
            }
        }
    }
    
    // CUBIC congestion control
    fn cubic_on_ack(&mut self, acked_bytes: u32, mss: u16) {
        let now = TcpControlBlock::get_timestamp();
        
        match self.state {
            CongestionState::SlowStart => {
                self.cwnd += acked_bytes;
                if self.cwnd >= self.ssthresh {
                    self.state = CongestionState::CongestionAvoidance;
                    self.cubic_epoch_start = now;
                }
            }
            CongestionState::CongestionAvoidance => {
                // CUBIC window increase function
                let t = ((now - self.cubic_epoch_start) as f32) / 1000.0; // Convert to seconds
                let k = (self.cubic_last_max as f32 * (1.0 - self.cubic_beta) / self.cubic_c).powf(1.0/3.0);
                let w_cubic = self.cubic_c * (t - k).powi(3) + self.cubic_last_max as f32;
                
                // Friendly mode - ensure fairness with Reno
                let w_reno = self.cwnd + (acked_bytes * mss as u32) / self.cwnd;
                
                // Use maximum of CUBIC and Reno
                self.cwnd = max(w_cubic as u32, w_reno);
            }
            CongestionState::FastRecovery => {
                self.cwnd += mss as u32;
            }
            CongestionState::Loss => {
                self.state = CongestionState::SlowStart;
                self.cubic_epoch_start = now;
            }
        }
    }
    
    // On packet loss detected
    pub fn on_loss(&mut self) {
        match self.algorithm {
            CongestionAlgorithm::Reno => {
                self.ssthresh = max(self.cwnd / 2, 2 * TCP_MSS_ETHERNET as u32);
                self.cwnd = 1 * TCP_MSS_ETHERNET as u32;
                self.state = CongestionState::Loss;
            }
            CongestionAlgorithm::Cubic => {
                self.cubic_last_max = self.cwnd;
                self.cwnd = (self.cwnd as f32 * self.cubic_beta) as u32;
                self.ssthresh = self.cwnd;
                self.state = CongestionState::Loss;
            }
            CongestionAlgorithm::Bbr => {} // BBR implementation
        }
        
        self.dup_ack_count = 0;
        self.update_send_window();
    }
    
    // On duplicate ACK (fast retransmit/recovery)
    pub fn on_dup_ack(&mut self, mss: u16) -> bool {
        self.dup_ack_count += 1;
        
        if self.dup_ack_count == 3 {
            // Enter fast recovery
            match self.algorithm {
                CongestionAlgorithm::Reno => {
                    self.ssthresh = max(self.cwnd / 2, 2 * mss as u32);
                    self.cwnd = self.ssthresh + 3 * mss as u32;
                }
                CongestionAlgorithm::Cubic => {
                    self.cubic_last_max = self.cwnd;
                    self.cwnd = (self.cwnd as f32 * self.cubic_beta) as u32 + 3 * mss as u32;
                    self.ssthresh = self.cwnd - 3 * mss as u32;
                }
                CongestionAlgorithm::Bbr => {}
            }
            
            self.state = CongestionState::FastRecovery;
            self.update_send_window();
            return true; // Trigger fast retransmit
        } else if self.dup_ack_count > 3 && self.state == CongestionState::FastRecovery {
            // Additional duplicate ACKs during fast recovery
            self.cwnd += mss as u32;
            self.update_send_window();
        }
        
        false
    }
    
    // Exit fast recovery
    pub fn exit_fast_recovery(&mut self) {
        if self.state == CongestionState::FastRecovery {
            self.cwnd = self.ssthresh;
            self.state = CongestionState::CongestionAvoidance;
            self.dup_ack_count = 0;
            self.update_send_window();
        }
    }
    
    // Update effective send window
    fn update_send_window(&mut self) {
        self.snd_wnd = min(self.cwnd, self.rwnd);
    }
    
    // On sending data
    pub fn on_send(&mut self, bytes: u32) {
        self.bytes_in_flight += bytes;
    }
}

// TCP Option structure
#[derive(Debug, Clone)]
pub enum TcpOption {
    End,
    Nop,
    Mss(u16),
    WindowScale(u8),
    SackPermitted,
    Sack(Vec<(u32, u32)>),
    Timestamp(u32, u32),
}

// TCP Timer types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TcpTimer {
    Retransmission,
    Persist,
    KeepAlive,
    TimeWait,
    DelayedAck,
}

// Timer state
#[derive(Debug, Clone)]
pub struct TimerState {
    pub timer_type: TcpTimer,
    pub expires_at: u64,
    pub retry_count: u32,
}

// Congestion control state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CongestionState {
    SlowStart,
    CongestionAvoidance,
    FastRecovery,
    Loss,
}

// Congestion control algorithm
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CongestionAlgorithm {
    Reno,
    Cubic,
    Bbr,
}

// Congestion control variables
#[derive(Debug, Clone)]
pub struct CongestionControl {
    pub algorithm: CongestionAlgorithm,
    pub state: CongestionState,
    pub cwnd: u32,           // Congestion window
    pub ssthresh: u32,       // Slow start threshold
    pub rwnd: u32,           // Receiver window
    pub snd_wnd: u32,        // Send window (min of cwnd and rwnd)
    pub recovery_point: u32, // Recovery point for fast recovery
    pub dup_ack_count: u32,  // Duplicate ACK counter
    pub bytes_in_flight: u32,// Bytes currently in flight
    
    // RTT estimation
    pub srtt: u32,           // Smoothed RTT (in ms)
    pub rttvar: u32,         // RTT variance
    pub rto: u64,            // Retransmission timeout
    pub rtt_seq: u32,        // Sequence number for RTT measurement
    pub rtt_time: u64,       // Time when RTT measurement started
    
    // CUBIC specific
    pub cubic_last_max: u32, // Last maximum window size
    pub cubic_epoch_start: u64, // Time when current epoch started
    pub cubic_k: u32,        // K parameter
    pub cubic_c: f32,        // C parameter (0.4)
    pub cubic_beta: f32,     // Beta parameter (0.7)
}

// Out-of-order segment
#[derive(Debug, Clone)]
pub struct OutOfOrderSegment {
    pub seq: u32,
    pub data: Vec<u8>,
    pub fin: bool,
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
    pub snd_max: u32,  // Maximum sequence sent
    pub snd_wl1: u32,  // Sequence for last window update
    pub snd_wl2: u32,  // Ack for last window update
    pub rcv_nxt: u32,  // Receive next
    pub rcv_wnd: u32,  // Receive window
    pub rcv_up: u32,   // Receive urgent pointer
    pub iss: u32,      // Initial send sequence
    pub irs: u32,      // Initial receive sequence
    
    // Window scaling
    pub snd_wnd_scale: u8,  // Send window scale
    pub rcv_wnd_scale: u8,  // Receive window scale
    pub window_scaling_enabled: bool,
    
    // Buffers
    pub send_buffer: VecDeque<u8>,
    pub recv_buffer: VecDeque<u8>,
    pub retransmit_queue: VecDeque<(u32, Vec<u8>, u64)>, // (seq_num, data, timestamp)
    pub out_of_order: BTreeMap<u32, OutOfOrderSegment>,
    
    // Congestion control
    pub congestion: CongestionControl,
    
    // Timers
    pub timers: Vec<TimerState>,
    pub last_recv_time: u64,
    pub last_send_time: u64,
    pub time_wait_start: u64,
    
    // Options
    pub mss: u16,              // Maximum segment size
    pub peer_mss: u16,         // Peer's MSS
    pub sack_permitted: bool,  // SACK permitted
    pub timestamps_enabled: bool,
    pub ts_recent: u32,        // Recent timestamp
    pub ts_recent_age: u64,    // When ts_recent was updated
    
    // Statistics
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub segments_sent: u64,
    pub segments_received: u64,
    pub retransmissions: u64,
    
    // Keep-alive
    pub keepalive_enabled: bool,
    pub keepalive_time: u64,
    pub keepalive_probes_sent: u32,
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
            snd_max: iss,
            snd_wl1: 0,
            snd_wl2: 0,
            rcv_nxt: 0,
            rcv_wnd: TCP_WINDOW_DEFAULT as u32,
            rcv_up: 0,
            iss,
            irs: 0,
            snd_wnd_scale: 0,
            rcv_wnd_scale: 7,  // Allow scaling up to 128x
            window_scaling_enabled: false,
            send_buffer: VecDeque::with_capacity(65536),
            recv_buffer: VecDeque::with_capacity(65536),
            retransmit_queue: VecDeque::new(),
            out_of_order: BTreeMap::new(),
            congestion: CongestionControl::new(CongestionAlgorithm::Reno),
            timers: Vec::new(),
            last_recv_time: Self::get_timestamp(),
            last_send_time: Self::get_timestamp(),
            time_wait_start: 0,
            mss: TCP_MSS_ETHERNET,
            peer_mss: TCP_MSS_DEFAULT,
            sack_permitted: false,
            timestamps_enabled: false,
            ts_recent: 0,
            ts_recent_age: 0,
            bytes_sent: 0,
            bytes_received: 0,
            segments_sent: 0,
            segments_received: 0,
            retransmissions: 0,
            keepalive_enabled: false,
            keepalive_time: TCP_KEEPALIVE_INTERVAL,
            keepalive_probes_sent: 0,
        }
    }
    
    fn generate_isn() -> u32 {
        // RFC 6528 compliant ISN generation with cryptographic hash
        // In production, use strong randomness + hash of connection tuple
        static mut COUNTER: u32 = 0x1000;
        unsafe {
            COUNTER = COUNTER.wrapping_add(64000);
            // Should hash (local_ip, local_port, remote_ip, remote_port, secret, timestamp)
            COUNTER ^ (Self::get_timestamp() as u32)
        }
    }
    
    fn get_timestamp() -> u64 {
        // In a real OS, this would return system uptime in milliseconds
        static mut TIMESTAMP: u64 = 0;
        unsafe {
            TIMESTAMP += 1;
            TIMESTAMP
        }
    }
    
    // Timer management
    pub fn set_timer(&mut self, timer_type: TcpTimer, timeout: u64) {
        let expires_at = Self::get_timestamp() + timeout;
        
        // Remove existing timer of same type
        self.timers.retain(|t| t.timer_type != timer_type);
        
        self.timers.push(TimerState {
            timer_type,
            expires_at,
            retry_count: 0,
        });
    }
    
    pub fn cancel_timer(&mut self, timer_type: TcpTimer) {
        self.timers.retain(|t| t.timer_type != timer_type);
    }
    
    pub fn check_timers(&mut self) -> Vec<TcpTimer> {
        let now = Self::get_timestamp();
        let mut expired = Vec::new();
        
        self.timers.retain(|timer| {
            if timer.expires_at <= now {
                expired.push(timer.timer_type);
                false
            } else {
                true
            }
        });
        
        expired
    }
    
    // RTT estimation (Jacobson/Karels algorithm)
    pub fn update_rtt(&mut self, measured_rtt: u32) {
        if self.congestion.srtt == 0 {
            // First measurement
            self.congestion.srtt = measured_rtt;
            self.congestion.rttvar = measured_rtt / 2;
        } else {
            // Update smoothed RTT and variance
            let diff = if measured_rtt > self.congestion.srtt {
                measured_rtt - self.congestion.srtt
            } else {
                self.congestion.srtt - measured_rtt
            };
            
            self.congestion.rttvar = (3 * self.congestion.rttvar + diff) / 4;
            self.congestion.srtt = (7 * self.congestion.srtt + measured_rtt) / 8;
        }
        
        // Calculate RTO
        let rto = self.congestion.srtt + max(1, 4 * self.congestion.rttvar);
        self.congestion.rto = max(TCP_RTO_MIN, min(rto as u64, TCP_RTO_MAX));
    }
    
    // Window management
    pub fn effective_send_window(&self) -> u32 {
        min(self.congestion.cwnd, self.congestion.rwnd).saturating_sub(self.congestion.bytes_in_flight)
    }
    
    pub fn update_send_window(&mut self, window: u16, seq: u32, ack: u32) {
        // Window update algorithm (RFC 793)
        if self.snd_wl1 < seq || (self.snd_wl1 == seq && self.snd_wl2 <= ack) {
            self.congestion.rwnd = (window as u32) << self.snd_wnd_scale;
            self.snd_wl1 = seq;
            self.snd_wl2 = ack;
        }
    }
    
    // Parse TCP options
    pub fn parse_options(options: &[u8]) -> Vec<TcpOption> {
        let mut opts = Vec::new();
        let mut i = 0;
        
        while i < options.len() {
            match options[i] {
                TCP_OPT_END => break,
                TCP_OPT_NOP => {
                    opts.push(TcpOption::Nop);
                    i += 1;
                }
                TCP_OPT_MSS => {
                    if i + 3 < options.len() && options[i + 1] == 4 {
                        let mss = u16::from_be_bytes([options[i + 2], options[i + 3]]);
                        opts.push(TcpOption::Mss(mss));
                        i += 4;
                    } else {
                        break;
                    }
                }
                TCP_OPT_WINDOW_SCALE => {
                    if i + 2 < options.len() && options[i + 1] == 3 {
                        opts.push(TcpOption::WindowScale(options[i + 2]));
                        i += 3;
                    } else {
                        break;
                    }
                }
                TCP_OPT_SACK_PERMITTED => {
                    if i + 1 < options.len() && options[i + 1] == 2 {
                        opts.push(TcpOption::SackPermitted);
                        i += 2;
                    } else {
                        break;
                    }
                }
                TCP_OPT_TIMESTAMP => {
                    if i + 9 < options.len() && options[i + 1] == 10 {
                        let ts_val = u32::from_be_bytes([
                            options[i + 2], options[i + 3], options[i + 4], options[i + 5]
                        ]);
                        let ts_ecr = u32::from_be_bytes([
                            options[i + 6], options[i + 7], options[i + 8], options[i + 9]
                        ]);
                        opts.push(TcpOption::Timestamp(ts_val, ts_ecr));
                        i += 10;
                    } else {
                        break;
                    }
                }
                _ => {
                    // Unknown option, try to skip based on length
                    if i + 1 < options.len() {
                        let len = options[i + 1] as usize;
                        if len >= 2 {
                            i += len;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        
        opts
    }
    
    // Build TCP options
    pub fn build_options(&self, syn: bool, ack: bool) -> Vec<u8> {
        let mut options = Vec::new();
        
        if syn {
            // MSS option
            options.push(TCP_OPT_MSS);
            options.push(4);
            options.extend_from_slice(&self.mss.to_be_bytes());
            
            // Window scale option
            if self.window_scaling_enabled {
                options.push(TCP_OPT_NOP);
                options.push(TCP_OPT_WINDOW_SCALE);
                options.push(3);
                options.push(self.rcv_wnd_scale);
            }
            
            // SACK permitted
            if self.sack_permitted {
                options.push(TCP_OPT_NOP);
                options.push(TCP_OPT_NOP);
                options.push(TCP_OPT_SACK_PERMITTED);
                options.push(2);
            }
            
            // Timestamps
            if self.timestamps_enabled {
                options.push(TCP_OPT_NOP);
                options.push(TCP_OPT_NOP);
                options.push(TCP_OPT_TIMESTAMP);
                options.push(10);
                let ts = Self::get_timestamp() as u32;
                options.extend_from_slice(&ts.to_be_bytes());
                options.extend_from_slice(&self.ts_recent.to_be_bytes());
            }
        } else if self.timestamps_enabled {
            // Include timestamps in all segments if enabled
            options.push(TCP_OPT_NOP);
            options.push(TCP_OPT_NOP);
            options.push(TCP_OPT_TIMESTAMP);
            options.push(10);
            let ts = Self::get_timestamp() as u32;
            options.extend_from_slice(&ts.to_be_bytes());
            options.extend_from_slice(&self.ts_recent.to_be_bytes());
        }
        
        // Pad to 4-byte boundary
        while options.len() % 4 != 0 {
            options.push(TCP_OPT_NOP);
        }
        
        options
    }
    
    pub fn send_syn(&mut self) -> TcpSegment {
        self.state = TcpState::SynSent;
        
        // Build SYN options
        self.window_scaling_enabled = true;
        self.sack_permitted = true;
        self.timestamps_enabled = true;
        let options = self.build_options(true, false);
        
        let mut header = TcpHeader::new(
            self.local_port,
            self.remote_port,
            self.snd_nxt,
            0,
            TCP_SYN,
            (self.rcv_wnd >> self.rcv_wnd_scale) as u16,
        );
        
        // Update data offset for options
        let data_offset = (20 + options.len()) / 4;
        header.data_offset_flags = ((data_offset as u16) << 12 | TCP_SYN as u16).to_be();
        
        self.snd_nxt = self.snd_nxt.wrapping_add(1);
        self.snd_max = self.snd_nxt;
        
        // Set retransmission timer
        self.set_timer(TcpTimer::Retransmission, self.congestion.rto);
        
        let mut segment = TcpSegment::new(header, Vec::new());
        segment.options = options;
        
        // Store in retransmit queue
        self.retransmit_queue.push_back((self.snd_una, segment.to_bytes(), Self::get_timestamp()));
        
        segment
    }
    
    pub fn send_syn_ack(&mut self) -> TcpSegment {
        self.state = TcpState::SynReceived;
        
        // Build SYN-ACK options
        let options = self.build_options(true, true);
        
        let mut header = TcpHeader::new(
            self.local_port,
            self.remote_port,
            self.snd_nxt,
            self.rcv_nxt,
            TCP_SYN | TCP_ACK,
            (self.rcv_wnd >> self.rcv_wnd_scale) as u16,
        );
        
        // Update data offset for options
        let data_offset = (20 + options.len()) / 4;
        header.data_offset_flags = ((data_offset as u16) << 12 | (TCP_SYN | TCP_ACK) as u16).to_be();
        
        self.snd_nxt = self.snd_nxt.wrapping_add(1);
        self.snd_max = self.snd_nxt;
        
        // Set retransmission timer
        self.set_timer(TcpTimer::Retransmission, self.congestion.rto);
        
        let mut segment = TcpSegment::new(header, Vec::new());
        segment.options = options;
        
        // Store in retransmit queue
        self.retransmit_queue.push_back((self.snd_una, segment.to_bytes(), Self::get_timestamp()));
        
        segment
    }
    
    pub fn send_ack(&mut self) -> TcpSegment {
        let options = self.build_options(false, true);
        
        let mut header = TcpHeader::new(
            self.local_port,
            self.remote_port,
            self.snd_nxt,
            self.rcv_nxt,
            TCP_ACK,
            (self.rcv_wnd >> self.rcv_wnd_scale) as u16,
        );
        
        // Update data offset for options
        if !options.is_empty() {
            let data_offset = (20 + options.len()) / 4;
            header.data_offset_flags = ((data_offset as u16) << 12 | TCP_ACK as u16).to_be();
        }
        
        // Cancel delayed ACK timer if set
        self.cancel_timer(TcpTimer::DelayedAck);
        
        let mut segment = TcpSegment::new(header, Vec::new());
        segment.options = options;
        segment
    }
    
    pub fn send_data(&mut self, data: &[u8]) -> Vec<TcpSegment> {
        let mut segments = Vec::new();
        let mut offset = 0;
        
        // Check congestion window
        let available_window = self.effective_send_window();
        if available_window == 0 {
            // Set persist timer if window is zero
            self.set_timer(TcpTimer::Persist, TCP_PERSIST_MIN);
            return segments;
        }
        
        let effective_mss = min(self.mss, self.peer_mss) as usize;
        let max_to_send = min(data.len(), available_window as usize);
        
        while offset < max_to_send {
            let chunk_size = min(effective_mss, max_to_send - offset);
            let chunk = data[offset..offset + chunk_size].to_vec();
            
            let options = self.build_options(false, true);
            
            // Decide on PUSH flag
            let flags = if offset + chunk_size >= data.len() {
                TCP_ACK | TCP_PSH  // Push on last segment
            } else {
                TCP_ACK
            };
            
            let mut header = TcpHeader::new(
                self.local_port,
                self.remote_port,
                self.snd_nxt,
                self.rcv_nxt,
                flags,
                (self.rcv_wnd >> self.rcv_wnd_scale) as u16,
            );
            
            // Update data offset for options
            if !options.is_empty() {
                let data_offset = (20 + options.len()) / 4;
                header.data_offset_flags = ((data_offset as u16) << 12 | flags as u16).to_be();
            }
            
            // Track RTT if not already measuring
            if self.congestion.rtt_seq == 0 {
                self.congestion.rtt_seq = self.snd_nxt;
                self.congestion.rtt_time = Self::get_timestamp();
            }
            
            // Update congestion control
            self.congestion.on_send(chunk_size as u32);
            
            // Add to retransmit queue
            let mut full_segment = Vec::new();
            full_segment.extend_from_slice(&header.to_bytes()[..20]);
            full_segment.extend_from_slice(&options);
            full_segment.extend_from_slice(&chunk);
            self.retransmit_queue.push_back((self.snd_nxt, full_segment, Self::get_timestamp()));
            
            self.snd_nxt = self.snd_nxt.wrapping_add(chunk_size as u32);
            self.snd_max = max(self.snd_max, self.snd_nxt);
            
            let mut segment = TcpSegment::new(header, chunk);
            segment.options = options;
            segments.push(segment);
            
            offset += chunk_size;
        }
        
        // Set retransmission timer if not already set
        if !segments.is_empty() {
            self.set_timer(TcpTimer::Retransmission, self.congestion.rto);
        }
        
        // Update statistics
        self.bytes_sent += offset as u64;
        self.segments_sent += segments.len() as u64;
        
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
        let window = segment.header.window();
        
        // Update last receive time and reset keepalive
        self.last_recv_time = Self::get_timestamp();
        self.segments_received += 1;
        self.reset_keepalive();
        
        // Process TCP options
        let options = Self::parse_options(&segment.options);
        for opt in &options {
            match opt {
                TcpOption::Mss(mss) => {
                    if self.state == TcpState::Listen || self.state == TcpState::SynSent {
                        self.peer_mss = *mss;
                    }
                }
                TcpOption::WindowScale(scale) => {
                    if self.state == TcpState::Listen || self.state == TcpState::SynSent {
                        self.snd_wnd_scale = min(*scale, TCP_WINDOW_SCALE_MAX);
                    }
                }
                TcpOption::SackPermitted => {
                    if self.state == TcpState::Listen || self.state == TcpState::SynSent {
                        self.sack_permitted = true;
                    }
                }
                TcpOption::Timestamp(ts_val, ts_ecr) => {
                    self.ts_recent = *ts_val;
                    self.ts_recent_age = Self::get_timestamp();
                    
                    // Calculate RTT if this is an ACK for timed segment
                    if *ts_ecr != 0 && flags & TCP_ACK != 0 {
                        let rtt = Self::get_timestamp().saturating_sub(*ts_ecr as u64) as u32;
                        if rtt > 0 {
                            self.update_rtt(rtt);
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Check for RST
        if flags & TCP_RST != 0 {
            match self.state {
                TcpState::SynSent => {
                    if ack == self.snd_nxt {
                        self.state = TcpState::Closed;
                        return None;
                    }
                }
                TcpState::Listen => return None,
                _ => {
                    // Validate RST
                    if self.is_acceptable_segment(seq, segment.data.len() as u32) {
                        self.state = TcpState::Closed;
                        self.cancel_all_timers();
                        return None;
                    }
                }
            }
        }
        
        match self.state {
            TcpState::Listen => {
                if flags & TCP_SYN != 0 {
                    // Received SYN, send SYN-ACK
                    self.irs = seq;
                    self.rcv_nxt = seq.wrapping_add(1);
                    self.remote_port = segment.header.src_port();
                    
                    // Process SYN options
                    for opt in &options {
                        match opt {
                            TcpOption::Mss(mss) => self.peer_mss = *mss,
                            TcpOption::WindowScale(scale) => {
                                self.snd_wnd_scale = min(*scale, TCP_WINDOW_SCALE_MAX);
                                self.window_scaling_enabled = true;
                            }
                            TcpOption::SackPermitted => self.sack_permitted = true,
                            TcpOption::Timestamp(ts_val, _) => {
                                self.timestamps_enabled = true;
                                self.ts_recent = *ts_val;
                                self.ts_recent_age = Self::get_timestamp();
                            }
                            _ => {}
                        }
                    }
                    
                    return Some(self.send_syn_ack());
                }
            }
            
            TcpState::SynSent => {
                if flags & TCP_ACK != 0 {
                    // Check if ACK is acceptable
                    if ack <= self.iss || ack > self.snd_nxt {
                        if flags & TCP_RST == 0 {
                            return Some(self.send_rst());
                        }
                        return None;
                    }
                }
                
                if flags & TCP_SYN != 0 {
                    self.irs = seq;
                    self.rcv_nxt = seq.wrapping_add(1);
                    self.update_send_window(window, seq, ack);
                    
                    if flags & TCP_ACK != 0 {
                        // Received SYN-ACK
                        self.snd_una = ack;
                        
                        // Remove SYN from retransmit queue
                        self.retransmit_queue.pop_front();
                        
                        // Cancel retransmission timer
                        self.cancel_timer(TcpTimer::Retransmission);
                        
                        // Update RTT
                        if self.congestion.rtt_seq != 0 && ack > self.congestion.rtt_seq {
                            let rtt = Self::get_timestamp().saturating_sub(self.congestion.rtt_time) as u32;
                            self.update_rtt(rtt);
                            self.congestion.rtt_seq = 0;
                        }
                        
                        self.state = TcpState::Established;
                        self.congestion.state = CongestionState::SlowStart;
                        
                        // Send ACK
                        return Some(self.send_ack());
                    } else {
                        // Simultaneous open - received SYN
                        self.state = TcpState::SynReceived;
                        return Some(self.send_syn_ack());
                    }
                }
            }
            
            TcpState::SynReceived | TcpState::Established | TcpState::FinWait1 | 
            TcpState::FinWait2 | TcpState::CloseWait | TcpState::Closing | 
            TcpState::LastAck | TcpState::TimeWait => {
                
                // Check sequence number
                if !self.is_acceptable_segment(seq, segment.data.len() as u32 + if flags & TCP_FIN != 0 { 1 } else { 0 }) {
                    // Unacceptable segment, send ACK
                    if flags & TCP_RST == 0 {
                        return Some(self.send_ack());
                    }
                    return None;
                }
                
                // Process ACK
                if flags & TCP_ACK != 0 {
                    match self.state {
                        TcpState::SynReceived => {
                            if self.snd_una <= ack && ack <= self.snd_nxt {
                                self.state = TcpState::Established;
                                self.snd_una = ack;
                                self.congestion.state = CongestionState::SlowStart;
                                
                                // Remove SYN-ACK from retransmit queue
                                self.retransmit_queue.pop_front();
                                self.cancel_timer(TcpTimer::Retransmission);
                                
                                crate::serial_println!("TCP connection established");
                            } else {
                                return Some(self.send_rst());
                            }
                        }
                        
                        TcpState::Established | TcpState::FinWait1 | TcpState::FinWait2 | 
                        TcpState::CloseWait | TcpState::Closing => {
                            if self.snd_una < ack && ack <= self.snd_nxt {
                                // New data acknowledged
                                let acked_bytes = ack.wrapping_sub(self.snd_una);
                                
                                // Update congestion control
                                if ack == self.snd_una {
                                    // Duplicate ACK
                                    if self.congestion.on_dup_ack(self.mss) {
                                        // Fast retransmit
                                        if let Some((seq, data, _)) = self.retransmit_queue.front() {
                                            self.retransmissions += 1;
                                            // Would retransmit here
                                        }
                                    }
                                } else {
                                    // New ACK
                                    self.congestion.on_ack(acked_bytes, self.mss);
                                    
                                    // Exit fast recovery if applicable
                                    if self.congestion.state == CongestionState::FastRecovery && 
                                       ack >= self.congestion.recovery_point {
                                        self.congestion.exit_fast_recovery();
                                    }
                                }
                                
                                self.snd_una = ack;
                                
                                // Update window
                                self.update_send_window(window, seq, ack);
                                
                                // Remove acknowledged segments from retransmit queue
                                while let Some((seq_num, _, _)) = self.retransmit_queue.front() {
                                    if seq_num.wrapping_add(1) <= self.snd_una {
                                        self.retransmit_queue.pop_front();
                                    } else {
                                        break;
                                    }
                                }
                                
                                // Update RTT if measuring
                                if self.congestion.rtt_seq != 0 && ack > self.congestion.rtt_seq {
                                    let rtt = Self::get_timestamp().saturating_sub(self.congestion.rtt_time) as u32;
                                    self.update_rtt(rtt);
                                    self.congestion.rtt_seq = 0;
                                }
                                
                                // Cancel retransmission timer if queue is empty
                                if self.retransmit_queue.is_empty() {
                                    self.cancel_timer(TcpTimer::Retransmission);
                                } else {
                                    // Reset timer for remaining segments
                                    self.set_timer(TcpTimer::Retransmission, self.congestion.rto);
                                }
                                
                                // State transitions for FIN ACK
                                match self.state {
                                    TcpState::FinWait1 => {
                                        if self.snd_una == self.snd_nxt {
                                            self.state = TcpState::FinWait2;
                                        }
                                    }
                                    TcpState::Closing => {
                                        if self.snd_una == self.snd_nxt {
                                            self.state = TcpState::TimeWait;
                                            self.set_timer(TcpTimer::TimeWait, TCP_TIME_WAIT_DURATION);
                                        }
                                    }
                                    _ => {}
                                }
                            } else if ack > self.snd_nxt {
                                // ACK for data not yet sent
                                return Some(self.send_ack());
                            }
                        }
                        
                        TcpState::LastAck => {
                            if self.snd_una <= ack && ack <= self.snd_nxt {
                                self.state = TcpState::Closed;
                                self.cancel_all_timers();
                                crate::serial_println!("TCP connection closed");
                                return None;
                            }
                        }
                        
                        _ => {}
                    }
                }
                
                // Process urgent data
                if flags & TCP_URG != 0 {
                    self.rcv_up = max(self.rcv_up, segment.header.urgent_ptr as u32 + seq);
                }
                
                // Process data
                if !segment.data.is_empty() {
                    self.process_data(seq, &segment.data, flags & TCP_PSH != 0);
                    self.bytes_received += segment.data.len() as u64;
                    
                    // Delayed ACK or immediate ACK
                    if flags & TCP_PSH != 0 || self.out_of_order.len() > 0 {
                        return Some(self.send_ack());
                    } else {
                        // Set delayed ACK timer (200ms typical)
                        self.set_timer(TcpTimer::DelayedAck, 200);
                    }
                }
                
                // Process FIN
                if flags & TCP_FIN != 0 {
                    // Check if all data up to FIN has been received
                    if seq == self.rcv_nxt {
                        self.rcv_nxt = self.rcv_nxt.wrapping_add(1);
                        
                        match self.state {
                            TcpState::SynReceived | TcpState::Established => {
                                self.state = TcpState::CloseWait;
                            }
                            TcpState::FinWait1 => {
                                if self.snd_una == self.snd_nxt {
                                    self.state = TcpState::TimeWait;
                                    self.set_timer(TcpTimer::TimeWait, TCP_TIME_WAIT_DURATION);
                                } else {
                                    self.state = TcpState::Closing;
                                }
                            }
                            TcpState::FinWait2 => {
                                self.state = TcpState::TimeWait;
                                self.set_timer(TcpTimer::TimeWait, TCP_TIME_WAIT_DURATION);
                            }
                            _ => {}
                        }
                        
                        return Some(self.send_ack());
                    }
                }
            }
            
            TcpState::Closed => {
                if flags & TCP_RST == 0 {
                    return Some(self.send_rst());
                }
            }
        }
        
        None
    }
    
    // Check if segment is acceptable
    fn is_acceptable_segment(&self, seq: u32, seg_len: u32) -> bool {
        let rcv_wnd = self.rcv_wnd;
        
        if seg_len == 0 && rcv_wnd == 0 {
            seq == self.rcv_nxt
        } else if seg_len == 0 && rcv_wnd > 0 {
            // SEG.SEQ = RCV.NXT or (RCV.NXT <= SEG.SEQ < RCV.NXT+RCV.WND)
            seq == self.rcv_nxt || 
            (self.rcv_nxt.wrapping_sub(seq) as i32 <= 0 && 
             seq.wrapping_sub(self.rcv_nxt.wrapping_add(rcv_wnd)) as i32 < 0)
        } else if seg_len > 0 && rcv_wnd == 0 {
            false  // Not acceptable
        } else {
            // Check if any part of segment is in window
            let seg_end = seq.wrapping_add(seg_len - 1);
            let wnd_end = self.rcv_nxt.wrapping_add(rcv_wnd - 1);
            
            // Either start or end of segment must be in window
            (self.rcv_nxt.wrapping_sub(seq) as i32 <= 0 && 
             seq.wrapping_sub(self.rcv_nxt.wrapping_add(rcv_wnd)) as i32 < 0) ||
            (self.rcv_nxt.wrapping_sub(seg_end) as i32 <= 0 && 
             seg_end.wrapping_sub(wnd_end) as i32 <= 0)
        }
    }
    
    // Process incoming data
    fn process_data(&mut self, seq: u32, data: &[u8], push: bool) {
        if seq == self.rcv_nxt {
            // In-order data
            self.recv_buffer.extend(data);
            self.rcv_nxt = self.rcv_nxt.wrapping_add(data.len() as u32);
            
            // Check for contiguous out-of-order segments
            while let Some(segment) = self.out_of_order.remove(&self.rcv_nxt) {
                self.recv_buffer.extend(&segment.data);
                self.rcv_nxt = self.rcv_nxt.wrapping_add(segment.data.len() as u32);
                
                if segment.fin {
                    // Process buffered FIN
                    self.rcv_nxt = self.rcv_nxt.wrapping_add(1);
                    break;
                }
            }
        } else if seq.wrapping_sub(self.rcv_nxt) as i32 > 0 {
            // Out-of-order future data
            if self.out_of_order.len() < 64 {  // Limit out-of-order buffer
                self.out_of_order.insert(seq, OutOfOrderSegment {
                    seq,
                    data: data.to_vec(),
                    fin: false,
                });
            }
        }
        // Else: old duplicate data, ignore
    }
    
    // Cancel all timers
    fn cancel_all_timers(&mut self) {
        self.timers.clear();
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