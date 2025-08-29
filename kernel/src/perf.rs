// Performance monitoring and profiling subsystem
// Provides CPU performance counters, profiling, and latency tracking

use core::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::process::{PROCESS_MANAGER, thread::THREAD_MANAGER};

// Performance monitoring counter (PMC) MSRs
const IA32_PERFEVTSEL0: u32 = 0x186;
const IA32_PERFEVTSEL1: u32 = 0x187;
const IA32_PERFEVTSEL2: u32 = 0x188;
const IA32_PERFEVTSEL3: u32 = 0x189;

const IA32_PMC0: u32 = 0xC1;
const IA32_PMC1: u32 = 0xC2;
const IA32_PMC2: u32 = 0xC3;
const IA32_PMC3: u32 = 0xC4;

const IA32_PERF_GLOBAL_CTRL: u32 = 0x38F;
const IA32_PERF_GLOBAL_STATUS: u32 = 0x38E;
const IA32_PERF_GLOBAL_OVF_CTRL: u32 = 0x390;

// Performance event types
#[derive(Debug, Clone, Copy)]
pub enum PerfEvent {
    // Core events
    CpuCycles,
    Instructions,
    CacheReferences,
    CacheMisses,
    BranchInstructions,
    BranchMisses,
    BusCycles,
    StalledCyclesFrontend,
    StalledCyclesBackend,
    
    // Memory events
    L1DCacheLoads,
    L1DCacheLoadMisses,
    L1ICacheLoads,
    L1ICacheLoadMisses,
    LLCLoads,
    LLCLoadMisses,
    DTLBLoads,
    DTLBLoadMisses,
    ITLBLoads,
    ITLBLoadMisses,
    
    // Custom events
    ContextSwitches,
    PageFaults,
    SystemCalls,
    Interrupts,
}

impl PerfEvent {
    fn to_event_select(&self) -> u64 {
        match self {
            PerfEvent::CpuCycles => 0x003C,              // UnHalted Core Cycles
            PerfEvent::Instructions => 0x00C0,           // Instructions Retired
            PerfEvent::CacheReferences => 0x4F2E,        // LLC References
            PerfEvent::CacheMisses => 0x412E,           // LLC Misses
            PerfEvent::BranchInstructions => 0x00C4,     // Branch Instructions Retired
            PerfEvent::BranchMisses => 0x00C5,          // Branch Misses Retired
            PerfEvent::L1DCacheLoads => 0x0151,         // L1D Cache Loads
            PerfEvent::L1DCacheLoadMisses => 0x0251,    // L1D Cache Load Misses
            _ => 0,
        }
    }
}

// Performance counter configuration
#[repr(C)]
struct PerfEventSelect {
    event_select: u8,
    unit_mask: u8,
    usr: bool,
    os: bool,
    edge: bool,
    pc: bool,
    interrupt: bool,
    enable: bool,
    invert: bool,
    counter_mask: u8,
}

impl PerfEventSelect {
    fn to_msr_value(&self) -> u64 {
        let mut value = 0u64;
        value |= self.event_select as u64;
        value |= (self.unit_mask as u64) << 8;
        value |= (self.usr as u64) << 16;
        value |= (self.os as u64) << 17;
        value |= (self.edge as u64) << 18;
        value |= (self.pc as u64) << 19;
        value |= (self.interrupt as u64) << 20;
        value |= (self.enable as u64) << 22;
        value |= (self.invert as u64) << 23;
        value |= (self.counter_mask as u64) << 24;
        value
    }
}

// Performance monitoring unit (PMU) state
pub struct PMU {
    counters: [AtomicU64; 4],
    events: [Option<PerfEvent>; 4],
    enabled: AtomicU32,
}

impl PMU {
    const fn new() -> Self {
        Self {
            counters: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
            events: [None; 4],
            enabled: AtomicU32::new(0),
        }
    }
    
    pub fn init(&mut self) {
        // Enable performance monitoring in CR4
        unsafe {
            let mut cr4: u64;
            core::arch::asm!("mov {}, cr4", out(reg) cr4);
            cr4 |= 1 << 8; // Set PCE (Performance Counter Enable)
            core::arch::asm!("mov cr4, {}", in(reg) cr4);
        }
        
        // Reset all counters
        self.reset_all();
        
        crate::serial_println!("PMU initialized with {} counters", 4);
    }
    
    pub fn configure_counter(&mut self, counter: usize, event: PerfEvent) -> Result<(), &'static str> {
        if counter >= 4 {
            return Err("Invalid counter index");
        }
        
