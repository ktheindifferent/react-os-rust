use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::serial_println;

#[derive(Debug, Clone)]
pub struct ThermalZone {
    pub id: u32,
    pub name: String,
    pub zone_type: ThermalZoneType,
    pub current_temp: i32,  // millidegrees Celsius
    pub trip_points: Vec<TripPoint>,
    pub cooling_devices: Vec<CoolingDevice>,
    pub polling_delay_ms: u32,
    pub passive_delay_ms: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum ThermalZoneType {
    CPU,
    GPU,
    Chipset,
    Memory,
    Storage,
    Battery,
    Skin,
}

#[derive(Debug, Clone)]
pub struct TripPoint {
    pub temperature: i32,  // millidegrees Celsius
    pub hysteresis: i32,
    pub trip_type: TripType,
    pub cooling_device_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TripType {
    Critical,   // System shutdown
    Hot,        // Aggressive throttling
    Passive,    // CPU frequency scaling
    Active0,    // Fan level 0
    Active1,    // Fan level 1
    Active2,    // Fan level 2
}

#[derive(Debug, Clone)]
pub struct CoolingDevice {
    pub id: u32,
    pub name: String,
    pub device_type: CoolingDeviceType,
    pub current_state: u32,
    pub max_state: u32,
    pub min_state: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum CoolingDeviceType {
    Fan,
    Processor,
    Memory,
    GPU,
}

#[derive(Debug)]
pub struct ThermalManager {
    zones: Vec<ThermalZone>,
    cooling_devices: Vec<CoolingDevice>,
    thermal_policy: ThermalPolicy,
    emergency_shutdown_temp: i32,
    throttling_active: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum ThermalPolicy {
    Performance,    // Higher temperature thresholds
    Balanced,       // Default thresholds
    Quiet,          // Lower fan speeds, more throttling
}

impl ThermalManager {
    pub fn new() -> Self {
        Self {
            zones: Vec::new(),
            cooling_devices: Vec::new(),
            thermal_policy: ThermalPolicy::Balanced,
            emergency_shutdown_temp: 105000, // 105°C
            throttling_active: false,
        }
    }
    
    pub fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("Thermal: Initializing thermal management");
        
        // Detect thermal zones
        self.detect_thermal_zones()?;
        
        // Detect cooling devices
        self.detect_cooling_devices()?;
        
        // Set up default trip points
        self.configure_trip_points()?;
        
        serial_println!("Thermal: Found {} zones and {} cooling devices",
                       self.zones.len(), self.cooling_devices.len());
        
        Ok(())
    }
    
    fn detect_thermal_zones(&mut self) -> Result<(), &'static str> {
        // Detect from ACPI thermal zones
        // This would parse ACPI _TZ objects
        
        // Create CPU thermal zone
        let cpu_zone = ThermalZone {
            id: 0,
            name: String::from("CPU"),
            zone_type: ThermalZoneType::CPU,
            current_temp: 45000, // 45°C
            trip_points: Vec::new(),
            cooling_devices: Vec::new(),
            polling_delay_ms: 1000,
            passive_delay_ms: 100,
        };
        self.zones.push(cpu_zone);
        
        // Create GPU thermal zone if present
        let gpu_zone = ThermalZone {
            id: 1,
            name: String::from("GPU"),
            zone_type: ThermalZoneType::GPU,
            current_temp: 40000, // 40°C
            trip_points: Vec::new(),
            cooling_devices: Vec::new(),
            polling_delay_ms: 1000,
            passive_delay_ms: 100,
        };
        self.zones.push(gpu_zone);
        
        Ok(())
    }
    
    fn detect_cooling_devices(&mut self) -> Result<(), &'static str> {
        // Detect from ACPI or hardware
        
        // CPU cooling device (frequency scaling)
        let cpu_cooling = CoolingDevice {
            id: 0,
            name: String::from("CPU"),
            device_type: CoolingDeviceType::Processor,
            current_state: 0,
            max_state: 10,
            min_state: 0,
        };
        self.cooling_devices.push(cpu_cooling);
        
