use x86_64::instructions::port::Port;
use spin::Mutex;
use lazy_static::lazy_static;
use raw_cpuid::CpuId;

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

// Get TSC frequency (approximate)
pub fn get_tsc_frequency() -> u64 {
    // This is a rough estimate - in production you'd calibrate this
    2_000_000_000 // 2GHz default
}