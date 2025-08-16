// KASAN - Kernel Address Sanitizer
// Runtime memory error detection: buffer overflows, use-after-free, etc.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;

// KASAN shadow memory constants
const KASAN_SHADOW_SCALE: usize = 8;  // 1 shadow byte per 8 bytes
const KASAN_SHADOW_OFFSET: u64 = 0xdffffc0000000000;  // Shadow memory base

// Shadow byte values
const KASAN_FREE: u8 = 0x00;           // Accessible memory
const KASAN_PARTIAL: u8 = 0x01;        // Partially accessible (1-7 bytes)
const KASAN_REDZONE: u8 = 0xFA;        // Redzone (buffer boundary)
const KASAN_FREED: u8 = 0xFB;          // Freed memory
const KASAN_STACK_LEFT: u8 = 0xF1;     // Stack left redzone
const KASAN_STACK_MID: u8 = 0xF2;      // Stack middle redzone
const KASAN_STACK_RIGHT: u8 = 0xF3;    // Stack right redzone
const KASAN_STACK_PARTIAL: u8 = 0xF4;  // Stack partial redzone
const KASAN_GLOBAL_REDZONE: u8 = 0xF9; // Global variable redzone
const KASAN_VMALLOC: u8 = 0xF8;        // VMalloc memory
const KASAN_ALLOCA_LEFT: u8 = 0xCA;    // Alloca left redzone
const KASAN_ALLOCA_RIGHT: u8 = 0xCB;   // Alloca right redzone

pub struct KasanState {
    enabled: AtomicBool,
    quarantine_enabled: AtomicBool,
    error_count: AtomicU64,
    allocation_map: Mutex<BTreeMap<u64, AllocationInfo>>,
    quarantine: Mutex<Vec<QuarantineEntry>>,
    stack_depot: Mutex<StackDepot>,
}

#[derive(Clone)]
struct AllocationInfo {
    address: u64,
    size: usize,
    stack_trace: Vec<u64>,
    freed: bool,
    free_stack: Option<Vec<u64>>,
}

struct QuarantineEntry {
    address: u64,
    size: usize,
    free_time: u64,
}

struct StackDepot {
    stacks: Vec<Vec<u64>>,
    hash_map: BTreeMap<u64, usize>,
}

lazy_static! {
    pub static ref KASAN: KasanState = KasanState::new();
}

