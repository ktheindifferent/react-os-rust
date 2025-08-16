//! Bus Infrastructure - Generic bus framework for device enumeration and management

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::{
    fmt,
    sync::atomic::{AtomicU32, Ordering},
};
use spin::RwLock;

use super::{
    Device, DeviceClass, DeviceId, DeviceResource, Driver, DriverError, Result,
    model::{DeviceBuilder, DeviceCapabilities, MemoryFlags, InterruptFlags},
};

/// Bus type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusType {
    Pci,
    Usb,
    I2c,
    Spi,
    Platform,
    Virtual,
    Isa,
    Scsi,
    Nvme,
}

/// Generic bus trait
pub trait Bus: Send + Sync {
    /// Get bus name
    fn name(&self) -> &str;
    
    /// Get bus type
    fn bus_type(&self) -> BusType;
    
    /// Enumerate devices on bus
    fn enumerate(&self) -> Result<Vec<Arc<Device>>>;
    
    /// Rescan bus for new devices
    fn rescan(&self) -> Result<()>;
    
    /// Match device to driver
    fn match_device(&self, device: &Device, driver: &dyn Driver) -> bool;
    
    /// Add device to bus
    fn add_device(&self, device: Arc<Device>) -> Result<()>;
    
    /// Remove device from bus
    fn remove_device(&self, id: DeviceId) -> Result<()>;
    
    /// Suspend bus
    fn suspend(&self) -> Result<()> {
        Ok(())
    }
    
    /// Resume bus
    fn resume(&self) -> Result<()> {
        Ok(())
    }
}

/// Bus driver trait for bus controllers
pub trait BusDriver: Driver {
    /// Initialize bus controller
    fn init_controller(&self) -> Result<()>;
    
    /// Scan bus for devices
    fn scan_bus(&self) -> Result<Vec<Arc<Device>>>;
    
    /// Configure bus settings
    fn configure_bus(&self, config: BusConfig) -> Result<()>;
}

/// Bus configuration
#[derive(Debug, Clone)]
pub struct BusConfig {
    pub speed: BusSpeed,
    pub width: u32,
    pub max_devices: u32,
    pub power_budget: u32, // in milliwatts
}

/// Bus speed settings
#[derive(Debug, Clone, Copy)]
pub enum BusSpeed {
    Low,
    Full,
    High,
    Super,
    Custom(u32), // in MHz
}

/// PCI bus implementation
pub struct PciBus {
    devices: RwLock<BTreeMap<PciAddress, Arc<Device>>>,
    config_space: RwLock<PciConfigSpace>,
}

/// PCI address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PciAddress {
    pub segment: u16,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}

impl PciAddress {
    pub const fn new(segment: u16, bus: u8, device: u8, function: u8) -> Self {
        Self {
            segment,
            bus,
            device,
            function,
        }
    }
}

/// PCI configuration space
pub struct PciConfigSpace {
    data: BTreeMap<PciAddress, PciConfig>,
}

/// PCI device configuration
#[derive(Debug, Clone)]
pub struct PciConfig {
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u32,
    pub bars: [PciBar; 6],
    pub irq: u8,
    pub capabilities: Vec<PciCapability>,
}

/// PCI Base Address Register
#[derive(Debug, Clone, Copy)]
pub enum PciBar {
    None,
    Memory {
        base: u64,
        size: u32,
        prefetchable: bool,
        is_64bit: bool,
    },
    Io {
        base: u32,
        size: u32,
    },
}

/// PCI capability
#[derive(Debug, Clone)]
pub struct PciCapability {
    pub id: u8,
    pub offset: u8,
    pub data: Vec<u8>,
}

impl PciBus {
    /// Create new PCI bus
    pub fn new() -> Self {
        Self {
            devices: RwLock::new(BTreeMap::new()),
            config_space: RwLock::new(PciConfigSpace {
                data: BTreeMap::new(),
            }),
        }
    }
    
    /// Scan PCI bus for devices
    fn scan_pci_bus(&self) -> Result<Vec<Arc<Device>>> {
        let mut found_devices = Vec::new();
        
        // Scan all possible PCI addresses
        for bus in 0..=255u8 {
            for device in 0..32u8 {
                for function in 0..8u8 {
                    let addr = PciAddress::new(0, bus, device, function);
                    
                    if let Some(pci_device) = self.probe_pci_device(addr)? {
                        found_devices.push(pci_device);
                    }
                }
            }
        }
        
        Ok(found_devices)
    }
    
