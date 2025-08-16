// System Profiling Tools
// Performance counters, function profiling, lock analysis, latency tracking

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::instructions::interrupts;

// Performance monitoring
pub struct SystemProfiler {
    enabled: AtomicBool,
    sampling_enabled: AtomicBool,
    function_profiling: Mutex<FunctionProfiler>,
    lock_profiling: Mutex<LockProfiler>,
    interrupt_profiling: Mutex<InterruptProfiler>,
    memory_profiling: Mutex<MemoryProfiler>,
    cpu_profiling: Mutex<CpuProfiler>,
    sample_buffer: Mutex<SampleBuffer>,
}

struct FunctionProfiler {
    functions: BTreeMap<u64, FunctionStats>,
    call_graph: BTreeMap<(u64, u64), u64>,  // (caller, callee) -> count
    active_calls: Vec<FunctionCall>,
}

struct FunctionStats {
    name: String,
    call_count: u64,
    total_time: u64,
    min_time: u64,
    max_time: u64,
    self_time: u64,
}

struct FunctionCall {
    address: u64,
    start_time: u64,
    parent: Option<u64>,
}

struct LockProfiler {
    locks: BTreeMap<u64, LockStats>,
    contention_events: Vec<ContentionEvent>,
}

struct LockStats {
    name: String,
    acquisitions: u64,
    contended: u64,
    total_wait_time: u64,
    max_wait_time: u64,
    total_hold_time: u64,
    max_hold_time: u64,
}

struct ContentionEvent {
    lock_addr: u64,
    waiter: u64,
    holder: u64,
    wait_time: u64,
    timestamp: u64,
}

struct InterruptProfiler {
    interrupts: [InterruptStats; 256],
    total_interrupts: u64,
}

struct InterruptStats {
    count: u64,
    total_time: u64,
    max_time: u64,
    min_time: u64,
}

struct MemoryProfiler {
    allocations: BTreeMap<u64, AllocationStats>,
    allocation_sites: BTreeMap<u64, SiteStats>,
    current_usage: u64,
    peak_usage: u64,
}

struct AllocationStats {
    size: usize,
    caller: u64,
    timestamp: u64,
}

struct SiteStats {
    allocations: u64,
    total_size: u64,
    current_size: u64,
    peak_size: u64,
}

struct CpuProfiler {
    samples: Vec<CpuSample>,
    cpu_usage: [f32; 256],  // Per-CPU usage percentage
    idle_time: [u64; 256],
    busy_time: [u64; 256],
}

struct CpuSample {
    timestamp: u64,
    cpu_id: u32,
    rip: u64,
    rsp: u64,
    in_kernel: bool,
}

struct SampleBuffer {
    samples: Vec<ProfileSample>,
    max_samples: usize,
    overflow_count: u64,
}

struct ProfileSample {
    timestamp: u64,
    cpu_id: u32,
    rip: u64,
    stack_trace: Vec<u64>,
    event_type: SampleEventType,
}

#[derive(Debug, Clone, Copy)]
enum SampleEventType {
    Timer,
    CacheMiss,
    BranchMiss,
    PageFault,
    ContextSwitch,
}

lazy_static! {
    pub static ref PROFILER: SystemProfiler = SystemProfiler::new();
}