        // System fan
        let fan = CoolingDevice {
            id: 1,
            name: String::from("System Fan"),
            device_type: CoolingDeviceType::Fan,
            current_state: 0,
            max_state: 255,
            min_state: 0,
        };
        self.cooling_devices.push(fan);
        
        Ok(())
    }
    
    fn configure_trip_points(&mut self) -> Result<(), &'static str> {
        // Configure trip points based on thermal policy
        
        for zone in &mut self.zones {
            match zone.zone_type {
                ThermalZoneType::CPU => {
                    self.configure_cpu_trip_points(zone);
                },
                ThermalZoneType::GPU => {
                    self.configure_gpu_trip_points(zone);
                },
                _ => {}
            }
        }
        
        Ok(())
    }
    
    fn configure_cpu_trip_points(&self, zone: &mut ThermalZone) {
        let (passive, active0, active1, hot, critical) = match self.thermal_policy {
            ThermalPolicy::Performance => (75000, 80000, 85000, 95000, 100000),
            ThermalPolicy::Balanced => (70000, 75000, 80000, 90000, 95000),
            ThermalPolicy::Quiet => (65000, 70000, 75000, 85000, 90000),
        };
        
        zone.trip_points.clear();
        
        // Passive cooling (CPU throttling)
        zone.trip_points.push(TripPoint {
            temperature: passive,
            hysteresis: 2000,
            trip_type: TripType::Passive,
            cooling_device_id: Some(0), // CPU cooling
        });
        
        // Active cooling levels (fan speeds)
        zone.trip_points.push(TripPoint {
            temperature: active0,
            hysteresis: 3000,
            trip_type: TripType::Active0,
            cooling_device_id: Some(1), // Fan
        });
        
        zone.trip_points.push(TripPoint {
            temperature: active1,
            hysteresis: 3000,
            trip_type: TripType::Active1,
            cooling_device_id: Some(1),
        });
        
        // Hot (aggressive throttling)
        zone.trip_points.push(TripPoint {
            temperature: hot,
            hysteresis: 2000,
            trip_type: TripType::Hot,
            cooling_device_id: Some(0),
        });
        
        // Critical (shutdown)
        zone.trip_points.push(TripPoint {
            temperature: critical,
            hysteresis: 0,
            trip_type: TripType::Critical,
            cooling_device_id: None,
        });
    }
    
    fn configure_gpu_trip_points(&self, zone: &mut ThermalZone) {
        // Similar to CPU but with different thresholds
        zone.trip_points.push(TripPoint {
            temperature: 80000,
            hysteresis: 3000,
            trip_type: TripType::Passive,
            cooling_device_id: Some(2), // GPU throttling if available
        });
    }
    
    pub fn update_temperatures(&mut self) -> Result<(), &'static str> {
        // First, collect temperatures
        let mut temperatures = Vec::new();
        for zone in &self.zones {
            temperatures.push((zone.id, self.read_temperature(zone.id)?));
        }
        
        // Then update zones and check trip points
        for (zone_id, temp) in temperatures {
            if let Some(zone) = self.zones.iter_mut().find(|z| z.id == zone_id) {
                zone.current_temp = temp;
                // Clone zone for checking to avoid borrow issues
                let zone_clone = zone.clone();
                self.check_trip_points(&zone_clone)?;
            }
        }
        
