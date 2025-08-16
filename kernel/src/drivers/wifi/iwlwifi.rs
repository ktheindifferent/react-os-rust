use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::net::wireless::mac80211::{Mac80211, Band, Channel};

const IWL_PCI_VENDOR_ID: u16 = 0x8086;

const IWL_DEVICE_7260: u16 = 0x08B1;
const IWL_DEVICE_7265: u16 = 0x095A;
const IWL_DEVICE_8260: u16 = 0x24F3;
const IWL_DEVICE_8265: u16 = 0x24FD;
const IWL_DEVICE_9260: u16 = 0x2526;
const IWL_DEVICE_9560: u16 = 0x9DF0;
const IWL_DEVICE_AX200: u16 = 0x2723;
const IWL_DEVICE_AX201: u16 = 0x06F0;
const IWL_DEVICE_AX210: u16 = 0x2725;

const IWL_CSR_HW_IF_CONFIG_REG: u32 = 0x000;
const IWL_CSR_INT_MASK: u32 = 0x00C;
const IWL_CSR_INT: u32 = 0x008;
const IWL_CSR_FH_INT_STATUS: u32 = 0x010;
const IWL_CSR_RESET: u32 = 0x020;
const IWL_CSR_GP_CNTRL: u32 = 0x024;
const IWL_CSR_HW_REV: u32 = 0x028;
const IWL_CSR_EEPROM_REG: u32 = 0x02C;
const IWL_CSR_OTP_GP_REG: u32 = 0x034;
const IWL_CSR_GIO_REG: u32 = 0x03C;
const IWL_CSR_UCODE_DRV_GP1: u32 = 0x054;
const IWL_CSR_UCODE_DRV_GP2: u32 = 0x058;

const IWL_FH_MEM_RCSR_CHNL0_CONFIG_REG: u32 = 0x1C00;
const IWL_FH_MEM_RCSR_CHNL0_RBDCB_BASE_REG: u32 = 0x1C04;
const IWL_FH_MEM_RCSR_CHNL0_WPTR_REG: u32 = 0x1C08;

const IWL_FH_RSCSR_CHNL0_RBDCB_BASE_REG: u32 = 0x1C04;
const IWL_FH_RSCSR_CHNL0_RBDCB_WPTR_REG: u32 = 0x1C08;
const IWL_FH_RSCSR_CHNL0_STTS_WPTR_REG: u32 = 0x1C0C;

const IWL_FH_TCSR_CHNL_TX_CONFIG_REG: u32 = 0x1D00;
const IWL_FH_TCSR_CHNL_TX_CREDIT_REG: u32 = 0x1D04;
const IWL_FH_TCSR_CHNL_TX_BUF_STS_REG: u32 = 0x1D08;

const IWL_RX_BUFFER_SIZE: usize = 4096;
const IWL_TX_BUFFER_SIZE: usize = 4096;
const IWL_RX_QUEUE_SIZE: usize = 256;
const IWL_TX_QUEUE_SIZE: usize = 256;
const IWL_NUM_QUEUES: usize = 32;

