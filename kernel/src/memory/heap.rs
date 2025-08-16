// Advanced Heap Memory Management
// Integrates with the hybrid allocator for efficient memory management

use x86_64::structures::paging::{PageTable, OffsetPageTable, Page, PageTableFlags, Mapper, Size4KiB};
use x86_64::VirtAddr;
use core::ops::Range;
use spin::Mutex;
use lazy_static::lazy_static;

// Heap configuration
pub const HEAP_START: usize = 0x4444_4444_0000;
pub const HEAP_SIZE: usize = 32 * 1024 * 1024; // 32 MiB heap
pub const HEAP_END: usize = HEAP_START + HEAP_SIZE;

// Guard pages for heap overflow detection
pub const HEAP_GUARD_PAGES: usize = 2;
pub const HEAP_GUARD_SIZE: usize = HEAP_GUARD_PAGES * 4096;

// Heap metadata tracking
pub struct HeapInfo {
    start_addr: VirtAddr,
    end_addr: VirtAddr,
    current_size: usize,
    max_size: usize,
    guard_enabled: bool,
    initialized: bool,
}

impl HeapInfo {
    fn new() -> Self {
        Self {
            start_addr: VirtAddr::new(HEAP_START as u64),
            end_addr: VirtAddr::new(HEAP_END as u64),
            current_size: 0,
            max_size: HEAP_SIZE,
            guard_enabled: false,
            initialized: false,
        }
    }

    pub fn is_heap_address(&self, addr: VirtAddr) -> bool {
        addr >= self.start_addr && addr < self.end_addr
    }

    pub fn available_space(&self) -> usize {
        self.max_size - self.current_size
    }
}

lazy_static! {
    static ref HEAP_INFO: Mutex<HeapInfo> = Mutex::new(HeapInfo::new());
}

// Initialize heap memory region
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl x86_64::structures::paging::FrameAllocator<Size4KiB>,
) -> Result<(), &'static str> {
    let mut heap_info = HEAP_INFO.lock();
    
    if heap_info.initialized {
        return Ok(());
    }

    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = VirtAddr::new(HEAP_END as u64);
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    // Map heap pages
    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or("Failed to allocate frame for heap")?;
        
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)
                .map_err(|_| "Failed to map heap page")?
                .flush();
        }
    }

    // Setup guard pages if requested
    if cfg!(debug_assertions) {
        setup_guard_pages(mapper, frame_allocator)?;
        heap_info.guard_enabled = true;
    }

    heap_info.current_size = HEAP_SIZE;
    heap_info.initialized = true;

    // Initialize the hybrid allocator
    unsafe {
        crate::allocator::init_heap();
    }

    crate::serial_println!("Heap initialized: start={:#x}, size={} MB", 
        HEAP_START, HEAP_SIZE / (1024 * 1024));

    Ok(())
}

// Setup guard pages for heap overflow detection
fn setup_guard_pages(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl x86_64::structures::paging::FrameAllocator<Size4KiB>,
) -> Result<(), &'static str> {
    // Guard pages before heap
    let guard_start = VirtAddr::new((HEAP_START - HEAP_GUARD_SIZE) as u64);
    let guard_page = Page::containing_address(guard_start);
    
    for i in 0..HEAP_GUARD_PAGES {
        let page = guard_page + i as u64;
        let frame = frame_allocator
            .allocate_frame()
            .ok_or("Failed to allocate guard page frame")?;
        
        // Map guard pages as non-present to catch underflows
        let flags = PageTableFlags::empty();
        
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)
                .map_err(|_| "Failed to map guard page")?
                .flush();
        }
    }

    // Guard pages after heap
    let guard_end_start = VirtAddr::new(HEAP_END as u64);
    let guard_end_page = Page::containing_address(guard_end_start);
    
    for i in 0..HEAP_GUARD_PAGES {
        let page = guard_end_page + i as u64;
        let frame = frame_allocator
            .allocate_frame()
            .ok_or("Failed to allocate guard page frame")?;
        
        // Map guard pages as non-present to catch overflows
        let flags = PageTableFlags::empty();
        
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)
                .map_err(|_| "Failed to map guard page")?
                .flush();
        }
    }

    crate::serial_println!("Heap guard pages enabled");
    Ok(())
}