        Ok(())
    }
    
    fn read_temperature(&self, zone_id: u32) -> Result<i32, &'static str> {
        // Read temperature from hardware sensors or ACPI
        match zone_id {
            0 => Ok(self.read_cpu_temperature()),
            1 => Ok(self.read_gpu_temperature()),
            _ => Ok(40000), // Default 40°C
        }
    }
    
    fn read_cpu_temperature(&self) -> i32 {
        // Read from CPU thermal sensor
        // This would use MSRs or ACPI
        45000 + (self.get_cpu_load() as i32 * 300) // Simulate based on load
    }
    
    fn read_gpu_temperature(&self) -> i32 {
        // Read from GPU thermal sensor
        40000
    }
    
    fn get_cpu_load(&self) -> u32 {
        // Get current CPU load percentage
        50 // Simulated
    }
    
    fn check_trip_points(&mut self, zone: &ThermalZone) -> Result<(), &'static str> {
        for trip in &zone.trip_points {
            if zone.current_temp >= trip.temperature {
                self.handle_trip_point(zone, trip)?;
            } else if zone.current_temp < (trip.temperature - trip.hysteresis) {
                self.clear_trip_point(zone, trip)?;
            }
        }
        
        Ok(())
    }
    
    fn handle_trip_point(&mut self, zone: &ThermalZone, trip: &TripPoint) -> Result<(), &'static str> {
        serial_println!("Thermal: Zone {} reached {:?} trip point at {}°C",
                       zone.name, trip.trip_type, trip.temperature / 1000);
        
        match trip.trip_type {
            TripType::Critical => {
                serial_println!("Thermal: CRITICAL temperature! Initiating emergency shutdown");
                self.emergency_shutdown()?;
            },
            TripType::Hot => {
                serial_println!("Thermal: HOT temperature! Aggressive throttling");
                self.apply_aggressive_throttling()?;
            },
            TripType::Passive => {
                self.apply_passive_cooling(trip.cooling_device_id)?;
            },
            TripType::Active0 | TripType::Active1 | TripType::Active2 => {
                self.apply_active_cooling(trip)?;
            },
        }
        
        Ok(())
    }
    
    fn clear_trip_point(&mut self, _zone: &ThermalZone, trip: &TripPoint) -> Result<(), &'static str> {
        match trip.trip_type {
            TripType::Hot => {
                self.clear_aggressive_throttling()?;
            },
            TripType::Passive => {
                self.clear_passive_cooling(trip.cooling_device_id)?;
            },
            _ => {}
        }
        
        Ok(())
    }
    
    fn apply_passive_cooling(&mut self, device_id: Option<u32>) -> Result<(), &'static str> {
        if let Some(id) = device_id {
            if let Some(device) = self.cooling_devices.iter_mut().find(|d| d.id == id) {
                // Increase cooling state (more throttling)
                device.current_state = (device.current_state + 1).min(device.max_state);
                
                if device.device_type == CoolingDeviceType::Processor {
                    // Apply CPU frequency reduction
                    crate::power::cpufreq::set_governor(crate::power::governor::CpuGovernor::PowerSave)?;
                    self.throttling_active = true;
                }
            }
        }
        
        Ok(())
    }
    
    fn clear_passive_cooling(&mut self, device_id: Option<u32>) -> Result<(), &'static str> {
        if let Some(id) = device_id {
            if let Some(device) = self.cooling_devices.iter_mut().find(|d| d.id == id) {
                device.current_state = device.min_state;
                
                if device.device_type == CoolingDeviceType::Processor && self.throttling_active {
                    // Restore normal governor
                    crate::power::cpufreq::set_governor(crate::power::governor::CpuGovernor::OnDemand)?;
                    self.throttling_active = false;
                }
            }
        }
        
        Ok(())
    }
    
    fn apply_active_cooling(&mut self, trip: &TripPoint) -> Result<(), &'static str> {
        if let Some(id) = trip.cooling_device_id {
            if let Some(device) = self.cooling_devices.iter_mut().find(|d| d.id == id) {
                if device.device_type == CoolingDeviceType::Fan {
                    // Set fan speed based on trip type
                    let fan_speed = match trip.trip_type {
                        TripType::Active0 => device.max_state * 30 / 100,  // 30%
                        TripType::Active1 => device.max_state * 60 / 100,  // 60%
                        TripType::Active2 => device.max_state * 90 / 100,  // 90%
                        _ => device.current_state,
                    };
                    
                    device.current_state = fan_speed;
                    self.set_fan_speed(device.id, fan_speed)?;
                }
            }
        }
        
        Ok(())
    }
    
    fn apply_aggressive_throttling(&mut self) -> Result<(), &'static str> {
        serial_println!("Thermal: Applying aggressive throttling");
        
        // Set minimum CPU frequency
        crate::power::cpufreq::set_governor(crate::power::governor::CpuGovernor::PowerSave)?;
        
        // Set maximum fan speed
        for device in &mut self.cooling_devices {
            if device.device_type == CoolingDeviceType::Fan {
                device.current_state = device.max_state;
                self.set_fan_speed(device.id, device.max_state)?;
            }
        }
        
        self.throttling_active = true;
        Ok(())
    }
    
    fn clear_aggressive_throttling(&mut self) -> Result<(), &'static str> {
        if self.throttling_active {
            serial_println!("Thermal: Clearing aggressive throttling");
            crate::power::cpufreq::set_governor(crate::power::governor::CpuGovernor::OnDemand)?;
            self.throttling_active = false;
        }
        Ok(())
    }
    
    fn set_fan_speed(&self, fan_id: u32, speed: u32) -> Result<(), &'static str> {
        // Set fan speed via hardware control
        serial_println!("Thermal: Setting fan {} to speed {}", fan_id, speed);
        Ok(())
    }
    
    fn emergency_shutdown(&self) -> Result<(), &'static str> {
        serial_println!("Thermal: EMERGENCY THERMAL SHUTDOWN!");
        // Initiate immediate shutdown
        crate::acpi::power::shutdown()
    }
    
    pub fn set_policy(&mut self, policy: ThermalPolicy) -> Result<(), &'static str> {
        self.thermal_policy = policy;
        self.configure_trip_points()?;
        serial_println!("Thermal: Policy set to {:?}", policy);
        Ok(())
    }
    
    pub fn get_thermal_status(&self) -> Vec<(String, i32)> {
        self.zones.iter().map(|z| (z.name.clone(), z.current_temp)).collect()
    }
}