    /// Probe PCI device at address
    fn probe_pci_device(&self, addr: PciAddress) -> Result<Option<Arc<Device>>> {
        // Read vendor ID (would be actual PCI config space read)
        let vendor_id = self.read_config_word(addr, 0x00)?;
        
        if vendor_id == 0xFFFF {
            return Ok(None); // No device
        }
        
        let device_id = self.read_config_word(addr, 0x02)?;
        let class_code = self.read_config_dword(addr, 0x08)?;
        
        // Create device
        let name = format!("pci:{:04x}:{:04x}", vendor_id, device_id);
        let class = self.class_from_code(class_code);
        
        let device = DeviceBuilder::new(name, class)
            .capabilities(DeviceCapabilities {
                dma_capable: true,
                bus_master: true,
                wake_capable: false,
                power_manageable: true,
                hot_pluggable: self.is_hotplug_slot(addr),
                iommu_mapped: false,
            })
            .build();
        
        // Add PCI resources
        self.add_pci_resources(&device, addr)?;
        
        let device = Arc::new(device);
        
        // Store in bus registry
        self.devices.write().insert(addr, device.clone());
        
        Ok(Some(device))
    }
    
    /// Read PCI config word
    fn read_config_word(&self, addr: PciAddress, offset: u16) -> Result<u16> {
        // Would perform actual PCI config space read
        Ok(0x0000)
    }
    
    /// Read PCI config dword
    fn read_config_dword(&self, addr: PciAddress, offset: u16) -> Result<u32> {
        // Would perform actual PCI config space read
        Ok(0x00000000)
    }
    
    /// Convert class code to device class
    fn class_from_code(&self, code: u32) -> DeviceClass {
        let class = (code >> 24) & 0xFF;
        let subclass = (code >> 16) & 0xFF;
        
        match (class, subclass) {
            (0x01, _) => DeviceClass::BlockDevice,      // Mass storage
            (0x02, _) => DeviceClass::NetworkInterface, // Network
            (0x03, _) => DeviceClass::DisplayController,// Display
            (0x04, _) => DeviceClass::AudioController,  // Multimedia
            (0x06, 0x04) => DeviceClass::PciBridge,    // PCI bridge
            (0x0C, 0x03) => DeviceClass::UsbController,// USB
            _ => DeviceClass::Unknown,
        }
    }
    
    /// Check if slot supports hot-plug
    fn is_hotplug_slot(&self, addr: PciAddress) -> bool {
        // Would check for hot-plug capability
        false
    }
    
    /// Add PCI resources to device
    fn add_pci_resources(&self, device: &Device, addr: PciAddress) -> Result<()> {
        // Read BARs
        for i in 0..6 {
            let bar_offset = 0x10 + (i * 4);
            let bar_value = self.read_config_dword(addr, bar_offset)?;
            
            if bar_value != 0 {
                // Decode BAR
                if bar_value & 0x1 == 0 {
                    // Memory BAR
                    let base = bar_value & !0xF;
                    let prefetchable = (bar_value & 0x8) != 0;
                    
                    device.add_resource(DeviceResource::Memory {
                        base: base as u64,
                        size: 0x1000, // Would probe actual size
                        flags: MemoryFlags {
                            prefetchable,
                            cacheable: false,
                            readonly: false,
                            mmio: true,
                        },
                    });
                } else {
                    // I/O BAR
                    let base = bar_value & !0x3;
                    
                    device.add_resource(DeviceResource::Io {
                        base: base as u16,
                        size: 0x100, // Would probe actual size
                    });
                }
            }
        }
        
        // Add interrupt
        let irq = self.read_config_word(addr, 0x3C)? & 0xFF;
        if irq != 0 && irq != 0xFF {
            device.add_resource(DeviceResource::Interrupt {
                irq: irq as u32,
                flags: InterruptFlags {
                    edge_triggered: false,
                    active_low: true,
                    shared: true,
                    msi: false,
                },
            });
        }
        
        Ok(())
    }
}

impl Bus for PciBus {
    fn name(&self) -> &str {
        "pci"
    }
    
    fn bus_type(&self) -> BusType {
        BusType::Pci
    }
    
    fn enumerate(&self) -> Result<Vec<Arc<Device>>> {
        self.scan_pci_bus()
    }
    
    fn rescan(&self) -> Result<()> {
        let new_devices = self.scan_pci_bus()?;
        
        // Register new devices
        for device in new_devices {
            super::driver_manager().register_device(device)?;
        }
        
        Ok(())
    }
    
    fn match_device(&self, device: &Device, driver: &dyn Driver) -> bool {
        // Check if driver supports device class
        driver.supported_classes().contains(&device.class())
    }
    
    fn add_device(&self, device: Arc<Device>) -> Result<()> {
        // Would add to PCI device tree
        Ok(())
    }
    
    fn remove_device(&self, id: DeviceId) -> Result<()> {
        let mut devices = self.devices.write();
        
        // Find and remove device
        devices.retain(|_, dev| dev.id() != id);
        
        Ok(())
    }
}

/// USB bus implementation
pub struct UsbBus {
    devices: RwLock<BTreeMap<UsbAddress, Arc<Device>>>,
    root_hub: RwLock<Option<Arc<Device>>>,
}

