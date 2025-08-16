use super::governor::CpuGovernor;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::serial_println;
use x86_64::registers::model_specific::Msr;

const MSR_IA32_PERF_STATUS: u32 = 0x198;
const MSR_IA32_PERF_CTL: u32 = 0x199;
const MSR_PLATFORM_INFO: u32 = 0xCE;
const MSR_IA32_MPERF: u32 = 0xE7;
const MSR_IA32_APERF: u32 = 0xE8;
const MSR_IA32_MISC_ENABLE: u32 = 0x1A0;
const MSR_IA32_ENERGY_PERF_BIAS: u32 = 0x1B0;
const MSR_RAPL_POWER_UNIT: u32 = 0x606;
const MSR_PKG_ENERGY_STATUS: u32 = 0x611;

const CPUID_THERMAL_POWER: u32 = 0x06;
const CPUID_FREQ_INFO: u32 = 0x16;

#[derive(Debug, Clone, Copy)]
pub struct PState {
    pub frequency_mhz: u32,
    pub voltage_mv: u32,
    pub power_mw: u32,
    pub control_value: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct CState {
    pub state_type: CStateType,
    pub latency_us: u32,
    pub power_mw: u32,
    pub target_residency_us: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CStateType {
    C0,     // Active
    C1,     // Halt
    C1E,    // Enhanced halt
    C2,     // Stop clock
    C3,     // Deep sleep
    C6,     // Deep power down
    C7,     // Deeper power down
    C10,    // Deepest power down
}

#[derive(Debug)]
pub struct CpuFrequencyScaling {
    p_states: Vec<PState>,
    c_states: Vec<CState>,
    current_pstate: usize,
    current_cstate: CStateType,
    min_frequency: u32,
    max_frequency: u32,
    base_frequency: u32,
    turbo_enabled: bool,
    governor: CpuGovernor,
    speedstep_enabled: bool,
    cpu_count: usize,
    per_core_scaling: bool,
    energy_perf_bias: u8,
}

impl CpuFrequencyScaling {
    pub fn new() -> Self {
        Self {
            p_states: Vec::new(),
            c_states: Vec::new(),
            current_pstate: 0,
            current_cstate: CStateType::C0,
            min_frequency: 0,
            max_frequency: 0,
            base_frequency: 0,
            turbo_enabled: false,
            governor: CpuGovernor::OnDemand,
            speedstep_enabled: false,
            cpu_count: 1,
            per_core_scaling: false,
            energy_perf_bias: 6, // Balanced
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        // Detect CPU features
        self.detect_cpu_features()?;
        
        // Enumerate P-states
        self.enumerate_pstates()?;
        
        // Enumerate C-states
        self.enumerate_cstates()?;
        
        // Enable SpeedStep/Cool'n'Quiet if available
        self.enable_dynamic_frequency_scaling()?;
        
        serial_println!("CPUFreq: Initialized with {} P-states and {} C-states",
                       self.p_states.len(), self.c_states.len());
        
        Ok(())
    }

    fn detect_cpu_features(&mut self) -> Result<(), &'static str> {
        // Check for thermal and power management features using raw_cpuid
        use raw_cpuid::CpuId;
        let cpuid = CpuId::new();
        
        // Check thermal and power features
        if let Some(features) = cpuid.get_thermal_power_info() {
            if features.has_dts() {
                serial_println!("CPUFreq: Digital temperature sensor supported");
            }
            
            if features.has_turbo_boost() {
                self.turbo_enabled = true;
                serial_println!("CPUFreq: Intel Turbo Boost / AMD Turbo Core supported");
            }
            
            if features.has_energy_bias_pref() {
                self.energy_perf_bias = self.read_energy_perf_bias();
                serial_println!("CPUFreq: Energy performance bias supported");
            }
        }
        
        // Get processor frequency information
        if let Some(freq_info) = cpuid.get_processor_frequency_info() {
            self.base_frequency = freq_info.processor_base_frequency() as u32;
            self.max_frequency = freq_info.processor_max_frequency() as u32;
            
            if self.base_frequency > 0 {
                serial_println!("CPUFreq: Base frequency: {} MHz, Max frequency: {} MHz",
                               self.base_frequency, self.max_frequency);
            }
        }
        
        // Check for Intel SpeedStep or AMD Cool'n'Quiet
        let misc_enable = self.read_msr(MSR_IA32_MISC_ENABLE)?;
        if misc_enable & (1 << 16) != 0 {
            self.speedstep_enabled = true;
            serial_println!("CPUFreq: Enhanced SpeedStep enabled");
        }
        
        Ok(())
    }

    fn enumerate_pstates(&mut self) -> Result<(), &'static str> {
        // Read platform info for P-state limits
        let platform_info = self.read_msr(MSR_PLATFORM_INFO)?;
        let max_ratio = ((platform_info >> 8) & 0xFF) as u32;
        let min_ratio = ((platform_info >> 40) & 0xFF) as u32;
        
        // Calculate frequencies based on bus speed (usually 100MHz)
        let bus_speed = 100; // MHz
        
        // Create P-states from min to max ratio
        for ratio in min_ratio..=max_ratio {
            let frequency = ratio * bus_speed;
            
            // Estimate power consumption (simplified model)
            let voltage = 800 + (ratio - min_ratio) * 20; // mV
            let power = (voltage * voltage * frequency) / 100000; // Simplified power model
            
            self.p_states.push(PState {
                frequency_mhz: frequency,
                voltage_mv: voltage,
                power_mw: power,
                control_value: ratio as u16,
            });
        }
        
        if !self.p_states.is_empty() {
            self.min_frequency = self.p_states[0].frequency_mhz;
            self.max_frequency = self.p_states[self.p_states.len() - 1].frequency_mhz;
        }
        
        Ok(())
    }

    fn enumerate_cstates(&mut self) -> Result<(), &'static str> {
        // C0 - Active state
        self.c_states.push(CState {
            state_type: CStateType::C0,
            latency_us: 0,
            power_mw: 35000, // Typical active power
            target_residency_us: 0,
        });
        
        // C1 - Halt state
        self.c_states.push(CState {
            state_type: CStateType::C1,
            latency_us: 2,
            power_mw: 20000,
            target_residency_us: 2,
        });
        
        // C1E - Enhanced halt
        self.c_states.push(CState {
            state_type: CStateType::C1E,
            latency_us: 10,
            power_mw: 15000,
            target_residency_us: 20,
        });
        
        // C3 - Deep sleep
        self.c_states.push(CState {
            state_type: CStateType::C3,
            latency_us: 100,
            power_mw: 5000,
            target_residency_us: 200,
        });
        
        // C6 - Deep power down
        self.c_states.push(CState {
            state_type: CStateType::C6,
            latency_us: 150,
            power_mw: 2000,
            target_residency_us: 300,
        });
        
        Ok(())
    }

