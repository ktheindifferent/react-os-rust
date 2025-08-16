#![no_std]

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub kernel_version: String,
    pub build_time: String,
    pub uptime_ms: u64,
    pub boot_time: u64,
    pub cpu_info: CpuInfo,
    pub memory_info: MemoryInfo,
    pub hardware_info: HardwareInfo,
}

#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub vendor: String,
    pub model: String,
    pub cores: u32,
    pub threads: u32,
    pub frequency_mhz: u32,
    pub features: Vec<String>,
    pub cache_sizes: CacheSizes,
}

#[derive(Debug, Clone)]
pub struct CacheSizes {
    pub l1_data_kb: u32,
    pub l1_inst_kb: u32,
    pub l2_kb: u32,
    pub l3_kb: u32,
}

#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub cached_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct HardwareInfo {
    pub motherboard: String,
    pub bios_version: String,
    pub devices: Vec<DeviceInfo>,
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_type: String,
    pub vendor: String,
    pub model: String,
    pub driver: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct DiagnosticReport {
    pub timestamp: u64,
    pub report_id: u64,
    pub system_info: SystemInfo,
    pub performance_baseline: PerformanceBaseline,
    pub error_summary: ErrorSummary,
    pub resource_usage: ResourceUsageSummary,
    pub configuration: ConfigurationDump,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PerformanceBaseline {
    pub cpu_baseline: u64,
    pub memory_baseline: u64,
    pub disk_io_baseline: u64,
    pub network_io_baseline: u64,
    pub latency_percentiles: LatencyPercentiles,
}

#[derive(Debug, Clone)]
pub struct LatencyPercentiles {
    pub p50_us: u64,
    pub p90_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub p999_us: u64,
}

#[derive(Debug, Clone)]
pub struct ErrorSummary {
    pub total_errors: u64,
    pub critical_errors: u64,
    pub recent_errors: Vec<ErrorEntry>,
    pub error_by_component: BTreeMap<String, u64>,
}

#[derive(Debug, Clone)]
pub struct ErrorEntry {
    pub timestamp: u64,
    pub component: String,
    pub error_code: u32,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ResourceUsageSummary {
    pub cpu_usage_avg: f32,
    pub cpu_usage_peak: f32,
    pub memory_usage_avg: u64,
    pub memory_usage_peak: u64,
    pub disk_usage_bytes: u64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
}

#[derive(Debug, Clone)]
pub struct ConfigurationDump {
    pub kernel_params: BTreeMap<String, String>,
    pub system_settings: BTreeMap<String, String>,
    pub module_versions: BTreeMap<String, String>,
    pub enabled_features: Vec<String>,
}

pub struct DiagnosticsCollector {
    report_counter: AtomicU64,
    system_info_cache: Mutex<Option<SystemInfo>>,
    performance_samples: Mutex<Vec<PerformanceSample>>,
    error_log: Mutex<Vec<ErrorEntry>>,
    troubleshooting_db: Mutex<BTreeMap<u32, TroubleshootingGuide>>,
}

#[derive(Debug, Clone)]
pub struct PerformanceSample {
    pub timestamp: u64,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub disk_io: u64,
    pub network_io: u64,
}

#[derive(Debug, Clone)]
pub struct TroubleshootingGuide {
    pub error_code: u32,
    pub title: String,
    pub description: String,
    pub possible_causes: Vec<String>,
    pub resolution_steps: Vec<String>,
    pub related_logs: Vec<String>,
}

static DIAGNOSTICS_COLLECTOR: DiagnosticsCollector = DiagnosticsCollector {
    report_counter: AtomicU64::new(0),
    system_info_cache: Mutex::new(None),
    performance_samples: Mutex::new(Vec::new()),
    error_log: Mutex::new(Vec::new()),
    troubleshooting_db: Mutex::new(BTreeMap::new()),
};

pub fn init() {
    // Initialize troubleshooting database
    {
        let mut possible_causes = Vec::new();
        possible_causes.push("Incorrect lock ordering".to_string());
        possible_causes.push("Resource contention".to_string());
        
        let mut resolution_steps = Vec::new();
        resolution_steps.push("Review lock acquisition order".to_string());
        resolution_steps.push("Implement lock timeout".to_string());
        resolution_steps.push("Use lock-free data structures".to_string());
        
        let mut related_logs = Vec::new();
        related_logs.push("deadlock_detector".to_string());
        
        register_troubleshooting_guide(TroubleshootingGuide {
            error_code: 1001,
            title: "System Deadlock".to_string(),
            description: "Multiple threads are waiting for each other".to_string(),
            possible_causes,
            resolution_steps,
            related_logs,
        });
    }
    
    {
        let mut possible_causes = Vec::new();
        possible_causes.push("Memory leak".to_string());
        possible_causes.push("Excessive allocations".to_string());
        possible_causes.push("Insufficient system memory".to_string());
        
        let mut resolution_steps = Vec::new();
        resolution_steps.push("Check for memory leaks".to_string());
        resolution_steps.push("Review allocation patterns".to_string());
        resolution_steps.push("Increase system memory".to_string());
        resolution_steps.push("Enable swap space".to_string());
        
        let mut related_logs = Vec::new();
        related_logs.push("memory".to_string());
        related_logs.push("allocator".to_string());
        
        register_troubleshooting_guide(TroubleshootingGuide {
            error_code: 2001,
            title: "Out of Memory".to_string(),
            description: "System has exhausted available memory".to_string(),
            possible_causes,
            resolution_steps,
            related_logs,
        });
    }
}

pub fn collect_system_info() -> SystemInfo {
    // Check cache first
    if let Some(cached) = DIAGNOSTICS_COLLECTOR.system_info_cache.lock().as_ref() {
        return cached.clone();
    }
    
    let cpu_info = CpuInfo {
        vendor: crate::cpu::get_vendor().unwrap_or_else(|| "Unknown".to_string()),
        model: crate::cpu::get_model().unwrap_or_else(|| "Unknown".to_string()),
        cores: crate::cpu::cpu_count() as u32,
        threads: crate::cpu::thread_count() as u32,
        frequency_mhz: crate::cpu::get_frequency_mhz(),
        features: crate::cpu::get_features(),
        cache_sizes: CacheSizes {
            l1_data_kb: 32,
            l1_inst_kb: 32,
            l2_kb: 256,
            l3_kb: 8192,
        },
    };
    
    let memory_info = MemoryInfo {
        total_bytes: crate::memory::get_total_memory(),
        available_bytes: crate::memory::get_available_memory(),
        used_bytes: crate::memory::get_used_memory(),
        cached_bytes: 0,
        swap_total_bytes: 0,
        swap_used_bytes: 0,
    };
    
    let hardware_info = HardwareInfo {
        motherboard: "Generic Motherboard".to_string(),
        bios_version: "1.0.0".to_string(),
        devices: collect_device_info(),
    };
    
    let system_info = SystemInfo {
        kernel_version: "1.0.0".to_string(),
        build_time: "2024-01-01 00:00:00".to_string(),
        uptime_ms: crate::timer::get_ticks(),
        boot_time: 0,
        cpu_info,
        memory_info,
        hardware_info,
    };
    
    // Cache the system info
    *DIAGNOSTICS_COLLECTOR.system_info_cache.lock() = Some(system_info.clone());
    
    system_info
}

fn collect_device_info() -> Vec<DeviceInfo> {
    let mut devices = Vec::new();
    
    // Collect PCI devices
    if let Some(pci_devices) = crate::acpi::pci::enumerate_devices() {
        for device in pci_devices {
            devices.push(DeviceInfo {
                device_type: "PCI".to_string(),
                vendor: format!("{:04x}", device.vendor_id),
                model: format!("{:04x}", device.device_id),
                driver: device.driver.unwrap_or_else(|| "none".to_string()),
                status: "active".to_string(),
            });
        }
    }
    
    // Collect USB devices
    if let Some(usb_devices) = crate::usb::enumerate_devices() {
        for device in usb_devices {
            devices.push(DeviceInfo {
                device_type: "USB".to_string(),
                vendor: format!("{:04x}", device.vendor_id),
                model: format!("{:04x}", device.product_id),
                driver: device.driver.unwrap_or_else(|| "none".to_string()),
                status: "active".to_string(),
            });
        }
    }
    
    devices
}

pub fn collect_performance_baseline() -> PerformanceBaseline {
    let samples = DIAGNOSTICS_COLLECTOR.performance_samples.lock();
    
    if samples.is_empty() {
        return PerformanceBaseline {
            cpu_baseline: 50,
            memory_baseline: 1024 * 1024 * 512,
            disk_io_baseline: 1024 * 1024 * 10,
            network_io_baseline: 1024 * 1024,
            latency_percentiles: LatencyPercentiles {
                p50_us: 100,
                p90_us: 500,
                p95_us: 1000,
                p99_us: 5000,
                p999_us: 10000,
            },
        };
    }
    
    // Calculate baselines from samples
    let cpu_sum: f32 = samples.iter().map(|s| s.cpu_usage).sum();
    let memory_sum: u64 = samples.iter().map(|s| s.memory_usage).sum();
    let disk_sum: u64 = samples.iter().map(|s| s.disk_io).sum();
    let network_sum: u64 = samples.iter().map(|s| s.network_io).sum();
    let count = samples.len() as u64;
    
    PerformanceBaseline {
        cpu_baseline: (cpu_sum / count as f32) as u64,
        memory_baseline: memory_sum / count,
        disk_io_baseline: disk_sum / count,
        network_io_baseline: network_sum / count,
        latency_percentiles: calculate_latency_percentiles(),
    }
}

fn calculate_latency_percentiles() -> LatencyPercentiles {
    // Placeholder - would calculate from actual latency measurements
    LatencyPercentiles {
        p50_us: 100,
        p90_us: 500,
        p95_us: 1000,
        p99_us: 5000,
        p999_us: 10000,
    }
}

pub fn record_performance_sample(
    cpu_usage: f32,
    memory_usage: u64,
    disk_io: u64,
    network_io: u64,
) {
    let sample = PerformanceSample {
        timestamp: crate::timer::get_ticks(),
        cpu_usage,
        memory_usage,
        disk_io,
        network_io,
    };
    
    let mut samples = DIAGNOSTICS_COLLECTOR.performance_samples.lock();
    samples.push(sample);
    
    // Keep only last 1000 samples
    if samples.len() > 1000 {
        samples.remove(0);
    }
}

pub fn log_error(component: &str, error_code: u32, message: &str) {
    let entry = ErrorEntry {
        timestamp: crate::timer::get_ticks(),
        component: component.to_string(),
        error_code,
        message: message.to_string(),
    };
    
    let mut error_log = DIAGNOSTICS_COLLECTOR.error_log.lock();
    error_log.push(entry);
    
    // Keep only last 1000 errors
    if error_log.len() > 1000 {
        error_log.remove(0);
    }
}

pub fn get_error_summary() -> ErrorSummary {
    let error_log = DIAGNOSTICS_COLLECTOR.error_log.lock();
    let mut error_by_component = BTreeMap::new();
    let mut critical_count = 0;
    
    for error in error_log.iter() {
        *error_by_component.entry(error.component.clone()).or_insert(0) += 1;
        
        if error.error_code >= 1000 && error.error_code < 2000 {
            critical_count += 1;
        }
    }
    
    ErrorSummary {
        total_errors: error_log.len() as u64,
        critical_errors: critical_count,
        recent_errors: error_log.iter().rev().take(10).cloned().collect(),
        error_by_component,
    }
}

pub fn get_configuration_dump() -> ConfigurationDump {
    let mut kernel_params = BTreeMap::new();
    kernel_params.insert("debug".to_string(), "enabled".to_string());
    kernel_params.insert("max_processes".to_string(), "1000".to_string());
    kernel_params.insert("scheduler".to_string(), "round_robin".to_string());
    
    let mut system_settings = BTreeMap::new();
    system_settings.insert("monitoring".to_string(), "enabled".to_string());
    system_settings.insert("telemetry".to_string(), "disabled".to_string());
    system_settings.insert("log_level".to_string(), "info".to_string());
    
    let mut module_versions = BTreeMap::new();
    module_versions.insert("kernel".to_string(), "1.0.0".to_string());
    module_versions.insert("memory".to_string(), "1.0.0".to_string());
    module_versions.insert("fs".to_string(), "1.0.0".to_string());
    module_versions.insert("net".to_string(), "1.0.0".to_string());
    
    let mut enabled_features = Vec::new();
    enabled_features.push("smp".to_string());
    enabled_features.push("preemption".to_string());
    enabled_features.push("virtual_memory".to_string());
    enabled_features.push("networking".to_string());
    enabled_features.push("monitoring".to_string());
    
    ConfigurationDump {
        kernel_params,
        system_settings,
        module_versions,
        enabled_features,
    }
}

pub fn generate_diagnostic_report() -> DiagnosticReport {
    let report_id = DIAGNOSTICS_COLLECTOR.report_counter.fetch_add(1, Ordering::SeqCst);
    
    let mut recommendations = Vec::new();
    
    // Check for issues and generate recommendations
    let error_summary = get_error_summary();
    if error_summary.critical_errors > 0 {
        recommendations.push("Critical errors detected - review error log".to_string());
    }
    
    let memory_info = collect_system_info().memory_info;
    let memory_usage_percent = (memory_info.used_bytes * 100) / memory_info.total_bytes;
    if memory_usage_percent > 80 {
        recommendations.push(format!("High memory usage ({}%) - consider increasing memory", memory_usage_percent));
    }
    
    DiagnosticReport {
        timestamp: crate::timer::get_ticks(),
        report_id,
        system_info: collect_system_info(),
        performance_baseline: collect_performance_baseline(),
        error_summary,
        resource_usage: get_resource_usage_summary(),
        configuration: get_configuration_dump(),
        recommendations,
    }
}

fn get_resource_usage_summary() -> ResourceUsageSummary {
    let samples = DIAGNOSTICS_COLLECTOR.performance_samples.lock();
    
    if samples.is_empty() {
        return ResourceUsageSummary {
            cpu_usage_avg: 0.0,
            cpu_usage_peak: 0.0,
            memory_usage_avg: 0,
            memory_usage_peak: 0,
            disk_usage_bytes: 0,
            network_bytes_sent: 0,
            network_bytes_received: 0,
        };
    }
    
    let cpu_sum: f32 = samples.iter().map(|s| s.cpu_usage).sum();
    let cpu_peak = samples.iter().map(|s| s.cpu_usage).fold(0.0, f32::max);
    let memory_sum: u64 = samples.iter().map(|s| s.memory_usage).sum();
    let memory_peak = samples.iter().map(|s| s.memory_usage).max().unwrap_or(0);
    let count = samples.len() as u64;
    
    ResourceUsageSummary {
        cpu_usage_avg: cpu_sum / count as f32,
        cpu_usage_peak: cpu_peak,
        memory_usage_avg: memory_sum / count,
        memory_usage_peak: memory_peak,
        disk_usage_bytes: crate::fs::get_disk_usage(),
        network_bytes_sent: crate::net::get_bytes_sent(),
        network_bytes_received: crate::net::get_bytes_received(),
    }
}

pub fn register_troubleshooting_guide(guide: TroubleshootingGuide) {
    DIAGNOSTICS_COLLECTOR.troubleshooting_db.lock().insert(guide.error_code, guide);
}

pub fn get_troubleshooting_guide(error_code: u32) -> Option<TroubleshootingGuide> {
    DIAGNOSTICS_COLLECTOR.troubleshooting_db.lock().get(&error_code).cloned()
}

pub fn export_diagnostic_report(report: &DiagnosticReport) -> String {
    let mut output = String::new();
    
    output.push_str("=== SYSTEM DIAGNOSTIC REPORT ===\n");
    output.push_str(&format!("Report ID: {}\n", report.report_id));
    output.push_str(&format!("Timestamp: {}\n\n", report.timestamp));
    
    output.push_str("SYSTEM INFORMATION:\n");
    output.push_str(&format!("  Kernel Version: {}\n", report.system_info.kernel_version));
    output.push_str(&format!("  Uptime: {} ms\n", report.system_info.uptime_ms));
    output.push_str(&format!("  CPU: {} {} ({} cores)\n",
        report.system_info.cpu_info.vendor,
        report.system_info.cpu_info.model,
        report.system_info.cpu_info.cores
    ));
    output.push_str(&format!("  Memory: {} MB total\n\n",
        report.system_info.memory_info.total_bytes / (1024 * 1024)
    ));
    
    output.push_str("PERFORMANCE BASELINE:\n");
    output.push_str(&format!("  CPU: {}%\n", report.performance_baseline.cpu_baseline));
    output.push_str(&format!("  Memory: {} MB\n", 
        report.performance_baseline.memory_baseline / (1024 * 1024)
    ));
    output.push_str(&format!("  Latency P99: {} us\n\n",
        report.performance_baseline.latency_percentiles.p99_us
    ));
    
    output.push_str("ERROR SUMMARY:\n");
    output.push_str(&format!("  Total Errors: {}\n", report.error_summary.total_errors));
    output.push_str(&format!("  Critical Errors: {}\n\n", report.error_summary.critical_errors));
    
    if !report.recommendations.is_empty() {
        output.push_str("RECOMMENDATIONS:\n");
        for rec in &report.recommendations {
            output.push_str(&format!("  - {}\n", rec));
        }
    }
    
    output
}