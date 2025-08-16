use x86_64::instructions::port::Port;
use spin::Mutex;
use lazy_static::lazy_static;
use raw_cpuid::CpuId;
use alloc::collections::BinaryHeap;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering as AtomicOrdering};

// APIC Timer constants
const APIC_BASE: u64 = 0xFEE00000;
const APIC_TIMER_LVT: u32 = 0x320;
const APIC_TIMER_INITIAL_COUNT: u32 = 0x380;
const APIC_TIMER_CURRENT_COUNT: u32 = 0x390;
const APIC_TIMER_DIVIDE_CONFIG: u32 = 0x3E0;

// PIT (Programmable Interval Timer) constants for calibration
const PIT_FREQUENCY: u32 = 1193182; // Hz
const PIT_CHANNEL0_DATA: u16 = 0x40;
const PIT_COMMAND: u16 = 0x43;

lazy_static! {
    pub static ref TIMER: Mutex<Timer> = Mutex::new(Timer::new());
    pub static ref TICKLESS_TIMER: Mutex<TicklessTimer> = Mutex::new(TicklessTimer::new());
}

// Timer event for tickless operation
#[derive(Debug, Clone, Copy)]
pub struct TimerEvent {
    pub deadline: u64,  // TSC cycles or nanoseconds
    pub callback: fn(),
    pub periodic: bool,
    pub period: u64,
    pub id: u64,
}

impl PartialEq for TimerEvent {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline && self.id == other.id
    }
}

impl Eq for TimerEvent {}

impl PartialOrd for TimerEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse ordering for min-heap (earliest deadline first)
        other.deadline.partial_cmp(&self.deadline)
    }
}

impl Ord for TimerEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.deadline.cmp(&self.deadline)
    }
}

// Tickless timer implementation
pub struct TicklessTimer {
    events: BinaryHeap<TimerEvent>,
    next_id: AtomicU64,
    tickless_enabled: AtomicBool,
    min_sleep_ns: u64,
    tsc_frequency: u64,
    hpet_enabled: bool,
    per_cpu_timers: Vec<BinaryHeap<TimerEvent>>,
}

impl TicklessTimer {
    const fn new() -> Self {
        Self {
            events: BinaryHeap::new(),
            next_id: AtomicU64::new(1),
            tickless_enabled: AtomicBool::new(false),
            min_sleep_ns: 1000, // 1 microsecond minimum
            tsc_frequency: 2_000_000_000, // Default 2GHz
            hpet_enabled: false,
            per_cpu_timers: Vec::new(),
        }
    }
    
    pub fn init(&mut self, cpu_count: usize) {
        // Initialize per-CPU timer queues
        for _ in 0..cpu_count {
            self.per_cpu_timers.push(BinaryHeap::new());
        }
        
        // Calibrate TSC frequency
        self.tsc_frequency = calibrate_tsc();
        
        // Try to enable HPET
        if let Ok(()) = init_hpet() {
            self.hpet_enabled = true;
            crate::serial_println!("HPET enabled for high-resolution timers");
        }
        
        self.tickless_enabled.store(true, AtomicOrdering::SeqCst);
        crate::serial_println!("Tickless timer initialized (dynamic ticks)");
    }
    
    pub fn add_event(&mut self, deadline_ns: u64, callback: fn(), periodic: bool, period_ns: u64) -> u64 {
        let id = self.next_id.fetch_add(1, AtomicOrdering::SeqCst);
        let deadline_tsc = self.ns_to_tsc(deadline_ns);
        
        let event = TimerEvent {
            deadline: deadline_tsc,
            callback,
            periodic,
            period: self.ns_to_tsc(period_ns),
            id,
        };
        
        self.events.push(event);
        
        // Reprogram timer if this is the next event
        if let Some(next) = self.events.peek() {
            if next.id == id {
                self.program_next_interrupt();
            }
        }
        
        id
    }
    
