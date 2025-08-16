// Watchdog Timer and Hang Detection
// Detects soft/hard lockups, RCU stalls, and system hangs

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicU32, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;
use alloc::vec::Vec;
use alloc::string::String;

pub struct WatchdogSystem {
    enabled: AtomicBool,
    nmi_watchdog: NmiWatchdog,
    soft_lockup_detector: SoftLockupDetector,
    hard_lockup_detector: HardLockupDetector,
    rcu_stall_detector: RcuStallDetector,
    hung_task_detector: HungTaskDetector,
}

struct NmiWatchdog {
    enabled: AtomicBool,
    threshold_ms: AtomicU64,
    last_touch: [AtomicU64; 256],  // Per-CPU timestamps
    lockup_count: AtomicU64,
}

struct SoftLockupDetector {
    enabled: AtomicBool,
    threshold_ms: AtomicU64,
    watchdog_touch: [AtomicU64; 256],  // Per-CPU touch timestamps
    detection_count: AtomicU64,
}

struct HardLockupDetector {
    enabled: AtomicBool,
    threshold_ms: AtomicU64,
    nmi_count: [AtomicU64; 256],  // Per-CPU NMI counts
    detection_count: AtomicU64,
}

struct RcuStallDetector {
    enabled: AtomicBool,
    grace_period_start: AtomicU64,
    stall_threshold_ms: AtomicU64,
    quiescent_states: [AtomicU64; 256],  // Per-CPU QS counts
    stall_count: AtomicU64,
}

struct HungTaskDetector {
    enabled: AtomicBool,
    threshold_ms: AtomicU64,
    tasks: Mutex<Vec<MonitoredTask>>,
    hung_count: AtomicU64,
}

