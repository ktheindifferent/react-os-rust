// Frame Allocator for Physical Memory Management
use x86_64::{
    structures::paging::{
        PhysFrame, Size4KiB, FrameAllocator, FrameDeallocator,
    },
    PhysAddr,
};
use spin::Mutex;
use lazy_static::lazy_static;
use alloc::vec::Vec;

// Memory map regions
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: PhysAddr,
    pub end: PhysAddr,
}

// Bitmap frame allocator
pub struct BitmapFrameAllocator {
    bitmap: Vec<u64>,
    next_free: usize,
    total_frames: usize,
    free_frames: usize,
    memory_regions: Vec<MemoryRegion>,
}

impl BitmapFrameAllocator {
    pub fn new() -> Self {
        Self {
            bitmap: Vec::new(),
            next_free: 0,
            total_frames: 0,
            free_frames: 0,
            memory_regions: Vec::new(),
        }
    }
    
    pub fn init(&mut self, memory_map: &[MemoryRegion]) {
        // Find total memory size
        let mut max_addr = PhysAddr::new(0);
        for region in memory_map {
            if region.end > max_addr {
                max_addr = region.end;
            }
        }
        
        // Calculate number of frames
        self.total_frames = (max_addr.as_u64() / 4096) as usize;
        let bitmap_size = (self.total_frames + 63) / 64;
        
        // Initialize bitmap (all frames marked as used initially)
        self.bitmap = Vec::with_capacity(bitmap_size);
        for _ in 0..bitmap_size {
            self.bitmap.push(u64::MAX);
        }
        
        // Mark usable regions as free
        for region in memory_map {
            self.memory_regions.push(*region);
            self.mark_region_free(region.start, region.end);
        }
        
        crate::serial_println!("Frame allocator initialized: {} total frames, {} free frames",
            self.total_frames, self.free_frames);
    }
    
    fn mark_region_free(&mut self, start: PhysAddr, end: PhysAddr) {
        let start_frame = start.as_u64() / 4096;
        let end_frame = end.as_u64() / 4096;
        
        for frame_num in start_frame..end_frame {
            self.mark_frame_free(frame_num as usize);
        }
    }
    
    fn mark_frame_free(&mut self, frame_num: usize) {
        let bitmap_idx = frame_num / 64;
        let bit_idx = frame_num % 64;
        
        if bitmap_idx < self.bitmap.len() {
            let was_used = self.bitmap[bitmap_idx] & (1 << bit_idx) != 0;
            self.bitmap[bitmap_idx] &= !(1 << bit_idx);
            if was_used {
                self.free_frames += 1;
            }
        }
    }
    
    fn mark_frame_used(&mut self, frame_num: usize) {
        let bitmap_idx = frame_num / 64;
        let bit_idx = frame_num % 64;
        
        if bitmap_idx < self.bitmap.len() {
            let was_free = self.bitmap[bitmap_idx] & (1 << bit_idx) == 0;
            self.bitmap[bitmap_idx] |= 1 << bit_idx;
            if was_free && self.free_frames > 0 {
                self.free_frames -= 1;
            }
        }
    }
    
    fn is_frame_free(&self, frame_num: usize) -> bool {
        let bitmap_idx = frame_num / 64;
        let bit_idx = frame_num % 64;
        
        if bitmap_idx < self.bitmap.len() {
            self.bitmap[bitmap_idx] & (1 << bit_idx) == 0
        } else {
            false
        }
    }
    
    pub fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Start searching from next_free
        for frame_num in self.next_free..self.total_frames {
            if self.is_frame_free(frame_num) {
                self.mark_frame_used(frame_num);
                self.next_free = frame_num + 1;
                
                let addr = PhysAddr::new((frame_num as u64) * 4096);
                return Some(PhysFrame::containing_address(addr));
            }
        }
        
        // Wrap around and search from beginning
        for frame_num in 0..self.next_free {
            if self.is_frame_free(frame_num) {
                self.mark_frame_used(frame_num);
                self.next_free = frame_num + 1;
                
                let addr = PhysAddr::new((frame_num as u64) * 4096);
                return Some(PhysFrame::containing_address(addr));
            }
        }
        
        None
    }
    
    pub fn deallocate_frame(&mut self, frame: PhysFrame) {
        let frame_num = frame.start_address().as_u64() / 4096;
        self.mark_frame_free(frame_num as usize);
    }
    
    pub fn free_frames(&self) -> usize {
        self.free_frames
    }
    
    pub fn used_frames(&self) -> usize {
        self.total_frames - self.free_frames
    }
}

unsafe impl FrameAllocator<Size4KiB> for BitmapFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        self.allocate_frame()
    }
}

impl FrameDeallocator<Size4KiB> for BitmapFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        self.deallocate_frame(frame);
    }
}

// Global frame allocator
lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<BitmapFrameAllocator> = 
        Mutex::new(BitmapFrameAllocator::new());
}

// Initialize the frame allocator with memory map
pub fn init_frame_allocator(memory_map: &[MemoryRegion]) {
    FRAME_ALLOCATOR.lock().init(memory_map);
}

// Allocate a physical frame
pub fn allocate_frame() -> Option<PhysFrame> {
    FRAME_ALLOCATOR.lock().allocate_frame()
}

// Deallocate a physical frame
pub fn deallocate_frame(frame: PhysFrame) {
    FRAME_ALLOCATOR.lock().deallocate_frame(frame);
}

// Get memory statistics
pub fn memory_stats() -> (usize, usize, usize) {
    let allocator = FRAME_ALLOCATOR.lock();
    (allocator.total_frames, allocator.free_frames, allocator.used_frames())
}

// Bootloader memory map entry types
pub const USABLE_MEMORY: u32 = 1;
pub const RESERVED_MEMORY: u32 = 2;
pub const ACPI_RECLAIMABLE: u32 = 3;
pub const ACPI_NVS: u32 = 4;
pub const BAD_MEMORY: u32 = 5;

// Parse bootloader memory map
pub fn parse_memory_map(boot_info: &[u8]) -> Vec<MemoryRegion> {
    let mut regions = Vec::new();
    
    // This is a simplified version - in reality, would parse actual boot info
    // For now, assume some standard memory regions
    
    // First MB is typically reserved
    // Usable memory from 1MB to 10MB for testing
    regions.push(MemoryRegion {
        start: PhysAddr::new(0x100000),  // 1MB
        end: PhysAddr::new(0xA00000),    // 10MB
    });
    
    regions
}