impl SystemProfiler {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            sampling_enabled: AtomicBool::new(false),
            function_profiling: Mutex::new(FunctionProfiler::new()),
            lock_profiling: Mutex::new(LockProfiler::new()),
            interrupt_profiling: Mutex::new(InterruptProfiler::new()),
            memory_profiling: Mutex::new(MemoryProfiler::new()),
            cpu_profiling: Mutex::new(CpuProfiler::new()),
            sample_buffer: Mutex::new(SampleBuffer::new(100000)),
        }
    }
    
    pub fn init(&self) {
        // Initialize performance monitoring
        self.init_performance_counters();
        
        self.enabled.store(true, Ordering::SeqCst);
        crate::serial_println!("[PROFILER] System profiler initialized");
    }
    
    fn init_performance_counters(&self) {
        // Initialize CPU performance counters
        unsafe {
            // Enable performance monitoring
            core::arch::asm!(
                "mov rcx, 0xC0000080",  // EFER MSR
                "rdmsr",
                "or eax, 0x800",        // Enable SYSCALL
                "wrmsr",
                out("eax") _,
                out("edx") _,
                out("rcx") _,
            );
            
            // Configure performance counters
            // Would set up PMC0-PMC3 for various events
        }
        
        crate::serial_println!("[PROFILER] Performance counters configured");
    }
    
    pub fn start_sampling(&self, frequency_hz: u32) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        self.sampling_enabled.store(true, Ordering::SeqCst);
        
        // Set up timer interrupt for sampling
        // Would configure APIC timer for periodic sampling
        
        crate::serial_println!("[PROFILER] Sampling started at {} Hz", frequency_hz);
    }
    
    pub fn stop_sampling(&self) {
        self.sampling_enabled.store(false, Ordering::SeqCst);
        crate::serial_println!("[PROFILER] Sampling stopped");
    }
    
    pub fn profile_function_enter(&self, addr: u64, name: &str) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let timestamp = self.get_timestamp();
        let mut profiler = self.function_profiling.lock();
        
        // Record function entry
        profiler.active_calls.push(FunctionCall {
            address: addr,
            start_time: timestamp,
            parent: profiler.active_calls.last().map(|c| c.address),
        });
        
        // Update call graph
        if let Some(parent) = profiler.active_calls.iter().rev().nth(1) {
            *profiler.call_graph.entry((parent.address, addr)).or_insert(0) += 1;
        }
        
        // Initialize stats if needed
        profiler.functions.entry(addr).or_insert_with(|| FunctionStats {
            name: name.to_string(),
            call_count: 0,
            total_time: 0,
            min_time: u64::MAX,
            max_time: 0,
            self_time: 0,
        });
    }
    
    pub fn profile_function_exit(&self, addr: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let timestamp = self.get_timestamp();
        let mut profiler = self.function_profiling.lock();
        
        // Find and remove the matching call
        if let Some(pos) = profiler.active_calls.iter().rposition(|c| c.address == addr) {
            let call = profiler.active_calls.remove(pos);
            let duration = timestamp - call.start_time;
            
            // Update function stats
            if let Some(stats) = profiler.functions.get_mut(&addr) {
                stats.call_count += 1;
                stats.total_time += duration;
                stats.min_time = stats.min_time.min(duration);
                stats.max_time = stats.max_time.max(duration);
            }
        }
    }
    
    pub fn profile_lock_acquire(&self, lock_addr: u64, name: &str, wait_time: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let mut profiler = self.lock_profiling.lock();
        
        let stats = profiler.locks.entry(lock_addr).or_insert_with(|| LockStats {
            name: name.to_string(),
            acquisitions: 0,
            contended: 0,
            total_wait_time: 0,
            max_wait_time: 0,
            total_hold_time: 0,
            max_hold_time: 0,
        });
        
        stats.acquisitions += 1;
        if wait_time > 0 {
            stats.contended += 1;
            stats.total_wait_time += wait_time;
            stats.max_wait_time = stats.max_wait_time.max(wait_time);
            
            // Record contention event
            if profiler.contention_events.len() < 1000 {
                profiler.contention_events.push(ContentionEvent {
                    lock_addr,
                    waiter: self.get_current_thread(),
                    holder: 0,  // Would get actual holder
                    wait_time,
                    timestamp: self.get_timestamp(),
                });
            }
        }
    }
    
    pub fn profile_lock_release(&self, lock_addr: u64, hold_time: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let mut profiler = self.lock_profiling.lock();
        
        if let Some(stats) = profiler.locks.get_mut(&lock_addr) {
            stats.total_hold_time += hold_time;
            stats.max_hold_time = stats.max_hold_time.max(hold_time);
        }
    }
    
    pub fn profile_interrupt(&self, vector: u8, duration: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let mut profiler = self.interrupt_profiling.lock();
        let stats = &mut profiler.interrupts[vector as usize];
        
        stats.count += 1;
        stats.total_time += duration;
        stats.max_time = stats.max_time.max(duration);
        stats.min_time = if stats.min_time == 0 {
            duration
        } else {
            stats.min_time.min(duration)
        };
        
        profiler.total_interrupts += 1;
    }
    
    pub fn profile_allocation(&self, addr: u64, size: usize, caller: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let mut profiler = self.memory_profiling.lock();
        
        // Track allocation
        profiler.allocations.insert(addr, AllocationStats {
            size,
            caller,
            timestamp: self.get_timestamp(),
        });
        
        // Update site statistics
        let site_stats = profiler.allocation_sites.entry(caller)
            .or_insert_with(|| SiteStats {
                allocations: 0,
                total_size: 0,
                current_size: 0,
                peak_size: 0,
            });
        
        site_stats.allocations += 1;
        site_stats.total_size += size as u64;
        site_stats.current_size += size as u64;
        site_stats.peak_size = site_stats.peak_size.max(site_stats.current_size);
        
        // Update global stats
        profiler.current_usage += size as u64;
        profiler.peak_usage = profiler.peak_usage.max(profiler.current_usage);
    }
    
    pub fn profile_free(&self, addr: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let mut profiler = self.memory_profiling.lock();
        
        if let Some(alloc) = profiler.allocations.remove(&addr) {
            // Update site statistics
            if let Some(site_stats) = profiler.allocation_sites.get_mut(&alloc.caller) {
                site_stats.current_size = site_stats.current_size.saturating_sub(alloc.size as u64);
            }
            
            // Update global stats
            profiler.current_usage = profiler.current_usage.saturating_sub(alloc.size as u64);
        }
    }
    
    pub fn sample_cpu(&self) {
        if !self.sampling_enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let mut rip: u64;
        let mut rsp: u64;
        
        unsafe {
            core::arch::asm!(
                "lea {}, [rip]",
                "mov {}, rsp",
                out(reg) rip,
                out(reg) rsp,
            );
        }
        
        let sample = CpuSample {
            timestamp: self.get_timestamp(),
            cpu_id: self.get_cpu_id(),
            rip,
            rsp,
            in_kernel: rip >= 0xFFFF800000000000,  // Kernel space check
        };
        
        self.cpu_profiling.lock().samples.push(sample);
        
        // Collect stack trace for sample buffer
        let stack_trace = self.capture_stack_trace();
        self.add_sample(ProfileSample {
            timestamp: self.get_timestamp(),
            cpu_id: self.get_cpu_id(),
            rip,
            stack_trace,
            event_type: SampleEventType::Timer,
        });
    }
    
    fn add_sample(&self, sample: ProfileSample) {
        let mut buffer = self.sample_buffer.lock();
        
        if buffer.samples.len() >= buffer.max_samples {
            buffer.overflow_count += 1;
            // Overwrite oldest sample
            buffer.samples[0] = sample;
        } else {
            buffer.samples.push(sample);
        }
    }
    
    fn capture_stack_trace(&self) -> Vec<u64> {
        let mut trace = Vec::new();
        let mut rbp: u64;
        
        unsafe {
            core::arch::asm!("mov {}, rbp", out(reg) rbp);
        }
        
        for _ in 0..16 {
            if rbp == 0 || rbp < 0x1000 {
                break;
            }
            
            unsafe {
                let return_addr = *((rbp + 8) as *const u64);
                if return_addr == 0 {
                    break;
                }
                trace.push(return_addr);
                rbp = *(rbp as *const u64);
            }
        }
        
        trace
    }
    
    pub fn generate_report(&self) -> ProfileReport {
        let functions = self.function_profiling.lock();
        let locks = self.lock_profiling.lock();
        let interrupts = self.interrupt_profiling.lock();
        let memory = self.memory_profiling.lock();
        let cpu = self.cpu_profiling.lock();
        let samples = self.sample_buffer.lock();
        
        ProfileReport {
            function_stats: functions.functions.values().cloned().collect(),
            lock_stats: locks.locks.values().cloned().collect(),
            interrupt_stats: interrupts.interrupts.to_vec(),
            memory_stats: MemoryStats {
                current_usage: memory.current_usage,
                peak_usage: memory.peak_usage,
                allocation_count: memory.allocations.len() as u64,
            },
            cpu_stats: CpuStats {
                sample_count: cpu.samples.len() as u64,
                kernel_samples: cpu.samples.iter().filter(|s| s.in_kernel).count() as u64,
                user_samples: cpu.samples.iter().filter(|s| !s.in_kernel).count() as u64,
            },
            sample_overflow: samples.overflow_count,
        }
    }
    
    pub fn print_report(&self) {
        let report = self.generate_report();
        
        crate::serial_println!("\n=== Profiling Report ===\n");
        
        // Function profiling
        crate::serial_println!("Top Functions by Time:");
        let mut functions = report.function_stats;
        functions.sort_by_key(|f| f.total_time);
        functions.reverse();
        
        for (i, func) in functions.iter().take(10).enumerate() {
            crate::serial_println!("  {}. {} - {} calls, {} us total, {} us avg",
                i + 1, func.name, func.call_count, 
                func.total_time, func.total_time / func.call_count.max(1));
        }
        
        // Lock contention
        crate::serial_println!("\nLock Contention:");
        let mut locks = report.lock_stats;
        locks.sort_by_key(|l| l.contended);
        locks.reverse();
        
        for lock in locks.iter().take(5) {
            if lock.contended > 0 {
                crate::serial_println!("  {} - {} contentions, {} us total wait",
                    lock.name, lock.contended, lock.total_wait_time);
            }
        }
        
        // Memory usage
        crate::serial_println!("\nMemory Usage:");
        crate::serial_println!("  Current: {} KB", report.memory_stats.current_usage / 1024);
        crate::serial_println!("  Peak: {} KB", report.memory_stats.peak_usage / 1024);
        crate::serial_println!("  Active allocations: {}", report.memory_stats.allocation_count);
        
        // CPU usage
        crate::serial_println!("\nCPU Samples:");
        crate::serial_println!("  Total: {}", report.cpu_stats.sample_count);
        crate::serial_println!("  Kernel: {} ({}%)", 
            report.cpu_stats.kernel_samples,
            report.cpu_stats.kernel_samples * 100 / report.cpu_stats.sample_count.max(1));
        crate::serial_println!("  User: {} ({}%)",
            report.cpu_stats.user_samples,
            report.cpu_stats.user_samples * 100 / report.cpu_stats.sample_count.max(1));
    }
    
    fn get_timestamp(&self) -> u64 {
        // Read TSC (Time Stamp Counter)
        unsafe {
            core::arch::x86_64::_rdtsc()
        }
    }
    
    fn get_cpu_id(&self) -> u32 {
        // Would get actual CPU ID
        0
    }
    
    fn get_current_thread(&self) -> u64 {
        // Would get current thread ID
        0
    }
}

