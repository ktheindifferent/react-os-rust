use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::serial_println;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DevicePowerState {
    D0,      // Fully operational
    D1,      // Light sleep
    D2,      // Deeper sleep
    D3Hot,   // Off but power maintained
    D3Cold,  // Off and no power
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuntimePmPolicy {
    Disabled,
    Auto,
    Aggressive,
}

#[derive(Debug)]
pub struct DevicePower {
    device_id: u64,
    device_name: String,
    current_state: DevicePowerState,
    supported_states: Vec<DevicePowerState>,
    device_type: DeviceType,
    runtime_pm_enabled: bool,
    idle_timeout_ms: u32,
    last_activity: u64,
    suspend_callback: Option<fn() -> Result<(), &'static str>>,
    resume_callback: Option<fn() -> Result<(), &'static str>>,
    power_consumption: DevicePowerConsumption,
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    PCI,
    USB,
    SATA,
    Network,
    Display,
    Audio,
    Input,
    Storage,
}

#[derive(Debug, Default)]
pub struct DevicePowerConsumption {
    d0_mw: u32,
    d1_mw: u32,
    d2_mw: u32,
    d3_mw: u32,
}

impl DevicePower {
    pub fn new(device_id: u64, name: String, device_type: DeviceType) -> Self {
        Self {
            device_id,
            device_name: name,
            current_state: DevicePowerState::D0,
            supported_states: vec![DevicePowerState::D0],
            device_type,
            runtime_pm_enabled: false,
            idle_timeout_ms: 5000,
            last_activity: 0,
            suspend_callback: None,
            resume_callback: None,
            power_consumption: DevicePowerConsumption::default(),
        }
    }
    
    pub fn set_power_state(&mut self, state: DevicePowerState) -> Result<(), &'static str> {
        if !self.supported_states.contains(&state) {
            return Err("Unsupported power state");
        }
        
        if state == self.current_state {
            return Ok(());
        }
        
        // Transition to new state
        match (self.current_state, state) {
            (DevicePowerState::D0, _) => {
                // Going to sleep
                if let Some(suspend) = self.suspend_callback {
                    suspend()?;
                }
            },
            (_, DevicePowerState::D0) => {
                // Waking up
                if let Some(resume) = self.resume_callback {
                    resume()?;
                }
            },
            _ => {}
        }
        
        self.current_state = state;
        serial_println!("Device {}: Power state changed to {:?}", 
                       self.device_name, state);
        
        Ok(())
    }
    
    pub fn enable_runtime_pm(&mut self, timeout_ms: u32) {
        self.runtime_pm_enabled = true;
        self.idle_timeout_ms = timeout_ms;
        serial_println!("Device {}: Runtime PM enabled with {}ms timeout",
                       self.device_name, timeout_ms);
    }
    
    pub fn disable_runtime_pm(&mut self) {
        self.runtime_pm_enabled = false;
        // Wake device if sleeping
        if self.current_state != DevicePowerState::D0 {
            let _ = self.set_power_state(DevicePowerState::D0);
        }
    }
    
    pub fn mark_active(&mut self) {
        self.last_activity = Self::get_current_time();
        
        // Wake if sleeping
        if self.runtime_pm_enabled && self.current_state != DevicePowerState::D0 {
            let _ = self.set_power_state(DevicePowerState::D0);
        }
    }
    
    pub fn check_idle(&mut self) {
        if !self.runtime_pm_enabled || self.current_state != DevicePowerState::D0 {
            return;
        }
        
        let current_time = Self::get_current_time();
        let idle_time = current_time - self.last_activity;
        
        if idle_time > self.idle_timeout_ms as u64 {
            // Put device to sleep
            let target_state = self.get_auto_sleep_state();
            let _ = self.set_power_state(target_state);
        }
    }
    
    fn get_auto_sleep_state(&self) -> DevicePowerState {
        // Choose appropriate sleep state based on device type
        match self.device_type {
            DeviceType::USB => DevicePowerState::D2,
            DeviceType::Network => DevicePowerState::D1,
            DeviceType::Display => DevicePowerState::D3Hot,
            DeviceType::Storage => DevicePowerState::D1,
            _ => DevicePowerState::D1,
        }
    }
    
    fn get_current_time() -> u64 {
        // This would use a real timer
        0
    }
    
    pub fn get_power_consumption(&self) -> u32 {
        match self.current_state {
            DevicePowerState::D0 => self.power_consumption.d0_mw,
            DevicePowerState::D1 => self.power_consumption.d1_mw,
            DevicePowerState::D2 => self.power_consumption.d2_mw,
            DevicePowerState::D3Hot | DevicePowerState::D3Cold => self.power_consumption.d3_mw,
        }
    }
}

