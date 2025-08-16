use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::serial_println;
use super::{PowerProfile, governor::CpuGovernor, device::RuntimePmPolicy};

#[derive(Debug, Clone)]
pub struct PowerProfileConfig {
    pub name: String,
    pub profile_type: PowerProfile,
    pub cpu_governor: CpuGovernor,
    pub cpu_min_freq_percent: u8,
    pub cpu_max_freq_percent: u8,
    pub turbo_boost_enabled: bool,
    pub device_pm_policy: RuntimePmPolicy,
    pub display_brightness_percent: u8,
    pub display_timeout_seconds: u32,
    pub disk_spindown_timeout_seconds: u32,
    pub wifi_power_save: bool,
    pub bluetooth_power_save: bool,
    pub usb_autosuspend: bool,
    pub pcie_aspm: PCIeASPM,
    pub thermal_policy: crate::thermal::ThermalPolicy,
    pub wake_on_lan: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum PCIeASPM {
    Disabled,
    L0s,
    L1,
    L0sL1,
}

impl PowerProfileConfig {
    pub fn performance() -> Self {
        Self {
            name: String::from("Performance"),
            profile_type: PowerProfile::Performance,
            cpu_governor: CpuGovernor::Performance,
            cpu_min_freq_percent: 50,
            cpu_max_freq_percent: 100,
            turbo_boost_enabled: true,
            device_pm_policy: RuntimePmPolicy::Disabled,
            display_brightness_percent: 100,
            display_timeout_seconds: 1800, // 30 minutes
            disk_spindown_timeout_seconds: 0, // Never
            wifi_power_save: false,
            bluetooth_power_save: false,
            usb_autosuspend: false,
            pcie_aspm: PCIeASPM::Disabled,
            thermal_policy: crate::thermal::ThermalPolicy::Performance,
            wake_on_lan: true,
        }
    }
    
    pub fn balanced() -> Self {
        Self {
            name: String::from("Balanced"),
            profile_type: PowerProfile::Balanced,
            cpu_governor: CpuGovernor::OnDemand,
            cpu_min_freq_percent: 20,
            cpu_max_freq_percent: 100,
            turbo_boost_enabled: true,
            device_pm_policy: RuntimePmPolicy::Auto,
            display_brightness_percent: 80,
            display_timeout_seconds: 600, // 10 minutes
            disk_spindown_timeout_seconds: 600,
            wifi_power_save: true,
            bluetooth_power_save: true,
            usb_autosuspend: true,
            pcie_aspm: PCIeASPM::L0sL1,
            thermal_policy: crate::thermal::ThermalPolicy::Balanced,
            wake_on_lan: true,
        }
    }
    
    pub fn power_saver() -> Self {
        Self {
            name: String::from("Power Saver"),
            profile_type: PowerProfile::PowerSaver,
            cpu_governor: CpuGovernor::PowerSave,
            cpu_min_freq_percent: 10,
            cpu_max_freq_percent: 60,
            turbo_boost_enabled: false,
            device_pm_policy: RuntimePmPolicy::Aggressive,
            display_brightness_percent: 50,
            display_timeout_seconds: 300, // 5 minutes
            disk_spindown_timeout_seconds: 120,
            wifi_power_save: true,
            bluetooth_power_save: true,
            usb_autosuspend: true,
            pcie_aspm: PCIeASPM::L0sL1,
            thermal_policy: crate::thermal::ThermalPolicy::Quiet,
            wake_on_lan: false,
        }
    }
}

#[derive(Debug)]
pub struct ProfileManager {
    profiles: Vec<PowerProfileConfig>,
    active_profile: PowerProfile,
    custom_profile: Option<PowerProfileConfig>,
    auto_switch_enabled: bool,
    battery_profile: PowerProfile,
    ac_profile: PowerProfile,
}

impl ProfileManager {
    pub fn new() -> Self {
        Self {
            profiles: vec![
                PowerProfileConfig::performance(),
                PowerProfileConfig::balanced(),
                PowerProfileConfig::power_saver(),
            ],
            active_profile: PowerProfile::Balanced,
            custom_profile: None,
            auto_switch_enabled: false,
            battery_profile: PowerProfile::PowerSaver,
            ac_profile: PowerProfile::Performance,
        }
    }
    
    pub fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("Profile: Initializing power profile manager");
        
        // Apply default profile
        self.apply_profile(PowerProfile::Balanced)?;
        
        Ok(())
    }
    
    pub fn apply_profile(&mut self, profile: PowerProfile) -> Result<(), &'static str> {
        let config = match profile {
            PowerProfile::Custom => {
                self.custom_profile.as_ref()
                    .ok_or("Custom profile not configured")?
            },
            _ => {
                self.profiles.iter()
                    .find(|p| p.profile_type == profile)
                    .ok_or("Profile not found")?
            }
        };
        
        serial_println!("Profile: Applying {} profile", config.name);
        
        // Apply CPU settings
        super::cpufreq::set_governor(config.cpu_governor)?;
        super::cpufreq::set_turbo_boost(config.turbo_boost_enabled)?;
        
        // Calculate and set frequency limits
        let cpu_freq = super::cpufreq::get_current_frequency();
        let min_freq = cpu_freq * config.cpu_min_freq_percent as u32 / 100;
        let max_freq = cpu_freq * config.cpu_max_freq_percent as u32 / 100;
        super::cpufreq::set_frequency_limits(min_freq, max_freq)?;
        
        // Apply device power management policy
        super::device::set_runtime_pm_policy(config.device_pm_policy)?;
        
        // Apply thermal policy
        crate::thermal::set_thermal_policy(config.thermal_policy)?;
        
