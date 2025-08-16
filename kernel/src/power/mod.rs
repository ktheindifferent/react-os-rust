pub mod cpufreq;
pub mod suspend;
pub mod hibernate;
pub mod device;
pub mod battery;
pub mod profile;
pub mod governor;

use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::serial_println;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerState {
    S0,     // Working
    S1,     // CPU stopped, RAM powered
    S2,     // CPU off, RAM powered
    S3,     // Suspend to RAM
    S4,     // Hibernate (suspend to disk)
    S5,     // Soft off
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DevicePowerState {
    D0,     // Fully on
    D1,     // Light sleep
    D2,     // Deep sleep
    D3Hot,  // Off but power maintained
    D3Cold, // Off and no power
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerProfile {
    Performance,
    Balanced,
    PowerSaver,
    Custom,
}

#[derive(Debug)]
pub struct PowerManagementSystem {
    current_state: PowerState,
    current_profile: PowerProfile,
    cpu_governor: governor::CpuGovernor,
    suspend_enabled: bool,
    hibernate_enabled: bool,
    battery_present: bool,
    thermal_zones: Vec<thermal::ThermalZone>,
    wake_sources: Vec<WakeSource>,
    power_consumption: PowerConsumption,
}

#[derive(Debug, Clone)]
pub struct WakeSource {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub source_type: WakeSourceType,
}

#[derive(Debug, Clone, Copy)]
pub enum WakeSourceType {
    PowerButton,
    Keyboard,
    Mouse,
    Network,
    RTC,
    USB,
    LidSwitch,
}

#[derive(Debug, Default)]
pub struct PowerConsumption {
    pub cpu_power_mw: u32,
    pub gpu_power_mw: u32,
    pub memory_power_mw: u32,
    pub disk_power_mw: u32,
    pub network_power_mw: u32,
    pub other_power_mw: u32,
}

impl PowerConsumption {
    pub fn total(&self) -> u32 {
        self.cpu_power_mw + self.gpu_power_mw + self.memory_power_mw +
        self.disk_power_mw + self.network_power_mw + self.other_power_mw
    }
}

impl PowerManagementSystem {
    pub fn new() -> Self {
        Self {
            current_state: PowerState::S0,
            current_profile: PowerProfile::Balanced,
            cpu_governor: governor::CpuGovernor::OnDemand,
            suspend_enabled: false,
            hibernate_enabled: false,
            battery_present: false,
            thermal_zones: Vec::new(),
            wake_sources: Vec::new(),
            power_consumption: PowerConsumption::default(),
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("Power: Initializing advanced power management");
        
        // Initialize CPU frequency scaling
        cpufreq::init()?;
        
        // Initialize suspend/hibernate support
        suspend::init()?;
        hibernate::init()?;
        
        // Initialize device power management
        device::init()?;
        
        // Check for battery
        if battery::init().is_ok() {
            self.battery_present = true;
            serial_println!("Power: Battery detected");
        }
        
        // Initialize thermal management
        self.init_thermal_zones()?;
        
        // Set up wake sources
        self.init_wake_sources()?;
        
        // Apply default power profile
        self.apply_profile(PowerProfile::Balanced)?;
        
        serial_println!("Power: Advanced power management initialized");
        Ok(())
    }

    fn init_thermal_zones(&mut self) -> Result<(), &'static str> {
        // Initialize thermal zones from ACPI or hardware sensors
        use super::thermal;
        
        let zones = thermal::detect_thermal_zones()?;
        self.thermal_zones = zones;
        
        if !self.thermal_zones.is_empty() {
            serial_println!("Power: Found {} thermal zones", self.thermal_zones.len());
        }
        
        Ok(())
    }

    fn init_wake_sources(&mut self) -> Result<(), &'static str> {
        // Initialize default wake sources
        self.wake_sources.push(WakeSource {
            id: 0,
            name: String::from("Power Button"),
            enabled: true,
            source_type: WakeSourceType::PowerButton,
        });
        
        self.wake_sources.push(WakeSource {
            id: 1,
            name: String::from("Keyboard"),
            enabled: true,
            source_type: WakeSourceType::Keyboard,
        });
        
        self.wake_sources.push(WakeSource {
            id: 2,
            name: String::from("RTC Alarm"),
            enabled: false,
            source_type: WakeSourceType::RTC,
        });
        
        Ok(())
    }

    pub fn set_profile(&mut self, profile: PowerProfile) -> Result<(), &'static str> {
        self.current_profile = profile;
        self.apply_profile(profile)
    }

    fn apply_profile(&mut self, profile: PowerProfile) -> Result<(), &'static str> {
        match profile {
            PowerProfile::Performance => {
                self.cpu_governor = governor::CpuGovernor::Performance;
                cpufreq::set_governor(governor::CpuGovernor::Performance)?;
                device::set_runtime_pm_policy(device::RuntimePmPolicy::Disabled)?;
            },
            PowerProfile::Balanced => {
                self.cpu_governor = governor::CpuGovernor::OnDemand;
                cpufreq::set_governor(governor::CpuGovernor::OnDemand)?;
                device::set_runtime_pm_policy(device::RuntimePmPolicy::Auto)?;
            },
            PowerProfile::PowerSaver => {
                self.cpu_governor = governor::CpuGovernor::PowerSave;
                cpufreq::set_governor(governor::CpuGovernor::PowerSave)?;
                device::set_runtime_pm_policy(device::RuntimePmPolicy::Aggressive)?;
            },
            PowerProfile::Custom => {
                // Custom profile allows fine-grained control
            }
        }
        
        serial_println!("Power: Applied {:?} profile", profile);
        Ok(())
    }

    pub fn suspend(&mut self) -> Result<(), &'static str> {
        if !self.suspend_enabled {
            return Err("Suspend not enabled");
        }
        
        serial_println!("Power: Preparing system suspend");
        
        // Freeze user processes
        suspend::freeze_processes()?;
        
        // Suspend devices
        device::suspend_all_devices()?;
        
        // Enter S3 state
        suspend::enter_s3_state()?;
        
        // System will resume here
        
        // Resume devices
        device::resume_all_devices()?;
        
        // Thaw processes
        suspend::thaw_processes()?;
        
        serial_println!("Power: System resumed from suspend");
        Ok(())
    }

    pub fn hibernate(&mut self) -> Result<(), &'static str> {
        if !self.hibernate_enabled {
            return Err("Hibernation not enabled");
        }
        
        serial_println!("Power: Preparing hibernation");
        
        // Create hibernation image
        hibernate::create_hibernation_image()?;
        
        // Write image to disk
        hibernate::write_hibernation_image()?;
        
        // Enter S4 state
        hibernate::enter_s4_state()?;
        
        // System will power off and resume from image on next boot
        unreachable!()
    }

    pub fn enable_wake_source(&mut self, source_type: WakeSourceType, enable: bool) {
        for source in &mut self.wake_sources {
            if source.source_type == source_type {
                source.enabled = enable;
                break;
            }
        }
    }

    pub fn get_power_consumption(&self) -> u32 {
        self.power_consumption.total()
    }

    pub fn update_power_consumption(&mut self) {
        // Update power consumption from various sources
        self.power_consumption.cpu_power_mw = cpufreq::get_cpu_power_consumption();
        
        if self.battery_present {
            battery::update_power_draw(&mut self.power_consumption);
        }
    }

    pub fn get_battery_status(&self) -> Option<battery::BatteryStatus> {
        if self.battery_present {
            battery::get_status()
        } else {
            None
        }
    }

    pub fn set_cpu_frequency_limits(&mut self, min_mhz: u32, max_mhz: u32) -> Result<(), &'static str> {
        cpufreq::set_frequency_limits(min_mhz, max_mhz)
    }

    pub fn enable_turbo_boost(&mut self, enable: bool) -> Result<(), &'static str> {
        cpufreq::set_turbo_boost(enable)
    }
}

