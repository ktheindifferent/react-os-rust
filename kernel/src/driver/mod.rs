//! Unified Device Driver Framework
//! 
//! Modern, safe device driver framework providing:
//! - Unified driver model with device tree representation
//! - Bus/device/driver abstraction
//! - Hot-plug support
//! - Driver verification and isolation
//! - Power management integration
//! - DMA and interrupt handling

#![no_std]
#![feature(async_fn_in_trait)]
#![feature(const_type_id)]

extern crate alloc;

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::{
    any::{Any, TypeId},
    fmt,
    sync::atomic::{AtomicU32, AtomicU64, Ordering},
};
use spin::{Mutex, RwLock};

pub mod model;
pub mod bus;
pub mod dma;
pub mod probe;
pub mod power;
pub mod interrupt;
pub mod hotplug;
pub mod userspace;
pub mod safety;

pub use model::{Device, DeviceId, DeviceClass, DeviceState};
pub use bus::{Bus, BusType, BusDriver};
pub use dma::{DmaBuffer, DmaDirection, DmaMapping};
pub use probe::{DriverProbe, ProbeResult};
pub use power::{PowerState, PowerManager};

/// Driver framework errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverError {
    NotFound,
    AlreadyExists,
    InvalidState,
    ResourceConflict,
    ProbeFailure,
    RemovalDenied,
    DmaError,
    InterruptError,
    PowerError,
    VerificationFailed,
    AccessDenied,
    Timeout,
    HardwareFault,
}

pub type Result<T> = core::result::Result<T, DriverError>;

/// Global driver manager
pub struct DriverManager {
    /// Registered drivers indexed by name
    drivers: RwLock<BTreeMap<String, Arc<dyn Driver>>>,
    /// Device tree root
    device_tree: RwLock<Option<Arc<Device>>>,
    /// Active buses
    buses: RwLock<Vec<Arc<dyn Bus>>>,
    /// Driver verification enabled
    verification_enabled: bool,
    /// Statistics
    stats: DriverStats,
}

/// Driver statistics
struct DriverStats {
    devices_registered: AtomicU64,
    drivers_loaded: AtomicU64,
    probe_attempts: AtomicU64,
    probe_failures: AtomicU64,
    hotplug_events: AtomicU64,
}

/// Core driver trait
pub trait Driver: Send + Sync {
    /// Driver name
    fn name(&self) -> &str;
    
    /// Driver version
    fn version(&self) -> u32;
    
    /// Supported device classes
    fn supported_classes(&self) -> &[DeviceClass];
    
    /// Probe device for driver compatibility
    fn probe(&self, device: &Device) -> ProbeResult;
    
    /// Attach driver to device
    fn attach(&self, device: Arc<Device>) -> Result<()>;
    
    /// Detach driver from device
    fn detach(&self, device: Arc<Device>) -> Result<()>;
    
    /// Handle device suspend
    fn suspend(&self, device: Arc<Device>) -> Result<()> {
        Ok(())
    }
    
    /// Handle device resume
    fn resume(&self, device: Arc<Device>) -> Result<()> {
        Ok(())
    }
    
    /// Handle device power state change
    fn power_changed(&self, device: Arc<Device>, state: PowerState) -> Result<()> {
        Ok(())
    }
    
    /// Get driver capabilities
    fn capabilities(&self) -> DriverCapabilities {
        DriverCapabilities::default()
    }
}

/// Driver capabilities
#[derive(Debug, Clone, Copy, Default)]
pub struct DriverCapabilities {
    pub hot_plug: bool,
    pub power_management: bool,
    pub dma_capable: bool,
    pub interrupt_driven: bool,
    pub userspace_accessible: bool,
    pub verified: bool,
    pub isolated: bool,
}

/// Driver registration info
pub struct DriverInfo {
    pub name: String,
    pub version: u32,
    pub author: String,
    pub description: String,
    pub license: String,
    pub dependencies: Vec<String>,
}