        let event_select = PerfEventSelect {
            event_select: (event.to_event_select() & 0xFF) as u8,
            unit_mask: ((event.to_event_select() >> 8) & 0xFF) as u8,
            usr: true,  // Count user mode
            os: true,   // Count kernel mode
            edge: false,
            pc: false,
            interrupt: false,
            enable: true,
            invert: false,
            counter_mask: 0,
        };
        
        unsafe {
            // Configure event select MSR
            let evtsel_msr = IA32_PERFEVTSEL0 + counter as u32;
            crate::cpu::write_msr(evtsel_msr, event_select.to_msr_value());
            
            // Reset counter
            let pmc_msr = IA32_PMC0 + counter as u32;
            crate::cpu::write_msr(pmc_msr, 0);
        }
        
        self.events[counter] = Some(event);
        self.enabled.fetch_or(1 << counter, Ordering::SeqCst);
        
        Ok(())
    }
    
    pub fn read_counter(&self, counter: usize) -> u64 {
        if counter >= 4 {
            return 0;
        }
        
        unsafe {
            let pmc_msr = IA32_PMC0 + counter as u32;
            crate::cpu::read_msr(pmc_msr)
        }
    }
    
    pub fn start_all(&self) {
        unsafe {
            // Enable all configured counters
            let enabled = self.enabled.load(Ordering::SeqCst) as u64;
            crate::cpu::write_msr(IA32_PERF_GLOBAL_CTRL, enabled);
        }
    }
    
    pub fn stop_all(&self) {
        unsafe {
            // Disable all counters
            crate::cpu::write_msr(IA32_PERF_GLOBAL_CTRL, 0);
        }
    }
    
    pub fn reset_all(&mut self) {
        self.stop_all();
        
        for i in 0..4 {
            unsafe {
                let pmc_msr = IA32_PMC0 + i as u32;
                crate::cpu::write_msr(pmc_msr, 0);
            }
            self.counters[i].store(0, Ordering::SeqCst);
        }
    }
}

lazy_static! {
    pub static ref PMU_INSTANCE: Mutex<PMU> = Mutex::new(PMU::new());
}

// Profiling sample
#[derive(Debug, Clone)]
pub struct ProfileSample {
    pub timestamp: u64,
    pub rip: u64,
    pub rsp: u64,
    pub pid: u32,
    pub tid: u32,
    pub cpu: u32,
    pub in_kernel: bool,
}

// Profiling buffer
pub struct ProfileBuffer {
    samples: Vec<ProfileSample>,
    max_samples: usize,
    current_index: AtomicU32,
    overflow_count: AtomicU32,
}

impl ProfileBuffer {
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: Vec::with_capacity(max_samples),
            max_samples,
            current_index: AtomicU32::new(0),
            overflow_count: AtomicU32::new(0),
        }
    }
    
    pub fn add_sample(&mut self, sample: ProfileSample) {
        let index = self.current_index.fetch_add(1, Ordering::SeqCst) as usize;
        
        if index < self.max_samples {
            if index >= self.samples.len() {
                self.samples.push(sample);
            } else {
                self.samples[index] = sample;
            }
        } else {
            self.overflow_count.fetch_add(1, Ordering::SeqCst);
        }
    }
    
    pub fn get_samples(&self) -> &[ProfileSample] {
        let count = self.current_index.load(Ordering::SeqCst) as usize;
        &self.samples[..count.min(self.samples.len())]
    }
    
    pub fn clear(&mut self) {
        self.samples.clear();
        self.current_index.store(0, Ordering::SeqCst);
        self.overflow_count.store(0, Ordering::SeqCst);
    }
}

lazy_static! {
    pub static ref PROFILE_BUFFER: Mutex<ProfileBuffer> = 
        Mutex::new(ProfileBuffer::new(10000));
}

// Latency tracking
pub struct LatencyTracker {
    pub name: String,
    pub count: AtomicU64,
    pub total_cycles: AtomicU64,
    pub min_cycles: AtomicU64,
    pub max_cycles: AtomicU64,
    pub buckets: [AtomicU64; 16], // Histogram buckets
}

impl LatencyTracker {
    pub fn new(name: String) -> Self {
        Self {
            name,
            count: AtomicU64::new(0),
            total_cycles: AtomicU64::new(0),
            min_cycles: AtomicU64::new(u64::MAX),
            max_cycles: AtomicU64::new(0),
            buckets: [const { AtomicU64::new(0) }; 16],
        }
    }
    
