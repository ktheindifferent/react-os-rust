use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::net::wireless::mac80211::{Mac80211, Band, Channel};

const RTL_PCI_VENDOR_ID: u16 = 0x10EC;

const RTL8188EE: u16 = 0x8179;
const RTL8192EE: u16 = 0x818B;
const RTL8723BE: u16 = 0xB723;
const RTL8821AE: u16 = 0x8821;
const RTL8822BE: u16 = 0xB822;
const RTL8723DE: u16 = 0xD723;
const RTL8821CE: u16 = 0xC821;
const RTL8822CE: u16 = 0xC822;

const RTL_REG_SYS_FUNC_EN: u16 = 0x0002;
const RTL_REG_APS_FSMCO: u16 = 0x0004;
const RTL_REG_SYS_CLKR: u16 = 0x0008;
const RTL_REG_AFE_XTAL_CTRL: u16 = 0x0024;
const RTL_REG_AFE_PLL_CTRL: u16 = 0x0028;
const RTL_REG_MAC_PHY_CTRL: u16 = 0x002C;
const RTL_REG_EFUSE_CTRL: u16 = 0x0030;
const RTL_REG_EFUSE_TEST: u16 = 0x0034;
const RTL_REG_PWR_DATA: u16 = 0x0038;
const RTL_REG_CAL_TIMER: u16 = 0x003C;
const RTL_REG_ACLK_MON: u16 = 0x003E;
const RTL_REG_GPIO_MUXCFG: u16 = 0x0040;
const RTL_REG_MAC_PINMUX_CFG: u16 = 0x0043;
const RTL_REG_GPIO_PIN_CTRL: u16 = 0x0044;
const RTL_REG_GPIO_INTM: u16 = 0x0048;
const RTL_REG_LEDCFG: u16 = 0x004C;
const RTL_REG_FSIMR: u16 = 0x0050;
const RTL_REG_FSISR: u16 = 0x0054;
const RTL_REG_HSIMR: u16 = 0x0058;
const RTL_REG_HSISR: u16 = 0x005C;
const RTL_REG_GPIO_EXT_CTRL: u16 = 0x0060;
const RTL_REG_PAD_CTRL1: u16 = 0x0064;
const RTL_REG_AFE_XTAL_CTRL_EXT: u16 = 0x0078;
const RTL_REG_XCK_OUT_CTRL: u16 = 0x007C;
const RTL_REG_RSV_CTRL: u16 = 0x007E;
const RTL_REG_RF_CTRL: u16 = 0x001F;
const RTL_REG_LDOA15_CTRL: u16 = 0x0020;
const RTL_REG_LDOV12D_CTRL: u16 = 0x0021;
const RTL_REG_LDOHCI12_CTRL: u16 = 0x0022;
const RTL_REG_HCI_OPT_CTRL: u16 = 0x0074;
const RTL_REG_AFE_XTAL_CTRL: u16 = 0x0024;
const RTL_REG_AFE_PLL_CTRL: u16 = 0x0028;
const RTL_REG_MAC_PHY_CTRL: u16 = 0x002C;

const RTL_REG_CR: u16 = 0x0100;
const RTL_REG_PBP: u16 = 0x0104;
const RTL_REG_PKT_BUFF_ACCESS_CTRL: u16 = 0x0106;
const RTL_REG_TRXDMA_CTRL: u16 = 0x010C;
const RTL_REG_TRXFF_BNDY: u16 = 0x0114;
const RTL_REG_TRXFF_STATUS: u16 = 0x0118;
const RTL_REG_RXFF_PTR: u16 = 0x011C;
const RTL_REG_CPWM: u16 = 0x012F;
const RTL_REG_FWIMR: u16 = 0x0130;
const RTL_REG_FWISR: u16 = 0x0134;
const RTL_REG_FTIMR: u16 = 0x0138;
const RTL_REG_FTISR: u16 = 0x013C;
const RTL_REG_PKTBUF_DBG_CTRL: u16 = 0x0140;
const RTL_REG_RXPKTBUF_CTRL: u16 = 0x0142;
const RTL_REG_PKTBUF_DBG_DATA_L: u16 = 0x0144;
const RTL_REG_PKTBUF_DBG_DATA_H: u16 = 0x0148;

