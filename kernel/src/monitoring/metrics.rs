#![no_std]

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

#[derive(Debug, Clone)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

#[derive(Debug, Clone)]
pub struct MetricValue {
    pub value: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub metric_type: MetricType,
    pub description: String,
    pub unit: String,
    pub labels: BTreeMap<String, String>,
    pub values: Vec<MetricValue>,
}

pub struct MetricsCollector {
    metrics: Mutex<BTreeMap<String, Metric>>,
    cpu_metrics: CpuMetrics,
    memory_metrics: MemoryMetrics,
    disk_metrics: DiskMetrics,
    network_metrics: NetworkMetrics,
    process_metrics: ProcessMetrics,
    fs_metrics: FileSystemMetrics,
}

pub struct CpuMetrics {
    pub usage_per_core: Vec<AtomicU64>,
    pub total_usage: AtomicU64,
    pub idle_time: AtomicU64,
    pub system_time: AtomicU64,
    pub user_time: AtomicU64,
    pub interrupt_time: AtomicU64,
    pub context_switches: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
}

pub struct MemoryMetrics {
    pub total_memory: AtomicU64,
    pub used_memory: AtomicU64,
    pub free_memory: AtomicU64,
    pub cached_memory: AtomicU64,
    pub buffer_memory: AtomicU64,
    pub swap_total: AtomicU64,
    pub swap_used: AtomicU64,
    pub page_faults: AtomicU64,
    pub page_ins: AtomicU64,
    pub page_outs: AtomicU64,
}

pub struct DiskMetrics {
    pub read_ops: AtomicU64,
    pub write_ops: AtomicU64,
    pub read_bytes: AtomicU64,
    pub write_bytes: AtomicU64,
    pub read_latency_us: AtomicU64,
    pub write_latency_us: AtomicU64,
    pub queue_depth: AtomicUsize,
    pub io_errors: AtomicU64,
}

pub struct NetworkMetrics {
    pub packets_sent: AtomicU64,
    pub packets_received: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub errors_tx: AtomicU64,
    pub errors_rx: AtomicU64,
    pub drops_tx: AtomicU64,
    pub drops_rx: AtomicU64,
    pub tcp_connections: AtomicUsize,
    pub udp_sockets: AtomicUsize,
}

pub struct ProcessMetrics {
    pub total_processes: AtomicUsize,
    pub running_processes: AtomicUsize,
    pub blocked_processes: AtomicUsize,
    pub zombie_processes: AtomicUsize,
    pub total_threads: AtomicUsize,
    pub process_creates: AtomicU64,
    pub process_exits: AtomicU64,
}

pub struct FileSystemMetrics {
    pub total_space: AtomicU64,
    pub used_space: AtomicU64,
    pub free_space: AtomicU64,
    pub inode_total: AtomicU64,
    pub inode_used: AtomicU64,
    pub file_opens: AtomicU64,
    pub file_closes: AtomicU64,
    pub dir_operations: AtomicU64,
}