#[derive(Debug, Clone, Copy)]
pub struct IwlDeviceConfig {
    pub device_id: u16,
    pub name: &'static str,
    pub fw_name: &'static str,
    pub max_txq_num: u32,
    pub max_rxq_num: u32,
    pub max_tx_agg_size: u32,
    pub num_rbds: u32,
    pub ucode_api_max: u32,
    pub ucode_api_min: u32,
    pub ht_params: HtParams,
    pub nvm_size: u32,
    pub eeprom_size: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct HtParams {
    pub ht40_bands: u8,
    pub use_rts_for_aggregation: bool,
    pub ht_greenfield_support: bool,
    pub stbc_tx: bool,
    pub stbc_rx: u8,
    pub ldpc: bool,
}

impl IwlDeviceConfig {
    pub fn for_device(device_id: u16) -> Option<Self> {
        match device_id {
            IWL_DEVICE_7260 => Some(Self {
                device_id,
                name: "Intel Dual Band Wireless AC 7260",
                fw_name: "iwlwifi-7260.ucode",
                max_txq_num: 20,
                max_rxq_num: 1,
                max_tx_agg_size: 64,
                num_rbds: 256,
                ucode_api_max: 17,
                ucode_api_min: 16,
                ht_params: HtParams {
                    ht40_bands: 0x3,
                    use_rts_for_aggregation: true,
                    ht_greenfield_support: true,
                    stbc_tx: true,
                    stbc_rx: 1,
                    ldpc: true,
                },
                nvm_size: 16384,
                eeprom_size: 2048,
            }),
            IWL_DEVICE_AX200 => Some(Self {
                device_id,
                name: "Intel Wi-Fi 6 AX200",
                fw_name: "iwlwifi-cc-a0.ucode",
                max_txq_num: 512,
                max_rxq_num: 16,
                max_tx_agg_size: 256,
                num_rbds: 4096,
                ucode_api_max: 55,
                ucode_api_min: 48,
                ht_params: HtParams {
                    ht40_bands: 0x3,
                    use_rts_for_aggregation: false,
                    ht_greenfield_support: true,
                    stbc_tx: true,
                    stbc_rx: 2,
                    ldpc: true,
                },
                nvm_size: 32768,
                eeprom_size: 0,
            }),
            IWL_DEVICE_AX210 => Some(Self {
                device_id,
                name: "Intel Wi-Fi 6E AX210",
                fw_name: "iwlwifi-ty-a0.ucode",
                max_txq_num: 512,
                max_rxq_num: 16,
                max_tx_agg_size: 256,
                num_rbds: 4096,
                ucode_api_max: 63,
                ucode_api_min: 56,
                ht_params: HtParams {
                    ht40_bands: 0x7,
                    use_rts_for_aggregation: false,
                    ht_greenfield_support: true,
                    stbc_tx: true,
                    stbc_rx: 2,
                    ldpc: true,
                },
                nvm_size: 32768,
                eeprom_size: 0,
            }),
            _ => None,
        }
    }
}

#[repr(C, packed)]
pub struct IwlRxPacket {
    pub len_n_flags: u32,
    pub hdr: IwlCmdHeader,
    pub data: [u8; 0],
}

#[repr(C, packed)]
pub struct IwlCmdHeader {
    pub cmd: u8,
    pub flags: u8,
    pub sequence: u16,
}

#[repr(C, packed)]
pub struct IwlTxCmd {
    pub len: u16,
    pub offload: u16,
    pub tx_flags: u32,
    pub rate_n_flags: u32,
    pub sta_id: u8,
    pub sec_ctl: u8,
    pub initial_rate_index: u8,
    pub reserved: u8,
    pub key: [u8; 16],
    pub reserved2: u16,
    pub life_time: u32,
    pub dram_lsb_ptr: u32,
    pub dram_msb_ptr: u8,
    pub rts_retry_limit: u8,
    pub data_retry_limit: u8,
    pub tid_tspec: u8,
    pub pm_frame_timeout: u16,
    pub reserved3: u16,
}

pub struct RxBuffer {
    pub dma_addr: u64,
    pub virtual_addr: Vec<u8>,
}

impl RxBuffer {
    pub fn new() -> Self {
        Self {
            dma_addr: 0,
            virtual_addr: vec![0u8; IWL_RX_BUFFER_SIZE],
        }
    }
}

pub struct TxBuffer {
    pub dma_addr: u64,
    pub virtual_addr: Vec<u8>,
    pub in_use: bool,
}

impl TxBuffer {
    pub fn new() -> Self {
        Self {
            dma_addr: 0,
            virtual_addr: vec![0u8; IWL_TX_BUFFER_SIZE],
            in_use: false,
        }
    }
}

pub struct RxQueue {
    pub buffers: Vec<RxBuffer>,
    pub read: AtomicU32,
    pub write: AtomicU32,
    pub free_count: AtomicU32,
}

impl RxQueue {
    pub fn new(size: usize) -> Self {
        let mut buffers = Vec::with_capacity(size);
        for _ in 0..size {
            buffers.push(RxBuffer::new());
        }
        
        Self {
            buffers,
            read: AtomicU32::new(0),
            write: AtomicU32::new(0),
            free_count: AtomicU32::new(size as u32),
        }
    }
}

pub struct TxQueue {
    pub buffers: Vec<TxBuffer>,
    pub read: AtomicU32,
    pub write: AtomicU32,
    pub used: AtomicU32,
    pub queue_id: u8,
}

impl TxQueue {
    pub fn new(size: usize, queue_id: u8) -> Self {
        let mut buffers = Vec::with_capacity(size);
        for _ in 0..size {
            buffers.push(TxBuffer::new());
        }
        
        Self {
            buffers,
            read: AtomicU32::new(0),
            write: AtomicU32::new(0),
            used: AtomicU32::new(0),
            queue_id,
        }
    }