const RTL_REG_RQPN: u32 = 0x0200;
const RTL_REG_FIFOPAGE: u32 = 0x0204;
const RTL_REG_DWBCN0_CTRL: u32 = 0x0208;
const RTL_REG_TXDMA_OFFSET_CHK: u32 = 0x020C;
const RTL_REG_TXDMA_STATUS: u32 = 0x0210;
const RTL_REG_RQPN_NPQ: u32 = 0x0214;
const RTL_REG_DWBCN1_CTRL: u32 = 0x0228;

const RTL_REG_RXDMA_AGG_PG_TH: u16 = 0x0280;
const RTL_REG_FW_UPD_RDPTR: u16 = 0x0284;
const RTL_REG_RXDMA_CONTROL: u16 = 0x0286;
const RTL_REG_RXPKT_NUM: u16 = 0x0287;
const RTL_REG_RXDMA_STATUS: u16 = 0x0288;
const RTL_REG_RXDMA_PRO: u16 = 0x0290;
const RTL_REG_EARLY_MODE_CONTROL: u16 = 0x02BC;

#[derive(Debug, Clone)]
pub struct RtlChipInfo {
    pub chip_id: u16,
    pub name: &'static str,
    pub fw_name: &'static str,
    pub rf_type: RfType,
    pub has_bluetooth: bool,
    pub max_tx_power: u8,
    pub tx_queue_num: u8,
    pub rx_queue_num: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum RfType {
    SinglePath,
    DualPath,
    TriplePath,
    QuadPath,
}

impl RtlChipInfo {
    pub fn for_device(device_id: u16) -> Option<Self> {
        match device_id {
            RTL8188EE => Some(Self {
                chip_id: device_id,
                name: "Realtek RTL8188EE",
                fw_name: "rtlwifi/rtl8188efw.bin",
                rf_type: RfType::SinglePath,
                has_bluetooth: false,
                max_tx_power: 20,
                tx_queue_num: 9,
                rx_queue_num: 2,
            }),
            RTL8192EE => Some(Self {
                chip_id: device_id,
                name: "Realtek RTL8192EE",
                fw_name: "rtlwifi/rtl8192eefw.bin",
                rf_type: RfType::DualPath,
                has_bluetooth: false,
                max_tx_power: 20,
                tx_queue_num: 9,
                rx_queue_num: 2,
            }),
            RTL8723BE => Some(Self {
                chip_id: device_id,
                name: "Realtek RTL8723BE",
                fw_name: "rtlwifi/rtl8723befw.bin",
                rf_type: RfType::SinglePath,
                has_bluetooth: true,
                max_tx_power: 20,
                tx_queue_num: 9,
                rx_queue_num: 2,
            }),
            RTL8821AE => Some(Self {
                chip_id: device_id,
                name: "Realtek RTL8821AE",
                fw_name: "rtlwifi/rtl8821aefw.bin",
                rf_type: RfType::SinglePath,
                has_bluetooth: false,
                max_tx_power: 23,
                tx_queue_num: 9,
                rx_queue_num: 2,
            }),
            RTL8822BE => Some(Self {
                chip_id: device_id,
                name: "Realtek RTL8822BE",
                fw_name: "rtlwifi/rtl8822befw.bin",
                rf_type: RfType::DualPath,
                has_bluetooth: true,
                max_tx_power: 23,
                tx_queue_num: 9,
                rx_queue_num: 2,
            }),
            RTL8821CE => Some(Self {
                chip_id: device_id,
                name: "Realtek RTL8821CE",
                fw_name: "rtlwifi/rtl8821cefw.bin",
                rf_type: RfType::SinglePath,
                has_bluetooth: true,
                max_tx_power: 23,
                tx_queue_num: 9,
                rx_queue_num: 2,
            }),
            RTL8822CE => Some(Self {
                chip_id: device_id,
                name: "Realtek RTL8822CE",
                fw_name: "rtlwifi/rtl8822cefw.bin",
                rf_type: RfType::DualPath,
                has_bluetooth: true,
                max_tx_power: 23,
                tx_queue_num: 9,
                rx_queue_num: 2,
            }),
            _ => None,
        }
    }
}

pub struct RtlTxDesc {
    pub pkt_size: u16,
    pub offset: u8,
    pub bmc: bool,
    pub htc: bool,
    pub last_seg: bool,
    pub first_seg: bool,
    pub linip: bool,
    pub no_acm: bool,
    pub gf: bool,
    pub own: bool,
    pub mac_id: u8,
    pub rate_id: u8,
    pub nav_use_hdr: bool,
    pub use_rate: bool,
    pub disable_fb: bool,
    pub cts2self: bool,
    pub rts_enable: bool,
    pub hw_rts_enable: bool,
    pub port_id: u8,
    pub pwr_status: u8,
    pub wait_dcts: bool,
    pub cts2ap_en: bool,
    pub tx_sub_carrier: u8,
    pub tx_stbc: u8,
    pub data_short: bool,
    pub data_bw: u8,
    pub rts_short: bool,
    pub rts_bw: u8,
    pub rts_sc: u8,
    pub rts_stbc: u8,
    pub tx_rate: u8,
    pub data_rate_fb_limit: u8,
    pub rts_rate_fb_limit: u8,
    pub retry_limit_enable: bool,
    pub data_retry_limit: u8,
    pub rts_retry_limit: u8,
    pub use_driver_rate: bool,
    pub olw_pmd: u8,
    pub mac_cp: u8,
    pub raw: bool,
    pub more_frag: bool,
    pub bk: bool,
    pub null_0: bool,
    pub null_1: bool,
    pub tx_ht: bool,
    pub tx_vht: bool,
    pub tx_he: bool,
    pub seq: u16,
}

impl RtlTxDesc {
    pub fn new() -> Self {
        Self {
            pkt_size: 0,
            offset: 0,
            bmc: false,
            htc: false,
            last_seg: true,
            first_seg: true,
            linip: false,
            no_acm: false,
            gf: false,
            own: true,
            mac_id: 0,
            rate_id: 0,
            nav_use_hdr: false,
            use_rate: false,
            disable_fb: false,
            cts2self: false,
            rts_enable: false,
            hw_rts_enable: false,
            port_id: 0,
            pwr_status: 0,
            wait_dcts: false,
            cts2ap_en: false,
            tx_sub_carrier: 0,
            tx_stbc: 0,
            data_short: false,
            data_bw: 0,
            rts_short: false,
            rts_bw: 0,
            rts_sc: 0,
            rts_stbc: 0,
            tx_rate: 0,
            data_rate_fb_limit: 0,
            rts_rate_fb_limit: 0,
            retry_limit_enable: false,
            data_retry_limit: 0,
            rts_retry_limit: 0,
            use_driver_rate: false,
            olw_pmd: 0,
            mac_cp: 0,
            raw: false,
            more_frag: false,
            bk: false,
            null_0: false,
            null_1: false,
            tx_ht: false,
            tx_vht: false,
            tx_he: false,
            seq: 0,
        }
    }

