pub mod paging;
pub mod heap;
pub mod physical;
pub mod virtual_memory;
pub mod demand_paging;
pub mod frame_allocator;

use x86_64::{
    structures::paging::{PageTable, OffsetPageTable, PhysFrame, Size4KiB},
    VirtAddr, PhysAddr,
};

// Windows NT compatible memory layout constants
pub const KERNEL_BASE: u64 = 0xFFFF_8000_0000_0000;
pub const USER_SPACE_END: u64 = 0x0000_7FFF_FFFF_FFFF;
pub const SYSTEM_SPACE_START: u64 = 0xFFFF_8000_0000_0000;

// Physical memory offset for accessing physical memory directly
pub const PHYS_MEM_OFFSET: u64 = 0xFFFF_8000_0000_0000;

// NT-style memory protection constants
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageProtection {
    NoAccess = 0x01,
    ReadOnly = 0x02,
    ReadWrite = 0x04,
    WriteCopy = 0x08,
    Execute = 0x10,
    ExecuteRead = 0x20,
    ExecuteReadWrite = 0x40,
    ExecuteWriteCopy = 0x80,
    Guard = 0x100,
    NoCache = 0x200,
    WriteCombine = 0x400,
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub base_address: VirtAddr,
    pub size: u64,
    pub protection: PageProtection,
    pub allocation_type: AllocationType,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationType {
    Commit = 0x1000,
    Reserve = 0x2000,
    Reset = 0x80000,
    Physical = 0x400000,
    TopDown = 0x100000,
    WriteWatch = 0x200000,
    LargePages = 0x20000000,
}

pub struct MemoryManager {
    page_allocator: physical::PhysicalAllocator,
    virtual_allocator: virtual_memory::VirtualAllocator,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            page_allocator: physical::PhysicalAllocator::new(),
            virtual_allocator: virtual_memory::VirtualAllocator::new(),
        }
    }

    /// Windows NT VirtualAlloc equivalent
    pub fn virtual_alloc(
        &mut self,
        address: Option<VirtAddr>,
        size: u64,
        allocation_type: AllocationType,
        protect: PageProtection,
    ) -> Result<VirtAddr, MemoryError> {
        self.virtual_allocator.allocate(address, size, allocation_type, protect)
    }

    /// Windows NT VirtualFree equivalent
    pub fn virtual_free(
        &mut self,
        address: VirtAddr,
        size: u64,
        free_type: FreeType,
    ) -> Result<(), MemoryError> {
        self.virtual_allocator.free(address, size, free_type)
    }

    /// Windows NT VirtualProtect equivalent
    pub fn virtual_protect(
        &mut self,
        address: VirtAddr,
        size: u64,
        new_protect: PageProtection,
    ) -> Result<PageProtection, MemoryError> {
        self.virtual_allocator.protect(address, size, new_protect)
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum FreeType {
    Decommit = 0x4000,
    Release = 0x8000,
}

#[derive(Debug)]
pub enum MemoryError {
    OutOfMemory,
    InvalidAddress,
    AccessDenied,
    InvalidParameter,
}

// Global memory manager instance
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref MEMORY_MANAGER: Mutex<MemoryManager> = Mutex::new(MemoryManager::new());
}

// NT-compatible memory functions
pub fn nt_allocate_virtual_memory(
    process_handle: usize,
    base_address: &mut VirtAddr,
    zero_bits: usize,
    region_size: &mut u64,
    allocation_type: AllocationType,
    protect: PageProtection,
) -> Result<(), MemoryError> {
    let mut mm = MEMORY_MANAGER.lock();
    
    let allocated_addr = mm.virtual_alloc(
        if base_address.as_u64() == 0 { None } else { Some(*base_address) },
        *region_size,
        allocation_type,
        protect,
    )?;
    
    *base_address = allocated_addr;
    Ok(())
}

pub fn nt_free_virtual_memory(
    process_handle: usize,
    base_address: &mut VirtAddr,
    region_size: &mut u64,
    free_type: FreeType,
) -> Result<(), MemoryError> {
    let mut mm = MEMORY_MANAGER.lock();
    mm.virtual_free(*base_address, *region_size, free_type)?;
    
    *base_address = VirtAddr::new(0);
    *region_size = 0;
    Ok(())
}