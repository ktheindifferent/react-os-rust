use x86_64::{
    structures::paging::{OffsetPageTable, Mapper, FrameAllocator, PhysFrame, Size4KiB},
    VirtAddr,
};
use spin::Mutex;

static mut MAPPER: Option<OffsetPageTable<'static>> = None;
static mut FRAME_ALLOCATOR: Option<BootInfoFrameAllocator> = None;

pub struct BootInfoFrameAllocator {
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init() -> Self {
        Self { next: 0 }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Simplified frame allocation
        let frame = PhysFrame::containing_address(x86_64::PhysAddr::new((self.next * 0x1000) as u64));
        self.next += 1;
        Some(frame)
    }
}

pub unsafe fn get_mapper() -> &'static mut OffsetPageTable<'static> {
    MAPPER.as_mut().expect("Mapper not initialized")
}

pub unsafe fn get_frame_allocator() -> &'static mut BootInfoFrameAllocator {
    FRAME_ALLOCATOR.as_mut().expect("Frame allocator not initialized")
}