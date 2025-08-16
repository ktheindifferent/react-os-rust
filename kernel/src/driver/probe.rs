//! Device Probing and Driver Binding System

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::{
    sync::atomic::{AtomicU32, Ordering},
};
use spin::{Mutex, RwLock};

use super::{
    Device, DeviceClass, DeviceId, Driver, DriverError, Result,
    model::DeviceState,
};

/// Driver probe result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeResult {
    /// Driver successfully matched device
    Success,
    /// Driver cannot handle this device
    Failed,
    /// Probe deferred (dependencies not ready)
    Deferred,
}

/// Device matching criteria
#[derive(Debug, Clone)]
pub enum MatchCriteria {
    /// Match by device class
    Class(DeviceClass),
    /// Match by vendor/device ID
    VendorDevice { vendor: u16, device: u16 },
    /// Match by compatible string
    Compatible(String),
    /// Match by device name pattern
    NamePattern(String),
    /// Custom match function
    Custom(fn(&Device) -> bool),
}

/// Driver probe context
pub struct ProbeContext {
    /// Device being probed
    pub device: Arc<Device>,
    /// Probe attempt number
    pub attempt: u32,
    /// Probe flags
    pub flags: ProbeFlags,
}

/// Probe flags
#[derive(Debug, Clone, Copy, Default)]
pub struct ProbeFlags {
    /// Force probe even if already bound
    pub force: bool,
    /// Probe asynchronously
    pub async_probe: bool,
    /// Allow partial binding
    pub partial: bool,
}

/// Driver matching and probing system
pub struct DriverProbe {
    /// Match criteria for drivers
    match_table: RwLock<BTreeMap<String, Vec<MatchCriteria>>>,
    /// Deferred probe list
    deferred_list: Mutex<Vec<DeferredProbe>>,
    /// Probe statistics
    stats: ProbeStats,
}

/// Deferred probe entry
struct DeferredProbe {
    device: Arc<Device>,
    driver: String,
    attempts: u32,
    reason: String,
}

/// Probe statistics
struct ProbeStats {
    total_probes: AtomicU32,
    successful_probes: AtomicU32,
    failed_probes: AtomicU32,
    deferred_probes: AtomicU32,
}

impl DriverProbe {
    /// Create new probe system
    pub const fn new() -> Self {
        Self {
            match_table: RwLock::new(BTreeMap::new()),
            deferred_list: Mutex::new(Vec::new()),
            stats: ProbeStats {
                total_probes: AtomicU32::new(0),
                successful_probes: AtomicU32::new(0),
                failed_probes: AtomicU32::new(0),
                deferred_probes: AtomicU32::new(0),
            },
        }
    }
    
    /// Register driver match criteria
    pub fn register_match_criteria(
        &self,
        driver: String,
        criteria: Vec<MatchCriteria>,
    ) -> Result<()> {
        self.match_table.write().insert(driver, criteria);
        Ok(())
    }
    
    /// Match device to driver
    pub fn match_device(&self, device: &Device, driver: &dyn Driver) -> bool {
        let table = self.match_table.read();
        
        if let Some(criteria_list) = table.get(driver.name()) {
            for criteria in criteria_list {
                if self.check_match(device, criteria) {
                    return true;
                }
            }
        }
        
        // Fall back to driver's own matching
        match driver.probe(device) {
            ProbeResult::Success => true,
            _ => false,
        }
    }
    
    /// Check if device matches criteria
    fn check_match(&self, device: &Device, criteria: &MatchCriteria) -> bool {
        match criteria {
            MatchCriteria::Class(class) => device.class() == *class,
            
            MatchCriteria::VendorDevice { vendor, device: dev } => {
                // Check vendor/device properties
                if let Some(v) = device.get_property("vendor_id") {
                    if let Some(d) = device.get_property("device_id") {
                        // Would compare actual values
                        return true;
                    }
                }
                false
            }
            
            MatchCriteria::Compatible(compat) => {
                if let Some(prop) = device.get_property("compatible") {
                    // Would check compatible string
                    return true;
                }
                false
            }
            
            MatchCriteria::NamePattern(pattern) => {
                // Simple pattern matching (would use proper pattern matching)
                device.name().contains(pattern)
            }
            
            MatchCriteria::Custom(func) => func(device),
        }
    }
    
    /// Probe device with specific driver
    pub fn probe_device_with_driver(
        &self,
        device: Arc<Device>,
        driver: &dyn Driver,
        flags: ProbeFlags,
    ) -> ProbeResult {
        self.stats.total_probes.fetch_add(1, Ordering::Relaxed);
        
        // Check if already bound and not forcing
        if device.is_bound() && !flags.force {
            return ProbeResult::Failed;
        }
        
        // Set device state
        device.set_state(DeviceState::Probing);
        
        // Create probe context
        let context = ProbeContext {
            device: device.clone(),
            attempt: 1,
            flags,
        };
        
        // Call driver probe
        let result = driver.probe(&device);
        
        match result {
            ProbeResult::Success => {
                // Bind driver
                if let Err(_) = device.bind_driver(driver.name().into()) {
                    device.set_state(DeviceState::Failed);
                    self.stats.failed_probes.fetch_add(1, Ordering::Relaxed);
                    return ProbeResult::Failed;
                }
                
                // Attach driver
                if let Err(_) = driver.attach(device.clone()) {
                    let _ = device.unbind_driver();
                    device.set_state(DeviceState::Failed);
                    self.stats.failed_probes.fetch_add(1, Ordering::Relaxed);
                    return ProbeResult::Failed;
                }
                
                device.set_state(DeviceState::Active);
                self.stats.successful_probes.fetch_add(1, Ordering::Relaxed);
                ProbeResult::Success
            }
            
            ProbeResult::Deferred => {
                // Add to deferred list
                self.defer_probe(device.clone(), driver.name().into(), "Dependencies not ready");
                device.set_state(DeviceState::Uninitialized);
                self.stats.deferred_probes.fetch_add(1, Ordering::Relaxed);
                ProbeResult::Deferred
            }
            
            ProbeResult::Failed => {
                device.set_state(DeviceState::Uninitialized);
                self.stats.failed_probes.fetch_add(1, Ordering::Relaxed);
                ProbeResult::Failed
            }
        }
    }
    
