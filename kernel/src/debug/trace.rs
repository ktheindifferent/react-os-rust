// Kernel Tracing Infrastructure
// Static and dynamic tracepoints, event tracing, ring buffer

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;

// Main tracing system
pub struct TraceSystem {
    enabled: AtomicBool,
    event_buffer: Mutex<RingBuffer>,
    tracepoints: Mutex<BTreeMap<String, Tracepoint>>,
    filters: Mutex<Vec<TraceFilter>>,
    statistics: TraceStatistics,
}

// Ring buffer for trace events
struct RingBuffer {
    buffer: Vec<TraceEvent>,
    capacity: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
    overrun_count: AtomicU64,
}

// Individual trace event
#[derive(Clone)]
struct TraceEvent {
    timestamp: u64,
    cpu_id: u32,
    pid: u32,
    event_id: u32,
    category: String,
    name: String,
    data: TraceData,
    stack_depth: u32,
}

#[derive(Clone)]
enum TraceData {
    None,
    U64(u64),
    I64(i64),
    String(String),
    Binary(Vec<u8>),
    Structured(BTreeMap<String, String>),
}

// Static tracepoint definition
struct Tracepoint {
    id: u32,
    name: String,
    category: String,
    enabled: AtomicBool,
    hit_count: AtomicU64,
    format: String,
}

// Trace filter for selective tracing
struct TraceFilter {
    category: Option<String>,
    name: Option<String>,
    pid: Option<u32>,
    cpu: Option<u32>,
    enabled: bool,
}

// Tracing statistics
struct TraceStatistics {
    total_events: AtomicU64,
    dropped_events: AtomicU64,
    categories: Mutex<BTreeMap<String, CategoryStats>>,
}

struct CategoryStats {
    event_count: u64,
    total_size: u64,
}

lazy_static! {
    pub static ref TRACE: TraceSystem = TraceSystem::new();
}