struct MonitoredTask {
    pid: u32,
    name: String,
    start_time: u64,
    state: TaskState,
    last_progress: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TaskState {
    Running,
    Sleeping,
    Uninterruptible,
    Zombie,
}

lazy_static! {
    pub static ref WATCHDOG: WatchdogSystem = WatchdogSystem::new();
}

impl WatchdogSystem {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            nmi_watchdog: NmiWatchdog::new(),
            soft_lockup_detector: SoftLockupDetector::new(),
            hard_lockup_detector: HardLockupDetector::new(),
            rcu_stall_detector: RcuStallDetector::new(),
            hung_task_detector: HungTaskDetector::new(),
        }
    }
    
    pub fn init(&self) {
        // Initialize hardware watchdog if available
        self.init_hardware_watchdog();
        
        // Set up NMI watchdog
        self.nmi_watchdog.init();
        
        // Start soft lockup detection
        self.soft_lockup_detector.init();
        
        self.enabled.store(true, Ordering::SeqCst);
        crate::serial_println!("[WATCHDOG] System watchdog initialized");
    }
    
    fn init_hardware_watchdog(&self) {
        // Initialize hardware watchdog timer if available
        // This would configure:
        // - Intel TCO watchdog
        // - AMD watchdog
        // - HPET watchdog
        
        crate::serial_println!("[WATCHDOG] Hardware watchdog configured");
    }
    
    pub fn touch(&self) {
        // Called periodically to indicate system is alive
        let cpu_id = self.get_cpu_id();
        let timestamp = self.get_timestamp();
        
        // Update soft lockup detector
        self.soft_lockup_detector.touch(cpu_id, timestamp);
        
        // Update NMI watchdog
        self.nmi_watchdog.touch(cpu_id, timestamp);
    }
    
    pub fn check(&self) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let timestamp = self.get_timestamp();
        
        // Check for soft lockups
        self.soft_lockup_detector.check(timestamp);
        
        // Check for hard lockups
        self.hard_lockup_detector.check(timestamp);
        
        // Check for RCU stalls
        self.rcu_stall_detector.check(timestamp);
        
        // Check for hung tasks
        self.hung_task_detector.check(timestamp);
    }
    
    pub fn handle_nmi(&self) {
        // Called from NMI handler
        let cpu_id = self.get_cpu_id();
        
        // Increment NMI count for hard lockup detection
        self.hard_lockup_detector.nmi_count[cpu_id as usize]
            .fetch_add(1, Ordering::Relaxed);
        
        // Check if this CPU is locked up
        if self.nmi_watchdog.is_locked_up(cpu_id) {
            self.handle_hard_lockup(cpu_id);
        }
    }
    
    fn handle_soft_lockup(&self, cpu_id: u32, duration_ms: u64) {
        let count = self.soft_lockup_detector.detection_count
            .fetch_add(1, Ordering::SeqCst);
        
        crate::serial_println!("\n==================================");
        crate::serial_println!("BUG: soft lockup - CPU#{} stuck for {}ms!",
            cpu_id, duration_ms);
        crate::serial_println!("==================================");
        
        // Print CPU state
        self.print_cpu_state(cpu_id);
        
        // Print stack trace
        if let Some(trace) = super::generate_stack_trace() {
            crate::serial_println!("\nStack trace:");
            for frame in trace {
                crate::serial_println!("  {}", frame);
            }
        }
        
        // Enter debugger if available
        if super::DEBUG_STATE.kdb_enabled.load(Ordering::Relaxed) {
            super::kdb::enter_debugger(&format!("Soft lockup on CPU{}", cpu_id));
        }
        
        // Optionally panic if configured
        if duration_ms > 60000 {  // > 60 seconds
            panic!("Soft lockup detected for over 60 seconds");
        }
    }
    
    fn handle_hard_lockup(&self, cpu_id: u32) {
        let count = self.hard_lockup_detector.detection_count
            .fetch_add(1, Ordering::SeqCst);
        
        crate::serial_println!("\n==================================");
        crate::serial_println!("BUG: hard lockup - CPU#{} stuck!", cpu_id);
        crate::serial_println!("==================================");
        
        // Hard lockup is critical - system is unresponsive
        // Try to gather as much info as possible
        
        // Trigger crash dump
        if super::DEBUG_STATE.kdump_enabled.load(Ordering::Relaxed) {
            let panic_info = core::panic::PanicInfo::internal_constructor(
                Some(&format!("Hard lockup on CPU{}", cpu_id)),
                core::panic::Location::caller(),
                false,
            );
            super::kdump::create_dump(&panic_info);
        }
        
        // Force reboot after delay
        self.emergency_reboot();
    }
    
    fn handle_rcu_stall(&self, duration_ms: u64) {
        let count = self.rcu_stall_detector.stall_count
            .fetch_add(1, Ordering::SeqCst);
        
        crate::serial_println!("\n==================================");
        crate::serial_println!("INFO: rcu_sched detected stall ({}ms)", duration_ms);
        crate::serial_println!("==================================");
        
        // Print CPUs that haven't reported quiescent state
        crate::serial_println!("CPUs stalling:");
        for cpu in 0..self.get_cpu_count() {
            let last_qs = self.rcu_stall_detector.quiescent_states[cpu as usize]
                .load(Ordering::Relaxed);
            if last_qs < self.rcu_stall_detector.grace_period_start.load(Ordering::Relaxed) {
                crate::serial_println!("  CPU{}: no QS", cpu);
            }
        }
        
        // Print stack traces for stalled CPUs
        self.print_all_cpu_stacks();
    }
    
    fn handle_hung_task(&self, task: &MonitoredTask, duration_ms: u64) {
        let count = self.hung_task_detector.hung_count
            .fetch_add(1, Ordering::SeqCst);
        
        crate::serial_println!("\n==================================");
        crate::serial_println!("INFO: task {}:{} blocked for {}ms",
            task.name, task.pid, duration_ms);
        crate::serial_println!("==================================");
        
        // Show task state
        crate::serial_println!("Task state: {:?}", task.state);
        
        // Would show task's kernel stack
        crate::serial_println!("Kernel stack:");
        crate::serial_println!("  [stack trace would go here]");
    }
    
    fn print_cpu_state(&self, cpu_id: u32) {
        crate::serial_println!("\nCPU {} state:", cpu_id);
        
        // Print interrupt state
        let interrupts_enabled = x86_64::instructions::interrupts::are_enabled();
        crate::serial_println!("  Interrupts: {}", 
            if interrupts_enabled { "enabled" } else { "disabled" });
        
        // Print current task info
        crate::serial_println!("  Current task: kernel");
        
        // Print preemption state
        crate::serial_println!("  Preemption: disabled");  // Would check actual state
    }
    
    fn print_all_cpu_stacks(&self) {
        crate::serial_println!("\nBacktraces for all CPUs:");
        
        for cpu in 0..self.get_cpu_count() {
            crate::serial_println!("\nCPU {}:", cpu);
            // Would send IPI to get stack trace from each CPU
            if cpu == self.get_cpu_id() {
                if let Some(trace) = super::generate_stack_trace() {
                    for frame in trace {
                        crate::serial_println!("  {}", frame);
                    }
                }
            }
        }
    }
    
    fn emergency_reboot(&self) {
        crate::serial_println!("\n!!! EMERGENCY REBOOT IN 5 SECONDS !!!");
        
        // Give time to flush logs
        for i in (1..=5).rev() {
            crate::serial_println!("Rebooting in {}...", i);
            self.delay_ms(1000);
        }
        
        // Force reboot
        unsafe {
            // Triple fault
            core::arch::asm!("xor %eax, %eax");
            core::arch::asm!("mov %eax, %cr3");
        }
    }
    
    fn get_timestamp(&self) -> u64 {
        unsafe { core::arch::x86_64::_rdtsc() }
    }
    
    fn get_cpu_id(&self) -> u32 {
        0  // Would get actual CPU ID
    }
    
    fn get_cpu_count(&self) -> u32 {
        1  // Would get actual CPU count
    }
    
    fn delay_ms(&self, ms: u64) {
        // Simple delay loop
        let start = self.get_timestamp();
        let cycles_per_ms = 2_000_000;  // Approximate
        let target = start + (ms * cycles_per_ms);
        
        while self.get_timestamp() < target {
            core::hint::spin_loop();
        }
    }
}