impl DriverManager {
    /// Create new driver manager
    pub const fn new() -> Self {
        Self {
            drivers: RwLock::new(BTreeMap::new()),
            device_tree: RwLock::new(None),
            buses: RwLock::new(Vec::new()),
            verification_enabled: true,
            stats: DriverStats {
                devices_registered: AtomicU64::new(0),
                drivers_loaded: AtomicU64::new(0),
                probe_attempts: AtomicU64::new(0),
                probe_failures: AtomicU64::new(0),
                hotplug_events: AtomicU64::new(0),
            },
        }
    }
    
    /// Register a driver
    pub fn register_driver(&self, driver: Arc<dyn Driver>) -> Result<()> {
        let name = driver.name().into();
        
        // Verify driver if required
        if self.verification_enabled {
            safety::verify_driver(&*driver)?;
        }
        
        let mut drivers = self.drivers.write();
        if drivers.contains_key(&name) {
            return Err(DriverError::AlreadyExists);
        }
        
        drivers.insert(name, driver);
        self.stats.drivers_loaded.fetch_add(1, Ordering::Relaxed);
        
        // Probe existing devices for this driver
        self.probe_all_devices()?;
        
        Ok(())
    }
    
    /// Unregister a driver
    pub fn unregister_driver(&self, name: &str) -> Result<()> {
        let mut drivers = self.drivers.write();
        
        if let Some(driver) = drivers.remove(name) {
            // Detach from all devices
            self.detach_driver_from_all(&*driver)?;
            self.stats.drivers_loaded.fetch_sub(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err(DriverError::NotFound)
        }
    }
    
    /// Register a bus
    pub fn register_bus(&self, bus: Arc<dyn Bus>) -> Result<()> {
        let mut buses = self.buses.write();
        
        // Check for duplicate
        for existing in buses.iter() {
            if existing.name() == bus.name() {
                return Err(DriverError::AlreadyExists);
            }
        }
        
        buses.push(bus.clone());
        
        // Start bus enumeration
        bus.enumerate()?;
        
        Ok(())
    }
    
    /// Register a device
    pub fn register_device(&self, device: Arc<Device>) -> Result<()> {
        // Add to device tree
        let mut tree = self.device_tree.write();
        
        if tree.is_none() {
            // First device becomes root
            *tree = Some(device.clone());
        } else {
            // Find parent and add as child
            self.add_to_device_tree(&device)?;
        }
        
        self.stats.devices_registered.fetch_add(1, Ordering::Relaxed);
        
        // Find and bind driver
        self.probe_device(&device)?;
        
        Ok(())
    }
    
    /// Handle hot-plug event
    pub fn handle_hotplug(&self, event: HotplugEvent) -> Result<()> {
        self.stats.hotplug_events.fetch_add(1, Ordering::Relaxed);
        
        match event {
            HotplugEvent::DeviceAdded(device) => {
                self.register_device(device)?;
            }
            HotplugEvent::DeviceRemoved(id) => {
                self.remove_device(id)?;
            }
            HotplugEvent::BusRescan(bus_name) => {
                self.rescan_bus(&bus_name)?;
            }
        }
        
        Ok(())
    }
    
    /// Find driver for device
    fn find_driver_for_device(&self, device: &Device) -> Option<Arc<dyn Driver>> {
        let drivers = self.drivers.read();
        
        for (_, driver) in drivers.iter() {
            self.stats.probe_attempts.fetch_add(1, Ordering::Relaxed);
            
            match driver.probe(device) {
                ProbeResult::Success => return Some(driver.clone()),
                ProbeResult::Failed => {
                    self.stats.probe_failures.fetch_add(1, Ordering::Relaxed);
                }
                ProbeResult::Deferred => {
                    // Will retry later
                }
            }
        }
        
        None
    }
    
    /// Probe device for compatible driver
    fn probe_device(&self, device: &Device) -> Result<()> {
        if let Some(driver) = self.find_driver_for_device(device) {
            driver.attach(Arc::new(device.clone()))?;
        }
        Ok(())
    }
    
    /// Probe all devices for new driver
    fn probe_all_devices(&self) -> Result<()> {
        // Walk device tree and probe each unbound device
        if let Some(root) = self.device_tree.read().clone() {
            self.walk_and_probe(&root)?;
        }
        Ok(())
    }
    
    /// Walk device tree and probe
    fn walk_and_probe(&self, device: &Arc<Device>) -> Result<()> {
        // Probe this device if not bound
        if !device.is_bound() {
            self.probe_device(device)?;
        }
        
        // Probe children
        for child in device.children() {
            self.walk_and_probe(&child)?;
        }
        
        Ok(())
    }
    
    /// Detach driver from all devices
    fn detach_driver_from_all(&self, driver: &dyn Driver) -> Result<()> {
        // Walk device tree and detach
        if let Some(root) = self.device_tree.read().clone() {
            self.walk_and_detach(&root, driver)?;
        }
        Ok(())
    }
    
    /// Walk device tree and detach driver
    fn walk_and_detach(&self, device: &Arc<Device>, driver: &dyn Driver) -> Result<()> {
        // Check if this device uses the driver
        if device.driver_name() == Some(driver.name()) {
            driver.detach(device.clone())?;
        }
        
        // Check children
        for child in device.children() {
            self.walk_and_detach(&child, driver)?;
        }
        
        Ok(())
    }
    
    /// Add device to tree
    fn add_to_device_tree(&self, device: &Arc<Device>) -> Result<()> {
        // Implementation would find parent based on device's parent_id
        // and add as child
        Ok(())
    }
    
    /// Remove device from system
    fn remove_device(&self, id: DeviceId) -> Result<()> {
        // Find and remove device from tree
        // Detach driver first
        Ok(())
    }
    
    /// Rescan bus for devices
    fn rescan_bus(&self, bus_name: &str) -> Result<()> {
        let buses = self.buses.read();
        
        for bus in buses.iter() {
            if bus.name() == bus_name {
                return bus.rescan();
            }
        }
        
        Err(DriverError::NotFound)
    }
    
    /// Get driver statistics
    pub fn statistics(&self) -> DriverStatistics {
        DriverStatistics {
            devices_registered: self.stats.devices_registered.load(Ordering::Relaxed),
            drivers_loaded: self.stats.drivers_loaded.load(Ordering::Relaxed),
            probe_attempts: self.stats.probe_attempts.load(Ordering::Relaxed),
            probe_failures: self.stats.probe_failures.load(Ordering::Relaxed),
            hotplug_events: self.stats.hotplug_events.load(Ordering::Relaxed),
        }
    }
}

/// Driver statistics for monitoring
#[derive(Debug, Clone, Copy)]
pub struct DriverStatistics {
    pub devices_registered: u64,
    pub drivers_loaded: u64,
    pub probe_attempts: u64,
    pub probe_failures: u64,
    pub hotplug_events: u64,
}

/// Hot-plug events
pub enum HotplugEvent {
    DeviceAdded(Arc<Device>),
    DeviceRemoved(DeviceId),
    BusRescan(String),
}

/// Global driver manager instance
static DRIVER_MANAGER: DriverManager = DriverManager::new();

/// Get global driver manager
pub fn driver_manager() -> &'static DriverManager {
    &DRIVER_MANAGER
}

/// Macro for driver registration
#[macro_export]
macro_rules! register_driver {
    ($driver:expr) => {
        $crate::driver_manager().register_driver(Arc::new($driver))
    };
}

/// Macro for device registration  
#[macro_export]
macro_rules! register_device {
    ($device:expr) => {
        $crate::driver_manager().register_device(Arc::new($device))
    };
}