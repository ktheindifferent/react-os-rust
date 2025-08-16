use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use spin::Mutex;
use crate::serial_println;

use super::ContainerError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CgroupController {
    Memory,
    Cpu,
    CpuSet,
    BlockIo,
    NetworkCls,
    NetworkPrio,
    Devices,
    Freezer,
    Pids,
}

pub struct Cgroup {
    name: String,
    controller: CgroupController,
    parent: Option<String>,
    children: Mutex<Vec<String>>,
    processes: Mutex<Vec<u32>>,
    settings: Mutex<CgroupSettings>,
    stats: CgroupStats,
}

#[derive(Debug, Clone)]
struct CgroupSettings {
    memory_limit: Option<u64>,
    memory_soft_limit: Option<u64>,
    memory_swap_limit: Option<u64>,
    cpu_shares: u32,
    cpu_quota: Option<u32>,
    cpu_period: u32,
    cpuset_cpus: Vec<u32>,
    cpuset_mems: Vec<u32>,
    blkio_weight: u32,
    blkio_throttle: Vec<BlkioThrottle>,
    net_cls_classid: u32,
    net_prio_map: Vec<NetPrioMap>,
    devices_allow: Vec<DeviceRule>,
    devices_deny: Vec<DeviceRule>,
    pids_limit: Option<u32>,
    frozen: bool,
}

#[derive(Debug, Clone)]
struct BlkioThrottle {
    major: u32,
    minor: u32,
    read_bps: Option<u64>,
    write_bps: Option<u64>,
    read_iops: Option<u64>,
    write_iops: Option<u64>,
}

#[derive(Debug, Clone)]
struct NetPrioMap {
    interface: String,
    priority: u32,
}

#[derive(Debug, Clone)]
struct DeviceRule {
    device_type: char,
    major: Option<u32>,
    minor: Option<u32>,
    access: String,
}

#[derive(Debug)]
pub struct CgroupStats {
    memory_usage: AtomicU64,
    memory_max_usage: AtomicU64,
    memory_cache: AtomicU64,
    memory_rss: AtomicU64,
    memory_swap: AtomicU64,
    memory_page_faults: AtomicU64,
    cpu_usage: AtomicU64,
    cpu_user_time: AtomicU64,
    cpu_system_time: AtomicU64,
    cpu_throttled_periods: AtomicU64,
    cpu_throttled_time: AtomicU64,
    blkio_read_bytes: AtomicU64,
    blkio_write_bytes: AtomicU64,
    blkio_read_ops: AtomicU64,
    blkio_write_ops: AtomicU64,
    pids_current: AtomicU32,
    pids_max: AtomicU32,
}

impl Cgroup {
    pub fn new(controller_type: &str, name: &str) -> Result<Self, ContainerError> {
        let controller = match controller_type {
            "memory" => CgroupController::Memory,
            "cpu" => CgroupController::Cpu,
            "cpuset" => CgroupController::CpuSet,
            "blkio" => CgroupController::BlockIo,
            "net_cls" => CgroupController::NetworkCls,
            "net_prio" => CgroupController::NetworkPrio,
            "devices" => CgroupController::Devices,
            "freezer" => CgroupController::Freezer,
            "pids" => CgroupController::Pids,
            _ => return Err(ContainerError::CgroupError(format!("Unknown controller: {}", controller_type))),
        };
        
        Ok(Self {
            name: name.to_string(),
            controller,
            parent: None,
            children: Mutex::new(Vec::new()),
            processes: Mutex::new(Vec::new()),
            settings: Mutex::new(CgroupSettings::default()),
            stats: CgroupStats::new(),
        })
    }
    
    pub fn set_memory_limit(&mut self, limit_bytes: u64) -> Result<(), ContainerError> {
        if self.controller != CgroupController::Memory {
            return Err(ContainerError::CgroupError("Not a memory cgroup".into()));
        }
        
        self.settings.lock().memory_limit = Some(limit_bytes);
        serial_println!("Set memory limit for cgroup {} to {} bytes", self.name, limit_bytes);
        Ok(())
    }
    