/// USB address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UsbAddress {
    pub bus: u8,
    pub port: u8,
    pub device: u8,
}

impl UsbBus {
    /// Create new USB bus
    pub fn new() -> Self {
        Self {
            devices: RwLock::new(BTreeMap::new()),
            root_hub: RwLock::new(None),
        }
    }
    
    /// Initialize root hub
    pub fn init_root_hub(&self) -> Result<()> {
        let root_hub = Arc::new(Device::new(
            "usb-root-hub".into(),
            DeviceClass::UsbController,
        ));
        
        *self.root_hub.write() = Some(root_hub);
        
        Ok(())
    }
    
    /// Enumerate USB devices
    fn enumerate_usb(&self) -> Result<Vec<Arc<Device>>> {
        let mut devices = Vec::new();
        
        // Would enumerate actual USB devices
        // For now, return empty list
        
        Ok(devices)
    }
}

impl Bus for UsbBus {
    fn name(&self) -> &str {
        "usb"
    }
    
    fn bus_type(&self) -> BusType {
        BusType::Usb
    }
    
    fn enumerate(&self) -> Result<Vec<Arc<Device>>> {
        self.enumerate_usb()
    }
    
    fn rescan(&self) -> Result<()> {
        let new_devices = self.enumerate_usb()?;
        
        for device in new_devices {
            super::driver_manager().register_device(device)?;
        }
        
        Ok(())
    }
    
    fn match_device(&self, device: &Device, driver: &dyn Driver) -> bool {
        driver.supported_classes().contains(&device.class())
    }
    
    fn add_device(&self, device: Arc<Device>) -> Result<()> {
        Ok(())
    }
    
    fn remove_device(&self, id: DeviceId) -> Result<()> {
        let mut devices = self.devices.write();
        devices.retain(|_, dev| dev.id() != id);
        Ok(())
    }
}

/// Platform bus for SoC and built-in devices
pub struct PlatformBus {
    devices: RwLock<Vec<Arc<Device>>>,
}

impl PlatformBus {
    /// Create new platform bus
    pub fn new() -> Self {
        Self {
            devices: RwLock::new(Vec::new()),
        }
    }
    
    /// Register platform device
    pub fn register_platform_device(&self, device: Arc<Device>) -> Result<()> {
        self.devices.write().push(device.clone());
        super::driver_manager().register_device(device)?;
        Ok(())
    }
}

impl Bus for PlatformBus {
    fn name(&self) -> &str {
        "platform"
    }
    
    fn bus_type(&self) -> BusType {
        BusType::Platform
    }
    
    fn enumerate(&self) -> Result<Vec<Arc<Device>>> {
        Ok(self.devices.read().clone())
    }
    
    fn rescan(&self) -> Result<()> {
        // Platform devices are typically static
        Ok(())
    }
    
    fn match_device(&self, device: &Device, driver: &dyn Driver) -> bool {
        // Match by name or compatible property
        if let Some(compatible) = device.get_property("compatible") {
            // Check if driver supports this compatible string
            true
        } else {
            driver.supported_classes().contains(&device.class())
        }
    }
    
    fn add_device(&self, device: Arc<Device>) -> Result<()> {
        self.devices.write().push(device);
        Ok(())
    }
    
    fn remove_device(&self, id: DeviceId) -> Result<()> {
        self.devices.write().retain(|dev| dev.id() != id);
        Ok(())
    }
}

/// Virtual bus for software devices
pub struct VirtualBus {
    devices: RwLock<Vec<Arc<Device>>>,
    next_id: AtomicU32,
}

impl VirtualBus {
    /// Create new virtual bus
    pub fn new() -> Self {
        Self {
            devices: RwLock::new(Vec::new()),
            next_id: AtomicU32::new(1),
        }
    }
    
    /// Create virtual device
    pub fn create_virtual_device(&self, name: String, class: DeviceClass) -> Arc<Device> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let device_name = format!("virtual-{}-{}", name, id);
        
        Arc::new(Device::new(device_name, class))
    }
}

impl Bus for VirtualBus {
    fn name(&self) -> &str {
        "virtual"
    }
    
    fn bus_type(&self) -> BusType {
        BusType::Virtual
    }
    
    fn enumerate(&self) -> Result<Vec<Arc<Device>>> {
        Ok(self.devices.read().clone())
    }
    
    fn rescan(&self) -> Result<()> {
        Ok(())
    }
    
    fn match_device(&self, device: &Device, driver: &dyn Driver) -> bool {
        driver.supported_classes().contains(&device.class())
    }
    
    fn add_device(&self, device: Arc<Device>) -> Result<()> {
        self.devices.write().push(device);
        Ok(())
    }
    
    fn remove_device(&self, id: DeviceId) -> Result<()> {
        self.devices.write().retain(|dev| dev.id() != id);
        Ok(())
    }
}