#[derive(Debug)]
pub struct DevicePowerManager {
    devices: BTreeMap<u64, DevicePower>,
    runtime_pm_policy: RuntimePmPolicy,
    suspended_devices: Vec<u64>,
}

impl DevicePowerManager {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            runtime_pm_policy: RuntimePmPolicy::Auto,
            suspended_devices: Vec::new(),
        }
    }
    
    pub fn register_device(&mut self, device: DevicePower) {
        let device_id = device.device_id;
        serial_println!("DevicePM: Registered device {} ({})", 
                       device.device_name, device_id);
        self.devices.insert(device_id, device);
        
        // Apply runtime PM policy
        self.apply_runtime_pm_to_device(device_id);
    }
    
    pub fn unregister_device(&mut self, device_id: u64) {
        if let Some(device) = self.devices.remove(&device_id) {
            serial_println!("DevicePM: Unregistered device {}", device.device_name);
        }
    }
    
    pub fn set_runtime_pm_policy(&mut self, policy: RuntimePmPolicy) {
        self.runtime_pm_policy = policy;
        
        // Apply to all devices
        let device_ids: Vec<u64> = self.devices.keys().cloned().collect();
        for device_id in device_ids {
            self.apply_runtime_pm_to_device(device_id);
        }
        
        serial_println!("DevicePM: Runtime PM policy set to {:?}", policy);
    }
    
    fn apply_runtime_pm_to_device(&mut self, device_id: u64) {
        if let Some(device) = self.devices.get_mut(&device_id) {
            match self.runtime_pm_policy {
                RuntimePmPolicy::Disabled => {
                    device.disable_runtime_pm();
                },
                RuntimePmPolicy::Auto => {
                    let timeout = match device.device_type {
                        DeviceType::USB => 2000,
                        DeviceType::Network => 5000,
                        DeviceType::Display => 60000,
                        _ => 10000,
                    };
                    device.enable_runtime_pm(timeout);
                },
                RuntimePmPolicy::Aggressive => {
                    let timeout = match device.device_type {
                        DeviceType::USB => 500,
                        DeviceType::Network => 1000,
                        DeviceType::Display => 30000,
                        _ => 2000,
                    };
                    device.enable_runtime_pm(timeout);
                },
            }
        }
    }
    
    pub fn suspend_all_devices(&mut self) -> Result<(), &'static str> {
        serial_println!("DevicePM: Suspending all devices");
        
        self.suspended_devices.clear();
        
        // Suspend in reverse order (leaf devices first)
        let device_ids: Vec<u64> = self.devices.keys().rev().cloned().collect();
        
        for device_id in device_ids {
            if let Some(device) = self.devices.get_mut(&device_id) {
                if device.current_state == DevicePowerState::D0 {
                    device.set_power_state(DevicePowerState::D3Hot)?;
                    self.suspended_devices.push(device_id);
                }
            }
        }
        
        Ok(())
    }
    
    pub fn resume_all_devices(&mut self) -> Result<(), &'static str> {
        serial_println!("DevicePM: Resuming all devices");
        
        // Resume in original order
        for device_id in self.suspended_devices.clone() {
            if let Some(device) = self.devices.get_mut(&device_id) {
                device.set_power_state(DevicePowerState::D0)?;
            }
        }
        
        self.suspended_devices.clear();
        Ok(())
    }
    
    pub fn update_idle_devices(&mut self) {
        for device in self.devices.values_mut() {
            device.check_idle();
        }
    }
    
    pub fn get_total_power_consumption(&self) -> u32 {
        self.devices.values().map(|d| d.get_power_consumption()).sum()
    }
}

// Specific device implementations

pub struct PCIDevicePower {
    base: DevicePower,
    config_space: [u8; 256],
}