    pub fn add_event_cpu(&mut self, cpu_id: usize, deadline_ns: u64, callback: fn()) -> u64 {
        if cpu_id >= self.per_cpu_timers.len() {
            return 0;
        }
        
        let id = self.next_id.fetch_add(1, AtomicOrdering::SeqCst);
        let deadline_tsc = self.ns_to_tsc(deadline_ns);
        
        let event = TimerEvent {
            deadline: deadline_tsc,
            callback,
            periodic: false,
            period: 0,
            id,
        };
        
        self.per_cpu_timers[cpu_id].push(event);
        id
    }
    
    pub fn cancel_event(&mut self, id: u64) -> bool {
        // This would require a more sophisticated data structure
        // For now, events run to completion
        false
    }
    
    pub fn handle_tick(&mut self) {
        if !self.tickless_enabled.load(AtomicOrdering::SeqCst) {
            return;
        }
        
        let current_tsc = rdtsc();
        let cpu_id = crate::cpu::get_cpu_id() as usize;
        
        // Process global events
        while let Some(event) = self.events.peek() {
            if event.deadline <= current_tsc {
                let event = self.events.pop().unwrap();
                (event.callback)();
                
                // Re-add periodic events
                if event.periodic {
                    let mut new_event = event;
                    new_event.deadline = current_tsc + event.period;
                    self.events.push(new_event);
                }
            } else {
                break;
            }
        }
        
        // Process per-CPU events
        if cpu_id < self.per_cpu_timers.len() {
            while let Some(event) = self.per_cpu_timers[cpu_id].peek() {
                if event.deadline <= current_tsc {
                    let event = self.per_cpu_timers[cpu_id].pop().unwrap();
                    (event.callback)();
                } else {
                    break;
                }
            }
        }
        
        // Program next interrupt
        self.program_next_interrupt();
    }
    
    fn program_next_interrupt(&self) {
        let current_tsc = rdtsc();
        let cpu_id = crate::cpu::get_cpu_id() as usize;
        
        // Find next deadline from both global and per-CPU queues
        let mut next_deadline = u64::MAX;
        
        if let Some(event) = self.events.peek() {
            next_deadline = next_deadline.min(event.deadline);
        }
        
        if cpu_id < self.per_cpu_timers.len() {
            if let Some(event) = self.per_cpu_timers[cpu_id].peek() {
                next_deadline = next_deadline.min(event.deadline);
            }
        }
        
        if next_deadline != u64::MAX && next_deadline > current_tsc {
            let delay_tsc = next_deadline - current_tsc;
            let delay_ns = self.tsc_to_ns(delay_tsc);
            
            if delay_ns > self.min_sleep_ns {
                // Program one-shot timer
                if self.hpet_enabled {
                    program_hpet_oneshot(delay_ns);
                } else {
                    program_apic_oneshot(delay_tsc);
                }
            }
        }
    }
    
    fn ns_to_tsc(&self, ns: u64) -> u64 {
        (ns * self.tsc_frequency) / 1_000_000_000
    }
    
    fn tsc_to_ns(&self, tsc: u64) -> u64 {
        (tsc * 1_000_000_000) / self.tsc_frequency
    }
    
    pub fn enable_tickless(&mut self) {
        self.tickless_enabled.store(true, AtomicOrdering::SeqCst);
        crate::serial_println!("Tickless mode enabled");
    }
    
    pub fn disable_tickless(&mut self) {
        self.tickless_enabled.store(false, AtomicOrdering::SeqCst);
        crate::serial_println!("Tickless mode disabled");
    }
}

// Timer coalescing for power efficiency
pub struct TimerCoalescing {
    window_ns: u64,
    slack_ns: u64,
}

impl TimerCoalescing {
    pub fn new(window_ns: u64, slack_ns: u64) -> Self {
        Self { window_ns, slack_ns }
    }
    
    pub fn coalesce_deadline(&self, requested: u64, existing: &[u64]) -> u64 {
        // Find nearby existing timer
        for &deadline in existing {
            if deadline >= requested && deadline - requested <= self.window_ns {
                // Coalesce with existing timer
                return deadline;
            }
            if requested >= deadline && requested - deadline <= self.slack_ns {
                // Can fire early with existing timer
                return deadline;
            }
        }
        requested
    }
}

