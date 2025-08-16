#![no_std]

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use core::sync::atomic::{AtomicU64, AtomicBool, AtomicUsize, Ordering};
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Critical,
}

#[derive(Debug, Clone)]
pub struct ServiceHealth {
    pub name: String,
    pub status: HealthStatus,
    pub last_check: u64,
    pub check_interval_ms: u64,
    pub consecutive_failures: u32,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub name: String,
    pub check_fn: fn() -> HealthCheckResult,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    pub max_failures: u32,
    pub recovery_action: Option<fn()>,
}

#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub metrics: BTreeMap<String, u64>,
}

pub struct WatchdogService {
    enabled: AtomicBool,
    services: Mutex<BTreeMap<String, WatchdogEntry>>,
    deadlock_detector: DeadlockDetector,
    memory_leak_detector: MemoryLeakDetector,
    performance_monitor: PerformanceMonitor,
}

#[derive(Debug, Clone)]
pub struct WatchdogEntry {
    pub name: String,
    pub last_heartbeat: u64,
    pub timeout_ms: u64,
    pub recovery_action: Option<fn()>,
    pub restart_count: u32,
    pub max_restarts: u32,
}

pub struct DeadlockDetector {
    enabled: AtomicBool,
    lock_acquisitions: Mutex<Vec<LockAcquisition>>,
    detected_deadlocks: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct LockAcquisition {
    pub thread_id: u32,
    pub lock_id: u64,
    pub timestamp: u64,
    pub file: String,
    pub line: u32,
}

pub struct MemoryLeakDetector {
    enabled: AtomicBool,
    allocations: Mutex<BTreeMap<u64, AllocationInfo>>,
    suspected_leaks: Mutex<Vec<MemoryLeak>>,
    threshold_bytes: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct AllocationInfo {
    pub address: u64,
    pub size: usize,
    pub timestamp: u64,
    pub allocator: String,
    pub backtrace: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct MemoryLeak {
    pub address: u64,
    pub size: usize,
    pub age_ms: u64,
    pub allocator: String,
}

pub struct PerformanceMonitor {
    enabled: AtomicBool,
    degradation_threshold: AtomicU64,
    baseline_metrics: Mutex<BTreeMap<String, u64>>,
    current_metrics: Mutex<BTreeMap<String, u64>>,
}

pub struct HealthManager {
    health_checks: Mutex<Vec<HealthCheck>>,
    service_health: Mutex<BTreeMap<String, ServiceHealth>>,
    overall_status: AtomicUsize,
    watchdog: WatchdogService,
}

static HEALTH_MANAGER: HealthManager = HealthManager {
    health_checks: Mutex::new(Vec::new()),
    service_health: Mutex::new(BTreeMap::new()),
    overall_status: AtomicUsize::new(HealthStatus::Healthy as usize),
    watchdog: WatchdogService {
        enabled: AtomicBool::new(true),
        services: Mutex::new(BTreeMap::new()),
        deadlock_detector: DeadlockDetector {
            enabled: AtomicBool::new(true),
            lock_acquisitions: Mutex::new(Vec::new()),
            detected_deadlocks: AtomicU64::new(0),
        },
        memory_leak_detector: MemoryLeakDetector {
            enabled: AtomicBool::new(true),
            allocations: Mutex::new(BTreeMap::new()),
            suspected_leaks: Mutex::new(Vec::new()),
            threshold_bytes: AtomicU64::new(1024 * 1024 * 10), // 10MB
        },
        performance_monitor: PerformanceMonitor {
            enabled: AtomicBool::new(true),
            degradation_threshold: AtomicU64::new(200), // 200% of baseline
            baseline_metrics: Mutex::new(BTreeMap::new()),
            current_metrics: Mutex::new(BTreeMap::new()),
        },
    },
};

pub fn init() {
    // Register default health checks
    register_health_check(HealthCheck {
        name: "memory".to_string(),
        check_fn: check_memory_health,
        interval_ms: 5000,
        timeout_ms: 1000,
        max_failures: 3,
        recovery_action: Some(recover_memory),
    });
    
    register_health_check(HealthCheck {
        name: "cpu".to_string(),
        check_fn: check_cpu_health,
        interval_ms: 5000,
        timeout_ms: 1000,
        max_failures: 3,
        recovery_action: None,
    });
    
    register_health_check(HealthCheck {
        name: "disk".to_string(),
        check_fn: check_disk_health,
        interval_ms: 10000,
        timeout_ms: 2000,
        max_failures: 5,
        recovery_action: None,
    });
    
    register_health_check(HealthCheck {
        name: "network".to_string(),
        check_fn: check_network_health,
        interval_ms: 5000,
        timeout_ms: 1000,
        max_failures: 5,
        recovery_action: None,
    });
}

pub fn register_health_check(check: HealthCheck) {
    HEALTH_MANAGER.health_checks.lock().push(check);
}

pub fn register_watchdog_service(
    name: &str,
    timeout_ms: u64,
    recovery_action: Option<fn()>,
    max_restarts: u32,
) {
    let entry = WatchdogEntry {
        name: name.to_string(),
        last_heartbeat: crate::timer::get_ticks(),
        timeout_ms,
        recovery_action,
        restart_count: 0,
        max_restarts,
    };
    
    HEALTH_MANAGER.watchdog.services.lock().insert(name.to_string(), entry);
}

pub fn heartbeat(service_name: &str) {
    if let Some(mut services) = HEALTH_MANAGER.watchdog.services.try_lock() {
        if let Some(entry) = services.get_mut(service_name) {
            entry.last_heartbeat = crate::timer::get_ticks();
        }
    }
}

pub fn check_services() {
    let now = crate::timer::get_ticks();
    let mut services = HEALTH_MANAGER.watchdog.services.lock();
    
    for (name, entry) in services.iter_mut() {
        let elapsed = now - entry.last_heartbeat;
        
        if elapsed > entry.timeout_ms {
            crate::log_error!("HEALTH", "Service {} failed to heartbeat", name);
            
            if entry.restart_count < entry.max_restarts {
                if let Some(recovery) = entry.recovery_action {
                    crate::log_info!("HEALTH", "Attempting to recover service {}", name);
                    recovery();
                    entry.restart_count += 1;
                    entry.last_heartbeat = now;
                }
            } else {
                crate::log_fatal!("HEALTH", "Service {} exceeded max restarts", name);
                update_service_health(name, HealthStatus::Critical, Some("Max restarts exceeded"));
            }
        }
    }
}

fn check_memory_health() -> HealthCheckResult {
    let total = crate::memory::get_total_memory();
    let used = crate::memory::get_used_memory();
    let percent = (used * 100) / total;
    
    let status = if percent < 80 {
        HealthStatus::Healthy
    } else if percent < 90 {
        HealthStatus::Degraded
    } else if percent < 95 {
        HealthStatus::Unhealthy
    } else {
        HealthStatus::Critical
    };
    
    let mut metrics = BTreeMap::new();
    metrics.insert("memory_used_bytes".to_string(), used);
    metrics.insert("memory_total_bytes".to_string(), total);
    metrics.insert("memory_percent".to_string(), percent);
    
    HealthCheckResult {
        status,
        message: if status != HealthStatus::Healthy {
            Some(format!("Memory usage at {}%", percent))
        } else {
            None
        },
        metrics,
    }
}

fn check_cpu_health() -> HealthCheckResult {
    let usage = crate::cpu::get_cpu_usage();
    
    let status = if usage < 80 {
        HealthStatus::Healthy
    } else if usage < 90 {
        HealthStatus::Degraded
    } else {
        HealthStatus::Unhealthy
    };
    
    let mut metrics = BTreeMap::new();
    metrics.insert("cpu_usage_percent".to_string(), usage);
    
    HealthCheckResult {
        status,
        message: if status != HealthStatus::Healthy {
            Some(format!("CPU usage at {}%", usage))
        } else {
            None
        },
        metrics,
    }
}

fn check_disk_health() -> HealthCheckResult {
    // Placeholder implementation
    HealthCheckResult {
        status: HealthStatus::Healthy,
        message: None,
        metrics: BTreeMap::new(),
    }
}

fn check_network_health() -> HealthCheckResult {
    // Placeholder implementation
    HealthCheckResult {
        status: HealthStatus::Healthy,
        message: None,
        metrics: BTreeMap::new(),
    }
}

fn recover_memory() {
    crate::log_info!("HEALTH", "Attempting memory recovery");
    // Implement memory recovery actions
    // - Clear caches
    // - Trigger garbage collection
    // - Kill low-priority processes
}

pub fn detect_deadlock(thread_id: u32, lock_id: u64, file: &str, line: u32) {
    if !HEALTH_MANAGER.watchdog.deadlock_detector.enabled.load(Ordering::Relaxed) {
        return;
    }
    
    let acquisition = LockAcquisition {
        thread_id,
        lock_id,
        timestamp: crate::timer::get_ticks(),
        file: file.to_string(),
        line,
    };
    
    let mut acquisitions = HEALTH_MANAGER.watchdog.deadlock_detector.lock_acquisitions.lock();
    
    // Check for circular dependency
    let mut visited = Vec::new();
    if is_circular_dependency(&acquisitions, thread_id, lock_id, &mut visited) {
        HEALTH_MANAGER.watchdog.deadlock_detector.detected_deadlocks.fetch_add(1, Ordering::Relaxed);
        crate::log_fatal!("HEALTH", "Deadlock detected! Thread {} at {}:{}", thread_id, file, line);
        
        // Emit deadlock event
        crate::monitoring::events::emit_error(
            "deadlock_detector",
            1001,
            "Deadlock detected in system",
            false,
        );
    }
    
    acquisitions.push(acquisition);
    
    // Clean old acquisitions
    let now = crate::timer::get_ticks();
    acquisitions.retain(|a| now - a.timestamp < 60000); // Keep last minute
}

fn is_circular_dependency(
    acquisitions: &[LockAcquisition],
    thread_id: u32,
    lock_id: u64,
    visited: &mut Vec<u32>,
) -> bool {
    if visited.contains(&thread_id) {
        return true;
    }
    
    visited.push(thread_id);
    
    // Check if another thread holds the lock we want
    for acq in acquisitions {
        if acq.lock_id == lock_id && acq.thread_id != thread_id {
            // Check what locks that thread is waiting for
            return is_circular_dependency(acquisitions, acq.thread_id, lock_id, visited);
        }
    }
    
    false
}

pub fn track_allocation(address: u64, size: usize, allocator: &str) {
    if !HEALTH_MANAGER.watchdog.memory_leak_detector.enabled.load(Ordering::Relaxed) {
        return;
    }
    
    let info = AllocationInfo {
        address,
        size,
        timestamp: crate::timer::get_ticks(),
        allocator: allocator.to_string(),
        backtrace: Vec::new(), // Would capture backtrace in real implementation
    };
    
    HEALTH_MANAGER.watchdog.memory_leak_detector.allocations.lock().insert(address, info);
}

pub fn track_deallocation(address: u64) {
    if !HEALTH_MANAGER.watchdog.memory_leak_detector.enabled.load(Ordering::Relaxed) {
        return;
    }
    
    HEALTH_MANAGER.watchdog.memory_leak_detector.allocations.lock().remove(&address);
}

pub fn detect_memory_leaks() {
    let now = crate::timer::get_ticks();
    let threshold = HEALTH_MANAGER.watchdog.memory_leak_detector.threshold_bytes.load(Ordering::Relaxed);
    let allocations = HEALTH_MANAGER.watchdog.memory_leak_detector.allocations.lock();
    let mut suspected_leaks = Vec::new();
    
    for (addr, info) in allocations.iter() {
        let age_ms = now - info.timestamp;
        
        // Consider it a potential leak if:
        // - Allocation is older than 5 minutes
        // - Size is above threshold
        if age_ms > 300000 && info.size as u64 > threshold {
            suspected_leaks.push(MemoryLeak {
                address: *addr,
                size: info.size,
                age_ms,
                allocator: info.allocator.clone(),
            });
        }
    }
    
    if !suspected_leaks.is_empty() {
        crate::log_warn!("HEALTH", "Detected {} potential memory leaks", suspected_leaks.len());
        *HEALTH_MANAGER.watchdog.memory_leak_detector.suspected_leaks.lock() = suspected_leaks;
    }
}

pub fn check_performance_degradation(metric: &str, current_value: u64) {
    if !HEALTH_MANAGER.watchdog.performance_monitor.enabled.load(Ordering::Relaxed) {
        return;
    }
    
    let baseline_metrics = HEALTH_MANAGER.watchdog.performance_monitor.baseline_metrics.lock();
    
    if let Some(&baseline) = baseline_metrics.get(metric) {
        let threshold = HEALTH_MANAGER.watchdog.performance_monitor.degradation_threshold.load(Ordering::Relaxed);
        let deviation = (current_value * 100) / baseline;
        
        if deviation > threshold {
            crate::log_warn!("HEALTH", "Performance degradation in {}: {}% of baseline", metric, deviation);
            crate::monitoring::events::emit_performance_threshold(metric, baseline, current_value);
        }
    }
    
    // Update current metrics
    HEALTH_MANAGER.watchdog.performance_monitor.current_metrics.lock().insert(metric.to_string(), current_value);
}

pub fn update_service_health(name: &str, status: HealthStatus, error: Option<&str>) {
    let mut service_health = HEALTH_MANAGER.service_health.lock();
    
    let health = service_health.entry(name.to_string()).or_insert_with(|| {
        ServiceHealth {
            name: name.to_string(),
            status: HealthStatus::Healthy,
            last_check: 0,
            check_interval_ms: 5000,
            consecutive_failures: 0,
            error_message: None,
        }
    });
    
    health.status = status;
    health.last_check = crate::timer::get_ticks();
    health.error_message = error.map(|s| s.to_string());
    
    if status != HealthStatus::Healthy {
        health.consecutive_failures += 1;
    } else {
        health.consecutive_failures = 0;
    }
    
    // Update overall status
    update_overall_status(&service_health);
}

fn update_overall_status(service_health: &BTreeMap<String, ServiceHealth>) {
    let mut worst_status = HealthStatus::Healthy;
    
    for (_, health) in service_health.iter() {
        if health.status as u8 > worst_status as u8 {
            worst_status = health.status;
        }
    }
    
    HEALTH_MANAGER.overall_status.store(worst_status as usize, Ordering::SeqCst);
}

pub fn get_overall_health() -> HealthStatus {
    match HEALTH_MANAGER.overall_status.load(Ordering::Relaxed) {
        0 => HealthStatus::Healthy,
        1 => HealthStatus::Degraded,
        2 => HealthStatus::Unhealthy,
        3 => HealthStatus::Critical,
        _ => HealthStatus::Unhealthy,
    }
}

pub fn get_service_health(name: &str) -> Option<ServiceHealth> {
    HEALTH_MANAGER.service_health.lock().get(name).cloned()
}

pub fn get_all_service_health() -> Vec<ServiceHealth> {
    HEALTH_MANAGER.service_health.lock().values().cloned().collect()
}

pub fn generate_health_report() -> String {
    let mut report = String::from("System Health Report\n");
    report.push_str("====================\n\n");
    
    let overall = get_overall_health();
    report.push_str(&format!("Overall Status: {:?}\n\n", overall));
    
    report.push_str("Service Health:\n");
    for health in get_all_service_health() {
        report.push_str(&format!("  {} - {:?}", health.name, health.status));
        if let Some(err) = health.error_message {
            report.push_str(&format!(" ({})", err));
        }
        report.push_str("\n");
    }
    
    let deadlocks = HEALTH_MANAGER.watchdog.deadlock_detector.detected_deadlocks.load(Ordering::Relaxed);
    report.push_str(&format!("\nDeadlocks Detected: {}\n", deadlocks));
    
    let leaks = HEALTH_MANAGER.watchdog.memory_leak_detector.suspected_leaks.lock();
    report.push_str(&format!("Suspected Memory Leaks: {}\n", leaks.len()));
    
    report
}