    fn enable_dynamic_frequency_scaling(&mut self) -> Result<(), &'static str> {
        if self.speedstep_enabled {
            // Enable dynamic frequency transitions
            let mut misc_enable = self.read_msr(MSR_IA32_MISC_ENABLE)?;
            misc_enable |= 1 << 16; // Enable Enhanced SpeedStep
            self.write_msr(MSR_IA32_MISC_ENABLE, misc_enable)?;
            
            serial_println!("CPUFreq: Dynamic frequency scaling enabled");
        }
        
        Ok(())
    }

    pub fn set_pstate(&mut self, pstate_index: usize) -> Result<(), &'static str> {
        if pstate_index >= self.p_states.len() {
            return Err("Invalid P-state index");
        }
        
        let pstate = &self.p_states[pstate_index];
        let control_value = (pstate.control_value as u64) << 8;
        
        // Write to performance control MSR
        self.write_msr(MSR_IA32_PERF_CTL, control_value)?;
        
        self.current_pstate = pstate_index;
        
        Ok(())
    }

    pub fn get_current_frequency(&self) -> u32 {
        // Read current performance status
        if let Ok(perf_status) = self.read_msr(MSR_IA32_PERF_STATUS) {
            let ratio = ((perf_status >> 8) & 0xFF) as u32;
            ratio * 100 // Bus speed is 100MHz
        } else {
            0
        }
    }

    pub fn set_governor(&mut self, governor: CpuGovernor) -> Result<(), &'static str> {
        self.governor = governor;
        self.apply_governor_policy()?;
        Ok(())
    }

    fn apply_governor_policy(&mut self) -> Result<(), &'static str> {
        match self.governor {
            CpuGovernor::Performance => {
                // Set to maximum P-state
                self.set_pstate(self.p_states.len() - 1)?;
                self.set_energy_perf_bias(0)?; // Maximum performance
            },
            CpuGovernor::PowerSave => {
                // Set to minimum P-state
                self.set_pstate(0)?;
                self.set_energy_perf_bias(15)?; // Maximum power saving
            },
            CpuGovernor::OnDemand => {
                // Dynamic scaling based on load
                self.set_energy_perf_bias(6)?; // Balanced
            },
            CpuGovernor::Conservative => {
                // Gradual frequency changes
                self.set_energy_perf_bias(10)?; // Favor power saving
            },
            CpuGovernor::Schedutil => {
                // Scheduler-based frequency selection
                self.set_energy_perf_bias(6)?;
            },
        }
        Ok(())
    }

    pub fn set_turbo_boost(&mut self, enable: bool) -> Result<(), &'static str> {
        let mut misc_enable = self.read_msr(MSR_IA32_MISC_ENABLE)?;
        
        if enable {
            misc_enable &= !(1 << 38); // Clear turbo disable bit
        } else {
            misc_enable |= 1 << 38; // Set turbo disable bit
        }
        
        self.write_msr(MSR_IA32_MISC_ENABLE, misc_enable)?;
        self.turbo_enabled = enable;
        
        serial_println!("CPUFreq: Turbo Boost {}", if enable { "enabled" } else { "disabled" });
        Ok(())
    }

    pub fn enter_cstate(&mut self, cstate: CStateType) -> Result<(), &'static str> {
        match cstate {
            CStateType::C0 => {
                // Active state, nothing to do
            },
            CStateType::C1 => {
                // Halt instruction
                x86_64::instructions::hlt();
            },
            CStateType::C1E => {
                // Enhanced halt with frequency reduction
                self.set_pstate(0)?;
                x86_64::instructions::hlt();
            },
            CStateType::C3 | CStateType::C6 => {
                // Deep sleep states require MWAIT instruction
                self.mwait_for_cstate(cstate)?;
            },
            _ => {
                // Other C-states
                self.mwait_for_cstate(cstate)?;
            }
        }
        
        self.current_cstate = cstate;
        Ok(())
    }

    fn mwait_for_cstate(&self, cstate: CStateType) -> Result<(), &'static str> {
        // MWAIT hints for different C-states
        let hint = match cstate {
            CStateType::C1 => 0x00,
            CStateType::C1E => 0x01,
            CStateType::C3 => 0x10,
            CStateType::C6 => 0x20,
            CStateType::C7 => 0x30,
            CStateType::C10 => 0x40,
            _ => 0x00,
        };
        
        unsafe {
            // MONITOR setup
            let monitor_addr = &self as *const _ as usize;
            core::arch::asm!(
                "monitor",
                in("rax") monitor_addr,
                in("ecx") 0u32,
                in("edx") 0u32,
            );
            
            // MWAIT with hint
            core::arch::asm!(
                "mwait",
                in("eax") hint,
                in("ecx") 0u32,
            );
        }
        
        Ok(())
    }

    pub fn get_power_consumption(&self) -> u32 {
        // Read RAPL MSRs for actual power consumption
        if let Ok(power_unit) = self.read_msr(MSR_RAPL_POWER_UNIT) {
            let power_units = 1.0 / (1 << ((power_unit & 0x1F) as u32)) as f32;
            
            if let Ok(energy_status) = self.read_msr(MSR_PKG_ENERGY_STATUS) {
                let energy = (energy_status & 0xFFFFFFFF) as f32 * power_units;
                // Convert to milliwatts (simplified)
                return (energy * 1000.0) as u32;
            }
        }
        
        // Fallback to P-state estimate
        if self.current_pstate < self.p_states.len() {
            self.p_states[self.current_pstate].power_mw
        } else {
            0
        }
    }

    fn read_msr(&self, msr: u32) -> Result<u64, &'static str> {
        unsafe {
            Ok(Msr::new(msr).read())
        }
    }

    fn write_msr(&self, msr: u32, value: u64) -> Result<(), &'static str> {
        unsafe {
            Msr::new(msr).write(value);
        }
        Ok(())
    }

    fn read_energy_perf_bias(&self) -> u8 {
        if let Ok(bias) = self.read_msr(MSR_IA32_ENERGY_PERF_BIAS) {
            (bias & 0x0F) as u8
        } else {
            6 // Default balanced
        }
    }

    fn set_energy_perf_bias(&self, bias: u8) -> Result<(), &'static str> {
        if bias > 15 {
            return Err("Invalid energy performance bias");
        }
        
        self.write_msr(MSR_IA32_ENERGY_PERF_BIAS, bias as u64)?;
        Ok(())
    }
}