impl KasanState {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            quarantine_enabled: AtomicBool::new(true),
            error_count: AtomicU64::new(0),
            allocation_map: Mutex::new(BTreeMap::new()),
            quarantine: Mutex::new(Vec::new()),
            stack_depot: Mutex::new(StackDepot::new()),
        }
    }
    
    pub fn init(&self) {
        // Initialize shadow memory
        self.init_shadow_memory();
        
        self.enabled.store(true, Ordering::SeqCst);
        crate::serial_println!("[KASAN] Kernel Address Sanitizer initialized");
        crate::serial_println!("[KASAN] Shadow memory at {:#x}", KASAN_SHADOW_OFFSET);
    }
    
    fn init_shadow_memory(&self) {
        // Map shadow memory region
        // Shadow memory covers 1/8th of the address space
        let shadow_size = (1usize << 47) / KASAN_SHADOW_SCALE;  // For 48-bit VA
        
        crate::serial_println!("[KASAN] Mapping {} GB of shadow memory", 
            shadow_size / (1024 * 1024 * 1024));
        
        // Would map shadow memory pages here
        // For now, we'll use direct memory access (unsafe)
    }
    
    pub fn check_memory_access(&self, addr: u64, size: usize, is_write: bool) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return true;
        }
        
        // Check each byte of the access
        for offset in 0..size {
            let check_addr = addr + offset as u64;
            let shadow_addr = self.addr_to_shadow(check_addr);
            
            let shadow_value = unsafe { *(shadow_addr as *const u8) };
            
            if shadow_value != KASAN_FREE {
                // Memory error detected!
                self.report_error(check_addr, size, is_write, shadow_value);
                return false;
            }
        }
        
        true
    }
    
    fn addr_to_shadow(&self, addr: u64) -> u64 {
        (addr >> 3) + KASAN_SHADOW_OFFSET
    }
    
    fn shadow_to_addr(&self, shadow: u64) -> u64 {
        (shadow - KASAN_SHADOW_OFFSET) << 3
    }
    
    pub fn poison_memory(&self, addr: u64, size: usize, poison_type: u8) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let shadow_start = self.addr_to_shadow(addr);
        let shadow_size = (size + KASAN_SHADOW_SCALE - 1) / KASAN_SHADOW_SCALE;
        
        unsafe {
            core::ptr::write_bytes(shadow_start as *mut u8, poison_type, shadow_size);
        }
    }
    
    pub fn unpoison_memory(&self, addr: u64, size: usize) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let shadow_start = self.addr_to_shadow(addr);
        let shadow_size = size / KASAN_SHADOW_SCALE;
        let partial = size % KASAN_SHADOW_SCALE;
        
        unsafe {
            // Mark full shadow bytes as accessible
            core::ptr::write_bytes(shadow_start as *mut u8, KASAN_FREE, shadow_size);
            
            // Handle partial byte at the end
            if partial > 0 {
                *((shadow_start + shadow_size as u64) as *mut u8) = partial as u8;
            }
        }
    }
    
    pub fn alloc_track(&self, addr: u64, size: usize) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        // Unpoison allocated memory
        self.unpoison_memory(addr, size);
        
        // Add redzones
        let redzone_size = 16;  // Bytes
        if addr >= redzone_size {
            self.poison_memory(addr - redzone_size, redzone_size, KASAN_REDZONE);
        }
        self.poison_memory(addr + size as u64, redzone_size, KASAN_REDZONE);
        
        // Track allocation
        let stack_trace = self.capture_stack_trace();
        self.allocation_map.lock().insert(addr, AllocationInfo {
            address: addr,
            size,
            stack_trace,
            freed: false,
            free_stack: None,
        });
    }
    
    pub fn free_track(&self, addr: u64, size: usize) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        // Check if already freed (double-free detection)
        if let Some(info) = self.allocation_map.lock().get_mut(&addr) {
            if info.freed {
                self.report_double_free(addr, info);
                return;
            }
            
            info.freed = true;
            info.free_stack = Some(self.capture_stack_trace());
        }
        
        // Poison freed memory
        self.poison_memory(addr, size, KASAN_FREED);
        
        // Add to quarantine if enabled
        if self.quarantine_enabled.load(Ordering::Relaxed) {
            self.quarantine.lock().push(QuarantineEntry {
                address: addr,
                size,
                free_time: self.get_timestamp(),
            });
        }
    }
    
    fn capture_stack_trace(&self) -> Vec<u64> {
        let mut trace = Vec::new();
        let mut rbp: u64;
        
        unsafe {
            core::arch::asm!("mov {}, rbp", out(reg) rbp);
        }
        
        // Walk stack frames
        for _ in 0..16 {  // Limit depth
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
    
    fn report_error(&self, addr: u64, size: usize, is_write: bool, shadow_value: u8) {
        let error_count = self.error_count.fetch_add(1, Ordering::SeqCst);
        
        crate::serial_println!("\n==================================================================");
        crate::serial_println!("KASAN: Memory error #{}", error_count + 1);
        crate::serial_println!("==================================================================");
        
        let error_type = match shadow_value {
            KASAN_FREED => "use-after-free",
            KASAN_REDZONE => "buffer overflow",
            KASAN_STACK_LEFT | KASAN_STACK_RIGHT => "stack buffer overflow",
            KASAN_GLOBAL_REDZONE => "global buffer overflow",
            _ => "invalid memory access",
        };
        
        crate::serial_println!("BUG: KASAN: {} in {}", error_type, 
            if is_write { "write" } else { "read" });
        crate::serial_println!("{} of size {} at addr {:#x}", 
            if is_write { "Write" } else { "Read" }, size, addr);
        
        // Print current stack trace
        crate::serial_println!("\nCall Trace:");
        let trace = self.capture_stack_trace();
        for frame in trace {
            if let Some(symbol) = super::symbols::resolve_address(frame) {
                crate::serial_println!(" [{:#x}] {}", frame, symbol);
            } else {
                crate::serial_println!(" [{:#x}] <unknown>", frame);
            }
        }
        
        // Print allocation info if available
        if let Some(info) = self.allocation_map.lock().get(&addr) {
            crate::serial_println!("\nAllocated by:");
            for frame in &info.stack_trace {
                if let Some(symbol) = super::symbols::resolve_address(*frame) {
                    crate::serial_println!(" [{:#x}] {}", frame, symbol);
                } else {
                    crate::serial_println!(" [{:#x}] <unknown>", frame);
                }
            }
            
            if let Some(ref free_stack) = info.free_stack {
                crate::serial_println!("\nFreed by:");
                for frame in free_stack {
                    if let Some(symbol) = super::symbols::resolve_address(*frame) {
                        crate::serial_println!(" [{:#x}] {}", frame, symbol);
                    } else {
                        crate::serial_println!(" [{:#x}] <unknown>", frame);
                    }
                }
            }
        }
        
        // Print memory state around the error
        self.print_memory_state(addr);
        
        crate::serial_println!("==================================================================");
        
        // Enter debugger if available
        if super::DEBUG_STATE.kdb_enabled.load(Ordering::Relaxed) {
            super::kdb::enter_debugger("KASAN error");
        }
    }
    
    fn report_double_free(&self, addr: u64, info: &AllocationInfo) {
        let error_count = self.error_count.fetch_add(1, Ordering::SeqCst);
        
        crate::serial_println!("\n==================================================================");
        crate::serial_println!("KASAN: Double-free detected #{}", error_count + 1);
        crate::serial_println!("==================================================================");
        crate::serial_println!("BUG: KASAN: double-free at addr {:#x}", addr);
        
        crate::serial_println!("\nCurrent call trace:");
        let trace = self.capture_stack_trace();
        for frame in trace {
            if let Some(symbol) = super::symbols::resolve_address(frame) {
                crate::serial_println!(" [{:#x}] {}", frame, symbol);
            } else {
                crate::serial_println!(" [{:#x}] <unknown>", frame);
            }
        }
        
        if let Some(ref free_stack) = info.free_stack {
            crate::serial_println!("\nFirst free by:");
            for frame in free_stack {
                if let Some(symbol) = super::symbols::resolve_address(*frame) {
                    crate::serial_println!(" [{:#x}] {}", frame, symbol);
                } else {
                    crate::serial_println!(" [{:#x}] <unknown>", frame);
                }
            }
        }
        
        crate::serial_println!("==================================================================");
    }
    
    fn print_memory_state(&self, addr: u64) {
        crate::serial_println!("\nMemory state around the buggy address:");
        
        // Print shadow memory around the error
        let shadow_addr = self.addr_to_shadow(addr);
        let start = shadow_addr.saturating_sub(8);
        
        for offset in 0..16 {
            let shadow = start + offset;
            let shadow_val = unsafe { *(shadow as *const u8) };
            let mem_addr = self.shadow_to_addr(shadow);
            
            let marker = if shadow == shadow_addr { ">" } else { " " };
            crate::serial_print!("{} [{:#x}]: {:02x} ", marker, mem_addr, shadow_val);
            
            // Decode shadow value
            match shadow_val {
                KASAN_FREE => crate::serial_println!("(accessible)"),
                KASAN_FREED => crate::serial_println!("(freed)"),
                KASAN_REDZONE => crate::serial_println!("(redzone)"),
                KASAN_STACK_LEFT => crate::serial_println!("(stack left redzone)"),
                KASAN_STACK_RIGHT => crate::serial_println!("(stack right redzone)"),
                1..=7 => crate::serial_println!("(partially accessible: {} bytes)", shadow_val),
                _ => crate::serial_println!("(poisoned)"),
            }
        }
    }
    
    pub fn quarantine_flush(&self) {
        if !self.quarantine_enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let mut quarantine = self.quarantine.lock();
        let current_time = self.get_timestamp();
        let quarantine_period = 1000000;  // microseconds
        
        // Remove old entries from quarantine
        quarantine.retain(|entry| {
            if current_time - entry.free_time > quarantine_period {
                // Really free the memory
                false
            } else {
                true
            }
        });
    }
    
    fn get_timestamp(&self) -> u64 {
        // Would get actual timestamp
        0
    }
    
    pub fn print_stats(&self) {
        let allocations = self.allocation_map.lock();
        let quarantine = self.quarantine.lock();
        
        crate::serial_println!("KASAN Statistics:");
        crate::serial_println!("  Errors detected: {}", self.error_count.load(Ordering::Relaxed));
        crate::serial_println!("  Active allocations: {}", 
            allocations.iter().filter(|(_, info)| !info.freed).count());
        crate::serial_println!("  Freed allocations: {}",
            allocations.iter().filter(|(_, info)| info.freed).count());
        crate::serial_println!("  Quarantine entries: {}", quarantine.len());
    }
}

impl StackDepot {
    fn new() -> Self {
        Self {
            stacks: Vec::new(),
            hash_map: BTreeMap::new(),
        }
    }
    
    fn store_stack(&mut self, trace: Vec<u64>) -> usize {
        // Calculate hash of stack trace
        let hash = trace.iter().fold(0u64, |acc, &addr| {
            acc.wrapping_mul(31).wrapping_add(addr)
        });
        
        if let Some(&idx) = self.hash_map.get(&hash) {
            return idx;
        }
        
        let idx = self.stacks.len();
        self.stacks.push(trace);
        self.hash_map.insert(hash, idx);
        idx
    }
}

// Compiler instrumentation hooks
#[no_mangle]
pub extern "C" fn __asan_load1(addr: u64) {
    KASAN.check_memory_access(addr, 1, false);
}

#[no_mangle]
pub extern "C" fn __asan_load2(addr: u64) {
    KASAN.check_memory_access(addr, 2, false);
}

#[no_mangle]
pub extern "C" fn __asan_load4(addr: u64) {
    KASAN.check_memory_access(addr, 4, false);
}

#[no_mangle]
pub extern "C" fn __asan_load8(addr: u64) {
    KASAN.check_memory_access(addr, 8, false);
}

#[no_mangle]
pub extern "C" fn __asan_store1(addr: u64) {
    KASAN.check_memory_access(addr, 1, true);
}

#[no_mangle]
pub extern "C" fn __asan_store2(addr: u64) {
    KASAN.check_memory_access(addr, 2, true);
}

#[no_mangle]
pub extern "C" fn __asan_store4(addr: u64) {
    KASAN.check_memory_access(addr, 4, true);
}

#[no_mangle]
pub extern "C" fn __asan_store8(addr: u64) {
    KASAN.check_memory_access(addr, 8, true);
}

// Public API
pub fn init() {
    KASAN.init();
}

pub fn check_memory_access(addr: u64, size: usize, is_write: bool) -> bool {
    KASAN.check_memory_access(addr, size, is_write)
}

pub fn alloc_track(addr: u64, size: usize) {
    KASAN.alloc_track(addr, size);
}

pub fn free_track(addr: u64, size: usize) {
    KASAN.free_track(addr, size);
}

pub fn poison_memory(addr: u64, size: usize, poison_type: u8) {
    KASAN.poison_memory(addr, size, poison_type);
}

pub fn unpoison_memory(addr: u64, size: usize) {
    KASAN.unpoison_memory(addr, size);
}

pub fn print_stats() {
    KASAN.print_stats();
}