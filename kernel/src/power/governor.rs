use alloc::collections::VecDeque;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::serial_println;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CpuGovernor {
    Performance,
    PowerSave,
    OnDemand,
    Conservative,
    Schedutil,
}

#[derive(Debug)]
pub struct GovernorState {
    governor: CpuGovernor,
    cpu_load: VecDeque<u32>,
    last_update: u64,
    up_threshold: u32,
    down_threshold: u32,
    sampling_rate_ms: u32,
    ignore_nice_load: bool,
    freq_step: u32,
}

impl GovernorState {
    pub fn new(governor: CpuGovernor) -> Self {
        let (up_threshold, down_threshold, sampling_rate) = match governor {
            CpuGovernor::OnDemand => (80, 20, 10),
            CpuGovernor::Conservative => (80, 40, 20),
            _ => (95, 5, 100),
        };
        
        Self {
            governor,
            cpu_load: VecDeque::with_capacity(10),
            last_update: 0,
            up_threshold,
            down_threshold,
            sampling_rate_ms: sampling_rate,
            ignore_nice_load: true,
            freq_step: 5, // 5% frequency step for conservative
        }
    }
    
    pub fn update_load(&mut self, current_load: u32) {
        self.cpu_load.push_back(current_load);
        if self.cpu_load.len() > 10 {
            self.cpu_load.pop_front();
        }
    }
    
    pub fn should_scale_up(&self) -> bool {
        match self.governor {
            CpuGovernor::Performance => true,
            CpuGovernor::PowerSave => false,
            CpuGovernor::OnDemand => {
                self.get_average_load() > self.up_threshold
            },
            CpuGovernor::Conservative => {
                self.get_average_load() > self.up_threshold &&
                self.get_load_trend() > 0
            },
            CpuGovernor::Schedutil => {
                // Scheduler-driven decisions
                self.get_average_load() > 70
            },
        }
    }
    
    pub fn should_scale_down(&self) -> bool {
        match self.governor {
            CpuGovernor::Performance => false,
            CpuGovernor::PowerSave => true,
            CpuGovernor::OnDemand => {
                self.get_average_load() < self.down_threshold
            },
            CpuGovernor::Conservative => {
                self.get_average_load() < self.down_threshold &&
                self.get_load_trend() < 0
            },
            CpuGovernor::Schedutil => {
                self.get_average_load() < 30
            },
        }
    }
    
    fn get_average_load(&self) -> u32 {
        if self.cpu_load.is_empty() {
            return 0;
        }
        
        let sum: u32 = self.cpu_load.iter().sum();
        sum / self.cpu_load.len() as u32
    }
    
    fn get_load_trend(&self) -> i32 {
        if self.cpu_load.len() < 2 {
            return 0;
        }
        
        let recent = self.cpu_load.back().unwrap_or(&0);
        let previous = self.cpu_load.get(self.cpu_load.len() - 2).unwrap_or(&0);
        
        (*recent as i32) - (*previous as i32)
    }
    
    pub fn calculate_target_frequency(&self, current_freq: u32, min_freq: u32, max_freq: u32) -> u32 {
        match self.governor {
            CpuGovernor::Performance => max_freq,
            CpuGovernor::PowerSave => min_freq,
            CpuGovernor::OnDemand => {
                if self.should_scale_up() {
                    max_freq
                } else if self.should_scale_down() {
                    min_freq
                } else {
                    current_freq
                }
            },
            CpuGovernor::Conservative => {
                let step = ((max_freq - min_freq) * self.freq_step) / 100;
                
                if self.should_scale_up() {
                    (current_freq + step).min(max_freq)
                } else if self.should_scale_down() {
                    (current_freq.saturating_sub(step)).max(min_freq)
                } else {
                    current_freq
                }
            },
            CpuGovernor::Schedutil => {
                // Linear scaling based on load
                let load = self.get_average_load();
                min_freq + ((max_freq - min_freq) * load) / 100
            },
        }
    }
    
    pub fn set_tuning_parameters(&mut self, up_threshold: u32, down_threshold: u32, sampling_rate_ms: u32) {
        self.up_threshold = up_threshold.min(100);
        self.down_threshold = down_threshold.min(up_threshold);
        self.sampling_rate_ms = sampling_rate_ms.max(1);
        
        serial_println!("Governor: Updated tuning - up:{}, down:{}, rate:{}ms",
                       self.up_threshold, self.down_threshold, self.sampling_rate_ms);
    }
}

