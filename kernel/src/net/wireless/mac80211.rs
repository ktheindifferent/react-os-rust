use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

pub const IEEE80211_MAX_SSID_LEN: usize = 32;
pub const IEEE80211_MAX_FRAME_LEN: usize = 2352;
pub const IEEE80211_MAX_QUEUES: usize = 16;
pub const IEEE80211_QOS_CTL_LEN: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Band {
    Band2GHz,
    Band5GHz,
    Band6GHz,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChannelWidth {
    Width20,
    Width40,
    Width80,
    Width160,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub frequency: u32,
    pub number: u8,
    pub band: Band,
    pub max_power: i8,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameType {
    Management,
    Control,
    Data,
    Extension,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ManagementSubtype {
    AssocRequest,
    AssocResponse,
    ReassocRequest,
    ReassocResponse,
    ProbeRequest,
    ProbeResponse,
    Beacon,
    Atim,
    Disassoc,
    Auth,
    Deauth,
    Action,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlSubtype {
    BlockAckReq,
    BlockAck,
    PsPoll,
    Rts,
    Cts,
    Ack,
    CfEnd,
    CfEndAck,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataSubtype {
    Data,
    DataCfAck,
    DataCfPoll,
    DataCfAckPoll,
    Null,
    CfAck,
    CfPoll,
    CfAckPoll,
    QosData,
    QosDataCfAck,
    QosDataCfPoll,
    QosDataCfAckPoll,
    QosNull,
    QosCfPoll,
    QosCfAckPoll,
}

#[repr(C, packed)]
pub struct FrameControl {
    pub version: u8,
    pub frame_type: u8,
    pub subtype: u8,
    pub to_ds: bool,
    pub from_ds: bool,
    pub more_frag: bool,
    pub retry: bool,
    pub pwr_mgmt: bool,
    pub more_data: bool,
    pub protected: bool,
    pub order: bool,
}

impl FrameControl {
    pub fn new(frame_type: FrameType, subtype: u8) -> Self {
        Self {
            version: 0,
            frame_type: match frame_type {
                FrameType::Management => 0,
                FrameType::Control => 1,
                FrameType::Data => 2,
                FrameType::Extension => 3,
            },
            subtype,
            to_ds: false,
            from_ds: false,
            more_frag: false,
            retry: false,
            pwr_mgmt: false,
            more_data: false,
            protected: false,
            order: false,
        }
    }

    pub fn to_bytes(&self) -> [u8; 2] {
        let mut fc = 0u16;
        fc |= (self.version as u16) & 0x3;
        fc |= ((self.frame_type as u16) & 0x3) << 2;
        fc |= ((self.subtype as u16) & 0xF) << 4;
        fc |= (self.to_ds as u16) << 8;
        fc |= (self.from_ds as u16) << 9;
        fc |= (self.more_frag as u16) << 10;
        fc |= (self.retry as u16) << 11;
        fc |= (self.pwr_mgmt as u16) << 12;
        fc |= (self.more_data as u16) << 13;
        fc |= (self.protected as u16) << 14;
        fc |= (self.order as u16) << 15;
        fc.to_le_bytes()
    }

    pub fn from_bytes(bytes: [u8; 2]) -> Self {
        let fc = u16::from_le_bytes(bytes);
        Self {
            version: (fc & 0x3) as u8,
            frame_type: ((fc >> 2) & 0x3) as u8,
            subtype: ((fc >> 4) & 0xF) as u8,
            to_ds: (fc >> 8) & 1 == 1,
            from_ds: (fc >> 9) & 1 == 1,
            more_frag: (fc >> 10) & 1 == 1,
            retry: (fc >> 11) & 1 == 1,
            pwr_mgmt: (fc >> 12) & 1 == 1,
            more_data: (fc >> 13) & 1 == 1,
            protected: (fc >> 14) & 1 == 1,
            order: (fc >> 15) & 1 == 1,
        }
    }
}

#[repr(C, packed)]
pub struct MacHeader {
    pub frame_control: [u8; 2],
    pub duration: u16,
    pub addr1: [u8; 6],
    pub addr2: [u8; 6],
    pub addr3: [u8; 6],
    pub seq_ctrl: u16,
}

impl MacHeader {
    pub fn new(fc: FrameControl, dst: [u8; 6], src: [u8; 6], bssid: [u8; 6]) -> Self {
        Self {
            frame_control: fc.to_bytes(),
            duration: 0,
            addr1: dst,
            addr2: src,
            addr3: bssid,
            seq_ctrl: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BeaconFrame {
    pub timestamp: u64,
    pub beacon_interval: u16,
    pub capability_info: u16,
    pub ssid: String,
    pub supported_rates: Vec<u8>,
    pub channel: u8,
    pub rsn_ie: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct ProbeRequest {
    pub ssid: Option<String>,
    pub supported_rates: Vec<u8>,
    pub extended_rates: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct AuthFrame {
    pub algorithm: u16,
    pub seq_num: u16,
    pub status_code: u16,
    pub challenge_text: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct AssocRequest {
    pub capability_info: u16,
    pub listen_interval: u16,
    pub ssid: String,
    pub supported_rates: Vec<u8>,
    pub rsn_ie: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StationState {
    Idle,
    Scanning,
    Authenticating,
    Associating,
    Associated,
    Disconnected,
}

pub struct Station {
    pub mac_addr: [u8; 6],
    pub state: StationState,
    pub bssid: Option<[u8; 6]>,
    pub ssid: Option<String>,
    pub channel: Option<Channel>,
    pub signal_strength: i8,
    pub tx_rate: u32,
    pub rx_rate: u32,
    pub tx_packets: AtomicU64,
    pub rx_packets: AtomicU64,
    pub tx_bytes: AtomicU64,
    pub rx_bytes: AtomicU64,
}

impl Station {
    pub fn new(mac_addr: [u8; 6]) -> Self {
        Self {
            mac_addr,
            state: StationState::Idle,
            bssid: None,
            ssid: None,
            channel: None,
            signal_strength: -100,
            tx_rate: 0,
            rx_rate: 0,
            tx_packets: AtomicU64::new(0),
            rx_packets: AtomicU64::new(0),
            tx_bytes: AtomicU64::new(0),
            rx_bytes: AtomicU64::new(0),
        }
    }
}

pub struct TxQueue {
    pub queue_id: usize,
    pub frames: VecDeque<Vec<u8>>,
    pub max_len: usize,
    pub dropped: AtomicU32,
}

impl TxQueue {
    pub fn new(queue_id: usize, max_len: usize) -> Self {
        Self {
            queue_id,
            frames: VecDeque::new(),
            max_len,
            dropped: AtomicU32::new(0),
        }
    }

    pub fn enqueue(&mut self, frame: Vec<u8>) -> Result<(), ()> {
        if self.frames.len() >= self.max_len {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return Err(());
        }
        self.frames.push_back(frame);
        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<Vec<u8>> {
        self.frames.pop_front()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RateInfo {
    pub bitrate: u32,
    pub mcs_index: Option<u8>,
    pub nss: u8,
    pub bandwidth: ChannelWidth,
    pub guard_interval: u16,
}

pub struct RateControl {
    pub current_rate: RateInfo,
    pub rates: Vec<RateInfo>,
    pub tx_success: AtomicU32,
    pub tx_failed: AtomicU32,
    pub last_update: u64,
}

impl RateControl {
    pub fn new() -> Self {
        let rates = vec![
            RateInfo { bitrate: 1000, mcs_index: None, nss: 1, bandwidth: ChannelWidth::Width20, guard_interval: 800 },
            RateInfo { bitrate: 2000, mcs_index: None, nss: 1, bandwidth: ChannelWidth::Width20, guard_interval: 800 },
            RateInfo { bitrate: 5500, mcs_index: None, nss: 1, bandwidth: ChannelWidth::Width20, guard_interval: 800 },
            RateInfo { bitrate: 11000, mcs_index: None, nss: 1, bandwidth: ChannelWidth::Width20, guard_interval: 800 },
            RateInfo { bitrate: 6000, mcs_index: None, nss: 1, bandwidth: ChannelWidth::Width20, guard_interval: 800 },
            RateInfo { bitrate: 9000, mcs_index: None, nss: 1, bandwidth: ChannelWidth::Width20, guard_interval: 800 },
            RateInfo { bitrate: 12000, mcs_index: None, nss: 1, bandwidth: ChannelWidth::Width20, guard_interval: 800 },
            RateInfo { bitrate: 18000, mcs_index: None, nss: 1, bandwidth: ChannelWidth::Width20, guard_interval: 800 },
            RateInfo { bitrate: 24000, mcs_index: None, nss: 1, bandwidth: ChannelWidth::Width20, guard_interval: 800 },
            RateInfo { bitrate: 36000, mcs_index: None, nss: 1, bandwidth: ChannelWidth::Width20, guard_interval: 800 },
            RateInfo { bitrate: 48000, mcs_index: None, nss: 1, bandwidth: ChannelWidth::Width20, guard_interval: 800 },
            RateInfo { bitrate: 54000, mcs_index: None, nss: 1, bandwidth: ChannelWidth::Width20, guard_interval: 800 },
        ];

        Self {
            current_rate: rates[0],
            rates,
            tx_success: AtomicU32::new(0),
            tx_failed: AtomicU32::new(0),
            last_update: 0,
        }
    }

    pub fn update_rate(&mut self, success: bool) {
        if success {
            self.tx_success.fetch_add(1, Ordering::Relaxed);
        } else {
            self.tx_failed.fetch_add(1, Ordering::Relaxed);
        }

        let success_count = self.tx_success.load(Ordering::Relaxed);
        let failed_count = self.tx_failed.load(Ordering::Relaxed);
        let total = success_count + failed_count;

        if total >= 100 {
            let success_rate = (success_count * 100) / total;
            
            if success_rate > 90 {
                self.increase_rate();
            } else if success_rate < 60 {
                self.decrease_rate();
            }

            self.tx_success.store(0, Ordering::Relaxed);
            self.tx_failed.store(0, Ordering::Relaxed);
        }
    }

    fn increase_rate(&mut self) {
        for (i, rate) in self.rates.iter().enumerate() {
            if rate.bitrate == self.current_rate.bitrate && i < self.rates.len() - 1 {
                self.current_rate = self.rates[i + 1];
                break;
            }
        }
    }

    fn decrease_rate(&mut self) {
        for (i, rate) in self.rates.iter().enumerate() {
            if rate.bitrate == self.current_rate.bitrate && i > 0 {
                self.current_rate = self.rates[i - 1];
                break;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PowerSaveMode {
    Active,
    LightSleep,
    DeepSleep,
}

pub struct PowerManagement {
    pub mode: PowerSaveMode,
    pub dtim_period: u8,
    pub listen_interval: u16,
    pub last_beacon: u64,
    pub buffered_frames: VecDeque<Vec<u8>>,
}

impl PowerManagement {
    pub fn new() -> Self {
        Self {
            mode: PowerSaveMode::Active,
            dtim_period: 1,
            listen_interval: 1,
            last_beacon: 0,
            buffered_frames: VecDeque::new(),
        }
    }

    pub fn enter_power_save(&mut self, mode: PowerSaveMode) {
        self.mode = mode;
    }

    pub fn exit_power_save(&mut self) {
        self.mode = PowerSaveMode::Active;
    }

    pub fn should_wake(&self, current_time: u64) -> bool {
        match self.mode {
            PowerSaveMode::Active => true,
            PowerSaveMode::LightSleep => {
                (current_time - self.last_beacon) >= (self.listen_interval as u64 * 100)
            }
            PowerSaveMode::DeepSleep => {
                (current_time - self.last_beacon) >= (self.dtim_period as u64 * self.listen_interval as u64 * 100)
            }
        }
    }
}

pub struct CsmaCA {
    pub cw_min: u32,
    pub cw_max: u32,
    pub current_cw: u32,
    pub backoff_counter: u32,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl CsmaCA {
    pub fn new() -> Self {
        Self {
            cw_min: 15,
            cw_max: 1023,
            current_cw: 15,
            backoff_counter: 0,
            retry_count: 0,
            max_retries: 7,
        }
    }

    pub fn reset(&mut self) {
        self.current_cw = self.cw_min;
        self.backoff_counter = 0;
        self.retry_count = 0;
    }

    pub fn collision(&mut self) {
        self.retry_count += 1;
        if self.retry_count <= self.max_retries {
            self.current_cw = core::cmp::min(self.current_cw * 2 + 1, self.cw_max);
            self.backoff_counter = self.random_backoff();
        }
    }

    pub fn success(&mut self) {
        self.reset();
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    fn random_backoff(&self) -> u32 {
        (self.current_cw + 1) / 2
    }

    pub fn decrement_backoff(&mut self) -> bool {
        if self.backoff_counter > 0 {
            self.backoff_counter -= 1;
            false
        } else {
            true
        }
    }
}

pub struct Aggregation {
    pub ampdu_enabled: bool,
    pub amsdu_enabled: bool,
    pub max_ampdu_length: u32,
    pub max_amsdu_length: u32,
    pub pending_frames: VecDeque<Vec<u8>>,
}

impl Aggregation {
    pub fn new() -> Self {
        Self {
            ampdu_enabled: true,
            amsdu_enabled: true,
            max_ampdu_length: 65535,
            max_amsdu_length: 7935,
            pending_frames: VecDeque::new(),
        }
    }

    pub fn aggregate_amsdu(&mut self, frames: Vec<Vec<u8>>) -> Vec<u8> {
        let mut aggregated = Vec::new();
        let mut current_len = 0;

        for frame in frames {
            if current_len + frame.len() + 14 > self.max_amsdu_length as usize {
                break;
            }

            aggregated.extend_from_slice(&[0xAA, 0xAA, 0x03, 0x00, 0x00, 0x00]);
            aggregated.extend_from_slice(&(frame.len() as u16).to_be_bytes());
            aggregated.extend_from_slice(&frame);
            
            let padding = (4 - (frame.len() % 4)) % 4;
            aggregated.resize(aggregated.len() + padding, 0);
            
            current_len = aggregated.len();
        }

        aggregated
    }
}

pub struct Mac80211 {
    pub station: Station,
    pub tx_queues: Vec<TxQueue>,
    pub rate_control: RateControl,
    pub power_mgmt: PowerManagement,
    pub csma_ca: CsmaCA,
    pub aggregation: Aggregation,
    pub sequence_number: AtomicU32,
}

impl Mac80211 {
    pub fn new(mac_addr: [u8; 6]) -> Self {
        let mut tx_queues = Vec::new();
        for i in 0..4 {
            tx_queues.push(TxQueue::new(i, 256));
        }

        Self {
            station: Station::new(mac_addr),
            tx_queues,
            rate_control: RateControl::new(),
            power_mgmt: PowerManagement::new(),
            csma_ca: CsmaCA::new(),
            aggregation: Aggregation::new(),
            sequence_number: AtomicU32::new(0),
        }
    }

    pub fn start_scan(&mut self) {
        self.station.state = StationState::Scanning;
    }

    pub fn process_beacon(&mut self, beacon: BeaconFrame, rssi: i8) {
        if self.station.state == StationState::Scanning {
            // Process and store beacon information
        }
    }

    pub fn connect(&mut self, ssid: String, bssid: [u8; 6]) {
        self.station.ssid = Some(ssid);
        self.station.bssid = Some(bssid);
        self.station.state = StationState::Authenticating;
    }

    pub fn disconnect(&mut self) {
        self.station.state = StationState::Disconnected;
        self.station.ssid = None;
        self.station.bssid = None;
        self.csma_ca.reset();
    }

    pub fn create_probe_request(&self, ssid: Option<&str>) -> Vec<u8> {
        let fc = FrameControl::new(FrameType::Management, ManagementSubtype::ProbeRequest as u8);
        let mut frame = Vec::new();
        
        let header = MacHeader::new(
            fc,
            [0xFF; 6],
            self.station.mac_addr,
            [0xFF; 6],
        );
        
        frame.extend_from_slice(&header.frame_control);
        frame.extend_from_slice(&header.duration.to_le_bytes());
        frame.extend_from_slice(&header.addr1);
        frame.extend_from_slice(&header.addr2);
        frame.extend_from_slice(&header.addr3);
        frame.extend_from_slice(&header.seq_ctrl.to_le_bytes());
        
        frame.push(0);
        if let Some(ssid) = ssid {
            frame.push(ssid.len() as u8);
            frame.extend_from_slice(ssid.as_bytes());
        } else {
            frame.push(0);
        }
        
        frame.push(1);
        frame.push(8);
        frame.extend_from_slice(&[0x02, 0x04, 0x0B, 0x16, 0x0C, 0x12, 0x18, 0x24]);
        
        frame
    }

    pub fn create_auth_frame(&self, bssid: [u8; 6]) -> Vec<u8> {
        let fc = FrameControl::new(FrameType::Management, ManagementSubtype::Auth as u8);
        let mut frame = Vec::new();
        
        let header = MacHeader::new(fc, bssid, self.station.mac_addr, bssid);
        
        frame.extend_from_slice(&header.frame_control);
        frame.extend_from_slice(&header.duration.to_le_bytes());
        frame.extend_from_slice(&header.addr1);
        frame.extend_from_slice(&header.addr2);
        frame.extend_from_slice(&header.addr3);
        frame.extend_from_slice(&header.seq_ctrl.to_le_bytes());
        
        frame.extend_from_slice(&0u16.to_le_bytes());
        frame.extend_from_slice(&1u16.to_le_bytes());
        frame.extend_from_slice(&0u16.to_le_bytes());
        
        frame
    }

    pub fn create_assoc_request(&self, ssid: &str, bssid: [u8; 6]) -> Vec<u8> {
        let fc = FrameControl::new(FrameType::Management, ManagementSubtype::AssocRequest as u8);
        let mut frame = Vec::new();
        
        let header = MacHeader::new(fc, bssid, self.station.mac_addr, bssid);
        
        frame.extend_from_slice(&header.frame_control);
        frame.extend_from_slice(&header.duration.to_le_bytes());
        frame.extend_from_slice(&header.addr1);
        frame.extend_from_slice(&header.addr2);
        frame.extend_from_slice(&header.addr3);
        frame.extend_from_slice(&header.seq_ctrl.to_le_bytes());
        
        frame.extend_from_slice(&0x31u16.to_le_bytes());
        frame.extend_from_slice(&10u16.to_le_bytes());
        
        frame.push(0);
        frame.push(ssid.len() as u8);
        frame.extend_from_slice(ssid.as_bytes());
        
        frame.push(1);
        frame.push(8);
        frame.extend_from_slice(&[0x02, 0x04, 0x0B, 0x16, 0x0C, 0x12, 0x18, 0x24]);
        
        frame
    }

    pub fn get_next_sequence(&self) -> u16 {
        let seq = self.sequence_number.fetch_add(1, Ordering::Relaxed);
        (seq as u16) & 0x0FFF
    }

    pub fn transmit(&mut self, data: Vec<u8>, queue_id: usize) -> Result<(), ()> {
        if queue_id >= self.tx_queues.len() {
            return Err(());
        }

        self.station.tx_packets.fetch_add(1, Ordering::Relaxed);
        self.station.tx_bytes.fetch_add(data.len() as u64, Ordering::Relaxed);

        self.tx_queues[queue_id].enqueue(data)
    }

    pub fn receive(&mut self, frame: &[u8]) {
        self.station.rx_packets.fetch_add(1, Ordering::Relaxed);
        self.station.rx_bytes.fetch_add(frame.len() as u64, Ordering::Relaxed);
        
        if frame.len() < 24 {
            return;
        }

        let fc = FrameControl::from_bytes([frame[0], frame[1]]);
        
        match fc.frame_type {
            0 => self.handle_management_frame(frame, fc.subtype),
            1 => self.handle_control_frame(frame, fc.subtype),
            2 => self.handle_data_frame(frame, fc.subtype),
            _ => {}
        }
    }

    fn handle_management_frame(&mut self, frame: &[u8], subtype: u8) {
        match subtype {
            5 => {},
            7 => {},
            8 => {},
            10 => {},
            11 => {},
            _ => {}
        }
    }

    fn handle_control_frame(&mut self, _frame: &[u8], _subtype: u8) {
    }

    fn handle_data_frame(&mut self, _frame: &[u8], _subtype: u8) {
    }
}