    pub fn to_bytes(&self) -> [u8; 40] {
        let mut desc = [0u8; 40];
        
        let dword0 = (self.pkt_size as u32) |
            ((self.offset as u32) << 16) |
            ((self.bmc as u32) << 24) |
            ((self.htc as u32) << 25) |
            ((self.last_seg as u32) << 26) |
            ((self.first_seg as u32) << 27) |
            ((self.linip as u32) << 28) |
            ((self.no_acm as u32) << 29) |
            ((self.gf as u32) << 30) |
            ((self.own as u32) << 31);
        
        desc[0..4].copy_from_slice(&dword0.to_le_bytes());
        
        desc
    }
}

pub struct RtlRxDesc {
    pub pkt_len: u16,
    pub crc32: bool,
    pub icv: bool,
    pub drv_info_size: u8,
    pub encrypt: u8,
    pub qos: bool,
    pub shift: u8,
    pub phy_status: bool,
    pub swdec: bool,
    pub last_seg: bool,
    pub first_seg: bool,
    pub eor: bool,
    pub own: bool,
    pub seq: u16,
    pub frag: u8,
    pub next_pkt_len: u16,
    pub next_ind: u8,
    pub rsvd: u8,
}

impl RtlRxDesc {
    pub fn from_bytes(desc: &[u8]) -> Self {
        if desc.len() < 32 {
            return Self::new();
        }
        
        let dword0 = u32::from_le_bytes([desc[0], desc[1], desc[2], desc[3]]);
        let dword1 = u32::from_le_bytes([desc[4], desc[5], desc[6], desc[7]]);
        
        Self {
            pkt_len: (dword0 & 0x3FFF) as u16,
            crc32: (dword0 >> 14) & 1 == 1,
            icv: (dword0 >> 15) & 1 == 1,
            drv_info_size: ((dword0 >> 16) & 0x0F) as u8,
            encrypt: ((dword0 >> 20) & 0x07) as u8,
            qos: (dword0 >> 23) & 1 == 1,
            shift: ((dword0 >> 24) & 0x03) as u8,
            phy_status: (dword0 >> 26) & 1 == 1,
            swdec: (dword0 >> 27) & 1 == 1,
            last_seg: (dword0 >> 28) & 1 == 1,
            first_seg: (dword0 >> 29) & 1 == 1,
            eor: (dword0 >> 30) & 1 == 1,
            own: (dword0 >> 31) & 1 == 1,
            seq: (dword1 & 0x0FFF) as u16,
            frag: ((dword1 >> 12) & 0x0F) as u8,
            next_pkt_len: ((dword1 >> 16) & 0x3FFF) as u16,
            next_ind: ((dword1 >> 30) & 0x01) as u8,
            rsvd: 0,
        }
    }