pub struct Timer {
    ticks_per_second: u64,
    uptime_ticks: u64,
    apic_available: bool,
    apic_frequency: u32,
}

impl Timer {
    const fn new() -> Self {
        Self {
            ticks_per_second: 100, // Default 100Hz
            uptime_ticks: 0,
            apic_available: false,
            apic_frequency: 0,
        }
    }
    
    pub fn init(&mut self) {
        // Check if APIC is available
        if self.detect_apic() {
            self.init_apic_timer();
            crate::serial_println!("Timer: APIC timer initialized at {} Hz", self.apic_frequency);
        } else {
            self.init_pit_timer();
            crate::serial_println!("Timer: PIT timer initialized at {} Hz", self.ticks_per_second);
        }
    }
    
    fn detect_apic(&mut self) -> bool {
        // Check CPUID for APIC support
        use raw_cpuid::CpuId;
        let cpuid = CpuId::new();
        
        if let Some(features) = cpuid.get_feature_info() {
            if features.has_apic() {
                self.apic_available = true;
                return true;
            }
        }
        false
    }
    
    fn init_apic_timer(&mut self) {
        unsafe {
            // Calibrate APIC timer using PIT
            self.apic_frequency = self.calibrate_apic_timer();
            
            // Configure APIC timer for periodic mode
            let apic_base = APIC_BASE as *mut u32;
            
            // Set divide configuration to 1
            apic_base.add((APIC_TIMER_DIVIDE_CONFIG / 4) as usize).write_volatile(0x0B);
            
            // Set timer LVT entry (vector 32, periodic mode)
            apic_base.add((APIC_TIMER_LVT / 4) as usize).write_volatile(0x20020);
            
            // Set initial count for desired frequency (100Hz)
            let initial_count = self.apic_frequency / 100;
            apic_base.add((APIC_TIMER_INITIAL_COUNT / 4) as usize).write_volatile(initial_count);
            
            self.ticks_per_second = 100;
        }
    }
    
    fn calibrate_apic_timer(&self) -> u32 {
        unsafe {
            let apic_base = APIC_BASE as *mut u32;
            
            // Configure PIT for one-shot mode
            let mut cmd_port = Port::<u8>::new(PIT_COMMAND);
            let mut data_port = Port::<u8>::new(PIT_CHANNEL0_DATA);
            
            // Set PIT to one-shot mode, 10ms delay
            cmd_port.write(0x30); // Channel 0, one-shot mode
            let count = (PIT_FREQUENCY / 100) as u16; // 10ms
            data_port.write((count & 0xFF) as u8);
            data_port.write((count >> 8) as u8);
            
            // Reset APIC timer
            apic_base.add((APIC_TIMER_INITIAL_COUNT / 4) as usize).write_volatile(0xFFFFFFFF);
            
            // Wait for PIT to count down
            while (data_port.read() as u16 | ((data_port.read() as u16) << 8)) > 0 {}
            
            // Read APIC timer count
            let apic_ticks = 0xFFFFFFFF - apic_base.add((APIC_TIMER_CURRENT_COUNT / 4) as usize).read_volatile();
            
            // Calculate frequency (ticks in 10ms * 100 = ticks per second)
            apic_ticks * 100
        }
    }
    
    fn init_pit_timer(&mut self) {
        unsafe {
            let mut cmd_port = Port::<u8>::new(PIT_COMMAND);
            let mut data_port = Port::<u8>::new(PIT_CHANNEL0_DATA);
            
            // Configure PIT for 100Hz (10ms intervals)
            let divisor = (PIT_FREQUENCY / 100) as u16;
            
            cmd_port.write(0x36); // Channel 0, square wave mode
            data_port.write((divisor & 0xFF) as u8);
            data_port.write((divisor >> 8) as u8);
            
            self.ticks_per_second = 100;
        }
    }
    
    pub fn tick(&mut self) {
        self.uptime_ticks += 1;
    }
    
    pub fn get_uptime_ms(&self) -> u64 {
        (self.uptime_ticks * 1000) / self.ticks_per_second
    }
    