    pub fn enqueue(&mut self, data: &[u8]) -> Result<(), ()> {
        let write_idx = self.write.load(Ordering::Acquire) as usize;
        let next_write = (write_idx + 1) % self.buffers.len();
        let read_idx = self.read.load(Ordering::Acquire) as usize;
        
        if next_write == read_idx {
            return Err(());
        }
        
        let buffer = &mut self.buffers[write_idx];
        if buffer.in_use {
            return Err(());
        }
        
        buffer.virtual_addr[..data.len()].copy_from_slice(data);
        buffer.in_use = true;
        
        self.write.store(next_write as u32, Ordering::Release);
        self.used.fetch_add(1, Ordering::AcqRel);
        
        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<Vec<u8>> {
        let read_idx = self.read.load(Ordering::Acquire) as usize;
        let write_idx = self.write.load(Ordering::Acquire) as usize;
        
        if read_idx == write_idx {
            return None;
        }
        
        let buffer = &mut self.buffers[read_idx];
        if !buffer.in_use {
            return None;
        }
        
        let data = buffer.virtual_addr.clone();
        buffer.in_use = false;
        
        let next_read = (read_idx + 1) % self.buffers.len();
        self.read.store(next_read as u32, Ordering::Release);
        self.used.fetch_sub(1, Ordering::AcqRel);
        
        Some(data)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FirmwareState {
    None,
    Loading,
    Loaded,
    Init,
    Running,
    Error,
}

pub struct Firmware {
    pub state: FirmwareState,
    pub ucode_ver: u32,
    pub build_ver: u32,
    pub inst_size: u32,
    pub data_size: u32,
    pub init_size: u32,
    pub boot_size: u32,
    pub inst_data: Vec<u8>,
    pub data_data: Vec<u8>,
    pub init_data: Vec<u8>,
    pub boot_data: Vec<u8>,
}

impl Firmware {
    pub fn new() -> Self {
        Self {
            state: FirmwareState::None,
            ucode_ver: 0,
            build_ver: 0,
            inst_size: 0,
            data_size: 0,
            init_size: 0,
            boot_size: 0,
            inst_data: Vec::new(),
            data_data: Vec::new(),
            init_data: Vec::new(),
            boot_data: Vec::new(),
        }
    }

    pub fn load(&mut self, _fw_data: &[u8]) -> Result<(), ()> {
        self.state = FirmwareState::Loading;
        
        self.state = FirmwareState::Loaded;
        Ok(())
    }
}

pub struct NvmData {
    pub hw_addr: [u8; 6],
    pub valid_tx_ant: u8,
    pub valid_rx_ant: u8,
    pub nvm_version: u16,
    pub board_type: u8,
    pub crystal_freq: u8,
}

impl NvmData {
    pub fn new() -> Self {
        Self {
            hw_addr: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
            valid_tx_ant: 0x3,
            valid_rx_ant: 0x3,
            nvm_version: 0,
            board_type: 0,
            crystal_freq: 0,
        }
    }
}

pub struct Statistics {
    pub rx_packets: AtomicU32,
    pub tx_packets: AtomicU32,
    pub rx_bytes: AtomicU32,
    pub tx_bytes: AtomicU32,
    pub rx_errors: AtomicU32,
    pub tx_errors: AtomicU32,
    pub rx_dropped: AtomicU32,
    pub tx_dropped: AtomicU32,
}

impl Statistics {
    pub fn new() -> Self {
        Self {
            rx_packets: AtomicU32::new(0),
            tx_packets: AtomicU32::new(0),
            rx_bytes: AtomicU32::new(0),
            tx_bytes: AtomicU32::new(0),
            rx_errors: AtomicU32::new(0),
            tx_errors: AtomicU32::new(0),
            rx_dropped: AtomicU32::new(0),
            tx_dropped: AtomicU32::new(0),
        }
    }
}

pub struct IwlWifi {
    pub config: IwlDeviceConfig,
    pub base_addr: u64,
    pub mac80211: Mac80211,
    pub firmware: Firmware,
    pub nvm_data: NvmData,
    pub rx_queue: RxQueue,
    pub tx_queues: Vec<TxQueue>,
    pub statistics: Statistics,
    pub rf_kill: AtomicBool,
    pub initialized: AtomicBool,
}

impl IwlWifi {
    pub fn new(device_id: u16, base_addr: u64) -> Result<Self, ()> {
        let config = IwlDeviceConfig::for_device(device_id).ok_or(())?;
        
        let nvm_data = NvmData::new();
        let mac80211 = Mac80211::new(nvm_data.hw_addr);
        
        let mut tx_queues = Vec::new();
        for i in 0..IWL_NUM_QUEUES {
            tx_queues.push(TxQueue::new(IWL_TX_QUEUE_SIZE, i as u8));
        }
        
        Ok(Self {
            config,
            base_addr,
            mac80211,
            firmware: Firmware::new(),
            nvm_data,
            rx_queue: RxQueue::new(IWL_RX_QUEUE_SIZE),
            tx_queues,
            statistics: Statistics::new(),
            rf_kill: AtomicBool::new(false),
            initialized: AtomicBool::new(false),
        })
    }

    pub fn init(&mut self) -> Result<(), ()> {
        self.reset_hardware()?;
        
        self.load_firmware()?;
        
        self.read_nvm()?;
        
        self.init_rx_queue()?;
        self.init_tx_queues()?;
        
        self.configure_device()?;
        
        self.initialized.store(true, Ordering::Release);
        Ok(())
    }

    fn reset_hardware(&self) -> Result<(), ()> {
        self.write_reg(IWL_CSR_RESET, 0x00000001);
        
        core::hint::spin_loop();
        
        self.write_reg(IWL_CSR_RESET, 0x00000000);
        
        let hw_rev = self.read_reg(IWL_CSR_HW_REV);
        if hw_rev == 0xFFFFFFFF {
            return Err(());
        }
        
        Ok(())
    }

    fn load_firmware(&mut self) -> Result<(), ()> {
        Ok(())
    }

    fn read_nvm(&mut self) -> Result<(), ()> {
        Ok(())
    }

    fn init_rx_queue(&mut self) -> Result<(), ()> {
        let rb_size = IWL_RX_BUFFER_SIZE;
        let rb_count = self.rx_queue.buffers.len();
        
        self.write_reg(IWL_FH_MEM_RCSR_CHNL0_CONFIG_REG, 
            (rb_size << 16) | (rb_count as u32));
        
        self.write_reg(IWL_FH_MEM_RCSR_CHNL0_RBDCB_BASE_REG, 0);
        
        self.write_reg(IWL_FH_MEM_RCSR_CHNL0_WPTR_REG, 0);
        
        Ok(())
    }

    fn init_tx_queues(&mut self) -> Result<(), ()> {
        for queue in &self.tx_queues {
            let queue_id = queue.queue_id;
            let reg_base = IWL_FH_TCSR_CHNL_TX_CONFIG_REG + (queue_id as u32 * 0x20);
            
            self.write_reg(reg_base, 0x80000000);
            
            self.write_reg(reg_base + 4, 0);
        }
        
        Ok(())
    }

    fn configure_device(&mut self) -> Result<(), ()> {
        self.write_reg(IWL_CSR_INT_MASK, 0xFFFFFFFF);
        
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), ()> {
        if !self.initialized.load(Ordering::Acquire) {
            return Err(());
        }
        
        if self.rf_kill.load(Ordering::Acquire) {
            return Err(());
        }
        
        self.firmware.state = FirmwareState::Running;
        
        self.enable_interrupts();
        
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), ()> {
        self.disable_interrupts();
        
        self.firmware.state = FirmwareState::Init;
        
        Ok(())
    }

    pub fn transmit(&mut self, data: &[u8], queue_id: usize) -> Result<(), ()> {
        if queue_id >= self.tx_queues.len() {
            return Err(());
        }
        
        self.tx_queues[queue_id].enqueue(data)?;
        
        self.statistics.tx_packets.fetch_add(1, Ordering::Relaxed);
        self.statistics.tx_bytes.fetch_add(data.len() as u32, Ordering::Relaxed);
        
        self.kick_tx_queue(queue_id as u8);
        
        Ok(())
    }

    pub fn receive(&mut self) -> Option<Vec<u8>> {
        let read_idx = self.rx_queue.read.load(Ordering::Acquire) as usize;
        let write_idx = self.rx_queue.write.load(Ordering::Acquire) as usize;
        
        if read_idx == write_idx {
            return None;
        }
        
        let buffer = &self.rx_queue.buffers[read_idx];
        let data = buffer.virtual_addr.clone();
        
        self.statistics.rx_packets.fetch_add(1, Ordering::Relaxed);
        self.statistics.rx_bytes.fetch_add(data.len() as u32, Ordering::Relaxed);
        
        let next_read = (read_idx + 1) % self.rx_queue.buffers.len();
        self.rx_queue.read.store(next_read as u32, Ordering::Release);
        self.rx_queue.free_count.fetch_add(1, Ordering::AcqRel);
        
        Some(data)
    }

    pub fn handle_interrupt(&mut self) {
        let inta = self.read_reg(IWL_CSR_INT);
        
        if inta == 0 || inta == 0xFFFFFFFF {
            return;
        }
        
        self.write_reg(IWL_CSR_INT, inta);
        
        if inta & 0x80000000 != 0 {
            self.handle_rx_interrupt();
        }
        
        if inta & 0x40000000 != 0 {
            self.handle_tx_interrupt();
        }
        
        if inta & 0x00000004 != 0 {
            self.handle_rf_kill();
        }
        
        if inta & 0x02000000 != 0 {
            self.handle_firmware_error();
        }
    }

    fn handle_rx_interrupt(&mut self) {
        while let Some(_packet) = self.receive() {
        }
    }

    fn handle_tx_interrupt(&mut self) {
        for queue in &mut self.tx_queues {
            while let Some(_) = queue.dequeue() {
            }
        }
    }

    fn handle_rf_kill(&mut self) {
        let hw_rf_kill = self.read_reg(IWL_CSR_GP_CNTRL) & 0x08000000 != 0;
        self.rf_kill.store(hw_rf_kill, Ordering::Release);
    }

    fn handle_firmware_error(&mut self) {
        self.firmware.state = FirmwareState::Error;
        self.stop().ok();
    }

    fn kick_tx_queue(&self, queue_id: u8) {
        let reg = IWL_FH_TCSR_CHNL_TX_BUF_STS_REG + (queue_id as u32 * 0x20);
        self.write_reg(reg, 0x00000001);
    }

    fn enable_interrupts(&self) {
        self.write_reg(IWL_CSR_INT_MASK, 0);
    }

    fn disable_interrupts(&self) {
        self.write_reg(IWL_CSR_INT_MASK, 0xFFFFFFFF);
    }

    fn read_reg(&self, offset: u32) -> u32 {
        unsafe {
            let addr = (self.base_addr + offset as u64) as *const u32;
            core::ptr::read_volatile(addr)
        }
    }

    fn write_reg(&self, offset: u32, value: u32) {
        unsafe {
            let addr = (self.base_addr + offset as u64) as *mut u32;
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
                let mut channels = Vec::new();
                for &ch in &[36, 40, 44, 48, 52, 56, 60, 64, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 149, 153, 157, 161, 165] {
                    channels.push(Channel {
                        frequency: 5000 + ch * 5,
                        number: ch,
                        band: Band::Band5GHz,
                        max_power: 23,
                        flags: if ch >= 52 && ch <= 140 { 1 } else { 0 },
                    });
                }
                channels
            }
            _ => Vec::new(),
        }
    }
}

pub fn probe_iwlwifi_devices() -> Vec<u16> {
    vec![
        IWL_DEVICE_7260,
        IWL_DEVICE_7265,
        IWL_DEVICE_8260,
        IWL_DEVICE_8265,
        IWL_DEVICE_9260,
        IWL_DEVICE_9560,
        IWL_DEVICE_AX200,
        IWL_DEVICE_AX201,
        IWL_DEVICE_AX210,
    ]
}