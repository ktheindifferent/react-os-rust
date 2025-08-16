//! Power Management Integration for Device Drivers

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::{
    sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    time::Duration,
};
use spin::{Mutex, RwLock};

use super::{Device, DeviceId, Driver, DriverError, Result};

/// Device power state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PowerState {
    /// Device is fully powered and operational
    D0,
    /// Light sleep - quick wake, some context retained
    D1,
    /// Deeper sleep - slower wake, less context
    D2,
    /// Deep sleep - slow wake, minimal power
    D3Hot,
    /// Power removed - requires full reinitialization
    D3Cold,
}

/// System power state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemState {
    /// System is running normally
    S0_Working,
    /// Standby/Sleep
    S1_Standby,
    /// Suspend to RAM
    S3_Suspend,
    /// Suspend to Disk (Hibernate)
    S4_Hibernate,
    /// Soft off
    S5_SoftOff,
}

/// Runtime PM state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    Active,
    Idle,
    Suspending,
    Suspended,
    Resuming,
}

/// Power management capabilities
#[derive(Debug, Clone, Copy, Default)]
pub struct PowerCapabilities {
    /// Supported device states
    pub d1_supported: bool,
    pub d2_supported: bool,
    pub d3hot_supported: bool,
    pub d3cold_supported: bool,
    
    /// Wake capabilities
    pub wake_from_d1: bool,
    pub wake_from_d2: bool,
    pub wake_from_d3hot: bool,
    pub wake_from_d3cold: bool,
    
    /// Runtime PM support
    pub runtime_pm: bool,
    
    /// Latencies (in microseconds)
    pub d1_latency: u32,
    pub d2_latency: u32,
    pub d3_latency: u32,
}

/// Power management operations
pub trait PowerOps: Send + Sync {
    /// Prepare for system suspend
    fn prepare(&self, device: &Device) -> Result<()> {
        Ok(())
    }
    
    /// Suspend device
    fn suspend(&self, device: &Device, state: PowerState) -> Result<()>;
    
    /// Resume device
    fn resume(&self, device: &Device) -> Result<()>;
    
    /// Complete resume
    fn complete(&self, device: &Device) -> Result<()> {
        Ok(())
    }
    
    /// Runtime suspend
    fn runtime_suspend(&self, device: &Device) -> Result<()> {
        self.suspend(device, PowerState::D3Hot)
    }
    
    /// Runtime resume
    fn runtime_resume(&self, device: &Device) -> Result<()> {
        self.resume(device)
    }
    
    /// Runtime idle callback
    fn runtime_idle(&self, device: &Device) -> Result<()> {
        Ok(())
    }
}

/// Device power management info
pub struct DevicePowerInfo {
    /// Current power state
    state: RwLock<PowerState>,
    /// Runtime PM state
    runtime_state: RwLock<RuntimeState>,
    /// Power capabilities
    capabilities: PowerCapabilities,
    /// Power operations
    ops: Option<Arc<dyn PowerOps>>,
    /// Usage counter
    usage_count: AtomicU32,
    /// Disable depth
    disable_depth: AtomicU32,
    /// Runtime suspended flag
    runtime_suspended: AtomicBool,
    /// Wake enabled
    wake_enabled: AtomicBool,
    /// Last active time
    last_active: AtomicU64,
    /// Suspend delay (ms)
    suspend_delay: AtomicU32,
}

impl DevicePowerInfo {
    /// Create new power info
    pub fn new(capabilities: PowerCapabilities) -> Self {
        Self {
            state: RwLock::new(PowerState::D0),
            runtime_state: RwLock::new(RuntimeState::Active),
            capabilities,
            ops: None,
            usage_count: AtomicU32::new(0),
            disable_depth: AtomicU32::new(0),
            runtime_suspended: AtomicBool::new(false),
            wake_enabled: AtomicBool::new(false),
            last_active: AtomicU64::new(0),
            suspend_delay: AtomicU32::new(1000), // 1 second default
        }
    }
    
    /// Set power operations
    pub fn set_ops(&mut self, ops: Arc<dyn PowerOps>) {
        self.ops = Some(ops);
    }
    
    /// Get current state
    pub fn state(&self) -> PowerState {
        *self.state.read()
    }
    
