#![no_std]

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use spin::Mutex;

#[derive(Debug, Clone)]
pub struct ResourceLimit {
    pub max_value: u64,
    pub current_value: u64,
    pub soft_limit: u64,
    pub hard_limit: u64,
}

#[derive(Debug, Clone)]
pub struct ResourceQuota {
    pub user_id: u32,
    pub group_id: u32,
    pub cpu_quota_ms: u64,
    pub memory_quota_bytes: u64,
    pub disk_quota_bytes: u64,
    pub network_quota_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub timestamp: u64,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub disk_io_bytes: u64,
    pub network_io_bytes: u64,
}

pub struct ResourceMonitor {
    tracking_enabled: AtomicBool,
    limits: Mutex<BTreeMap<String, ResourceLimit>>,
    quotas: Mutex<BTreeMap<u32, ResourceQuota>>,
    usage_history: Mutex<BTreeMap<u32, Vec<ResourceUsage>>>,
    thresholds: Mutex<BTreeMap<String, ResourceThreshold>>,
    anomaly_detector: AnomalyDetector,
}

#[derive(Debug, Clone)]
pub struct ResourceThreshold {
    pub resource_name: String,
    pub warning_level: u64,
    pub critical_level: u64,
    pub action: ThresholdAction,
}

#[derive(Debug, Clone, Copy)]
pub enum ThresholdAction {
    LogOnly,
    Alert,
    Throttle,
    Kill,
}

pub struct AnomalyDetector {
    enabled: AtomicBool,
    baseline_cpu: AtomicU64,
    baseline_memory: AtomicU64,
    baseline_disk: AtomicU64,
    baseline_network: AtomicU64,
    deviation_threshold: AtomicU64,
}

static RESOURCE_MONITOR: ResourceMonitor = ResourceMonitor {
    tracking_enabled: AtomicBool::new(true),
    limits: Mutex::new(BTreeMap::new()),
    quotas: Mutex::new(BTreeMap::new()),
    usage_history: Mutex::new(BTreeMap::new()),
    thresholds: Mutex::new(BTreeMap::new()),
    anomaly_detector: AnomalyDetector {
        enabled: AtomicBool::new(true),
        baseline_cpu: AtomicU64::new(50),
        baseline_memory: AtomicU64::new(1024 * 1024 * 512), // 512MB
        baseline_disk: AtomicU64::new(1024 * 1024 * 10), // 10MB/s
        baseline_network: AtomicU64::new(1024 * 1024), // 1MB/s
        deviation_threshold: AtomicU64::new(200), // 200% deviation
    },
};

const MAX_HISTORY_SIZE: usize = 1000;

pub fn init() {
    // Initialize default resource limits
    set_resource_limit("max_processes", 1000, 500, 1000);
    set_resource_limit("max_threads", 10000, 5000, 10000);
    set_resource_limit("max_open_files", 10000, 5000, 10000);
    set_resource_limit("max_memory_mb", 8192, 4096, 8192);
    
    // Initialize default thresholds
    set_threshold(
        "cpu_usage",
        80,
        95,
        ThresholdAction::Alert,
    );
    set_threshold(
        "memory_usage",
        85,
        95,
        ThresholdAction::Alert,
    );
    set_threshold(
        "disk_usage",
        90,
        95,
        ThresholdAction::LogOnly,
    );
}

pub fn set_resource_limit(name: &str, max_value: u64, soft_limit: u64, hard_limit: u64) {
    let limit = ResourceLimit {
        max_value,
        current_value: 0,
        soft_limit,
        hard_limit,
    };
    
    RESOURCE_MONITOR.limits.lock().insert(name.to_string(), limit);
}

pub fn check_resource_limit(name: &str, requested: u64) -> Result<(), &'static str> {
    let mut limits = RESOURCE_MONITOR.limits.lock();
    
    if let Some(limit) = limits.get_mut(name) {
        if limit.current_value + requested > limit.hard_limit {
            return Err("Resource limit exceeded");
        }
        
        if limit.current_value + requested > limit.soft_limit {
            crate::log_warn!("RESOURCES", "Approaching resource limit for {}", name);
        }
        
        limit.current_value += requested;
        Ok(())
    } else {
        Ok(()) // No limit set
    }
}