        // Apply other settings
        self.apply_display_settings(config)?;
        self.apply_network_settings(config)?;
        self.apply_storage_settings(config)?;
        
        self.active_profile = profile;
        
        serial_println!("Profile: {} profile activated", config.name);
        Ok(())
    }
    
    fn apply_display_settings(&self, config: &PowerProfileConfig) -> Result<(), &'static str> {
        // Set display brightness
        // This would interface with display driver
        serial_println!("Profile: Display brightness set to {}%", 
                       config.display_brightness_percent);
        
        // Set display timeout
        serial_println!("Profile: Display timeout set to {} seconds",
                       config.display_timeout_seconds);
        
        Ok(())
    }
    
    fn apply_network_settings(&self, config: &PowerProfileConfig) -> Result<(), &'static str> {
        // Configure WiFi power save
        if config.wifi_power_save {
            serial_println!("Profile: WiFi power save enabled");
        }
        
        // Configure Bluetooth power save
        if config.bluetooth_power_save {
            serial_println!("Profile: Bluetooth power save enabled");
        }
        
        // Configure Wake-on-LAN
        if config.wake_on_lan {
            serial_println!("Profile: Wake-on-LAN enabled");
        }
        
        Ok(())
    }
    
    fn apply_storage_settings(&self, config: &PowerProfileConfig) -> Result<(), &'static str> {
        // Set disk spin-down timeout
        if config.disk_spindown_timeout_seconds > 0 {
            serial_println!("Profile: Disk spin-down timeout set to {} seconds",
                           config.disk_spindown_timeout_seconds);
        }
        
        Ok(())
    }
    
    pub fn create_custom_profile(&mut self, config: PowerProfileConfig) {
        self.custom_profile = Some(config);
        serial_println!("Profile: Custom profile created");
    }
    
    pub fn enable_auto_switching(&mut self, battery_profile: PowerProfile, ac_profile: PowerProfile) {
        self.auto_switch_enabled = true;
        self.battery_profile = battery_profile;
        self.ac_profile = ac_profile;
        
        serial_println!("Profile: Auto-switching enabled (Battery: {:?}, AC: {:?})",
                       battery_profile, ac_profile);
    }
    
    pub fn disable_auto_switching(&mut self) {
        self.auto_switch_enabled = false;
        serial_println!("Profile: Auto-switching disabled");
    }
    
    pub fn handle_power_source_change(&mut self, on_battery: bool) -> Result<(), &'static str> {
        if !self.auto_switch_enabled {
            return Ok(());
        }
        
        let target_profile = if on_battery {
            self.battery_profile
        } else {
            self.ac_profile
        };
        
        if target_profile != self.active_profile {
            serial_println!("Profile: Power source changed, switching to {:?} profile",
                           target_profile);
            self.apply_profile(target_profile)?;
        }
        
        Ok(())
    }
    
    pub fn get_active_profile(&self) -> PowerProfile {
        self.active_profile
    }
    
    pub fn get_profile_config(&self, profile: PowerProfile) -> Option<&PowerProfileConfig> {
        match profile {
            PowerProfile::Custom => self.custom_profile.as_ref(),
            _ => self.profiles.iter().find(|p| p.profile_type == profile),
        }
    }
}

// Advanced power policies

#[derive(Debug)]
pub struct PowerPolicy {
    pub timer_coalescing: bool,
    pub cpu_core_parking: bool,
    pub memory_compression: bool,
    pub swap_prefetch: bool,
    pub process_priority_boost: bool,
}

impl PowerPolicy {
    pub fn default() -> Self {
        Self {
            timer_coalescing: true,
            cpu_core_parking: true,
            memory_compression: true,
            swap_prefetch: false,
            process_priority_boost: true,
        }
    }
    
    pub fn apply(&self) -> Result<(), &'static str> {
        if self.timer_coalescing {
            Self::enable_timer_coalescing()?;
        }
        
        if self.cpu_core_parking {
            Self::enable_core_parking()?;
        }
        
        if self.memory_compression {
            Self::enable_memory_compression()?;
        }
        
        Ok(())
    }
    
    fn enable_timer_coalescing() -> Result<(), &'static str> {
        // Coalesce timer interrupts to reduce wake-ups
        serial_println!("Policy: Timer coalescing enabled");
        Ok(())
    }
    
    fn enable_core_parking() -> Result<(), &'static str> {
        // Park idle CPU cores to save power
        serial_println!("Policy: CPU core parking enabled");
        Ok(())
    }
    
    fn enable_memory_compression() -> Result<(), &'static str> {
        // Compress inactive memory pages
        serial_println!("Policy: Memory compression enabled");
        Ok(())
    }
}

lazy_static! {
    static ref PROFILE_MGR: Mutex<ProfileManager> = Mutex::new(ProfileManager::new());
}

pub fn init() -> Result<(), &'static str> {
    PROFILE_MGR.lock().init()
}

pub fn set_power_profile(profile: PowerProfile) -> Result<(), &'static str> {
    PROFILE_MGR.lock().apply_profile(profile)
}

pub fn create_custom_profile(config: PowerProfileConfig) {
    PROFILE_MGR.lock().create_custom_profile(config);
}

pub fn enable_auto_switching(battery_profile: PowerProfile, ac_profile: PowerProfile) {
    PROFILE_MGR.lock().enable_auto_switching(battery_profile, ac_profile);
}

pub fn handle_power_source_change(on_battery: bool) -> Result<(), &'static str> {
    PROFILE_MGR.lock().handle_power_source_change(on_battery)
}

pub fn get_active_profile() -> PowerProfile {
    PROFILE_MGR.lock().get_active_profile()
}