lazy_static! {
    static ref THERMAL_MGR: Mutex<ThermalManager> = Mutex::new(ThermalManager::new());
}

pub fn init() -> Result<(), &'static str> {
    THERMAL_MGR.lock().init()
}

pub fn update_temperatures() -> Result<(), &'static str> {
    THERMAL_MGR.lock().update_temperatures()
}

pub fn set_thermal_policy(policy: ThermalPolicy) -> Result<(), &'static str> {
    THERMAL_MGR.lock().set_policy(policy)
}

pub fn get_thermal_status() -> Vec<(String, i32)> {
    THERMAL_MGR.lock().get_thermal_status()
}

pub fn detect_thermal_zones() -> Result<Vec<super::power::thermal::ThermalZone>, &'static str> {
    // Convert internal zones to power module format
    let zones = THERMAL_MGR.lock().zones.clone();
    
    Ok(zones.into_iter().map(|z| {
        super::power::thermal::ThermalZone {
            id: z.id,
            name: z.name,
            current_temp: z.current_temp,
            trip_points: z.trip_points.into_iter().map(|t| {
                super::power::thermal::TripPoint {
                    temperature: t.temperature,
                    trip_type: match t.trip_type {
                        TripType::Critical => super::power::thermal::TripType::Critical,
                        TripType::Hot => super::power::thermal::TripType::Hot,
                        TripType::Passive => super::power::thermal::TripType::Passive,
                        _ => super::power::thermal::TripType::Active,
                    },
                    cooling_level: t.cooling_device_id.unwrap_or(0),
                }
            }).collect(),
            cooling_devices: z.cooling_devices.into_iter().map(|d| {
                super::power::thermal::CoolingDevice {
                    id: d.id,
                    device_type: match d.device_type {
                        CoolingDeviceType::Fan => super::power::thermal::CoolingDeviceType::Fan,
                        CoolingDeviceType::Processor => super::power::thermal::CoolingDeviceType::Processor,
                        CoolingDeviceType::Memory => super::power::thermal::CoolingDeviceType::Memory,
                        _ => super::power::thermal::CoolingDeviceType::Fan,
                    },
                    current_state: d.current_state,
                    max_state: d.max_state,
                }
            }).collect(),
        }
    }).collect())
}