static METRICS_COLLECTOR: MetricsCollector = MetricsCollector {
    metrics: Mutex::new(BTreeMap::new()),
    cpu_metrics: CpuMetrics {
        usage_per_core: Vec::new(),
        total_usage: AtomicU64::new(0),
        idle_time: AtomicU64::new(0),
        system_time: AtomicU64::new(0),
        user_time: AtomicU64::new(0),
        interrupt_time: AtomicU64::new(0),
        context_switches: AtomicU64::new(0),
        cache_hits: AtomicU64::new(0),
        cache_misses: AtomicU64::new(0),
    },
    memory_metrics: MemoryMetrics {
        total_memory: AtomicU64::new(0),
        used_memory: AtomicU64::new(0),
        free_memory: AtomicU64::new(0),
        cached_memory: AtomicU64::new(0),
        buffer_memory: AtomicU64::new(0),
        swap_total: AtomicU64::new(0),
        swap_used: AtomicU64::new(0),
        page_faults: AtomicU64::new(0),
        page_ins: AtomicU64::new(0),
        page_outs: AtomicU64::new(0),
    },
    disk_metrics: DiskMetrics {
        read_ops: AtomicU64::new(0),
        write_ops: AtomicU64::new(0),
        read_bytes: AtomicU64::new(0),
        write_bytes: AtomicU64::new(0),
        read_latency_us: AtomicU64::new(0),
        write_latency_us: AtomicU64::new(0),
        queue_depth: AtomicUsize::new(0),
        io_errors: AtomicU64::new(0),
    },
    network_metrics: NetworkMetrics {
        packets_sent: AtomicU64::new(0),
        packets_received: AtomicU64::new(0),
        bytes_sent: AtomicU64::new(0),
        bytes_received: AtomicU64::new(0),
        errors_tx: AtomicU64::new(0),
        errors_rx: AtomicU64::new(0),
        drops_tx: AtomicU64::new(0),
        drops_rx: AtomicU64::new(0),
        tcp_connections: AtomicUsize::new(0),
        udp_sockets: AtomicUsize::new(0),
    },
    process_metrics: ProcessMetrics {
        total_processes: AtomicUsize::new(0),
        running_processes: AtomicUsize::new(0),
        blocked_processes: AtomicUsize::new(0),
        zombie_processes: AtomicUsize::new(0),
        total_threads: AtomicUsize::new(0),
        process_creates: AtomicU64::new(0),
        process_exits: AtomicU64::new(0),
    },
    fs_metrics: FileSystemMetrics {
        total_space: AtomicU64::new(0),
        used_space: AtomicU64::new(0),
        free_space: AtomicU64::new(0),
        inode_total: AtomicU64::new(0),
        inode_used: AtomicU64::new(0),
        file_opens: AtomicU64::new(0),
        file_closes: AtomicU64::new(0),
        dir_operations: AtomicU64::new(0),
    },
};

pub fn init() {
    // Initialize CPU metrics for each core
    let cpu_count = crate::cpu::cpu_count();
    // Note: This would need proper initialization in a real implementation
}

pub fn register_metric(
    name: &str,
    metric_type: MetricType,
    description: &str,
    unit: &str,
) -> Result<(), &'static str> {
    let mut metrics = METRICS_COLLECTOR.metrics.lock();
    
    if metrics.contains_key(name) {
        return Err("Metric already registered");
    }
    
    let metric = Metric {
        name: name.to_string(),
        metric_type,
        description: description.to_string(),
        unit: unit.to_string(),
        labels: BTreeMap::new(),
        values: Vec::new(),
    };
    
    metrics.insert(name.to_string(), metric);
    Ok(())
}

pub fn record_value(name: &str, value: u64) -> Result<(), &'static str> {
    let mut metrics = METRICS_COLLECTOR.metrics.lock();
    
    if let Some(metric) = metrics.get_mut(name) {
        let timestamp = crate::timer::get_ticks();
        metric.values.push(MetricValue { value, timestamp });
        
        // Keep only last 1000 values
        if metric.values.len() > 1000 {
            metric.values.remove(0);
        }
        
        Ok(())
    } else {
        Err("Metric not found")
    }
}

// CPU metric update functions
pub fn update_cpu_usage(core: usize, usage: u64) {
    if core < METRICS_COLLECTOR.cpu_metrics.usage_per_core.len() {
        METRICS_COLLECTOR.cpu_metrics.usage_per_core[core].store(usage, Ordering::Relaxed);
    }
}

pub fn increment_context_switches() {
    METRICS_COLLECTOR.cpu_metrics.context_switches.fetch_add(1, Ordering::Relaxed);
}

pub fn update_cache_stats(hits: u64, misses: u64) {
    METRICS_COLLECTOR.cpu_metrics.cache_hits.store(hits, Ordering::Relaxed);
    METRICS_COLLECTOR.cpu_metrics.cache_misses.store(misses, Ordering::Relaxed);
}

// Memory metric update functions
pub fn update_memory_usage(used: u64, free: u64, cached: u64) {
    METRICS_COLLECTOR.memory_metrics.used_memory.store(used, Ordering::Relaxed);
    METRICS_COLLECTOR.memory_metrics.free_memory.store(free, Ordering::Relaxed);
    METRICS_COLLECTOR.memory_metrics.cached_memory.store(cached, Ordering::Relaxed);
}

pub fn increment_page_fault() {
    METRICS_COLLECTOR.memory_metrics.page_faults.fetch_add(1, Ordering::Relaxed);
}