    pub fn new() -> Self {
        Self {
            pkt_len: 0,
            crc32: false,
            icv: false,
            drv_info_size: 0,
            encrypt: 0,
            qos: false,
            shift: 0,
            phy_status: false,
            swdec: false,
            last_seg: false,
            first_seg: false,
            eor: false,
            own: false,
            seq: 0,
            frag: 0,
            next_pkt_len: 0,
            next_ind: 0,
            rsvd: 0,
        }
    }
}

pub struct RtlFirmware {
    pub version: u16,
    pub subversion: u8,
    pub data: Vec<u8>,
    pub size: usize,
    pub loaded: bool,
}

impl RtlFirmware {
    pub fn new() -> Self {
        Self {
            version: 0,
            subversion: 0,
            data: Vec::new(),
            size: 0,
            loaded: false,
        }
    }

    pub fn load(&mut self, fw_data: &[u8]) -> Result<(), ()> {
        if fw_data.len() < 32 {
            return Err(());
        }
        
        self.data = fw_data.to_vec();
        self.size = fw_data.len();
        self.loaded = true;
        
        Ok(())
    }
}

pub struct RealtekWifi {
    pub chip_info: RtlChipInfo,
    pub base_addr: u64,
    pub mac80211: Mac80211,
    pub firmware: RtlFirmware,
    pub mac_addr: [u8; 6],
    pub rf_enabled: AtomicBool,
    pub initialized: AtomicBool,
    pub tx_packets: AtomicU32,
    pub rx_packets: AtomicU32,
    pub tx_errors: AtomicU32,
    pub rx_errors: AtomicU32,
}

impl RealtekWifi {
    pub fn new(device_id: u16, base_addr: u64) -> Result<Self, ()> {
        let chip_info = RtlChipInfo::for_device(device_id).ok_or(())?;
        
        let mac_addr = [0x00, 0x11, 0x22, 0x33, 0x44, 0x66];
        let mac80211 = Mac80211::new(mac_addr);
        
        Ok(Self {
            chip_info,
            base_addr,
            mac80211,
            firmware: RtlFirmware::new(),
            mac_addr,
            rf_enabled: AtomicBool::new(false),
            initialized: AtomicBool::new(false),
            tx_packets: AtomicU32::new(0),
            rx_packets: AtomicU32::new(0),
            tx_errors: AtomicU32::new(0),
            rx_errors: AtomicU32::new(0),
        })
    }

    pub fn init(&mut self) -> Result<(), ()> {
        self.power_on()?;
        
        self.init_llh()?;
        
        self.download_firmware()?;
        
        self.init_mac()?;
        
        self.init_bb_rf()?;
        
        self.enable_interrupt()?;
        
        self.initialized.store(true, Ordering::Release);
        Ok(())
    }

    fn power_on(&self) -> Result<(), ()> {
        self.write8(RTL_REG_APS_FSMCO, 0x02);
        
        let mut value = self.read8(RTL_REG_SYS_FUNC_EN);
        value |= 0x13;
        self.write8(RTL_REG_SYS_FUNC_EN, value);
        
        value = self.read8(RTL_REG_APS_FSMCO);
        value |= 0x10;
        self.write8(RTL_REG_APS_FSMCO, value);
        
        value = self.read8(RTL_REG_SYS_FUNC_EN);
        value &= 0x73;
        self.write8(RTL_REG_SYS_FUNC_EN, value);
        
        value = self.read8(RTL_REG_CR);
        value |= 0x01;
        self.write8(RTL_REG_CR, value);
        
        Ok(())
    }

    fn init_llh(&self) -> Result<(), ()> {
        self.write8(RTL_REG_AFE_PLL_CTRL, 0x80);
        
        self.write8(RTL_REG_AFE_XTAL_CTRL + 1, 0x80);
        
        self.write8(RTL_REG_SYS_FUNC_EN, 0xE3);
        
        self.write8(RTL_REG_AFE_PLL_CTRL, 0x82);
        
        Ok(())
    }