impl FunctionProfiler {
    fn new() -> Self {
        Self {
            functions: BTreeMap::new(),
            call_graph: BTreeMap::new(),
            active_calls: Vec::new(),
        }
    }
}

impl LockProfiler {
    fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            contention_events: Vec::new(),
        }
    }
}

impl InterruptProfiler {
    fn new() -> Self {
        Self {
            interrupts: [InterruptStats {
                count: 0,
                total_time: 0,
                max_time: 0,
                min_time: 0,
            }; 256],
            total_interrupts: 0,
        }
    }
}

impl MemoryProfiler {
    fn new() -> Self {
        Self {
            allocations: BTreeMap::new(),
            allocation_sites: BTreeMap::new(),
            current_usage: 0,
            peak_usage: 0,
        }
    }
}

impl CpuProfiler {
    fn new() -> Self {
        Self {
            samples: Vec::new(),
            cpu_usage: [0.0; 256],
            idle_time: [0; 256],
            busy_time: [0; 256],
        }
    }
}

impl SampleBuffer {
    fn new(max_samples: usize) -> Self {
        Self {
            samples: Vec::with_capacity(max_samples),
            max_samples,
            overflow_count: 0,
        }
    }
}

#[derive(Clone)]
pub struct ProfileReport {
    pub function_stats: Vec<FunctionStats>,
    pub lock_stats: Vec<LockStats>,
    pub interrupt_stats: Vec<InterruptStats>,
    pub memory_stats: MemoryStats,
    pub cpu_stats: CpuStats,
    pub sample_overflow: u64,
}

#[derive(Clone)]
pub struct MemoryStats {
    pub current_usage: u64,
    pub peak_usage: u64,
    pub allocation_count: u64,
}

#[derive(Clone)]
pub struct CpuStats {
    pub sample_count: u64,
    pub kernel_samples: u64,
    pub user_samples: u64,
}

// Public API
pub fn init() {
    PROFILER.init();
}

pub fn begin_measurement(name: &str) -> u64 {
    let timestamp = PROFILER.get_timestamp();
    let addr = timestamp;  // Use timestamp as unique ID
    PROFILER.profile_function_enter(addr, name);
    addr
}

pub fn end_measurement(handle: u64) {
    PROFILER.profile_function_exit(handle);
}

pub fn start_sampling(frequency_hz: u32) {
    PROFILER.start_sampling(frequency_hz);
}

pub fn stop_sampling() {
    PROFILER.stop_sampling();
}

pub fn print_report() {
    PROFILER.print_report();
}