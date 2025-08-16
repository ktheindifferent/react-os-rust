use linked_list_allocator::LockedHeap;
use core::alloc::{GlobalAlloc, Layout};

pub const HEAP_SIZE: usize = 1024 * 1024; // 1 MiB - larger heap for all subsystems
pub const HEAP_GUARD_SIZE: usize = 4096; // Guard page size

#[global_allocator]
static ALLOCATOR: HeapAllocator = HeapAllocator::new();

// Static heap allocation for simplicity
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

// Wrapper around LockedHeap with additional safety checks
pub struct HeapAllocator {
    inner: LockedHeap,
}

impl HeapAllocator {
    const fn new() -> Self {
        Self {
            inner: LockedHeap::empty(),
        }
    }
    
    pub fn init(&self) {
        unsafe {
            let mut allocator = self.inner.lock();
            allocator.init(HEAP.as_mut_ptr(), HEAP_SIZE);
        }
    }
    
    fn check_allocation(&self, layout: Layout) -> bool {
        // Check for reasonable allocation sizes
        if layout.size() > HEAP_SIZE / 2 {
            crate::serial_println!("WARNING: Large allocation request: {} bytes", layout.size());
            return false;
        }
        
        // Check alignment is power of 2
        if !layout.align().is_power_of_two() {
            crate::serial_println!("ERROR: Invalid alignment: {}", layout.align());
            return false;
        }
        
        true
    }
}

unsafe impl GlobalAlloc for HeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !self.check_allocation(layout) {
            return core::ptr::null_mut();
        }
        
        let ptr = self.inner.alloc(layout);
        
        #[cfg(debug_assertions)]
        if !ptr.is_null() {
            // Fill allocated memory with pattern in debug mode for easier debugging
            core::ptr::write_bytes(ptr, 0xAA, layout.size());
        }
        
        ptr
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        #[cfg(debug_assertions)]
        {
            // Fill deallocated memory with pattern in debug mode
            core::ptr::write_bytes(ptr, 0xDD, layout.size());
        }
        
        self.inner.dealloc(ptr, layout);
    }
}

pub fn init_heap() {
    ALLOCATOR.init();
    crate::serial_println!("Heap initialized: {} bytes available", HEAP_SIZE);
}