    pub fn set_memory_soft_limit(&mut self, limit_bytes: u64) -> Result<(), ContainerError> {
        if self.controller != CgroupController::Memory {
            return Err(ContainerError::CgroupError("Not a memory cgroup".into()));
        }
        
        self.settings.lock().memory_soft_limit = Some(limit_bytes);
        Ok(())
    }
    
    pub fn set_swap_limit(&mut self, limit_bytes: u64) -> Result<(), ContainerError> {
        if self.controller != CgroupController::Memory {
            return Err(ContainerError::CgroupError("Not a memory cgroup".into()));
        }
        
        self.settings.lock().memory_swap_limit = Some(limit_bytes);
        Ok(())
    }
    
    pub fn set_cpu_shares(&mut self, shares: u32) -> Result<(), ContainerError> {
        if self.controller != CgroupController::Cpu {
            return Err(ContainerError::CgroupError("Not a CPU cgroup".into()));
        }
        
        self.settings.lock().cpu_shares = shares;
        serial_println!("Set CPU shares for cgroup {} to {}", self.name, shares);
        Ok(())
    }
    
    pub fn set_cpu_quota(&mut self, quota_us: u32) -> Result<(), ContainerError> {
        if self.controller != CgroupController::Cpu {
            return Err(ContainerError::CgroupError("Not a CPU cgroup".into()));
        }
        
        self.settings.lock().cpu_quota = Some(quota_us);
        serial_println!("Set CPU quota for cgroup {} to {} microseconds", self.name, quota_us);
        Ok(())
    }
    
    pub fn set_cpu_period(&mut self, period_us: u32) -> Result<(), ContainerError> {
        if self.controller != CgroupController::Cpu {
            return Err(ContainerError::CgroupError("Not a CPU cgroup".into()));
        }
        
        self.settings.lock().cpu_period = period_us;
        Ok(())
    }
    
    pub fn set_cpuset(&mut self, cpus: Vec<u32>) -> Result<(), ContainerError> {
        if self.controller != CgroupController::CpuSet {
            return Err(ContainerError::CgroupError("Not a cpuset cgroup".into()));
        }
        
        self.settings.lock().cpuset_cpus = cpus;
        Ok(())
    }
    
    pub fn set_blkio_weight(&mut self, weight: u32) -> Result<(), ContainerError> {
        if self.controller != CgroupController::BlockIo {
            return Err(ContainerError::CgroupError("Not a blkio cgroup".into()));
        }
        
        if weight < 10 || weight > 1000 {
            return Err(ContainerError::CgroupError("Invalid blkio weight (10-1000)".into()));
        }
        
        self.settings.lock().blkio_weight = weight;
        Ok(())
    }
    
    pub fn add_blkio_throttle(&mut self, major: u32, minor: u32, read_bps: Option<u64>, write_bps: Option<u64>) -> Result<(), ContainerError> {
        if self.controller != CgroupController::BlockIo {
            return Err(ContainerError::CgroupError("Not a blkio cgroup".into()));
        }
        
        self.settings.lock().blkio_throttle.push(BlkioThrottle {
            major,
            minor,
            read_bps,
            write_bps,
            read_iops: None,
            write_iops: None,
        });
        Ok(())
    }
    
    pub fn set_pids_limit(&mut self, limit: u32) -> Result<(), ContainerError> {
        if self.controller != CgroupController::Pids {
            return Err(ContainerError::CgroupError("Not a pids cgroup".into()));
        }
        
        self.settings.lock().pids_limit = Some(limit);
        Ok(())
    }
    
