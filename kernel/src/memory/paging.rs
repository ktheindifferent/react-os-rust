use x86_64::{
    structures::paging::{
        Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
        mapper::Mapper,
        FrameAllocator, OffsetPageTable,
    },
    VirtAddr, PhysAddr,
};
use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<Option<BootInfoFrameAllocator>> = Mutex::new(None);
}

pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions
            .filter(|r| r.region_type == MemoryRegionType::Usable);
        let addr_ranges = usable_regions
            .map(|r| r.range.start_addr()..r.range.end_addr());
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

/// Initialize a new OffsetPageTable from the active level 4 table
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// Returns a mutable reference to the active level 4 table
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

/// Map a virtual page to a physical frame
pub fn map_page(
    mapper: &mut impl Mapper<Size4KiB>,
    page: Page<Size4KiB>,
    frame: PhysFrame<Size4KiB>,
    flags: PageTableFlags,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), &'static str> {
    use x86_64::structures::paging::mapper::MapToError;
    
    match unsafe { mapper.map_to(page, frame, flags, frame_allocator) } {
        Ok(flush) => {
            flush.flush();
            Ok(())
        }
        Err(MapToError::FrameAllocationFailed) => Err("Frame allocation failed"),
        Err(MapToError::ParentEntryHugePage) => Err("Parent entry is huge page"),
        Err(MapToError::PageAlreadyMapped(_)) => Err("Page already mapped"),
    }
}

/// Unmap a virtual page
pub fn unmap_page(
    mapper: &mut impl Mapper<Size4KiB>,
    page: Page<Size4KiB>,
) -> Result<PhysFrame<Size4KiB>, &'static str> {
    mapper.unmap(page)
        .map(|(frame, flush)| {
            flush.flush();
            frame
        })
        .map_err(|_| "Page not mapped")
}

/// Translate a virtual address to physical address
pub fn translate_addr(
    mapper: &impl Mapper<Size4KiB>,
    addr: VirtAddr,
) -> Option<PhysAddr> {
    let page = Page::<Size4KiB>::containing_address(addr);
    mapper.translate_page(page).ok().map(|frame| {
        let offset = addr.as_u64() & 0xfff;
        frame.start_address() + offset
    })
}

/// Create a new mapping for a given virtual address range
pub fn create_mapping(
    mapper: &mut impl Mapper<Size4KiB>,
    virt_start: VirtAddr,
    phys_start: PhysAddr,
    size: u64,
    flags: PageTableFlags,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), &'static str> {
    let page_count = (size + 4095) / 4096;
    
    for i in 0..page_count {
        let virt_addr = virt_start + i * 4096;
        let phys_addr = phys_start + i * 4096;
        
        let page = Page::containing_address(virt_addr);
        let frame = PhysFrame::containing_address(phys_addr);
        
        map_page(mapper, page, frame, flags, frame_allocator)?;
    }
    
    Ok(())
}

/// Initialize frame allocator from bootloader memory map
pub fn init_frame_allocator(memory_map: &'static MemoryMap) {
    let mut allocator = FRAME_ALLOCATOR.lock();
    *allocator = Some(unsafe { BootInfoFrameAllocator::init(memory_map) });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_translate_addr() {
        // Test that we can translate kernel addresses
        // This would need proper setup in tests
    }
}