impl PCIDevicePower {
    pub fn new(bus: u8, device: u8, function: u8) -> Self {
        let device_id = ((bus as u64) << 16) | ((device as u64) << 8) | (function as u64);
        let name = format!("PCI {:02x}:{:02x}.{}", bus, device, function);
        
        let mut base = DevicePower::new(device_id, name, DeviceType::PCI);
        base.supported_states = vec![
            DevicePowerState::D0,
            DevicePowerState::D1,
            DevicePowerState::D2,
            DevicePowerState::D3Hot,
        ];
        
        Self {
            base,
            config_space: [0; 256],
        }
    }
    
    pub fn save_config_space(&mut self) {
        // Save PCI configuration space
        serial_println!("PCI: Saving config space for {}", self.base.device_name);
    }
    
    pub fn restore_config_space(&mut self) {
        // Restore PCI configuration space
        serial_println!("PCI: Restoring config space for {}", self.base.device_name);
    }
}

pub struct USBDevicePower {
    base: DevicePower,
    port_number: u8,
    selective_suspend: bool,
}

impl USBDevicePower {
    pub fn new(port: u8) -> Self {
        let device_id = 0x1000 | port as u64;
        let name = format!("USB Port {}", port);
        
        let mut base = DevicePower::new(device_id, name, DeviceType::USB);
        base.supported_states = vec![
            DevicePowerState::D0,
            DevicePowerState::D2,
            DevicePowerState::D3Hot,
        ];
        
        Self {
            base,
            port_number: port,
            selective_suspend: true,
        }
    }
    
    pub fn enable_selective_suspend(&mut self) {
        self.selective_suspend = true;
        self.base.enable_runtime_pm(1000);
    }
}

pub struct SATADevicePower {
    base: DevicePower,
    port_number: u8,
    link_power_management: bool,
    spin_down_timeout: u32,
}

impl SATADevicePower {
    pub fn new(port: u8) -> Self {
        let device_id = 0x2000 | port as u64;
        let name = format!("SATA Port {}", port);
        
        let mut base = DevicePower::new(device_id, name, DeviceType::SATA);
        base.supported_states = vec![
            DevicePowerState::D0,
            DevicePowerState::D1,
            DevicePowerState::D3Hot,
        ];
        
        Self {
            base,
            port_number: port,
            link_power_management: true,
            spin_down_timeout: 600000, // 10 minutes
        }
    }
    
    pub fn set_spin_down_timeout(&mut self, timeout_ms: u32) {
        self.spin_down_timeout = timeout_ms;
        serial_println!("SATA: Spin-down timeout set to {}ms", timeout_ms);
    }
}

lazy_static! {
    static ref DEVICE_PM: Mutex<DevicePowerManager> = Mutex::new(DevicePowerManager::new());
}

pub fn init() -> Result<(), &'static str> {
    serial_println!("DevicePM: Initializing device power management");
    
    // Register sample devices for testing
    register_sample_devices();
    
    Ok(())
}

fn register_sample_devices() {
    let mut pm = DEVICE_PM.lock();
    
    // Register PCI devices
    for bus in 0..2 {
        for device in 0..4 {
            let pci = PCIDevicePower::new(bus, device, 0);
            pm.register_device(pci.base);
        }
    }
    
    // Register USB ports
    for port in 0..4 {
        let usb = USBDevicePower::new(port);
        pm.register_device(usb.base);
    }
    
    // Register SATA ports
    for port in 0..2 {
        let sata = SATADevicePower::new(port);
        pm.register_device(sata.base);
    }
}

pub fn set_runtime_pm_policy(policy: RuntimePmPolicy) -> Result<(), &'static str> {
    DEVICE_PM.lock().set_runtime_pm_policy(policy);
    Ok(())
}

pub fn suspend_all_devices() -> Result<(), &'static str> {
    DEVICE_PM.lock().suspend_all_devices()
}

pub fn resume_all_devices() -> Result<(), &'static str> {
    DEVICE_PM.lock().resume_all_devices()
}

pub fn update_idle_devices() {
    DEVICE_PM.lock().update_idle_devices();
}

pub fn get_total_device_power() -> u32 {
    DEVICE_PM.lock().get_total_power_consumption()
}