    pub fn freeze(&mut self) -> Result<(), ContainerError> {
        if self.controller != CgroupController::Freezer {
            return Err(ContainerError::CgroupError("Not a freezer cgroup".into()));
        }
        
        self.settings.lock().frozen = true;
        serial_println!("Freezing cgroup {}", self.name);
        
        for pid in self.processes.lock().iter() {
            Self::freeze_process(*pid)?;
        }
        
        Ok(())
    }
    
    pub fn thaw(&mut self) -> Result<(), ContainerError> {
        if self.controller != CgroupController::Freezer {
            return Err(ContainerError::CgroupError("Not a freezer cgroup".into()));
        }
        
        self.settings.lock().frozen = false;
        serial_println!("Thawing cgroup {}", self.name);
        
        for pid in self.processes.lock().iter() {
            Self::thaw_process(*pid)?;
        }
        
        Ok(())
    }
    
    pub fn add_device_allow(&mut self, device_type: char, major: Option<u32>, minor: Option<u32>, access: &str) -> Result<(), ContainerError> {
        if self.controller != CgroupController::Devices {
            return Err(ContainerError::CgroupError("Not a devices cgroup".into()));
        }
        
        self.settings.lock().devices_allow.push(DeviceRule {
            device_type,
            major,
            minor,
            access: access.to_string(),
        });
        Ok(())
    }
    
    pub fn add_device_deny(&mut self, device_type: char, major: Option<u32>, minor: Option<u32>, access: &str) -> Result<(), ContainerError> {
        if self.controller != CgroupController::Devices {
            return Err(ContainerError::CgroupError("Not a devices cgroup".into()));
        }
        
        self.settings.lock().devices_deny.push(DeviceRule {
            device_type,
            major,
            minor,
            access: access.to_string(),
        });
        Ok(())
    }
    
    pub fn add_process(&mut self, pid: u32) -> Result<(), ContainerError> {
        let mut processes = self.processes.lock();
        if !processes.contains(&pid) {
            processes.push(pid);
            self.stats.pids_current.fetch_add(1, Ordering::Relaxed);
            
            if let Some(limit) = self.settings.lock().pids_limit {
                if processes.len() > limit as usize {
                    processes.pop();
                    self.stats.pids_current.fetch_sub(1, Ordering::Relaxed);
                    return Err(ContainerError::ResourceLimitExceeded);
                }
            }
            
            serial_println!("Added process {} to cgroup {}", pid, self.name);
        }
        Ok(())
    }
    
    pub fn remove_process(&mut self, pid: u32) -> Result<(), ContainerError> {
        let mut processes = self.processes.lock();
        if let Some(pos) = processes.iter().position(|&p| p == pid) {
            processes.remove(pos);
            self.stats.pids_current.fetch_sub(1, Ordering::Relaxed);
            serial_println!("Removed process {} from cgroup {}", pid, self.name);
        }
        Ok(())
    }
    
    pub fn get_processes(&self) -> Vec<u32> {
        self.processes.lock().clone()
    }
    
    pub fn check_memory_usage(&self) -> bool {
        if let Some(limit) = self.settings.lock().memory_limit {
            let usage = self.stats.memory_usage.load(Ordering::Relaxed);
            if usage > limit {
                return false;
            }
        }
        true
    }
    
    pub fn update_memory_stats(&self, usage: u64, cache: u64, rss: u64) {
        self.stats.memory_usage.store(usage, Ordering::Relaxed);
        self.stats.memory_cache.store(cache, Ordering::Relaxed);
        self.stats.memory_rss.store(rss, Ordering::Relaxed);
        
        let max_usage = self.stats.memory_max_usage.load(Ordering::Relaxed);
        if usage > max_usage {
            self.stats.memory_max_usage.store(usage, Ordering::Relaxed);
        }
    }
    
    pub fn update_cpu_stats(&self, usage: u64, user_time: u64, system_time: u64) {
        self.stats.cpu_usage.fetch_add(usage, Ordering::Relaxed);
        self.stats.cpu_user_time.fetch_add(user_time, Ordering::Relaxed);
        self.stats.cpu_system_time.fetch_add(system_time, Ordering::Relaxed);
    }
    