#[derive(Debug)]
pub struct GovernorManager {
    states: Vec<GovernorState>,
    active_governor: CpuGovernor,
    boost_active: bool,
    boost_duration_ms: u32,
    boost_start_time: Option<u64>,
}

impl GovernorManager {
    pub fn new() -> Self {
        Self {
            states: Vec::new(),
            active_governor: CpuGovernor::OnDemand,
            boost_active: false,
            boost_duration_ms: 0,
            boost_start_time: None,
        }
    }
    
    pub fn init(&mut self, num_cpus: usize) -> Result<(), &'static str> {
        for _ in 0..num_cpus {
            self.states.push(GovernorState::new(self.active_governor));
        }
        
        serial_println!("Governor: Initialized for {} CPUs with {:?} governor",
                       num_cpus, self.active_governor);
        Ok(())
    }
    
    pub fn set_governor(&mut self, governor: CpuGovernor) -> Result<(), &'static str> {
        self.active_governor = governor;
        
        for state in &mut self.states {
            *state = GovernorState::new(governor);
        }
        
        serial_println!("Governor: Switched to {:?}", governor);
        Ok(())
    }
    
    pub fn update_cpu_load(&mut self, cpu_id: usize, load: u32) {
        if cpu_id < self.states.len() {
            self.states[cpu_id].update_load(load);
        }
    }
    
    pub fn get_target_frequency(&self, cpu_id: usize, current: u32, min: u32, max: u32) -> u32 {
        if self.boost_active {
            return max;
        }
        
        if cpu_id < self.states.len() {
            self.states[cpu_id].calculate_target_frequency(current, min, max)
        } else {
            current
        }
    }
    
    pub fn request_boost(&mut self, duration_ms: u32) {
        self.boost_active = true;
        self.boost_duration_ms = duration_ms;
        self.boost_start_time = Some(Self::get_current_time());
        
        serial_println!("Governor: Performance boost activated for {}ms", duration_ms);
    }
    
    pub fn cancel_boost(&mut self) {
        self.boost_active = false;
        self.boost_start_time = None;
        
        serial_println!("Governor: Performance boost cancelled");
    }
    
    pub fn update(&mut self) {
        // Check if boost should expire
        if self.boost_active {
            if let Some(start_time) = self.boost_start_time {
                let elapsed = Self::get_current_time() - start_time;
                if elapsed >= self.boost_duration_ms as u64 {
                    self.cancel_boost();
                }
            }
        }
    }
    
    fn get_current_time() -> u64 {
        // This would use a real timer
        0
    }
    
    pub fn get_governor_stats(&self, cpu_id: usize) -> Option<GovernorStats> {
        if cpu_id < self.states.len() {
            let state = &self.states[cpu_id];
            Some(GovernorStats {
                governor: state.governor,
                average_load: state.get_average_load(),
                up_threshold: state.up_threshold,
                down_threshold: state.down_threshold,
                sampling_rate_ms: state.sampling_rate_ms,
            })
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct GovernorStats {
    pub governor: CpuGovernor,
    pub average_load: u32,
    pub up_threshold: u32,
    pub down_threshold: u32,
    pub sampling_rate_ms: u32,
}

lazy_static! {
    static ref GOVERNOR_MGR: Mutex<GovernorManager> = Mutex::new(GovernorManager::new());
}

pub fn init_governors(num_cpus: usize) -> Result<(), &'static str> {
    GOVERNOR_MGR.lock().init(num_cpus)
}

pub fn set_active_governor(governor: CpuGovernor) -> Result<(), &'static str> {
    GOVERNOR_MGR.lock().set_governor(governor)
}

pub fn update_cpu_load(cpu_id: usize, load: u32) {
    GOVERNOR_MGR.lock().update_cpu_load(cpu_id, load);
}

pub fn get_target_frequency(cpu_id: usize, current: u32, min: u32, max: u32) -> u32 {
    GOVERNOR_MGR.lock().get_target_frequency(cpu_id, current, min, max)
}

pub fn request_performance_boost(duration_ms: u32) {
    GOVERNOR_MGR.lock().request_boost(duration_ms);
}

pub fn cancel_performance_boost() {
    GOVERNOR_MGR.lock().cancel_boost();
}

pub fn update_governor_state() {
    GOVERNOR_MGR.lock().update();
}