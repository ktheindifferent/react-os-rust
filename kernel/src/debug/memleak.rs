// Memory Leak Detection
// Tracks allocations and detects memory leaks

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;

pub struct MemleakDetector {
    enabled: AtomicBool,
    tracking: Mutex<AllocationTracker>,
    statistics: LeakStatistics,
}

struct AllocationTracker {
    allocations: BTreeMap<u64, Allocation>,
    call_sites: BTreeMap<u64, CallSiteInfo>,
    total_allocated: u64,
    total_freed: u64,
    peak_usage: u64,
    current_usage: u64,
}

#[derive(Clone)]
struct Allocation {
    address: u64,
    size: usize,
    caller: u64,
    stack_trace: Vec<u64>,
    timestamp: u64,
    allocation_type: AllocationType,
}

#[derive(Clone, Copy, Debug)]
enum AllocationType {
    Kmalloc,
    Vmalloc,
    PageAlloc,
    SlabAlloc,
}

struct CallSiteInfo {
    address: u64,
    total_allocated: u64,
    total_freed: u64,
    current_usage: u64,
    allocation_count: u64,
    free_count: u64,
    peak_usage: u64,
}

struct LeakStatistics {
    suspected_leaks: AtomicU64,
    confirmed_leaks: AtomicU64,
    total_leaked_bytes: AtomicU64,
    scan_count: AtomicU64,
}

lazy_static! {
    pub static ref MEMLEAK: MemleakDetector = MemleakDetector::new();
}