    pub fn update_blkio_stats(&self, read_bytes: u64, write_bytes: u64, read_ops: u64, write_ops: u64) {
        self.stats.blkio_read_bytes.fetch_add(read_bytes, Ordering::Relaxed);
        self.stats.blkio_write_bytes.fetch_add(write_bytes, Ordering::Relaxed);
        self.stats.blkio_read_ops.fetch_add(read_ops, Ordering::Relaxed);
        self.stats.blkio_write_ops.fetch_add(write_ops, Ordering::Relaxed);
    }
    
    fn freeze_process(pid: u32) -> Result<(), ContainerError> {
        serial_println!("Freezing process {}", pid);
        Ok(())
    }
    
    fn thaw_process(pid: u32) -> Result<(), ContainerError> {
        serial_println!("Thawing process {}", pid);
        Ok(())
    }
    
    pub fn get_stats(&self) -> &CgroupStats {
        &self.stats
    }
    
    pub fn get_name(&self) -> &str {
        &self.name
    }
    
    pub fn get_controller(&self) -> CgroupController {
        self.controller
    }
}

impl Default for CgroupSettings {
    fn default() -> Self {
        Self {
            memory_limit: None,
            memory_soft_limit: None,
            memory_swap_limit: None,
            cpu_shares: 1024,
            cpu_quota: None,
            cpu_period: 100000,
            cpuset_cpus: Vec::new(),
            cpuset_mems: Vec::new(),
            blkio_weight: 500,
            blkio_throttle: Vec::new(),
            net_cls_classid: 0,
            net_prio_map: Vec::new(),
            devices_allow: Vec::new(),
            devices_deny: Vec::new(),
            pids_limit: None,
            frozen: false,
        }
    }
}

impl CgroupStats {
    fn new() -> Self {
        Self {
            memory_usage: AtomicU64::new(0),
            memory_max_usage: AtomicU64::new(0),
            memory_cache: AtomicU64::new(0),
            memory_rss: AtomicU64::new(0),
            memory_swap: AtomicU64::new(0),
            memory_page_faults: AtomicU64::new(0),
            cpu_usage: AtomicU64::new(0),
            cpu_user_time: AtomicU64::new(0),
            cpu_system_time: AtomicU64::new(0),
            cpu_throttled_periods: AtomicU64::new(0),
            cpu_throttled_time: AtomicU64::new(0),
            blkio_read_bytes: AtomicU64::new(0),
            blkio_write_bytes: AtomicU64::new(0),
            blkio_read_ops: AtomicU64::new(0),
            blkio_write_ops: AtomicU64::new(0),
            pids_current: AtomicU32::new(0),
            pids_max: AtomicU32::new(0),
        }
    }
}

pub struct CgroupManager {
    cgroups: Mutex<BTreeMap<String, Cgroup>>,
}

impl CgroupManager {
    pub fn new() -> Self {
        Self {
            cgroups: Mutex::new(BTreeMap::new()),
        }
    }
    
    pub fn create_cgroup(&self, controller: &str, name: &str) -> Result<(), ContainerError> {
        let cgroup = Cgroup::new(controller, name)?;
        self.cgroups.lock().insert(name.to_string(), cgroup);
        Ok(())
    }
    
    pub fn delete_cgroup(&self, name: &str) -> Result<(), ContainerError> {
        if let Some(cgroup) = self.cgroups.lock().get(name) {
            if !cgroup.processes.lock().is_empty() {
                return Err(ContainerError::CgroupError("Cgroup has active processes".into()));
            }
        }
        
        self.cgroups.lock().remove(name);
        Ok(())
    }
    
    pub fn get_cgroup(&self, name: &str) -> Option<Cgroup> {
        self.cgroups.lock().get(name).cloned()
    }
}