pub fn release_resource(name: &str, amount: u64) {
    if let Some(mut limits) = RESOURCE_MONITOR.limits.try_lock() {
        if let Some(limit) = limits.get_mut(name) {
            limit.current_value = limit.current_value.saturating_sub(amount);
        }
    }
}

pub fn set_user_quota(user_id: u32, quota: ResourceQuota) {
    RESOURCE_MONITOR.quotas.lock().insert(user_id, quota);
}

pub fn check_user_quota(user_id: u32, resource: &str, amount: u64) -> Result<(), &'static str> {
    let quotas = RESOURCE_MONITOR.quotas.lock();
    
    if let Some(quota) = quotas.get(&user_id) {
        match resource {
            "cpu" if amount > quota.cpu_quota_ms => Err("CPU quota exceeded"),
            "memory" if amount > quota.memory_quota_bytes => Err("Memory quota exceeded"),
            "disk" if amount > quota.disk_quota_bytes => Err("Disk quota exceeded"),
            "network" if amount > quota.network_quota_bytes => Err("Network quota exceeded"),
            _ => Ok(()),
        }
    } else {
        Ok(())
    }
}

pub fn track_resource_usage(
    process_id: u32,
    cpu_percent: f32,
    memory_bytes: u64,
    disk_io_bytes: u64,
    network_io_bytes: u64,
) {
    if !RESOURCE_MONITOR.tracking_enabled.load(Ordering::Relaxed) {
        return;
    }
    
    let usage = ResourceUsage {
        timestamp: crate::timer::get_ticks(),
        cpu_percent,
        memory_bytes,
        disk_io_bytes,
        network_io_bytes,
    };
    
    let mut history = RESOURCE_MONITOR.usage_history.lock();
    let process_history = history.entry(process_id).or_insert_with(Vec::new);
    
    process_history.push(usage.clone());
    
    // Keep only recent history
    if process_history.len() > MAX_HISTORY_SIZE {
        process_history.remove(0);
    }
    
    // Check thresholds
    check_thresholds(&usage);
    
    // Check for anomalies
    if RESOURCE_MONITOR.anomaly_detector.enabled.load(Ordering::Relaxed) {
        detect_anomalies(&usage);
    }
}

fn check_thresholds(usage: &ResourceUsage) {
    let thresholds = RESOURCE_MONITOR.thresholds.lock();
    
    // Check CPU threshold
    if let Some(threshold) = thresholds.get("cpu_usage") {
        let cpu_usage = usage.cpu_percent as u64;
        if cpu_usage > threshold.critical_level {
            handle_threshold_violation(threshold, "CPU", cpu_usage);
        } else if cpu_usage > threshold.warning_level {
            crate::log_warn!("RESOURCES", "CPU usage at {}%", cpu_usage);
        }
    }
    
    // Check memory threshold
    if let Some(threshold) = thresholds.get("memory_usage") {
        let total_memory = crate::memory::get_total_memory();
        let memory_percent = (usage.memory_bytes * 100) / total_memory;
        
        if memory_percent > threshold.critical_level {
            handle_threshold_violation(threshold, "Memory", memory_percent);
        } else if memory_percent > threshold.warning_level {
            crate::log_warn!("RESOURCES", "Memory usage at {}%", memory_percent);
        }
    }
}

fn handle_threshold_violation(threshold: &ResourceThreshold, resource: &str, value: u64) {
    match threshold.action {
        ThresholdAction::LogOnly => {
            crate::log_error!("RESOURCES", "{} threshold exceeded: {}%", resource, value);
        }
        ThresholdAction::Alert => {
            crate::log_error!("RESOURCES", "ALERT: {} threshold exceeded: {}%", resource, value);
            crate::monitoring::events::emit_performance_threshold(
                resource,
                threshold.critical_level,
                value,
            );
        }
        ThresholdAction::Throttle => {
            crate::log_error!("RESOURCES", "Throttling due to {} threshold: {}%", resource, value);
            // Implement throttling logic
        }
        ThresholdAction::Kill => {
            crate::log_error!("RESOURCES", "Killing process due to {} threshold: {}%", resource, value);
            // Implement process termination logic
        }
    }
}