lazy_static! {
    static ref CPU_FREQ: Mutex<CpuFrequencyScaling> = Mutex::new(CpuFrequencyScaling::new());
}

pub fn init() -> Result<(), &'static str> {
    CPU_FREQ.lock().init()
}

pub fn set_governor(governor: CpuGovernor) -> Result<(), &'static str> {
    CPU_FREQ.lock().set_governor(governor)
}

pub fn set_frequency_limits(min_mhz: u32, max_mhz: u32) -> Result<(), &'static str> {
    let mut cpufreq = CPU_FREQ.lock();
    
    // Find P-states within the limits
    let mut min_pstate = 0;
    let mut max_pstate = cpufreq.p_states.len() - 1;
    
    for (i, pstate) in cpufreq.p_states.iter().enumerate() {
        if pstate.frequency_mhz >= min_mhz && min_pstate == 0 {
            min_pstate = i;
        }
        if pstate.frequency_mhz <= max_mhz {
            max_pstate = i;
        }
    }
    
    cpufreq.min_frequency = min_mhz;
    cpufreq.max_frequency = max_mhz;
    
    Ok(())
}

pub fn set_turbo_boost(enable: bool) -> Result<(), &'static str> {
    CPU_FREQ.lock().set_turbo_boost(enable)
}

pub fn get_cpu_power_consumption() -> u32 {
    CPU_FREQ.lock().get_power_consumption()
}

pub fn get_current_frequency() -> u32 {
    CPU_FREQ.lock().get_current_frequency()
}

pub fn enter_idle_state(cstate: CStateType) -> Result<(), &'static str> {
    CPU_FREQ.lock().enter_cstate(cstate)
}