impl TraceSystem {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            event_buffer: Mutex::new(RingBuffer::new(65536)),  // 64K events
            tracepoints: Mutex::new(BTreeMap::new()),
            filters: Mutex::new(Vec::new()),
            statistics: TraceStatistics::new(),
        }
    }
    
    pub fn init(&self) {
        // Register built-in tracepoints
        self.register_builtin_tracepoints();
        
        self.enabled.store(true, Ordering::SeqCst);
        crate::serial_println!("[TRACE] Tracing infrastructure initialized");
    }
    
    fn register_builtin_tracepoints(&self) {
        // Register common kernel tracepoints
        self.register_tracepoint("scheduler", "context_switch");
        self.register_tracepoint("scheduler", "wake_up");
        self.register_tracepoint("scheduler", "sleep");
        
        self.register_tracepoint("mm", "page_alloc");
        self.register_tracepoint("mm", "page_free");
        self.register_tracepoint("mm", "page_fault");
        self.register_tracepoint("mm", "kmalloc");
        self.register_tracepoint("mm", "kfree");
        
        self.register_tracepoint("irq", "irq_entry");
        self.register_tracepoint("irq", "irq_exit");
        self.register_tracepoint("irq", "softirq_entry");
        self.register_tracepoint("irq", "softirq_exit");
        
        self.register_tracepoint("syscall", "sys_enter");
        self.register_tracepoint("syscall", "sys_exit");
        
        self.register_tracepoint("block", "bio_queue");
        self.register_tracepoint("block", "bio_complete");
        
        self.register_tracepoint("net", "netif_receive");
        self.register_tracepoint("net", "net_dev_xmit");
        
        self.register_tracepoint("lock", "lock_acquire");
        self.register_tracepoint("lock", "lock_release");
        self.register_tracepoint("lock", "lock_contended");
    }
    
    pub fn register_tracepoint(&self, category: &str, name: &str) -> u32 {
        let mut tracepoints = self.tracepoints.lock();
        let id = tracepoints.len() as u32;
        
        let tp = Tracepoint {
            id,
            name: name.to_string(),
            category: category.to_string(),
            enabled: AtomicBool::new(false),
            hit_count: AtomicU64::new(0),
            format: String::new(),
        };
        
        tracepoints.insert(format!("{}:{}", category, name), tp);
        id
    }
    
    pub fn enable_tracepoint(&self, category: &str, name: &str) {
        let key = format!("{}:{}", category, name);
        if let Some(tp) = self.tracepoints.lock().get(&key) {
            tp.enabled.store(true, Ordering::SeqCst);
            crate::serial_println!("[TRACE] Enabled tracepoint {}:{}", category, name);
        }
    }
    
    pub fn disable_tracepoint(&self, category: &str, name: &str) {
        let key = format!("{}:{}", category, name);
        if let Some(tp) = self.tracepoints.lock().get(&key) {
            tp.enabled.store(false, Ordering::SeqCst);
            crate::serial_println!("[TRACE] Disabled tracepoint {}:{}", category, name);
        }
    }
    
    pub fn trace_event(&self, category: &str, name: &str, data: TraceData) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        // Check if tracepoint is enabled
        let key = format!("{}:{}", category, name);
        let tracepoints = self.tracepoints.lock();
        if let Some(tp) = tracepoints.get(&key) {
            if !tp.enabled.load(Ordering::Relaxed) {
                return;
            }
            tp.hit_count.fetch_add(1, Ordering::Relaxed);
        }
        drop(tracepoints);
        
        // Check filters
        if !self.should_trace(category, name) {
            return;
        }
        
        // Create trace event
        let event = TraceEvent {
            timestamp: self.get_timestamp(),
            cpu_id: self.get_cpu_id(),
            pid: self.get_current_pid(),
            event_id: self.get_next_event_id(),
            category: category.to_string(),
            name: name.to_string(),
            data,
            stack_depth: self.get_stack_depth(),
        };
        
        // Add to ring buffer
        self.event_buffer.lock().push(event.clone());
        
        // Update statistics
        self.statistics.total_events.fetch_add(1, Ordering::Relaxed);
        self.statistics.update_category_stats(&event.category, 1);
    }
    
    fn should_trace(&self, category: &str, name: &str) -> bool {
        let filters = self.filters.lock();
        
        if filters.is_empty() {
            return true;  // No filters = trace everything
        }
        
        for filter in filters.iter() {
            if !filter.enabled {
                continue;
            }
            
            let category_match = filter.category.as_ref()
                .map_or(true, |c| c == category);
            let name_match = filter.name.as_ref()
                .map_or(true, |n| n == name);
            
            if category_match && name_match {
                return true;
            }
        }
        
        false
    }
    
    pub fn add_filter(&self, filter: TraceFilter) {
        self.filters.lock().push(filter);
        crate::serial_println!("[TRACE] Filter added");
    }
    
    pub fn clear_filters(&self) {
        self.filters.lock().clear();
        crate::serial_println!("[TRACE] All filters cleared");
    }
    
    pub fn dump_buffer(&self) -> Vec<TraceEvent> {
        self.event_buffer.lock().dump()
    }
    
    pub fn clear_buffer(&self) {
        self.event_buffer.lock().clear();
        crate::serial_println!("[TRACE] Buffer cleared");
    }
    
    pub fn print_trace(&self, limit: usize) {
        let events = self.dump_buffer();
        let start = events.len().saturating_sub(limit);
        
        crate::serial_println!("\n=== Trace Events (last {}) ===", limit);
        
        for event in &events[start..] {
            crate::serial_println!("[{:12}] CPU{} PID{:5} {}:{} {}",
                event.timestamp,
                event.cpu_id,
                event.pid,
                event.category,
                event.name,
                format_trace_data(&event.data)
            );
        }
        
        let buffer = self.event_buffer.lock();
        if buffer.overrun_count.load(Ordering::Relaxed) > 0 {
            crate::serial_println!("\nWarning: {} events lost due to buffer overrun",
                buffer.overrun_count.load(Ordering::Relaxed));
        }
    }
    
    pub fn print_statistics(&self) {
        crate::serial_println!("\n=== Trace Statistics ===");
        crate::serial_println!("Total events: {}", 
            self.statistics.total_events.load(Ordering::Relaxed));
        crate::serial_println!("Dropped events: {}",
            self.statistics.dropped_events.load(Ordering::Relaxed));
        
        crate::serial_println!("\nEvents by category:");
        let categories = self.statistics.categories.lock();
        for (name, stats) in categories.iter() {
            crate::serial_println!("  {}: {} events", name, stats.event_count);
        }
        
        crate::serial_println!("\nTracepoint hit counts:");
        let tracepoints = self.tracepoints.lock();
        for (key, tp) in tracepoints.iter() {
            let hits = tp.hit_count.load(Ordering::Relaxed);
            if hits > 0 {
                crate::serial_println!("  {}: {} hits", key, hits);
            }
        }
    }
    
    fn get_timestamp(&self) -> u64 {
        unsafe { core::arch::x86_64::_rdtsc() }
    }
    
    fn get_cpu_id(&self) -> u32 {
        0  // Would get actual CPU ID
    }
    
    fn get_current_pid(&self) -> u32 {
        0  // Would get current process ID
    }
    
    fn get_next_event_id(&self) -> u32 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed) as u32
    }
    
    fn get_stack_depth(&self) -> u32 {
        let mut depth = 0;
        let mut rbp: u64;
        
        unsafe {
            core::arch::asm!("mov {}, rbp", out(reg) rbp);
        }
        
        while rbp != 0 && rbp > 0x1000 && depth < 32 {
            unsafe {
                rbp = *(rbp as *const u64);
            }
            depth += 1;
        }
        
        depth
    }
}