    pub fn get_uptime_ticks(&self) -> u64 {
        self.uptime_ticks
    }
    
    pub fn sleep_ms(&self, ms: u64) {
        let start = self.uptime_ticks;
        let ticks_to_wait = (ms * self.ticks_per_second) / 1000;
        
        while self.uptime_ticks - start < ticks_to_wait {
            x86_64::instructions::hlt();
        }
    }
}

// High-resolution timestamp counter
pub fn rdtsc() -> u64 {
    unsafe {
        core::arch::x86_64::_rdtsc()
    }
}

// Get TSC frequency (calibrated)
pub fn get_tsc_frequency() -> u64 {
    calibrate_tsc()
}

// Calibrate TSC frequency using PIT
fn calibrate_tsc() -> u64 {
    unsafe {
        // Use PIT channel 2 for calibration
        let mut cmd_port = Port::<u8>::new(0x43);
        let mut data_port = Port::<u8>::new(0x42);
        let mut gate_port = Port::<u8>::new(0x61);
        
        // Enable PIT channel 2
        let gate = gate_port.read();
        gate_port.write((gate & 0xFC) | 0x01);
        
        // Configure PIT for one-shot mode
        cmd_port.write(0xB0); // Channel 2, one-shot
        
        // Set count for 10ms
        let count = 11932; // ~10ms at 1.193182 MHz
        data_port.write((count & 0xFF) as u8);
        data_port.write((count >> 8) as u8);
        
        // Measure TSC over 10ms
        let start_tsc = rdtsc();
        
        // Wait for PIT to count down
        gate_port.write(gate | 0x01); // Start counting
        while (gate_port.read() & 0x20) == 0 {} // Wait for output
        
        let end_tsc = rdtsc();
        
        // Restore gate
        gate_port.write(gate);
        
        // Calculate frequency (TSC ticks in 10ms * 100 = ticks per second)
        (end_tsc - start_tsc) * 100
    }
}

// HPET (High Precision Event Timer) support
const HPET_BASE: u64 = 0xFED00000;

struct HpetRegisters {
    capabilities: u64,
    configuration: u64,
    interrupt_status: u64,
    _reserved: u64,
    main_counter: u64,
}

fn init_hpet() -> Result<(), &'static str> {
    unsafe {
        let hpet = HPET_BASE as *mut HpetRegisters;
        
        // Check if HPET is present
        if (*hpet).capabilities == 0 || (*hpet).capabilities == u64::MAX {
            return Err("HPET not available");
        }
        
        // Enable HPET
        (*hpet).configuration |= 0x1;
        
        Ok(())
    }
}

fn program_hpet_oneshot(delay_ns: u64) {
    unsafe {
        let hpet = HPET_BASE as *mut HpetRegisters;
        
        // Convert nanoseconds to HPET ticks
        let capabilities = (*hpet).capabilities;
        let period_fs = capabilities >> 32; // Period in femtoseconds
        let ticks = (delay_ns * 1_000_000) / period_fs;
        
        // Program comparator 0 for one-shot
        let comparator0 = (HPET_BASE + 0x100) as *mut u64;
        let current = (*hpet).main_counter;
        *comparator0 = current + ticks;
    }
}

fn program_apic_oneshot(delay_tsc: u64) {
    unsafe {
        const APIC_BASE: u64 = 0xFEE00000;
        const APIC_TIMER_INITIAL: u32 = 0x380;
        const APIC_TIMER_LVT: u32 = 0x320;
        const APIC_TIMER_DIVIDE: u32 = 0x3E0;
        
        let apic = APIC_BASE as *mut u32;
        
        // Set divide value to 1
        apic.add((APIC_TIMER_DIVIDE / 4) as usize).write_volatile(0x0B);
        
        // Set timer mode to one-shot
        apic.add((APIC_TIMER_LVT / 4) as usize).write_volatile(0x20000);
        
        // Set initial count
        let count = (delay_tsc / 100).min(u32::MAX as u64) as u32;
        apic.add((APIC_TIMER_INITIAL / 4) as usize).write_volatile(count);
    }
}