    /// Set state
    pub fn set_state(&self, state: PowerState) {
        *self.state.write() = state;
    }
}

/// Global power manager
pub struct PowerManager {
    /// Device power info
    device_power: RwLock<BTreeMap<DeviceId, Arc<Mutex<DevicePowerInfo>>>>,
    /// System state
    system_state: RwLock<SystemState>,
    /// Suspend/resume order
    suspend_order: RwLock<Vec<DeviceId>>,
    /// Runtime PM workqueue
    runtime_queue: Mutex<Vec<RuntimeWork>>,
    /// Statistics
    stats: PowerStats,
}

/// Runtime PM work item
struct RuntimeWork {
    device_id: DeviceId,
    action: RuntimeAction,
    scheduled_at: u64,
}

/// Runtime PM action
enum RuntimeAction {
    Suspend,
    Resume,
    Idle,
}

/// Power management statistics
struct PowerStats {
    suspends: AtomicU64,
    resumes: AtomicU64,
    runtime_suspends: AtomicU64,
    runtime_resumes: AtomicU64,
    failed_suspends: AtomicU64,
    failed_resumes: AtomicU64,
}

impl PowerManager {
    /// Create new power manager
    pub const fn new() -> Self {
        Self {
            device_power: RwLock::new(BTreeMap::new()),
            system_state: RwLock::new(SystemState::S0_Working),
            suspend_order: RwLock::new(Vec::new()),
            runtime_queue: Mutex::new(Vec::new()),
            stats: PowerStats {
                suspends: AtomicU64::new(0),
                resumes: AtomicU64::new(0),
                runtime_suspends: AtomicU64::new(0),
                runtime_resumes: AtomicU64::new(0),
                failed_suspends: AtomicU64::new(0),
                failed_resumes: AtomicU64::new(0),
            },
        }
    }
    
    /// Register device for power management
    pub fn register_device(
        &self,
        device: &Device,
        capabilities: PowerCapabilities,
        ops: Option<Arc<dyn PowerOps>>,
    ) -> Result<()> {
        let mut power_info = DevicePowerInfo::new(capabilities);
        
        if let Some(ops) = ops {
            power_info.set_ops(ops);
        }
        
        self.device_power.write().insert(
            device.id(),
            Arc::new(Mutex::new(power_info)),
        );
        
        // Add to suspend order (children before parents)
        self.update_suspend_order(device)?;
        
        Ok(())
    }
    
    /// Unregister device
    pub fn unregister_device(&self, device_id: DeviceId) -> Result<()> {
        self.device_power.write().remove(&device_id);
        self.suspend_order.write().retain(|id| *id != device_id);
        Ok(())
    }
    
