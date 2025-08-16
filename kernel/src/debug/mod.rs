// Advanced Debugging Infrastructure for Kernel Development

pub mod kdb;        // Interactive kernel debugger
pub mod kdump;      // Crash dump system  
pub mod kasan;      // Kernel Address Sanitizer
pub mod kgdb;       // GDB remote protocol support
pub mod profiler;   // System profiling tools
pub mod trace;      // Tracing infrastructure
pub mod watchdog;   // Watchdog and hang detection
pub mod sysrq;      // Magic SysRq support
pub mod memleak;    // Memory leak detection
pub mod symbols;    // Symbol resolution

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;

// Global debugging state
pub struct DebugState {
    pub kdb_enabled: AtomicBool,
    pub kdump_enabled: AtomicBool,
    pub kasan_enabled: AtomicBool,
    pub profiling_enabled: AtomicBool,
    pub tracing_enabled: AtomicBool,
    pub memleak_detection: AtomicBool,
    pub watchdog_enabled: AtomicBool,
    pub panic_count: AtomicU64,
    pub last_panic_addr: AtomicU64,
}

lazy_static! {
    pub static ref DEBUG_STATE: DebugState = DebugState {
        kdb_enabled: AtomicBool::new(true),
        kdump_enabled: AtomicBool::new(true),
        kasan_enabled: AtomicBool::new(false),
        profiling_enabled: AtomicBool::new(false),
        tracing_enabled: AtomicBool::new(false),
        memleak_detection: AtomicBool::new(false),
        watchdog_enabled: AtomicBool::new(true),
        panic_count: AtomicU64::new(0),
        last_panic_addr: AtomicU64::new(0),
    };
}

// Debug initialization
pub fn init() {
    crate::serial_println!("[DEBUG] Initializing advanced debugging infrastructure");
    
    // Initialize kernel debugger
    if DEBUG_STATE.kdb_enabled.load(Ordering::Relaxed) {
        kdb::init();
    }
    
    // Initialize crash dump system
    if DEBUG_STATE.kdump_enabled.load(Ordering::Relaxed) {
        kdump::init();
    }
    
    // Initialize KASAN if enabled
    if DEBUG_STATE.kasan_enabled.load(Ordering::Relaxed) {
        kasan::init();
    }
    
    // Initialize watchdog
    if DEBUG_STATE.watchdog_enabled.load(Ordering::Relaxed) {
        watchdog::init();
    }
    
    // Initialize symbol resolution
    symbols::init();
    
    // Register SysRq handlers
    sysrq::init();
    
    crate::serial_println!("[DEBUG] Debug infrastructure initialized");
}

// Enhanced panic handler with debugging features
pub fn enhanced_panic_handler(info: &core::panic::PanicInfo) {
    // Increment panic count
    let panic_count = DEBUG_STATE.panic_count.fetch_add(1, Ordering::SeqCst);
    
    // Disable interrupts
    x86_64::instructions::interrupts::disable();
    
    // Print panic header
    crate::serial_println!("\n\n=== KERNEL PANIC #{} ===", panic_count + 1);
    crate::serial_println!("{}", info);
    
    // Generate stack trace
    if let Some(trace) = generate_stack_trace() {
        crate::serial_println!("\nStack Trace:");
        for frame in trace {
            crate::serial_println!("  {}", frame);
        }
    }
    
    // Create crash dump if enabled
    if DEBUG_STATE.kdump_enabled.load(Ordering::Relaxed) {
        kdump::create_dump(info);
    }
    
    // Enter kernel debugger if available
    if DEBUG_STATE.kdb_enabled.load(Ordering::Relaxed) {
        kdb::enter_on_panic(info);
    }
    
    // Halt the system
    loop {
        x86_64::instructions::hlt();
    }
}

// Generate stack trace with symbol resolution
pub fn generate_stack_trace() -> Option<Vec<String>> {
    let mut trace = Vec::new();
    let mut rbp: u64;
    
    unsafe {
        core::arch::asm!("mov {}, rbp", out(reg) rbp);
    }
    
    // Walk the stack
    for _ in 0..32 {  // Limit depth to prevent infinite loops
        if rbp == 0 || rbp < 0x1000 {
            break;
        }
        
        unsafe {
            let return_addr = *(rbp as *const u64).offset(1);
            if return_addr == 0 {
                break;
            }
            
            // Resolve symbol if possible
            if let Some(symbol) = symbols::resolve_address(return_addr) {
                trace.push(format!("  [{:#016x}] {}", return_addr, symbol));
            } else {
                trace.push(format!("  [{:#016x}] <unknown>", return_addr));
            }
            
            // Move to next frame
            rbp = *(rbp as *const u64);
        }
    }
    
    if trace.is_empty() {
        None
    } else {
        Some(trace)
    }
}

// Debug output helpers
#[macro_export]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        if $crate::debug::should_print_debug() {
            $crate::serial_print!("[DEBUG] ");
            $crate::serial_print!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! debug_println {
    () => ($crate::debug_print!("\n"));
    ($($arg:tt)*) => {
        if $crate::debug::should_print_debug() {
            $crate::serial_print!("[DEBUG] ");
            $crate::serial_println!($($arg)*);
        }
    };
}

pub fn should_print_debug() -> bool {
    // Check if debug output is enabled
    true  // Can be configured via kernel parameters
}

// Memory debugging helpers
pub fn check_memory_corruption(addr: u64, size: usize) -> bool {
    if DEBUG_STATE.kasan_enabled.load(Ordering::Relaxed) {
        kasan::check_memory_access(addr, size, false)
    } else {
        true  // Assume OK if KASAN disabled
    }
}

// Profiling helpers
pub fn profile_begin(name: &str) -> u64 {
    if DEBUG_STATE.profiling_enabled.load(Ordering::Relaxed) {
        profiler::begin_measurement(name)
    } else {
        0
    }
}

pub fn profile_end(handle: u64) {
    if DEBUG_STATE.profiling_enabled.load(Ordering::Relaxed) {
        profiler::end_measurement(handle);
    }
}

// Tracing helpers
pub fn trace_event(category: &str, name: &str, data: &str) {
    if DEBUG_STATE.tracing_enabled.load(Ordering::Relaxed) {
        trace::record_event(category, name, data);
    }
}

// Assert with debugging info
#[macro_export]
macro_rules! debug_assert_msg {
    ($cond:expr, $msg:expr) => {
        if cfg!(debug_assertions) && !$cond {
            $crate::serial_println!("ASSERTION FAILED: {}", $msg);
            $crate::serial_println!("  at {}:{}", file!(), line!());
            if $crate::debug::DEBUG_STATE.kdb_enabled.load(core::sync::atomic::Ordering::Relaxed) {
                $crate::debug::kdb::enter_debugger("Assertion failed");
            }
            panic!("Assertion failed: {}", $msg);
        }
    };
}