fn detect_anomalies(usage: &ResourceUsage) {
    let detector = &RESOURCE_MONITOR.anomaly_detector;
    let threshold = detector.deviation_threshold.load(Ordering::Relaxed);
    
    // Check CPU anomaly
    let baseline_cpu = detector.baseline_cpu.load(Ordering::Relaxed);
    let cpu_deviation = ((usage.cpu_percent as u64) * 100) / baseline_cpu;
    if cpu_deviation > threshold {
        crate::log_warn!("RESOURCES", "CPU anomaly detected: {}% deviation", cpu_deviation);
        crate::monitoring::events::emit_performance_threshold(
            "cpu_anomaly",
            baseline_cpu,
            usage.cpu_percent as u64,
        );
    }
    
    // Check memory anomaly
    let baseline_memory = detector.baseline_memory.load(Ordering::Relaxed);
    let memory_deviation = (usage.memory_bytes * 100) / baseline_memory;
    if memory_deviation > threshold {
        crate::log_warn!("RESOURCES", "Memory anomaly detected: {}% deviation", memory_deviation);
        crate::monitoring::events::emit_performance_threshold(
            "memory_anomaly",
            baseline_memory,
            usage.memory_bytes,
        );
    }
    
    // Check disk I/O anomaly
    let baseline_disk = detector.baseline_disk.load(Ordering::Relaxed);
    if usage.disk_io_bytes > baseline_disk * threshold / 100 {
        crate::log_warn!("RESOURCES", "Disk I/O anomaly detected");
    }
    
    // Check network I/O anomaly
    let baseline_network = detector.baseline_network.load(Ordering::Relaxed);
    if usage.network_io_bytes > baseline_network * threshold / 100 {
        crate::log_warn!("RESOURCES", "Network I/O anomaly detected");
    }
}

pub fn set_threshold(
    resource_name: &str,
    warning_level: u64,
    critical_level: u64,
    action: ThresholdAction,
) {
    let threshold = ResourceThreshold {
        resource_name: resource_name.to_string(),
        warning_level,
        critical_level,
        action,
    };
    
    RESOURCE_MONITOR.thresholds.lock().insert(resource_name.to_string(), threshold);
}

pub fn get_resource_usage(process_id: u32) -> Option<Vec<ResourceUsage>> {
    RESOURCE_MONITOR
        .usage_history
        .lock()
        .get(&process_id)
        .cloned()
}

pub fn get_resource_stats() -> ResourceStats {
    let history = RESOURCE_MONITOR.usage_history.lock();
    let mut total_cpu = 0.0;
    let mut total_memory = 0;
    let mut total_disk = 0;
    let mut total_network = 0;
    let mut count = 0;
    
    for (_, usages) in history.iter() {
        if let Some(latest) = usages.last() {
            total_cpu += latest.cpu_percent;
            total_memory += latest.memory_bytes;
            total_disk += latest.disk_io_bytes;
            total_network += latest.network_io_bytes;
            count += 1;
        }
    }
    
    ResourceStats {
        avg_cpu_percent: if count > 0 { total_cpu / count as f32 } else { 0.0 },
        total_memory_bytes: total_memory,
        total_disk_io_bytes: total_disk,
        total_network_io_bytes: total_network,
        process_count: count,
    }
}

pub struct ResourceStats {
    pub avg_cpu_percent: f32,
    pub total_memory_bytes: u64,
    pub total_disk_io_bytes: u64,
    pub total_network_io_bytes: u64,
    pub process_count: usize,
}

pub fn enable_tracking() {
    RESOURCE_MONITOR.tracking_enabled.store(true, Ordering::SeqCst);
}

pub fn disable_tracking() {
    RESOURCE_MONITOR.tracking_enabled.store(false, Ordering::SeqCst);
}

pub fn update_baseline(
    cpu: u64,
    memory: u64,
    disk: u64,
    network: u64,
) {
    let detector = &RESOURCE_MONITOR.anomaly_detector;
    detector.baseline_cpu.store(cpu, Ordering::Relaxed);
    detector.baseline_memory.store(memory, Ordering::Relaxed);
    detector.baseline_disk.store(disk, Ordering::Relaxed);
    detector.baseline_network.store(network, Ordering::Relaxed);
}

pub fn clear_history() {
    RESOURCE_MONITOR.usage_history.lock().clear();
}