impl MemleakDetector {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            tracking: Mutex::new(AllocationTracker::new()),
            statistics: LeakStatistics::new(),
        }
    }
    
    pub fn init(&self) {
        self.enabled.store(true, Ordering::SeqCst);
        crate::serial_println!("[MEMLEAK] Memory leak detector initialized");
    }
    
    pub fn track_alloc(&self, addr: u64, size: usize, alloc_type: AllocationType) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let stack_trace = self.capture_stack_trace();
        let caller = stack_trace.first().copied().unwrap_or(0);
        
        let allocation = Allocation {
            address: addr,
            size,
            caller,
            stack_trace,
            timestamp: self.get_timestamp(),
            allocation_type: alloc_type,
        };
        
        let mut tracker = self.tracking.lock();
        
        // Track allocation
        tracker.allocations.insert(addr, allocation);
        tracker.total_allocated += size as u64;
        tracker.current_usage += size as u64;
        tracker.peak_usage = tracker.peak_usage.max(tracker.current_usage);
        
        // Update call site info
        let site_info = tracker.call_sites.entry(caller)
            .or_insert_with(|| CallSiteInfo {
                address: caller,
                total_allocated: 0,
                total_freed: 0,
                current_usage: 0,
                allocation_count: 0,
                free_count: 0,
                peak_usage: 0,
            });
        
        site_info.total_allocated += size as u64;
        site_info.current_usage += size as u64;
        site_info.allocation_count += 1;
        site_info.peak_usage = site_info.peak_usage.max(site_info.current_usage);
    }
    
    pub fn track_free(&self, addr: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let mut tracker = self.tracking.lock();
        
        if let Some(allocation) = tracker.allocations.remove(&addr) {
            tracker.total_freed += allocation.size as u64;
            tracker.current_usage = tracker.current_usage.saturating_sub(allocation.size as u64);
            
            // Update call site info
            if let Some(site_info) = tracker.call_sites.get_mut(&allocation.caller) {
                site_info.total_freed += allocation.size as u64;
                site_info.current_usage = site_info.current_usage.saturating_sub(allocation.size as u64);
                site_info.free_count += 1;
            }
        } else {
            // Free of untracked allocation - possible corruption
            crate::serial_println!("[MEMLEAK] Warning: free of untracked address {:#x}", addr);
        }
    }
    
    pub fn scan_for_leaks(&self) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let scan_count = self.statistics.scan_count.fetch_add(1, Ordering::SeqCst);
        crate::serial_println!("[MEMLEAK] Starting leak scan #{}", scan_count + 1);
        
        let tracker = self.tracking.lock();
        let current_time = self.get_timestamp();
        
        // Find suspected leaks (allocations older than threshold)
        let age_threshold = 60_000_000_000;  // 60 seconds in TSC cycles (approximate)
        let mut suspected_leaks = Vec::new();
        
        for (_addr, allocation) in &tracker.allocations {
            let age = current_time - allocation.timestamp;
            if age > age_threshold {
                suspected_leaks.push(allocation.clone());
            }
        }
        
        if !suspected_leaks.is_empty() {
            crate::serial_println!("[MEMLEAK] Found {} suspected leaks:", suspected_leaks.len());
            
            // Group leaks by call site
            let mut leaks_by_site: BTreeMap<u64, Vec<&Allocation>> = BTreeMap::new();
            for leak in &suspected_leaks {
                leaks_by_site.entry(leak.caller)
                    .or_insert_with(Vec::new)
                    .push(leak);
            }
            
            // Report leaks by call site
            for (caller, leaks) in leaks_by_site {
                let total_size: usize = leaks.iter().map(|a| a.size).sum();
                crate::serial_println!("\n  Call site {:#x}:", caller);
                
                if let Some(symbol) = super::symbols::resolve_address(caller) {
                    crate::serial_println!("    Function: {}", symbol);
                }
                
                crate::serial_println!("    {} allocations, {} bytes total", 
                    leaks.len(), total_size);
                
                // Show a few example allocations
                for (i, leak) in leaks.iter().take(3).enumerate() {
                    crate::serial_println!("    Example {}: addr={:#x} size={} type={:?}",
                        i + 1, leak.address, leak.size, leak.allocation_type);
                }
            }
            
            // Update statistics
            self.statistics.suspected_leaks.store(suspected_leaks.len() as u64, Ordering::Relaxed);
            let total_leaked: usize = suspected_leaks.iter().map(|a| a.size).sum();
            self.statistics.total_leaked_bytes.store(total_leaked as u64, Ordering::Relaxed);
        } else {
            crate::serial_println!("[MEMLEAK] No leaks detected");
        }
    }
    
    pub fn print_summary(&self) {
        let tracker = self.tracking.lock();
        
        crate::serial_println!("\n=== Memory Leak Detection Summary ===");
        crate::serial_println!("Status: {}", 
            if self.enabled.load(Ordering::Relaxed) { "Enabled" } else { "Disabled" });
        crate::serial_println!("\nAllocation Statistics:");
        crate::serial_println!("  Total allocated: {} bytes", tracker.total_allocated);
        crate::serial_println!("  Total freed: {} bytes", tracker.total_freed);
        crate::serial_println!("  Current usage: {} bytes", tracker.current_usage);
        crate::serial_println!("  Peak usage: {} bytes", tracker.peak_usage);
        crate::serial_println!("  Active allocations: {}", tracker.allocations.len());
        
        crate::serial_println!("\nTop Allocation Sites:");
        let mut sites: Vec<_> = tracker.call_sites.values().collect();
        sites.sort_by_key(|s| s.current_usage);
        sites.reverse();
        
        for (i, site) in sites.iter().take(10).enumerate() {
            crate::serial_println!("  {}. Address {:#x}:", i + 1, site.address);
            
            if let Some(symbol) = super::symbols::resolve_address(site.address) {
                crate::serial_println!("     Function: {}", symbol);
            }
            
            crate::serial_println!("     Current: {} bytes in {} allocations",
                site.current_usage, 
                site.allocation_count - site.free_count);
            crate::serial_println!("     Total: {} allocs, {} frees",
                site.allocation_count, site.free_count);
        }
        
        let suspected = self.statistics.suspected_leaks.load(Ordering::Relaxed);
        if suspected > 0 {
            crate::serial_println!("\nLeak Detection:");
            crate::serial_println!("  Suspected leaks: {}", suspected);
            crate::serial_println!("  Leaked bytes: {}", 
                self.statistics.total_leaked_bytes.load(Ordering::Relaxed));
        }
    }
    
    pub fn clear(&self) {
        let mut tracker = self.tracking.lock();
        tracker.allocations.clear();
        tracker.call_sites.clear();
        tracker.current_usage = 0;
        crate::serial_println!("[MEMLEAK] Allocation tracking cleared");
    }
    
    fn capture_stack_trace(&self) -> Vec<u64> {
        let mut trace = Vec::new();
        let mut rbp: u64;
        
        unsafe {
            core::arch::asm!("mov {}, rbp", out(reg) rbp);
        }
        
        // Skip memleak detector frames
        let mut skip = 2;
        
        for _ in 0..16 {
            if rbp == 0 || rbp < 0x1000 {
                break;
            }
            
            unsafe {
                let return_addr = *((rbp + 8) as *const u64);
                if return_addr == 0 {
                    break;
                }
                
                if skip > 0 {
                    skip -= 1;
                } else {
                    trace.push(return_addr);
                }
                
                rbp = *(rbp as *const u64);
            }
        }
        
        trace
    }
    
    fn get_timestamp(&self) -> u64 {
        unsafe { core::arch::x86_64::_rdtsc() }
    }
}

impl AllocationTracker {
    fn new() -> Self {
        Self {
            allocations: BTreeMap::new(),
            call_sites: BTreeMap::new(),
            total_allocated: 0,
            total_freed: 0,
            peak_usage: 0,
            current_usage: 0,
        }
    }
}

impl LeakStatistics {
    fn new() -> Self {
        Self {
            suspected_leaks: AtomicU64::new(0),
            confirmed_leaks: AtomicU64::new(0),
            total_leaked_bytes: AtomicU64::new(0),
            scan_count: AtomicU64::new(0),
        }
    }
}

// Public API
pub fn init() {
    MEMLEAK.init();
}

pub fn track_allocation(addr: u64, size: usize) {
    MEMLEAK.track_alloc(addr, size, AllocationType::Kmalloc);
}

pub fn track_free(addr: u64) {
    MEMLEAK.track_free(addr);
}

pub fn scan() {
    MEMLEAK.scan_for_leaks();
}

pub fn print_summary() {
    MEMLEAK.print_summary();
}

pub fn enable() {
    MEMLEAK.enabled.store(true, Ordering::SeqCst);
    crate::serial_println!("[MEMLEAK] Memory leak detection enabled");
}

pub fn disable() {
    MEMLEAK.enabled.store(false, Ordering::SeqCst);
    crate::serial_println!("[MEMLEAK] Memory leak detection disabled");
}

pub fn clear() {
    MEMLEAK.clear();
}