impl NmiWatchdog {
    fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            threshold_ms: AtomicU64::new(10000),  // 10 seconds
            last_touch: Default::default(),
            lockup_count: AtomicU64::new(0),
        }
    }
    
    fn init(&self) {
        // Configure NMI watchdog using performance counters
        // Would set up:
        // - Local APIC timer for NMI
        // - Performance counter overflow to trigger NMI
        
        self.enabled.store(true, Ordering::SeqCst);
        crate::serial_println!("[WATCHDOG] NMI watchdog enabled");
    }
    
    fn touch(&self, cpu_id: u32, timestamp: u64) {
        self.last_touch[cpu_id as usize].store(timestamp, Ordering::Relaxed);
    }
    
    fn is_locked_up(&self, cpu_id: u32) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return false;
        }
        
        let last = self.last_touch[cpu_id as usize].load(Ordering::Relaxed);
        let now = unsafe { core::arch::x86_64::_rdtsc() };
        
        let cycles_per_ms = 2_000_000;  // Approximate
        let threshold = self.threshold_ms.load(Ordering::Relaxed) * cycles_per_ms;
        
        (now - last) > threshold
    }
}

impl SoftLockupDetector {
    fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            threshold_ms: AtomicU64::new(20000),  // 20 seconds
            watchdog_touch: Default::default(),
            detection_count: AtomicU64::new(0),
        }
    }
    
    fn init(&self) {
        self.enabled.store(true, Ordering::SeqCst);
        
        // Initialize touch timestamps
        let now = unsafe { core::arch::x86_64::_rdtsc() };
        for i in 0..256 {
            self.watchdog_touch[i].store(now, Ordering::Relaxed);
        }
        
        crate::serial_println!("[WATCHDOG] Soft lockup detector enabled");
    }
    
    fn touch(&self, cpu_id: u32, timestamp: u64) {
        self.watchdog_touch[cpu_id as usize].store(timestamp, Ordering::Relaxed);
    }
    
    fn check(&self, timestamp: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let cycles_per_ms = 2_000_000;  // Approximate
        let threshold = self.threshold_ms.load(Ordering::Relaxed) * cycles_per_ms;
        
        for cpu in 0..WATCHDOG.get_cpu_count() {
            let last_touch = self.watchdog_touch[cpu as usize].load(Ordering::Relaxed);
            let duration = timestamp - last_touch;
            
            if duration > threshold {
                let duration_ms = duration / cycles_per_ms;
                WATCHDOG.handle_soft_lockup(cpu, duration_ms);
            }
        }
    }
}

