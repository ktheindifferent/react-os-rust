use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::mac80211::{Band, Channel, ChannelWidth, StationState};

pub const NL80211_MAX_NR_CIPHER_SUITES: usize = 5;
pub const NL80211_MAX_NR_AKM_SUITES: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterfaceType {
    Station,
    AccessPoint,
    AdHoc,
    Monitor,
    Mesh,
    P2PClient,
    P2PGO,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthType {
    Open,
    Shared,
    Sae,
    Fils,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyManagement {
    None,
    Wpa2Psk,
    Wpa2Enterprise,
    Wpa3Psk,
    Wpa3Enterprise,
    Owe,
    Fils,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CipherSuite {
    None,
    Wep40,
    Wep104,
    Tkip,
    Ccmp128,
    Ccmp256,
    Gcmp128,
    Gcmp256,
}

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub auth_type: AuthType,
    pub key_mgmt: KeyManagement,
    pub pairwise_ciphers: Vec<CipherSuite>,
    pub group_cipher: CipherSuite,
    pub akm_suites: Vec<u32>,
    pub pmf: PmfMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PmfMode {
    Disabled,
    Optional,
    Required,
}

impl SecurityConfig {
    pub fn new_open() -> Self {
        Self {
            auth_type: AuthType::Open,
            key_mgmt: KeyManagement::None,
            pairwise_ciphers: vec![CipherSuite::None],
            group_cipher: CipherSuite::None,
            akm_suites: Vec::new(),
            pmf: PmfMode::Disabled,
        }
    }

    pub fn new_wpa2_psk() -> Self {
        Self {
            auth_type: AuthType::Open,
            key_mgmt: KeyManagement::Wpa2Psk,
            pairwise_ciphers: vec![CipherSuite::Ccmp128],
            group_cipher: CipherSuite::Ccmp128,
            akm_suites: vec![0x000FAC02],
            pmf: PmfMode::Optional,
        }
    }

    pub fn new_wpa3_psk() -> Self {
        Self {
            auth_type: AuthType::Sae,
            key_mgmt: KeyManagement::Wpa3Psk,
            pairwise_ciphers: vec![CipherSuite::Ccmp128, CipherSuite::Ccmp256],
            group_cipher: CipherSuite::Ccmp128,
            akm_suites: vec![0x000FAC08],
            pmf: PmfMode::Required,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BssInfo {
    pub bssid: [u8; 6],
    pub ssid: String,
    pub frequency: u32,
    pub signal: i8,
    pub capability: u16,
    pub beacon_interval: u16,
    pub security: SecurityConfig,
    pub ies: Vec<u8>,
    pub last_seen: u64,
}

#[derive(Debug, Clone)]
pub struct NetworkProfile {
    pub ssid: String,
    pub bssid: Option<[u8; 6]>,
    pub security: SecurityConfig,
    pub passphrase: Option<String>,
    pub priority: u32,
    pub auto_connect: bool,
    pub hidden: bool,
}

impl NetworkProfile {
    pub fn new(ssid: String) -> Self {
        Self {
            ssid,
            bssid: None,
            security: SecurityConfig::new_open(),
            passphrase: None,
            priority: 0,
            auto_connect: true,
            hidden: false,
        }
    }
}

pub struct RegulatoryDomain {
    pub alpha2: [u8; 2],
    pub dfs_region: u8,
    pub rules: Vec<RegulatoryRule>,
}

pub struct RegulatoryRule {
    pub freq_range: FrequencyRange,
    pub power_rule: PowerRule,
    pub flags: u32,
}

pub struct FrequencyRange {
    pub start_freq_khz: u32,
    pub end_freq_khz: u32,
    pub max_bandwidth_khz: u32,
}

pub struct PowerRule {
    pub max_antenna_gain: u32,
    pub max_eirp: u32,
}

impl RegulatoryDomain {
    pub fn new_world() -> Self {
        Self {
            alpha2: [b'0', b'0'],
            dfs_region: 0,
            rules: vec![
                RegulatoryRule {
                    freq_range: FrequencyRange {
                        start_freq_khz: 2402000,
                        end_freq_khz: 2482000,
                        max_bandwidth_khz: 40000,
                    },
                    power_rule: PowerRule {
                        max_antenna_gain: 0,
                        max_eirp: 2000,
                    },
                    flags: 0,
                },
                RegulatoryRule {
                    freq_range: FrequencyRange {
                        start_freq_khz: 5170000,
                        end_freq_khz: 5250000,
                        max_bandwidth_khz: 80000,
                    },
                    power_rule: PowerRule {
                        max_antenna_gain: 0,
                        max_eirp: 2000,
                    },
                    flags: 0,
                },
                RegulatoryRule {
                    freq_range: FrequencyRange {
                        start_freq_khz: 5250000,
                        end_freq_khz: 5330000,
                        max_bandwidth_khz: 80000,
                    },
                    power_rule: PowerRule {
                        max_antenna_gain: 0,
                        max_eirp: 2000,
                    },
                    flags: 1,
                },
                RegulatoryRule {
                    freq_range: FrequencyRange {
                        start_freq_khz: 5490000,
                        end_freq_khz: 5730000,
                        max_bandwidth_khz: 160000,
                    },
                    power_rule: PowerRule {
                        max_antenna_gain: 0,
                        max_eirp: 2000,
                    },
                    flags: 1,
                },
                RegulatoryRule {
                    freq_range: FrequencyRange {
                        start_freq_khz: 5735000,
                        end_freq_khz: 5835000,
                        max_bandwidth_khz: 80000,
                    },
                    power_rule: PowerRule {
                        max_antenna_gain: 0,
                        max_eirp: 3000,
                    },
                    flags: 0,
                },
            ],
        }
    }
}

pub struct WirelessInterface {
    pub ifindex: u32,
    pub name: String,
    pub mac_addr: [u8; 6],
    pub if_type: InterfaceType,
    pub active: AtomicBool,
}

impl WirelessInterface {
    pub fn new(ifindex: u32, name: String, mac_addr: [u8; 6]) -> Self {
        Self {
            ifindex,
            name,
            mac_addr,
            if_type: InterfaceType::Station,
            active: AtomicBool::new(false),
        }
    }

    pub fn set_type(&mut self, if_type: InterfaceType) {
        self.if_type = if_type;
    }

    pub fn up(&self) {
        self.active.store(true, Ordering::Relaxed);
    }

    pub fn down(&self) {
        self.active.store(false, Ordering::Relaxed);
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

pub struct ScanRequest {
    pub ssids: Vec<String>,
    pub frequencies: Vec<u32>,
    pub ie: Vec<u8>,
    pub flags: u32,
    pub rates: Vec<u32>,
}

impl ScanRequest {
    pub fn new() -> Self {
        Self {
            ssids: Vec::new(),
            frequencies: Vec::new(),
            ie: Vec::new(),
            flags: 0,
            rates: Vec::new(),
        }
    }

    pub fn add_ssid(&mut self, ssid: String) {
        self.ssids.push(ssid);
    }

    pub fn add_frequency(&mut self, freq: u32) {
        self.frequencies.push(freq);
    }
}

pub struct ScanResult {
    pub bss_list: Vec<BssInfo>,
    pub scan_time: u64,
    pub aborted: bool,
}

pub struct ConnectParams {
    pub ssid: String,
    pub bssid: Option<[u8; 6]>,
    pub security: SecurityConfig,
    pub passphrase: Option<String>,
    pub frequency: Option<u32>,
    pub ie: Vec<u8>,
}

pub struct DisconnectParams {
    pub reason_code: u16,
    pub local_state_change: bool,
}

pub struct VirtualInterface {
    pub wiphy: u32,
    pub ifindex: u32,
    pub name: String,
    pub if_type: InterfaceType,
    pub mac_addr: [u8; 6],
}

pub struct StationInfo {
    pub mac_addr: [u8; 6],
    pub state: StationState,
    pub signal: i8,
    pub signal_avg: i8,
    pub tx_rate: u32,
    pub rx_rate: u32,
    pub tx_packets: u64,
    pub rx_packets: u64,
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub tx_failed: u32,
    pub tx_retries: u32,
    pub beacon_loss: u32,
    pub connected_time: u32,
    pub inactive_time: u32,
}

pub struct ApConfig {
    pub ssid: String,
    pub channel: Channel,
    pub beacon_interval: u16,
    pub dtim_period: u8,
    pub hidden_ssid: bool,
    pub security: SecurityConfig,
    pub max_stations: u16,
}

impl ApConfig {
    pub fn new(ssid: String) -> Self {
        Self {
            ssid,
            channel: Channel {
                frequency: 2412,
                number: 1,
                band: Band::Band2GHz,
                max_power: 20,
                flags: 0,
            },
            beacon_interval: 100,
            dtim_period: 2,
            hidden_ssid: false,
            security: SecurityConfig::new_open(),
            max_stations: 128,
        }
    }
}

pub struct MeshConfig {
    pub mesh_id: String,
    pub path_sel_protocol: u32,
    pub path_metric: u32,
    pub congestion_control: u32,
    pub sync_method: u32,
    pub auth_protocol: u32,
}

pub struct P2PConfig {
    pub device_name: String,
    pub primary_dev_type: [u8; 8],
    pub listen_channel: Channel,
    pub operating_channel: Channel,
    pub go_intent: u8,
}

pub struct Cfg80211 {
    pub wiphy_id: AtomicU32,
    pub interfaces: BTreeMap<u32, WirelessInterface>,
    pub regulatory: RegulatoryDomain,
    pub scan_results: Vec<BssInfo>,
    pub network_profiles: Vec<NetworkProfile>,
    pub current_bss: Option<BssInfo>,
}

impl Cfg80211 {
    pub fn new() -> Self {
        Self {
            wiphy_id: AtomicU32::new(0),
            interfaces: BTreeMap::new(),
            regulatory: RegulatoryDomain::new_world(),
            scan_results: Vec::new(),
            network_profiles: Vec::new(),
            current_bss: None,
        }
    }

    pub fn register_wiphy(&self) -> u32 {
        self.wiphy_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn add_interface(&mut self, iface: WirelessInterface) {
        self.interfaces.insert(iface.ifindex, iface);
    }

    pub fn remove_interface(&mut self, ifindex: u32) -> Option<WirelessInterface> {
        self.interfaces.remove(&ifindex)
    }

    pub fn get_interface(&self, ifindex: u32) -> Option<&WirelessInterface> {
        self.interfaces.get(&ifindex)
    }

    pub fn get_interface_mut(&mut self, ifindex: u32) -> Option<&mut WirelessInterface> {
        self.interfaces.get_mut(&ifindex)
    }

    pub fn start_scan(&mut self, _ifindex: u32, _params: ScanRequest) -> Result<(), ()> {
        Ok(())
    }

    pub fn abort_scan(&mut self, _ifindex: u32) -> Result<(), ()> {
        Ok(())
    }

    pub fn get_scan_results(&self) -> Vec<BssInfo> {
        self.scan_results.clone()
    }

    pub fn connect(&mut self, _ifindex: u32, params: ConnectParams) -> Result<(), ()> {
        let profile = NetworkProfile {
            ssid: params.ssid,
            bssid: params.bssid,
            security: params.security,
            passphrase: params.passphrase,
            priority: 0,
            auto_connect: true,
            hidden: false,
        };
        
        self.network_profiles.push(profile);
        Ok(())
    }

    pub fn disconnect(&mut self, _ifindex: u32, _params: DisconnectParams) -> Result<(), ()> {
        self.current_bss = None;
        Ok(())
    }

    pub fn add_network_profile(&mut self, profile: NetworkProfile) {
        self.network_profiles.push(profile);
    }

    pub fn remove_network_profile(&mut self, ssid: &str) -> bool {
        if let Some(pos) = self.network_profiles.iter().position(|p| p.ssid == ssid) {
            self.network_profiles.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn get_network_profiles(&self) -> Vec<NetworkProfile> {
        self.network_profiles.clone()
    }

    pub fn set_regulatory_domain(&mut self, alpha2: [u8; 2]) {
        self.regulatory.alpha2 = alpha2;
    }

    pub fn get_supported_bands(&self) -> Vec<Band> {
        vec![Band::Band2GHz, Band::Band5GHz]
    }

    pub fn get_supported_channels(&self, band: Band) -> Vec<Channel> {
        match band {
            Band::Band2GHz => {
                let mut channels = Vec::new();
                for i in 1..=13 {
                    channels.push(Channel {
                        frequency: 2412 + (i - 1) * 5,
                        number: i as u8,
                        band: Band::Band2GHz,
                        max_power: 20,
                        flags: 0,
                    });
                }
                channels
            }
            Band::Band5GHz => {
                vec![
                    Channel { frequency: 5180, number: 36, band: Band::Band5GHz, max_power: 23, flags: 0 },
                    Channel { frequency: 5200, number: 40, band: Band::Band5GHz, max_power: 23, flags: 0 },
                    Channel { frequency: 5220, number: 44, band: Band::Band5GHz, max_power: 23, flags: 0 },
                    Channel { frequency: 5240, number: 48, band: Band::Band5GHz, max_power: 23, flags: 0 },
                    Channel { frequency: 5260, number: 52, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5280, number: 56, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5300, number: 60, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5320, number: 64, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5500, number: 100, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5520, number: 104, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5540, number: 108, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5560, number: 112, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5580, number: 116, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5600, number: 120, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5620, number: 124, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5640, number: 128, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5660, number: 132, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5680, number: 136, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5700, number: 140, band: Band::Band5GHz, max_power: 23, flags: 1 },
                    Channel { frequency: 5745, number: 149, band: Band::Band5GHz, max_power: 30, flags: 0 },
                    Channel { frequency: 5765, number: 153, band: Band::Band5GHz, max_power: 30, flags: 0 },
                    Channel { frequency: 5785, number: 157, band: Band::Band5GHz, max_power: 30, flags: 0 },
                    Channel { frequency: 5805, number: 161, band: Band::Band5GHz, max_power: 30, flags: 0 },
                    Channel { frequency: 5825, number: 165, band: Band::Band5GHz, max_power: 30, flags: 0 },
                ]
            }
            Band::Band6GHz => Vec::new(),
        }
    }

    pub fn start_ap(&mut self, _ifindex: u32, _config: ApConfig) -> Result<(), ()> {
        Ok(())
    }

    pub fn stop_ap(&mut self, _ifindex: u32) -> Result<(), ()> {
        Ok(())
    }

    pub fn join_mesh(&mut self, _ifindex: u32, _config: MeshConfig) -> Result<(), ()> {
        Ok(())
    }

    pub fn leave_mesh(&mut self, _ifindex: u32) -> Result<(), ()> {
        Ok(())
    }

    pub fn start_p2p(&mut self, _ifindex: u32, _config: P2PConfig) -> Result<(), ()> {
        Ok(())
    }

    pub fn stop_p2p(&mut self, _ifindex: u32) -> Result<(), ()> {
        Ok(())
    }
}