use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use spin::Mutex;
use crate::serial_println;
use crate::serial_print;

use super::HypervisorError;

pub trait VirtualDevice: Send + Sync {
    fn name(&self) -> &str;
    fn device_id(&self) -> u32;
    
    fn io_read(&mut self, port: u16, size: u32) -> Result<u32, HypervisorError>;
    fn io_write(&mut self, port: u16, value: u32, size: u32) -> Result<(), HypervisorError>;
    
    fn mmio_read(&mut self, addr: u64, size: u32) -> Result<u64, HypervisorError>;
    fn mmio_write(&mut self, addr: u64, value: u64, size: u32) -> Result<(), HypervisorError>;
    
    fn reset(&mut self) -> Result<(), HypervisorError>;
}

pub struct DeviceManager {
    devices: Mutex<Vec<Box<dyn VirtualDevice>>>,
    io_map: Mutex<BTreeMap<u16, usize>>,
    mmio_map: Mutex<BTreeMap<u64, usize>>,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            devices: Mutex::new(Vec::new()),
            io_map: Mutex::new(BTreeMap::new()),
            mmio_map: Mutex::new(BTreeMap::new()),
        }
    }
    
    pub fn add_device(&self, device: Box<dyn VirtualDevice>) -> Result<(), HypervisorError> {
        let mut devices = self.devices.lock();
        devices.push(device);
        Ok(())
    }
    
    pub fn register_io_handler(&self, port: u16, device_idx: usize) -> Result<(), HypervisorError> {
        let mut io_map = self.io_map.lock();
        io_map.insert(port, device_idx);
        Ok(())
    }
    
    pub fn register_mmio_handler(&self, addr: u64, device_idx: usize) -> Result<(), HypervisorError> {
        let mut mmio_map = self.mmio_map.lock();
        mmio_map.insert(addr, device_idx);
        Ok(())
    }
    
    pub fn io_read(&self, port: u16, size: u32) -> Result<u32, HypervisorError> {
        let io_map = self.io_map.lock();
        if let Some(&device_idx) = io_map.get(&port) {
            let mut devices = self.devices.lock();
            if device_idx < devices.len() {
                return devices[device_idx].io_read(port, size);
            }
        }
        Ok(0xFFFFFFFF)
    }
    
    pub fn io_write(&self, port: u16, value: u32, size: u32) -> Result<(), HypervisorError> {
        let io_map = self.io_map.lock();
        if let Some(&device_idx) = io_map.get(&port) {
            let mut devices = self.devices.lock();
            if device_idx < devices.len() {
                return devices[device_idx].io_write(port, value, size);
            }
        }
        Ok(())
    }
    
    pub fn mmio_read(&self, addr: u64, size: u32) -> Result<u64, HypervisorError> {
        let mmio_map = self.mmio_map.lock();
        let base_addr = addr & !0xFFF;
        if let Some(&device_idx) = mmio_map.get(&base_addr) {
            let mut devices = self.devices.lock();
            if device_idx < devices.len() {
                return devices[device_idx].mmio_read(addr, size);
            }
        }
        Ok(0xFFFFFFFFFFFFFFFF)
    }
    
    pub fn mmio_write(&self, addr: u64, value: u64, size: u32) -> Result<(), HypervisorError> {
        let mmio_map = self.mmio_map.lock();
        let base_addr = addr & !0xFFF;
        if let Some(&device_idx) = mmio_map.get(&base_addr) {
            let mut devices = self.devices.lock();
            if device_idx < devices.len() {
                return devices[device_idx].mmio_write(addr, value, size);
            }
        }
        Ok(())
    }
}

pub struct VirtioDevice {
    device_type: VirtioDeviceType,
    device_id: u32,
    device_features: u64,
    driver_features: u64,
    config_generation: u32,
    status: u8,
    queues: Vec<VirtQueue>,
}

#[derive(Debug, Clone, Copy)]
pub enum VirtioDeviceType {
    Network = 1,
    Block = 2,
    Console = 3,
    Rng = 4,
    Balloon = 5,
    Scsi = 8,
    Gpu = 16,
    Input = 18,
    Vsock = 19,
    Crypto = 20,
    Iommu = 23,
    Memory = 24,
    Sound = 25,
}

struct VirtQueue {
    size: u16,
    ready: bool,
    descriptor_table: u64,
    available_ring: u64,
    used_ring: u64,
    last_avail_idx: u16,
    last_used_idx: u16,
}

impl VirtioDevice {
    pub fn new(device_type: VirtioDeviceType, device_id: u32) -> Self {
        Self {
            device_type,
            device_id,
            device_features: Self::get_default_features(device_type),
            driver_features: 0,
            config_generation: 0,
            status: 0,
            queues: Vec::new(),
        }
    }
    
    fn get_default_features(device_type: VirtioDeviceType) -> u64 {
        let mut features = 0u64;
        
        features |= 1 << 32;
        features |= 1 << 33;
        features |= 1 << 28;
        
        match device_type {
            VirtioDeviceType::Network => {
                features |= 1 << 0;
                features |= 1 << 1;
                features |= 1 << 5;
                features |= 1 << 6;
                features |= 1 << 10;
            }
            VirtioDeviceType::Block => {
                features |= 1 << 0;
                features |= 1 << 1;
                features |= 1 << 5;
                features |= 1 << 6;
                features |= 1 << 10;
            }
            _ => {}
        }
        
        features
    }
}