    pub fn record(&self, cycles: u64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.total_cycles.fetch_add(cycles, Ordering::Relaxed);
        self.min_cycles.fetch_min(cycles, Ordering::Relaxed);
        self.max_cycles.fetch_max(cycles, Ordering::Relaxed);
        
        // Update histogram
        let bucket = (64 - cycles.leading_zeros()).min(15) as usize;
        self.buckets[bucket].fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_stats(&self) -> LatencyStats {
        let count = self.count.load(Ordering::Relaxed);
        let total = self.total_cycles.load(Ordering::Relaxed);
        
        LatencyStats {
            count,
            total_cycles: total,
            avg_cycles: if count > 0 { total / count } else { 0 },
            min_cycles: self.min_cycles.load(Ordering::Relaxed),
            max_cycles: self.max_cycles.load(Ordering::Relaxed),
        }
    }
    
    pub fn print_histogram(&self) {
        println!("Latency histogram for {}:", self.name);
        println!("Bucket (2^n cycles) | Count");
        println!("--------------------|-------");
        
        for (i, bucket) in self.buckets.iter().enumerate() {
            let count = bucket.load(Ordering::Relaxed);
            if count > 0 {
                println!("2^{:2} - 2^{:2}        | {}", i, i+1, count);
            }
        }
    }
}

#[derive(Debug)]
pub struct LatencyStats {
    pub count: u64,
    pub total_cycles: u64,
    pub avg_cycles: u64,
    pub min_cycles: u64,
    pub max_cycles: u64,
}

// Global latency trackers
lazy_static! {
    pub static ref CONTEXT_SWITCH_LATENCY: LatencyTracker = 
        LatencyTracker::new(String::from("Context Switch"));
    
    pub static ref SYSCALL_LATENCY: LatencyTracker = 
        LatencyTracker::new(String::from("System Call"));
    
    pub static ref INTERRUPT_LATENCY: LatencyTracker = 
        LatencyTracker::new(String::from("Interrupt"));
    
    pub static ref PAGE_FAULT_LATENCY: LatencyTracker = 
        LatencyTracker::new(String::from("Page Fault"));
}

// Context switch profiling
pub struct ContextSwitchProbe {
    start_tsc: u64,
}

impl ContextSwitchProbe {
    pub fn start() -> Self {
        Self {
            start_tsc: crate::timer::rdtsc(),
        }
    }
    
    pub fn end(self) {
        let end_tsc = crate::timer::rdtsc();
        let cycles = end_tsc - self.start_tsc;
        CONTEXT_SWITCH_LATENCY.record(cycles);
    }
}

// Syscall profiling
pub struct SyscallProbe {
    start_tsc: u64,
    syscall_num: usize,
}

impl SyscallProbe {
    pub fn start(syscall_num: usize) -> Self {
        Self {
            start_tsc: crate::timer::rdtsc(),
            syscall_num,
        }
    }
    
