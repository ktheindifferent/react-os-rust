use x86_64::{PhysAddr, structures::paging::{PhysFrame, Size4KiB}};
use alloc::vec::Vec;

pub struct PhysicalAllocator {
    free_frames: Vec<PhysFrame>,
    total_memory: u64,
    used_memory: u64,
}

impl PhysicalAllocator {
    pub fn new() -> Self {
        // In a real implementation, this would be initialized from the bootloader
        // memory map. For now, we'll create a simple allocator.
        Self {
            free_frames: Vec::new(),
            total_memory: 0,
            used_memory: 0,
        }
    }

    pub fn allocate_frame(&mut self) -> Option<PhysFrame> {
        if let Some(frame) = self.free_frames.pop() {
            self.used_memory += 4096;
            Some(frame)
        } else {
            None
        }
    }

    pub fn deallocate_frame(&mut self, frame: PhysFrame) {
        self.free_frames.push(frame);
        self.used_memory = self.used_memory.saturating_sub(4096);
    }

    pub fn allocate_contiguous(&mut self, count: usize) -> Option<Vec<PhysFrame>> {
        if self.free_frames.len() < count {
            return None;
        }

        let mut frames = Vec::with_capacity(count);
        for _ in 0..count {
            if let Some(frame) = self.allocate_frame() {
                frames.push(frame);
            } else {
                // Rollback allocation
                for frame in frames {
                    self.deallocate_frame(frame);
                }
                return None;
            }
        }
        
        Some(frames)
    }

    pub fn get_memory_info(&self) -> MemoryInfo {
        MemoryInfo {
            total_memory: self.total_memory,
            available_memory: self.total_memory - self.used_memory,
            used_memory: self.used_memory,
            free_frames: self.free_frames.len(),
        }
    }

    pub fn initialize_from_memory_map(&mut self, memory_regions: &[MemoryRegion]) {
        self.free_frames.clear();
        self.total_memory = 0;
        
        for region in memory_regions {
            if region.region_type == MemoryRegionType::Usable {
                let start_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(region.start));
                let end_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(region.end - 1));
                
                for frame_addr in (start_frame.start_address().as_u64()..=end_frame.start_address().as_u64()).step_by(4096) {
                    let frame = PhysFrame::containing_address(PhysAddr::new(frame_addr));
                    self.free_frames.push(frame);
                }
                
                self.total_memory += region.end - region.start;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: u64,
    pub end: u64,
    pub region_type: MemoryRegionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    Usable,
    Reserved,
    AcpiReclaimable,
    AcpiNvs,
    BadMemory,
    Bootloader,
    Kernel,
    FrameBuffer,
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryInfo {
    pub total_memory: u64,
    pub available_memory: u64,
    pub used_memory: u64,
    pub free_frames: usize,
}