// Dynamic heap expansion (for future use)
pub fn expand_heap(
    size: usize,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl x86_64::structures::paging::FrameAllocator<Size4KiB>,
) -> Result<(), &'static str> {
    let mut heap_info = HEAP_INFO.lock();
    
    if !heap_info.initialized {
        return Err("Heap not initialized");
    }

    let new_size = heap_info.current_size + size;
    if new_size > heap_info.max_size {
        return Err("Heap expansion would exceed maximum size");
    }

    let pages_needed = (size + 4095) / 4096;
    let current_end = VirtAddr::new((HEAP_START + heap_info.current_size) as u64);
    let start_page = Page::containing_address(current_end);

    for i in 0..pages_needed {
        let page = start_page + i as u64;
        let frame = frame_allocator
            .allocate_frame()
            .ok_or("Failed to allocate frame for heap expansion")?;
        
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)
                .map_err(|_| "Failed to map expanded heap page")?
                .flush();
        }
    }

    heap_info.current_size = new_size;
    heap_info.end_addr = VirtAddr::new((HEAP_START + new_size) as u64);

    crate::serial_println!("Heap expanded by {} KB, new size: {} MB", 
        size / 1024, heap_info.current_size / (1024 * 1024));

    Ok(())
}

// Heap validation and debugging
pub fn validate_heap_pointer(ptr: *const u8) -> bool {
    let addr = VirtAddr::new(ptr as u64);
    let heap_info = HEAP_INFO.lock();
    heap_info.is_heap_address(addr)
}

pub fn heap_stats() -> HeapStatistics {
    let heap_info = HEAP_INFO.lock();
    let allocator_stats = crate::allocator::memory_stats();
    
    HeapStatistics {
        start_address: heap_info.start_addr.as_u64() as usize,
        end_address: heap_info.end_addr.as_u64() as usize,
        total_size: heap_info.current_size,
        max_size: heap_info.max_size,
        used_bytes: allocator_stats.current_allocated,
        free_bytes: heap_info.current_size - allocator_stats.current_allocated,
        guard_pages_enabled: heap_info.guard_enabled,
        total_allocations: allocator_stats.total_allocations,
        total_deallocations: allocator_stats.total_deallocations,
        peak_usage: allocator_stats.peak_allocated,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HeapStatistics {
    pub start_address: usize,
    pub end_address: usize,
    pub total_size: usize,
    pub max_size: usize,
    pub used_bytes: usize,
    pub free_bytes: usize,
    pub guard_pages_enabled: bool,
    pub total_allocations: usize,
    pub total_deallocations: usize,
    pub peak_usage: usize,
}

impl HeapStatistics {
    pub fn print_summary(&self) {
        crate::serial_println!("=== Heap Memory Statistics ===");
        crate::serial_println!("Address Range: {:#x} - {:#x}", self.start_address, self.end_address);
        crate::serial_println!("Total Size: {} MB", self.total_size / (1024 * 1024));
        crate::serial_println!("Max Size: {} MB", self.max_size / (1024 * 1024));
        crate::serial_println!("Used: {} KB ({:.1}%)", 
            self.used_bytes / 1024, 
            (self.used_bytes as f64 / self.total_size as f64) * 100.0);
        crate::serial_println!("Free: {} KB ({:.1}%)", 
            self.free_bytes / 1024,
            (self.free_bytes as f64 / self.total_size as f64) * 100.0);
        crate::serial_println!("Peak Usage: {} KB", self.peak_usage / 1024);
        crate::serial_println!("Total Allocations: {}", self.total_allocations);
        crate::serial_println!("Total Deallocations: {}", self.total_deallocations);
        if self.guard_pages_enabled {
            crate::serial_println!("Guard Pages: ENABLED");
        }
    }

    pub fn fragmentation_ratio(&self) -> f64 {
        // Simple fragmentation metric
        if self.total_allocations == 0 {
            return 0.0;
        }
        
        let average_allocation = self.used_bytes / self.total_allocations.max(1);
        let external_fragmentation = if average_allocation > 0 {
            1.0 - (average_allocation as f64 / 4096.0)
        } else {
            0.0
        };
        
        external_fragmentation.max(0.0).min(1.0)
    }
}

// Heap defragmentation (placeholder for future implementation)
pub fn defragment_heap() -> Result<usize, &'static str> {
    // This would require a compacting collector or similar mechanism
    // For now, return 0 bytes reclaimed
    Ok(0)
}

// Emergency heap recovery
pub fn emergency_heap_cleanup() {
    crate::serial_println!("WARNING: Emergency heap cleanup initiated");
    
    // In a real implementation, this would:
    // 1. Free all non-essential allocations
    // 2. Compact remaining allocations
    // 3. Reset caches
    // 4. Clear any leaked memory
    
    // For now, just print statistics
    let stats = heap_stats();
    stats.print_summary();
}