    /// Add device to deferred probe list
    fn defer_probe(&self, device: Arc<Device>, driver: String, reason: &str) {
        let mut deferred = self.deferred_list.lock();
        
        // Check if already deferred
        for entry in deferred.iter_mut() {
            if entry.device.id() == device.id() && entry.driver == driver {
                entry.attempts += 1;
                return;
            }
        }
        
        // Add new deferred entry
        deferred.push(DeferredProbe {
            device,
            driver,
            attempts: 1,
            reason: reason.into(),
        });
    }
    
    /// Retry deferred probes
    pub fn retry_deferred_probes(&self) -> Result<()> {
        let mut deferred = self.deferred_list.lock();
        let mut completed = Vec::new();
        
        for (i, entry) in deferred.iter().enumerate() {
            // Find driver
            if let Some(driver) = self.find_driver(&entry.driver) {
                let result = self.probe_device_with_driver(
                    entry.device.clone(),
                    &*driver,
                    ProbeFlags::default(),
                );
                
                if result == ProbeResult::Success {
                    completed.push(i);
                }
            }
        }
        
        // Remove successful probes
        for i in completed.iter().rev() {
            deferred.remove(*i);
        }
        
        Ok(())
    }
    
    /// Find driver by name
    fn find_driver(&self, name: &str) -> Option<Arc<dyn Driver>> {
        // Would look up in driver registry
        None
    }
    
    /// Get probe statistics
    pub fn statistics(&self) -> ProbeStatistics {
        ProbeStatistics {
            total_probes: self.stats.total_probes.load(Ordering::Relaxed),
            successful_probes: self.stats.successful_probes.load(Ordering::Relaxed),
            failed_probes: self.stats.failed_probes.load(Ordering::Relaxed),
            deferred_probes: self.stats.deferred_probes.load(Ordering::Relaxed),
            deferred_count: self.deferred_list.lock().len() as u32,
        }
    }
}

/// Probe statistics
#[derive(Debug, Clone, Copy)]
pub struct ProbeStatistics {
    pub total_probes: u32,
    pub successful_probes: u32,
    pub failed_probes: u32,
    pub deferred_probes: u32,
    pub deferred_count: u32,
}

/// Global probe system
static PROBE_SYSTEM: DriverProbe = DriverProbe::new();

/// Get global probe system
pub fn probe_system() -> &'static DriverProbe {
    &PROBE_SYSTEM
}

/// Device ID table for matching
#[derive(Debug, Clone)]
pub struct DeviceIdTable {
    entries: Vec<DeviceIdEntry>,
}

/// Device ID entry
#[derive(Debug, Clone)]
pub struct DeviceIdEntry {
    pub vendor: Option<u16>,
    pub device: Option<u16>,
    pub subvendor: Option<u16>,
    pub subdevice: Option<u16>,
    pub class: Option<u32>,
    pub class_mask: Option<u32>,
    pub driver_data: usize,
}

impl DeviceIdTable {
    /// Create new device ID table
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    
    /// Add entry to table
    pub fn add_entry(mut self, entry: DeviceIdEntry) -> Self {
        self.entries.push(entry);
        self
    }
    
    /// Match device against table
    pub fn match_device(&self, device: &Device) -> Option<&DeviceIdEntry> {
        for entry in &self.entries {
            if self.match_entry(device, entry) {
                return Some(entry);
            }
        }
        None
    }
    
    /// Check if device matches entry
    fn match_entry(&self, device: &Device, entry: &DeviceIdEntry) -> bool {
        // Check vendor
        if let Some(vendor) = entry.vendor {
            if let Some(dev_vendor) = device.get_property("vendor_id") {
                // Would compare actual values
            } else {
                return false;
            }
        }
        
        // Check device
        if let Some(dev_id) = entry.device {
            if let Some(dev_device) = device.get_property("device_id") {
                // Would compare actual values
            } else {
                return false;
            }
        }
        
        // Check class
        if let Some(class) = entry.class {
            if let Some(mask) = entry.class_mask {
                // Would check class with mask
            }
        }
        
        true
    }
}

/// Probe helper macros
#[macro_export]
macro_rules! device_id_table {
    ( $( { $($field:ident : $value:expr),* } ),* ) => {
        {
            let mut table = DeviceIdTable::new();
            $(
                table = table.add_entry(DeviceIdEntry {
                    $( $field: Some($value), )*
                    ..Default::default()
                });
            )*
            table
        }
    };
}