    pub fn end(self) {
        let end_tsc = crate::timer::rdtsc();
        let cycles = end_tsc - self.start_tsc;
        SYSCALL_LATENCY.record(cycles);
    }
}

// Real-time scheduling analysis
pub struct SchedLatency {
    pub wakeup_latency: LatencyTracker,
    pub dispatch_latency: LatencyTracker,
    pub preemption_latency: LatencyTracker,
}

impl SchedLatency {
    pub fn new() -> Self {
        Self {
            wakeup_latency: LatencyTracker::new(String::from("Wakeup")),
            dispatch_latency: LatencyTracker::new(String::from("Dispatch")),
            preemption_latency: LatencyTracker::new(String::from("Preemption")),
        }
    }
}

lazy_static! {
    pub static ref SCHED_LATENCY: SchedLatency = SchedLatency::new();
}

// Print performance summary
pub fn print_performance_summary() {
    println!("\n=== Performance Summary ===\n");
    
    // PMU counters
    let pmu = PMU_INSTANCE.lock();
    println!("Performance Counters:");
    for i in 0..4 {
        if let Some(event) = pmu.events[i] {
            let value = pmu.read_counter(i);
            println!("  {:?}: {}", event, value);
        }
    }
    
    // Latency statistics
    println!("\nLatency Statistics:");
    
    let cs_stats = CONTEXT_SWITCH_LATENCY.get_stats();
    println!("  Context Switch: avg={} cycles, min={}, max={}, count={}", 
        cs_stats.avg_cycles, cs_stats.min_cycles, cs_stats.max_cycles, cs_stats.count);
    
    let syscall_stats = SYSCALL_LATENCY.get_stats();
    println!("  System Call: avg={} cycles, min={}, max={}, count={}", 
        syscall_stats.avg_cycles, syscall_stats.min_cycles, syscall_stats.max_cycles, syscall_stats.count);
    
    let int_stats = INTERRUPT_LATENCY.get_stats();
    println!("  Interrupt: avg={} cycles, min={}, max={}, count={}", 
        int_stats.avg_cycles, int_stats.min_cycles, int_stats.max_cycles, int_stats.count);
    
    let pf_stats = PAGE_FAULT_LATENCY.get_stats();
    println!("  Page Fault: avg={} cycles, min={}, max={}, count={}", 
        pf_stats.avg_cycles, pf_stats.min_cycles, pf_stats.max_cycles, pf_stats.count);
    
    // Scheduling latencies
    println!("\nScheduling Latencies:");
    let wakeup_stats = SCHED_LATENCY.wakeup_latency.get_stats();
    println!("  Wakeup: avg={} cycles", wakeup_stats.avg_cycles);
    
    let dispatch_stats = SCHED_LATENCY.dispatch_latency.get_stats();
    println!("  Dispatch: avg={} cycles", dispatch_stats.avg_cycles);
    
    let preempt_stats = SCHED_LATENCY.preemption_latency.get_stats();
    println!("  Preemption: avg={} cycles", preempt_stats.avg_cycles);
}

// Enable profiling
pub fn enable_profiling(sample_period: u64) {
    let mut pmu = PMU_INSTANCE.lock();
    
    // Configure counter 0 for instruction retired with interrupt on overflow
    pmu.configure_counter(0, PerfEvent::Instructions).unwrap();
    
    // Set sample period
    unsafe {
        crate::cpu::write_msr(IA32_PMC0, -(sample_period as i64) as u64);
    }
    
    pmu.start_all();
    
    crate::serial_println!("Profiling enabled with period {}", sample_period);
}

// Disable profiling
pub fn disable_profiling() {
    let pmu = PMU_INSTANCE.lock();
    pmu.stop_all();
    
    crate::serial_println!("Profiling disabled");
}

// Performance monitoring interrupt handler
pub extern "x86-interrupt" fn pmu_interrupt_handler(
    stack_frame: x86_64::structures::idt::InterruptStackFrame
) {
    // Get current process and thread IDs in a thread-safe manner
    let (pid, tid) = get_current_context();
    
    // Collect sample
    let sample = ProfileSample {
        timestamp: crate::timer::rdtsc(),
        rip: stack_frame.instruction_pointer.as_u64(),
        rsp: stack_frame.stack_pointer.as_u64(),
        pid,
        tid,
        cpu: crate::cpu::get_cpu_id(),
        in_kernel: stack_frame.code_segment == 0x08,
    };
    
    // Add to profile buffer
    if let Some(mut buffer) = PROFILE_BUFFER.try_lock() {
        buffer.add_sample(sample);
    }
    
    // Reset counter for next sample
    unsafe {
        // Clear overflow flag
        crate::cpu::write_msr(IA32_PERF_GLOBAL_OVF_CTRL, 0x1);
        
        // Reset counter with sample period
        let sample_period = 1000000; // Sample every million instructions
        crate::cpu::write_msr(IA32_PMC0, -(sample_period as i64) as u64);
    }
    
    // Send EOI
    crate::interrupts::send_eoi_apic();
}

// Helper function to get current process and thread context
// Returns (PID, TID) in a thread-safe manner
fn get_current_context() -> (u32, u32) {
    // Try to get thread ID first, as it's more specific
    let tid = if let Some(thread_manager) = THREAD_MANAGER.try_lock() {
        thread_manager.get_current_thread()
            .map(|tid| tid.0)
            .unwrap_or(0)
    } else {
        // If we can't lock, return 0 to avoid blocking in interrupt context
        0
    };
    
    // Get process ID
    let pid = if let Some(process_manager) = PROCESS_MANAGER.try_lock() {
        if let Some(current_pid) = process_manager.current_process {
            current_pid.0
        } else {
            // Check if we have a thread and can get its process
            if tid != 0 {
                if let Some(thread_manager) = THREAD_MANAGER.try_lock() {
                    if let Some(thread_id) = thread_manager.get_current_thread() {
                        if let Some(thread) = thread_manager.get_thread(thread_id) {
                            thread.process_id.0
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            }
        }
    } else {
        // If we can't lock, return 0 to avoid blocking in interrupt context
        0
    };
    
    (pid, tid)
}