    fn download_firmware(&mut self) -> Result<(), ()> {
        Ok(())
    }

    fn init_mac(&self) -> Result<(), ()> {
        self.write8(RTL_REG_CR, 0x00);
        
        self.write8(RTL_REG_CR, 0xFF);
        
        self.write16(RTL_REG_TRXFF_BNDY, 0x27FF);
        
        self.write8(RTL_REG_PBP, 0x11);
        
        self.write8(RTL_REG_TRXDMA_CTRL, 0x0E);
        
        Ok(())
    }

    fn init_bb_rf(&self) -> Result<(), ()> {
        Ok(())
    }

    fn enable_interrupt(&self) -> Result<(), ()> {
        self.write32(RTL_REG_HSIMR as u32, 0xFFFFFFFF);
        self.write32(RTL_REG_HIMR as u32, 0xFFFFFFFF);
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), ()> {
        if !self.initialized.load(Ordering::Acquire) {
            return Err(());
        }
        
        self.rf_enabled.store(true, Ordering::Release);
        
        let mut cr = self.read8(RTL_REG_CR);
        cr |= 0x0C;
        self.write8(RTL_REG_CR, cr);
        
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), ()> {
        self.rf_enabled.store(false, Ordering::Release);
        
        let mut cr = self.read8(RTL_REG_CR);
        cr &= !0x0C;
        self.write8(RTL_REG_CR, cr);
        
        Ok(())
    }

    pub fn transmit(&mut self, data: &[u8]) -> Result<(), ()> {
        if !self.rf_enabled.load(Ordering::Acquire) {
            return Err(());
        }
        
        let mut desc = RtlTxDesc::new();
        desc.pkt_size = data.len() as u16;
        desc.first_seg = true;
        desc.last_seg = true;
        desc.own = true;
        
        self.tx_packets.fetch_add(1, Ordering::Relaxed);
        
        Ok(())
    }

    pub fn receive(&mut self) -> Option<Vec<u8>> {
        if !self.rf_enabled.load(Ordering::Acquire) {
            return None;
        }
        
        self.rx_packets.fetch_add(1, Ordering::Relaxed);
        
        None
    }

    fn read8(&self, reg: u16) -> u8 {
        unsafe {
            let addr = (self.base_addr + reg as u64) as *const u8;
            core::ptr::read_volatile(addr)
        }
    }

    fn write8(&self, reg: u16, value: u8) {
        unsafe {
            let addr = (self.base_addr + reg as u64) as *mut u8;
            core::ptr::write_volatile(addr, value);
        }
    }

    fn read16(&self, reg: u16) -> u16 {
        unsafe {
            let addr = (self.base_addr + reg as u64) as *const u16;
            core::ptr::read_volatile(addr)
        }
    }

    fn write16(&self, reg: u16, value: u16) {
        unsafe {
            let addr = (self.base_addr + reg as u64) as *mut u16;
            core::ptr::write_volatile(addr, value);
        }
    }

    fn read32(&self, reg: u32) -> u32 {
        unsafe {
            let addr = (self.base_addr + reg as u64) as *const u32;
            core::ptr::read_volatile(addr)
        }
    }

    fn write32(&self, reg: u32, value: u32) {
        unsafe {
            let addr = (self.base_addr + reg as u64) as *mut u32;
            core::ptr::write_volatile(addr, value);
        }
    }

    pub fn scan(&mut self) -> Result<(), ()> {
        self.mac80211.start_scan();
        Ok(())
    }

    pub fn connect(&mut self, ssid: String, bssid: [u8; 6]) -> Result<(), ()> {
        self.mac80211.connect(ssid, bssid);
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), ()> {
        self.mac80211.disconnect();
        Ok(())
    }

    pub fn get_supported_bands(&self) -> Vec<Band> {
        match self.chip_info.chip_id {
            RTL8821AE | RTL8821CE | RTL8822BE | RTL8822CE => {
                vec![Band::Band2GHz, Band::Band5GHz]
            }
            _ => vec![Band::Band2GHz],
        }
    }
}

const RTL_REG_HIMR: u16 = 0x00B0;

pub fn probe_realtek_devices() -> Vec<u16> {
    vec![
        RTL8188EE,
        RTL8192EE,
        RTL8723BE,
        RTL8821AE,
        RTL8822BE,
        RTL8723DE,
        RTL8821CE,
        RTL8822CE,
    ]
}