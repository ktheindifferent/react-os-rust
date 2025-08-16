// PCIe Device Management
use super::*;
use alloc::vec::Vec;

impl PciDevice {
    pub fn get_bar_info(&self, bar_index: usize, controller: &PcieController) -> Option<BarInfo> {
        if bar_index >= 6 {
            return None;
        }
        
        let bars = self.decode_bars(controller);
        bars.get(bar_index).map(|bar| match bar {
            BarType::Memory32 { address, size, prefetchable } => BarInfo {
                index: bar_index,
                base_address: *address as u64,
                size: *size as u64,
                is_io: false,
                is_64bit: false,
                is_prefetchable: *prefetchable,
            },
            BarType::Memory64 { address, size, prefetchable } => BarInfo {
                index: bar_index,
                base_address: *address,
                size: *size,
                is_io: false,
                is_64bit: true,
                is_prefetchable: *prefetchable,
            },
            BarType::Io { address, size } => BarInfo {
                index: bar_index,
                base_address: *address as u64,
                size: *size as u64,
                is_io: true,
                is_64bit: false,
                is_prefetchable: false,
            },
        })
    }
    
    pub fn get_device_name(&self) -> String {
        format!("{:04x}:{:04x}", self.vendor_id, self.device_id)
    }
    
    pub fn is_bridge(&self) -> bool {
        self.class == PCI_CLASS_BRIDGE
    }
    
    pub fn is_storage(&self) -> bool {
        self.class == PCI_CLASS_STORAGE
    }
    
    pub fn is_network(&self) -> bool {
        self.class == PCI_CLASS_NETWORK
    }
    
    pub fn is_display(&self) -> bool {
        self.class == PCI_CLASS_DISPLAY
    }
    
    pub fn supports_msi(&self) -> bool {
        self.has_capability(PCI_CAP_ID_MSI)
    }
    
    pub fn supports_msix(&self) -> bool {
        self.has_capability(PCI_CAP_ID_MSIX)
    }
    
    pub fn supports_pcie(&self) -> bool {
        self.has_capability(PCI_CAP_ID_EXP)
    }
}

#[derive(Debug, Clone)]
pub struct BarInfo {
    pub index: usize,
    pub base_address: u64,
    pub size: u64,
    pub is_io: bool,
    pub is_64bit: bool,
    pub is_prefetchable: bool,
}

// Device driver registration
pub trait PciDriver: Send + Sync {
    fn probe(&mut self, device: &PciDevice) -> Result<(), &'static str>;
    fn remove(&mut self, device: &PciDevice);
    fn suspend(&mut self, device: &PciDevice);
    fn resume(&mut self, device: &PciDevice);
}

pub struct PciDriverManager {
    drivers: Vec<(u16, u16, Box<dyn PciDriver>)>, // (vendor_id, device_id, driver)
}

impl PciDriverManager {
    pub fn new() -> Self {
        Self {
            drivers: Vec::new(),
        }
    }
    
    pub fn register_driver(&mut self, vendor_id: u16, device_id: u16, driver: Box<dyn PciDriver>) {
        self.drivers.push((vendor_id, device_id, driver));
    }
    
    pub fn probe_device(&mut self, device: &PciDevice) -> Result<(), &'static str> {
        for (vid, did, driver) in &mut self.drivers {
            if *vid == device.vendor_id && *did == device.device_id {
                return driver.probe(device);
            }
        }
        Ok(()) // No driver found
    }
}