lazy_static! {
    pub static ref POWER_MGMT: Mutex<PowerManagementSystem> = Mutex::new(PowerManagementSystem::new());
}

pub fn init() -> Result<(), &'static str> {
    POWER_MGMT.lock().init()
}

pub fn set_power_profile(profile: PowerProfile) -> Result<(), &'static str> {
    POWER_MGMT.lock().set_profile(profile)
}

pub fn suspend_system() -> Result<(), &'static str> {
    POWER_MGMT.lock().suspend()
}

pub fn hibernate_system() -> Result<(), &'static str> {
    POWER_MGMT.lock().hibernate()
}

pub fn get_current_profile() -> PowerProfile {
    POWER_MGMT.lock().current_profile
}

pub fn get_power_state() -> PowerState {
    POWER_MGMT.lock().current_state
}

pub mod thermal {
    use alloc::string::String;
    use alloc::vec::Vec;
    
    #[derive(Debug, Clone)]
    pub struct ThermalZone {
        pub id: u32,
        pub name: String,
        pub current_temp: i32,  // millidegrees Celsius
        pub trip_points: Vec<TripPoint>,
        pub cooling_devices: Vec<CoolingDevice>,
    }
    
    #[derive(Debug, Clone)]
    pub struct TripPoint {
        pub temperature: i32,
        pub trip_type: TripType,
        pub cooling_level: u32,
    }
    
    #[derive(Debug, Clone, Copy)]
    pub enum TripType {
        Critical,
        Hot,
        Passive,
        Active,
    }
    
    #[derive(Debug, Clone)]
    pub struct CoolingDevice {
        pub id: u32,
        pub device_type: CoolingDeviceType,
        pub current_state: u32,
        pub max_state: u32,
    }
    
    #[derive(Debug, Clone, Copy)]
    pub enum CoolingDeviceType {
        Fan,
        Processor,
        Memory,
    }
    
    pub fn detect_thermal_zones() -> Result<Vec<ThermalZone>, &'static str> {
        // This would detect thermal zones from ACPI or hardware
        Ok(Vec::new())
    }
}