// Disk metric update functions
pub fn record_disk_io(read: bool, bytes: u64, latency_us: u64) {
    if read {
        METRICS_COLLECTOR.disk_metrics.read_ops.fetch_add(1, Ordering::Relaxed);
        METRICS_COLLECTOR.disk_metrics.read_bytes.fetch_add(bytes, Ordering::Relaxed);
        METRICS_COLLECTOR.disk_metrics.read_latency_us.store(latency_us, Ordering::Relaxed);
    } else {
        METRICS_COLLECTOR.disk_metrics.write_ops.fetch_add(1, Ordering::Relaxed);
        METRICS_COLLECTOR.disk_metrics.write_bytes.fetch_add(bytes, Ordering::Relaxed);
        METRICS_COLLECTOR.disk_metrics.write_latency_us.store(latency_us, Ordering::Relaxed);
    }
}

// Network metric update functions
pub fn record_network_packet(sent: bool, bytes: u64) {
    if sent {
        METRICS_COLLECTOR.network_metrics.packets_sent.fetch_add(1, Ordering::Relaxed);
        METRICS_COLLECTOR.network_metrics.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    } else {
        METRICS_COLLECTOR.network_metrics.packets_received.fetch_add(1, Ordering::Relaxed);
        METRICS_COLLECTOR.network_metrics.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }
}

pub fn update_tcp_connections(count: usize) {
    METRICS_COLLECTOR.network_metrics.tcp_connections.store(count, Ordering::Relaxed);
}

// Process metric update functions
pub fn update_process_counts(total: usize, running: usize, blocked: usize) {
    METRICS_COLLECTOR.process_metrics.total_processes.store(total, Ordering::Relaxed);
    METRICS_COLLECTOR.process_metrics.running_processes.store(running, Ordering::Relaxed);
    METRICS_COLLECTOR.process_metrics.blocked_processes.store(blocked, Ordering::Relaxed);
}

pub fn increment_process_create() {
    METRICS_COLLECTOR.process_metrics.process_creates.fetch_add(1, Ordering::Relaxed);
}

pub fn increment_process_exit() {
    METRICS_COLLECTOR.process_metrics.process_exits.fetch_add(1, Ordering::Relaxed);
}

// File system metric update functions
pub fn update_fs_usage(total: u64, used: u64, free: u64) {
    METRICS_COLLECTOR.fs_metrics.total_space.store(total, Ordering::Relaxed);
    METRICS_COLLECTOR.fs_metrics.used_space.store(used, Ordering::Relaxed);
    METRICS_COLLECTOR.fs_metrics.free_space.store(free, Ordering::Relaxed);
}

pub fn increment_file_open() {
    METRICS_COLLECTOR.fs_metrics.file_opens.fetch_add(1, Ordering::Relaxed);
}

pub fn increment_file_close() {
    METRICS_COLLECTOR.fs_metrics.file_closes.fetch_add(1, Ordering::Relaxed);
}

// Export metrics in Prometheus format
pub fn export_prometheus() -> String {
    let mut output = String::new();
    let metrics = METRICS_COLLECTOR.metrics.lock();
    
    for (_, metric) in metrics.iter() {
        output.push_str(&format!("# HELP {} {}\n", metric.name, metric.description));
        output.push_str(&format!("# TYPE {} {:?}\n", metric.name, metric.metric_type));
        
        if let Some(latest) = metric.values.last() {
            output.push_str(&format!("{} {}\n", metric.name, latest.value));
        }
    }
    
    // Add system metrics
    output.push_str(&format!("cpu_usage_total {}\n", 
        METRICS_COLLECTOR.cpu_metrics.total_usage.load(Ordering::Relaxed)));
    output.push_str(&format!("memory_used_bytes {}\n",
        METRICS_COLLECTOR.memory_metrics.used_memory.load(Ordering::Relaxed)));
    output.push_str(&format!("disk_read_ops_total {}\n",
        METRICS_COLLECTOR.disk_metrics.read_ops.load(Ordering::Relaxed)));
    output.push_str(&format!("network_packets_sent_total {}\n",
        METRICS_COLLECTOR.network_metrics.packets_sent.load(Ordering::Relaxed)));
    
    output
}

pub fn flush() {
    // Persist metrics if needed
}

pub fn get_all_metrics() -> Vec<Metric> {
    METRICS_COLLECTOR.metrics.lock().values().cloned().collect()
}