impl RingBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            overrun_count: AtomicU64::new(0),
        }
    }
    
    fn push(&mut self, event: TraceEvent) {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        
        let next_head = (head + 1) % self.capacity;
        
        if next_head == tail {
            // Buffer full, overwrite oldest
            self.overrun_count.fetch_add(1, Ordering::Relaxed);
            self.tail.store((tail + 1) % self.capacity, Ordering::Release);
        }
        
        if self.buffer.len() <= head {
            self.buffer.resize(head + 1, TraceEvent::default());
        }
        
        self.buffer[head] = event;
        self.head.store(next_head, Ordering::Release);
    }
    
    fn dump(&self) -> Vec<TraceEvent> {
        let mut result = Vec::new();
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        
        let mut idx = tail;
        while idx != head {
            if idx < self.buffer.len() {
                result.push(self.buffer[idx].clone());
            }
            idx = (idx + 1) % self.capacity;
        }
        
        result
    }
    
    fn clear(&mut self) {
        self.head.store(0, Ordering::Release);
        self.tail.store(0, Ordering::Release);
        self.overrun_count.store(0, Ordering::Release);
        self.buffer.clear();
    }
}

impl TraceStatistics {
    fn new() -> Self {
        Self {
            total_events: AtomicU64::new(0),
            dropped_events: AtomicU64::new(0),
            categories: Mutex::new(BTreeMap::new()),
        }
    }
    
    fn update_category_stats(&self, category: &str, count: u64) {
        let mut categories = self.categories.lock();
        let stats = categories.entry(category.to_string())
            .or_insert_with(|| CategoryStats {
                event_count: 0,
                total_size: 0,
            });
        stats.event_count += count;
    }
}

impl Default for TraceEvent {
    fn default() -> Self {
        Self {
            timestamp: 0,
            cpu_id: 0,
            pid: 0,
            event_id: 0,
            category: String::new(),
            name: String::new(),
            data: TraceData::None,
            stack_depth: 0,
        }
    }
}

fn format_trace_data(data: &TraceData) -> String {
    match data {
        TraceData::None => String::new(),
        TraceData::U64(v) => format!("value={}", v),
        TraceData::I64(v) => format!("value={}", v),
        TraceData::String(s) => s.clone(),
        TraceData::Binary(b) => format!("binary[{}]", b.len()),
        TraceData::Structured(map) => {
            let items: Vec<String> = map.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            items.join(" ")
        }
    }
}

// Trace macros for easy instrumentation
#[macro_export]
macro_rules! trace {
    ($category:expr, $name:expr) => {
        $crate::debug::trace::record_event($category, $name, "");
    };
    ($category:expr, $name:expr, $data:expr) => {
        $crate::debug::trace::record_event($category, $name, $data);
    };
}

#[macro_export]
macro_rules! trace_fn {
    () => {
        let _trace_guard = $crate::debug::trace::FunctionTracer::new(
            module_path!(),
            function_name!()
        );
    };
}

// Function tracer for automatic entry/exit tracing
pub struct FunctionTracer {
    category: String,
    name: String,
    start_time: u64,
}

impl FunctionTracer {
    pub fn new(category: &str, name: &str) -> Self {
        let start_time = unsafe { core::arch::x86_64::_rdtsc() };
        
        TRACE.trace_event(category, &format!("{}_enter", name), TraceData::None);
        
        Self {
            category: category.to_string(),
            name: name.to_string(),
            start_time,
        }
    }
}

impl Drop for FunctionTracer {
    fn drop(&mut self) {
        let end_time = unsafe { core::arch::x86_64::_rdtsc() };
        let duration = end_time - self.start_time;
        
        TRACE.trace_event(&self.category, &format!("{}_exit", self.name), 
            TraceData::U64(duration));
    }
}

// Public API
pub fn init() {
    TRACE.init();
}

pub fn record_event(category: &str, name: &str, data: &str) {
    TRACE.trace_event(category, name, TraceData::String(data.to_string()));
}

pub fn trace_value(category: &str, name: &str, value: u64) {
    TRACE.trace_event(category, name, TraceData::U64(value));
}

pub fn enable_category(category: &str) {
    TRACE.add_filter(TraceFilter {
        category: Some(category.to_string()),
        name: None,
        pid: None,
        cpu: None,
        enabled: true,
    });
}

pub fn print_trace(limit: usize) {
    TRACE.print_trace(limit);
}

pub fn print_statistics() {
    TRACE.print_statistics();
}