    /// System suspend
    pub fn system_suspend(&self, target_state: SystemState) -> Result<()> {
        *self.system_state.write() = target_state;
        
        let order = self.suspend_order.read().clone();
        
        // Suspend devices in order
        for device_id in order.iter() {
            if let Err(e) = self.suspend_device(*device_id, PowerState::D3Hot) {
                self.stats.failed_suspends.fetch_add(1, Ordering::Relaxed);
                
                // Resume already suspended devices
                for resumed_id in order.iter().take_while(|id| **id != *device_id) {
                    let _ = self.resume_device(*resumed_id);
                }
                
                *self.system_state.write() = SystemState::S0_Working;
                return Err(e);
            }
        }
        
        self.stats.suspends.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    
    /// System resume
    pub fn system_resume(&self) -> Result<()> {
        let order = self.suspend_order.read().clone();
        
        // Resume devices in reverse order
        for device_id in order.iter().rev() {
            if let Err(e) = self.resume_device(*device_id) {
                self.stats.failed_resumes.fetch_add(1, Ordering::Relaxed);
                // Continue resuming other devices
            }
        }
        
        *self.system_state.write() = SystemState::S0_Working;
        self.stats.resumes.fetch_add(1, Ordering::Relaxed);
        
        Ok(())
    }
    
    /// Suspend individual device
    pub fn suspend_device(&self, device_id: DeviceId, state: PowerState) -> Result<()> {
        let power_map = self.device_power.read();
        
        if let Some(power_info) = power_map.get(&device_id) {
            let mut info = power_info.lock();
            
            // Check if already suspended
            if info.state() >= state {
                return Ok(());
            }
            
            // Call driver suspend
            if let Some(ref ops) = info.ops {
                // Would get actual device reference
                // ops.suspend(device, state)?;
            }
            
            info.set_state(state);
            
            Ok(())
        } else {
            Err(DriverError::NotFound)
        }
    }
    
    /// Resume individual device
    pub fn resume_device(&self, device_id: DeviceId) -> Result<()> {
        let power_map = self.device_power.read();
        
        if let Some(power_info) = power_map.get(&device_id) {
            let mut info = power_info.lock();
            
            // Check if already active
            if info.state() == PowerState::D0 {
                return Ok(());
            }
            
            // Call driver resume
            if let Some(ref ops) = info.ops {
                // Would get actual device reference
                // ops.resume(device)?;
            }
            
            info.set_state(PowerState::D0);
            
            Ok(())
        } else {
            Err(DriverError::NotFound)
        }
    }
    
    /// Runtime PM get (increment usage count)
    pub fn runtime_get(&self, device_id: DeviceId) -> Result<()> {
        let power_map = self.device_power.read();
        
        if let Some(power_info) = power_map.get(&device_id) {
            let info = power_info.lock();
            
            let count = info.usage_count.fetch_add(1, Ordering::AcqRel);
            
            if count == 0 {
                // First reference, resume device
                drop(info);
                self.runtime_resume(device_id)?;
            }
            
            Ok(())
        } else {
            Err(DriverError::NotFound)
        }
    }
    
    /// Runtime PM put (decrement usage count)
    pub fn runtime_put(&self, device_id: DeviceId) -> Result<()> {
        let power_map = self.device_power.read();
        
        if let Some(power_info) = power_map.get(&device_id) {
            let info = power_info.lock();
            
            let count = info.usage_count.fetch_sub(1, Ordering::AcqRel);
            
            if count == 1 {
                // Last reference, schedule suspend
                let delay = info.suspend_delay.load(Ordering::Acquire);
                drop(info);
                self.schedule_runtime_suspend(device_id, delay)?;
            }
            
            Ok(())
        } else {
            Err(DriverError::NotFound)
        }
    }
    
    /// Runtime suspend
    fn runtime_suspend(&self, device_id: DeviceId) -> Result<()> {
        let power_map = self.device_power.read();
        
        if let Some(power_info) = power_map.get(&device_id) {
            let mut info = power_info.lock();
            
            // Check usage count
            if info.usage_count.load(Ordering::Acquire) > 0 {
                return Ok(()); // Still in use
            }
            
            *info.runtime_state.write() = RuntimeState::Suspending;
            
            // Call driver runtime suspend
            if let Some(ref ops) = info.ops {
                // Would get actual device reference
                // ops.runtime_suspend(device)?;
            }
            
            *info.runtime_state.write() = RuntimeState::Suspended;
            info.runtime_suspended.store(true, Ordering::Release);
            
            self.stats.runtime_suspends.fetch_add(1, Ordering::Relaxed);
            
            Ok(())
        } else {
            Err(DriverError::NotFound)
        }
    }
    
    /// Runtime resume
    fn runtime_resume(&self, device_id: DeviceId) -> Result<()> {
        let power_map = self.device_power.read();
        
        if let Some(power_info) = power_map.get(&device_id) {
            let mut info = power_info.lock();
            
            if !info.runtime_suspended.load(Ordering::Acquire) {
                return Ok(()); // Already resumed
            }
            
            *info.runtime_state.write() = RuntimeState::Resuming;
            
            // Call driver runtime resume
            if let Some(ref ops) = info.ops {
                // Would get actual device reference
                // ops.runtime_resume(device)?;
            }
            
            *info.runtime_state.write() = RuntimeState::Active;
            info.runtime_suspended.store(false, Ordering::Release);
            
            self.stats.runtime_resumes.fetch_add(1, Ordering::Relaxed);
            
            Ok(())
        } else {
            Err(DriverError::NotFound)
        }
    }
    
    /// Schedule runtime suspend
    fn schedule_runtime_suspend(&self, device_id: DeviceId, delay_ms: u32) -> Result<()> {
        let mut queue = self.runtime_queue.lock();
        
        queue.push(RuntimeWork {
            device_id,
            action: RuntimeAction::Suspend,
            scheduled_at: self.current_time() + delay_ms as u64,
        });
        
        Ok(())
    }
    
    /// Process runtime PM queue
    pub fn process_runtime_queue(&self) {
        let mut queue = self.runtime_queue.lock();
        let current = self.current_time();
        
        let mut i = 0;
        while i < queue.len() {
            if queue[i].scheduled_at <= current {
                let work = queue.remove(i);
                
                match work.action {
                    RuntimeAction::Suspend => {
                        let _ = self.runtime_suspend(work.device_id);
                    }
                    RuntimeAction::Resume => {
                        let _ = self.runtime_resume(work.device_id);
                    }
                    RuntimeAction::Idle => {
                        // Call idle callback
                    }
                }
            } else {
                i += 1;
            }
        }
    }
    
    /// Update suspend order based on device tree
    fn update_suspend_order(&self, device: &Device) -> Result<()> {
        let mut order = self.suspend_order.write();
        
        // Add device if not present
        if !order.contains(&device.id()) {
            // Add children first
            for child in device.children() {
                self.update_suspend_order(&child)?;
            }
            
            // Then add this device
            order.push(device.id());
        }
        
        Ok(())
    }
    
    /// Get current time in milliseconds
    fn current_time(&self) -> u64 {
        // Would get actual system time
        0
    }
    
    /// Get power statistics
    pub fn statistics(&self) -> PowerStatistics {
        PowerStatistics {
            suspends: self.stats.suspends.load(Ordering::Relaxed),
            resumes: self.stats.resumes.load(Ordering::Relaxed),
            runtime_suspends: self.stats.runtime_suspends.load(Ordering::Relaxed),
            runtime_resumes: self.stats.runtime_resumes.load(Ordering::Relaxed),
            failed_suspends: self.stats.failed_suspends.load(Ordering::Relaxed),
            failed_resumes: self.stats.failed_resumes.load(Ordering::Relaxed),
            registered_devices: self.device_power.read().len() as u32,
        }
    }
}

/// Power statistics
#[derive(Debug, Clone, Copy)]
pub struct PowerStatistics {
    pub suspends: u64,
    pub resumes: u64,
    pub runtime_suspends: u64,
    pub runtime_resumes: u64,
    pub failed_suspends: u64,
    pub failed_resumes: u64,
    pub registered_devices: u32,
}

/// Global power manager instance
static POWER_MANAGER: PowerManager = PowerManager::new();

/// Get global power manager
pub fn power_manager() -> &'static PowerManager {
    &POWER_MANAGER
}