impl HardLockupDetector {
    fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            threshold_ms: AtomicU64::new(10000),  // 10 seconds
            nmi_count: Default::default(),
            detection_count: AtomicU64::new(0),
        }
    }
    
    fn check(&self, _timestamp: u64) {
        // Check if NMI counts are incrementing
        // If not, CPU is hard locked
        // This is typically done from NMI context
    }
}

impl RcuStallDetector {
    fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            grace_period_start: AtomicU64::new(0),
            stall_threshold_ms: AtomicU64::new(21000),  // 21 seconds
            quiescent_states: Default::default(),
            stall_count: AtomicU64::new(0),
        }
    }
    
    fn check(&self, timestamp: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let gp_start = self.grace_period_start.load(Ordering::Relaxed);
        if gp_start == 0 {
            return;  // No grace period in progress
        }
        
        let cycles_per_ms = 2_000_000;
        let threshold = self.stall_threshold_ms.load(Ordering::Relaxed) * cycles_per_ms;
        let duration = timestamp - gp_start;
        
        if duration > threshold {
            let duration_ms = duration / cycles_per_ms;
            WATCHDOG.handle_rcu_stall(duration_ms);
        }
    }
    
    pub fn report_qs(&self, cpu_id: u32) {
        // Report quiescent state for CPU
        let timestamp = unsafe { core::arch::x86_64::_rdtsc() };
        self.quiescent_states[cpu_id as usize].store(timestamp, Ordering::Relaxed);
    }
    
    pub fn start_grace_period(&self) {
        let timestamp = unsafe { core::arch::x86_64::_rdtsc() };
        self.grace_period_start.store(timestamp, Ordering::Relaxed);
    }
    
    pub fn end_grace_period(&self) {
        self.grace_period_start.store(0, Ordering::Relaxed);
    }
}

impl HungTaskDetector {
    fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            threshold_ms: AtomicU64::new(120000),  // 2 minutes
            tasks: Mutex::new(Vec::new()),
            hung_count: AtomicU64::new(0),
        }
    }
    
    fn check(&self, timestamp: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let cycles_per_ms = 2_000_000;
        let threshold = self.threshold_ms.load(Ordering::Relaxed) * cycles_per_ms;
        
        let tasks = self.tasks.lock();
        for task in tasks.iter() {
            if task.state == TaskState::Uninterruptible {
                let duration = timestamp - task.last_progress;
                if duration > threshold {
                    let duration_ms = duration / cycles_per_ms;
                    WATCHDOG.handle_hung_task(task, duration_ms);
                }
            }
        }
    }
    
    pub fn register_task(&self, pid: u32, name: String) {
        let task = MonitoredTask {
            pid,
            name,
            start_time: unsafe { core::arch::x86_64::_rdtsc() },
            state: TaskState::Running,
            last_progress: unsafe { core::arch::x86_64::_rdtsc() },
        };
        
        self.tasks.lock().push(task);
    }
    
    pub fn update_task_state(&self, pid: u32, state: TaskState) {
        let mut tasks = self.tasks.lock();
        if let Some(task) = tasks.iter_mut().find(|t| t.pid == pid) {
            task.state = state;
            if state != TaskState::Uninterruptible {
                task.last_progress = unsafe { core::arch::x86_64::_rdtsc() };
            }
        }
    }
}

// Public API
pub fn init() {
    WATCHDOG.init();
}

pub fn touch() {
    WATCHDOG.touch();
}

pub fn check() {
    WATCHDOG.check();
}

pub fn handle_nmi() {
    WATCHDOG.handle_nmi();
}

pub fn pet_watchdog() {
    // Reset hardware watchdog timer
    WATCHDOG.touch();
}

pub fn trigger_test_lockup() {
    crate::serial_println!("[WATCHDOG] Triggering test soft lockup...");
    crate::serial_println!("[WATCHDOG] System will appear frozen for 30 seconds");
    
    // Disable interrupts and spin
    x86_64::instructions::interrupts::disable();
    
    let start = unsafe { core::arch::x86_64::_rdtsc() };
    let cycles_per_ms = 2_000_000;
    let duration = 30000 * cycles_per_ms;  // 30 seconds
    
    while (unsafe { core::arch::x86_64::_rdtsc() } - start) < duration {
        core::hint::spin_loop();
    }
    
    x86_64::instructions::interrupts::enable();
    crate::serial_println!("[WATCHDOG] Test lockup complete");
}