impl VirtualDevice for VirtioDevice {
    fn name(&self) -> &str {
        match self.device_type {
            VirtioDeviceType::Network => "virtio-net",
            VirtioDeviceType::Block => "virtio-blk",
            VirtioDeviceType::Console => "virtio-console",
            VirtioDeviceType::Rng => "virtio-rng",
            VirtioDeviceType::Balloon => "virtio-balloon",
            VirtioDeviceType::Scsi => "virtio-scsi",
            VirtioDeviceType::Gpu => "virtio-gpu",
            VirtioDeviceType::Input => "virtio-input",
            VirtioDeviceType::Vsock => "virtio-vsock",
            VirtioDeviceType::Crypto => "virtio-crypto",
            VirtioDeviceType::Iommu => "virtio-iommu",
            VirtioDeviceType::Memory => "virtio-mem",
            VirtioDeviceType::Sound => "virtio-snd",
        }
    }
    
    fn device_id(&self) -> u32 {
        self.device_id
    }
    
    fn io_read(&mut self, _port: u16, _size: u32) -> Result<u32, HypervisorError> {
        Ok(0)
    }
    
    fn io_write(&mut self, _port: u16, _value: u32, _size: u32) -> Result<(), HypervisorError> {
        Ok(())
    }
    
    fn mmio_read(&mut self, addr: u64, size: u32) -> Result<u64, HypervisorError> {
        let offset = addr & 0xFFF;
        
        let value = match offset {
            0x00 => 0x74726976,
            0x04 => 0x2,
            0x08 => self.device_type as u64,
            0x0C => 0x554D4551,
            0x10 => self.device_features & 0xFFFFFFFF,
            0x14 => (self.device_features >> 32) & 0xFFFFFFFF,
            0x20 => self.driver_features & 0xFFFFFFFF,
            0x24 => (self.driver_features >> 32) & 0xFFFFFFFF,
            0x70 => self.status as u64,
            _ => 0,
        };
        
        Ok(value)
    }
    
    fn mmio_write(&mut self, addr: u64, value: u64, _size: u32) -> Result<(), HypervisorError> {
        let offset = addr & 0xFFF;
        
        match offset {
            0x20 => self.driver_features = (self.driver_features & !0xFFFFFFFF) | (value & 0xFFFFFFFF),
            0x24 => self.driver_features = (self.driver_features & 0xFFFFFFFF) | ((value & 0xFFFFFFFF) << 32),
            0x70 => {
                self.status = value as u8;
                if self.status == 0 {
                    self.reset()?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn reset(&mut self) -> Result<(), HypervisorError> {
        self.status = 0;
        self.driver_features = 0;
        self.config_generation += 1;
        self.queues.clear();
        Ok(())
    }
}

pub struct VirtualSerialPort {
    device_id: u32,
    ier: u8,
    iir: u8,
    lcr: u8,
    mcr: u8,
    lsr: u8,
    msr: u8,
    scratch: u8,
    divisor_latch: u16,
    rx_buffer: Vec<u8>,
    tx_buffer: Vec<u8>,
}

impl VirtualSerialPort {
    pub fn new(device_id: u32) -> Self {
        Self {
            device_id,
            ier: 0,
            iir: 0x01,
            lcr: 0,
            mcr: 0,
            lsr: 0x60,
            msr: 0,
            scratch: 0,
            divisor_latch: 1,
            rx_buffer: Vec::new(),
            tx_buffer: Vec::new(),
        }
    }
}

impl VirtualDevice for VirtualSerialPort {
    fn name(&self) -> &str {
        "serial"
    }
    
    fn device_id(&self) -> u32 {
        self.device_id
    }
    
    fn io_read(&mut self, port: u16, _size: u32) -> Result<u32, HypervisorError> {
        let offset = port & 0x7;
        let value = match offset {
            0 => {
                if self.lcr & 0x80 != 0 {
                    (self.divisor_latch & 0xFF) as u32
                } else if !self.rx_buffer.is_empty() {
                    self.rx_buffer.remove(0) as u32
                } else {
                    0
                }
            }
            1 => {
                if self.lcr & 0x80 != 0 {
                    ((self.divisor_latch >> 8) & 0xFF) as u32
                } else {
                    self.ier as u32
                }
            }
            2 => self.iir as u32,
            3 => self.lcr as u32,
            4 => self.mcr as u32,
            5 => self.lsr as u32,
            6 => self.msr as u32,
            7 => self.scratch as u32,
            _ => 0,
        };
        Ok(value)
    }
    
    fn io_write(&mut self, port: u16, value: u32, _size: u32) -> Result<(), HypervisorError> {
        let offset = port & 0x7;
        match offset {
            0 => {
                if self.lcr & 0x80 != 0 {
                    self.divisor_latch = (self.divisor_latch & 0xFF00) | (value as u16 & 0xFF);
                } else {
                    self.tx_buffer.push(value as u8);
                    serial_print!("{}", value as u8 as char);
                }
            }
            1 => {
                if self.lcr & 0x80 != 0 {
                    self.divisor_latch = (self.divisor_latch & 0x00FF) | ((value as u16 & 0xFF) << 8);
                } else {
                    self.ier = value as u8;
                }
            }
            2 => {}
            3 => self.lcr = value as u8,
            4 => self.mcr = value as u8,
            5 => {}
            6 => {}
            7 => self.scratch = value as u8,
            _ => {}
        }
        Ok(())
    }
    
    fn mmio_read(&mut self, _addr: u64, _size: u32) -> Result<u64, HypervisorError> {
        Ok(0)
    }
    
    fn mmio_write(&mut self, _addr: u64, _value: u64, _size: u32) -> Result<(), HypervisorError> {
        Ok(())
    }
    
    fn reset(&mut self) -> Result<(), HypervisorError> {
        self.ier = 0;
        self.iir = 0x01;
        self.lcr = 0;
        self.mcr = 0;
        self.lsr = 0x60;
        self.msr = 0;
        self.scratch = 0;
        self.divisor_latch = 1;
        self.rx_buffer.clear();
        self.tx_buffer.clear();
        Ok(())
    }
}