/// Helper macros for runtime PM
#[macro_export]
macro_rules! pm_runtime_get {
    ($dev:expr) => {
        $crate::power_manager().runtime_get($dev.id())
    };
}

#[macro_export]
macro_rules! pm_runtime_put {
    ($dev:expr) => {
        $crate::power_manager().runtime_put($dev.id())
    };
}

/// Device power constraints
pub struct PowerConstraint {
    /// Minimum required state
    pub min_state: PowerState,
    /// Maximum latency (microseconds)
    pub max_latency: u32,
    /// Required for wake
    pub wake_required: bool,
}

/// Quality of Service (QoS) requirements
pub struct PowerQoS {
    constraints: RwLock<Vec<PowerConstraint>>,
}

impl PowerQoS {
    /// Create new QoS manager
    pub fn new() -> Self {
        Self {
            constraints: RwLock::new(Vec::new()),
        }
    }
    
    /// Add constraint
    pub fn add_constraint(&self, constraint: PowerConstraint) {
        self.constraints.write().push(constraint);
    }
    
    /// Get effective constraint
    pub fn effective_constraint(&self) -> PowerConstraint {
        let constraints = self.constraints.read();
        
        let mut effective = PowerConstraint {
            min_state: PowerState::D3Cold,
            max_latency: u32::MAX,
            wake_required: false,
        };
        
        for c in constraints.iter() {
            if c.min_state < effective.min_state {
                effective.min_state = c.min_state;
            }
            if c.max_latency < effective.max_latency {
                effective.max_latency = c.max_latency;
            }
            if c.wake_required {
                effective.wake_required = true;
            }
        }
        
        effective
    }
}