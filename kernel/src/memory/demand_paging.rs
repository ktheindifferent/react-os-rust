// Demand Paging Implementation
use x86_64::{
    structures::paging::{
        Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
        mapper::{Mapper, MapperAllSizes, MappedFrame},
        frame::PhysFrameRange,
    },
    VirtAddr, PhysAddr,
};
use spin::Mutex;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use lazy_static::lazy_static;

// Page fault error codes
pub const PAGE_FAULT_PRESENT: u64 = 1 << 0;
pub const PAGE_FAULT_WRITE: u64 = 1 << 1;
pub const PAGE_FAULT_USER: u64 = 1 << 2;
pub const PAGE_FAULT_RESERVED_WRITE: u64 = 1 << 3;
pub const PAGE_FAULT_INSTRUCTION_FETCH: u64 = 1 << 4;

// Page states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageState {
    NotPresent,           // Page not allocated
    OnDisk,              // Page swapped to disk
    InMemory,            // Page in physical memory
    CopyOnWrite,         // COW page, shared until write
    Zero,                // Zero page, allocated on first access
}

// Page metadata
#[derive(Debug, Clone)]
pub struct PageInfo {
    pub state: PageState,
    pub frame: Option<PhysFrame>,
    pub swap_slot: Option<usize>,
    pub ref_count: usize,
    pub flags: PageTableFlags,
    pub cow_source: Option<PhysFrame>,
}

impl PageInfo {
    pub fn new_zero() -> Self {
        Self {
            state: PageState::Zero,
            frame: None,
            swap_slot: None,
            ref_count: 0,
            flags: PageTableFlags::empty(),
            cow_source: None,
        }
    }
    
    pub fn new_cow(source: PhysFrame, flags: PageTableFlags) -> Self {
        Self {
            state: PageState::CopyOnWrite,
            frame: Some(source),
            swap_slot: None,
            ref_count: 1,
            flags: flags & !PageTableFlags::WRITABLE, // Remove write permission
            cow_source: Some(source),
        }
    }
}

// Swap space management
pub struct SwapManager {
    swap_file: Vec<[u8; 4096]>,  // Simplified: in-memory swap
    free_slots: Vec<usize>,
    used_slots: BTreeMap<usize, Page>,
}

impl SwapManager {
    pub fn new(size_pages: usize) -> Self {
        let mut free_slots = Vec::new();
        for i in 0..size_pages {
            free_slots.push(i);
        }
        
        Self {
            swap_file: Vec::with_capacity(size_pages),
            free_slots,
            used_slots: BTreeMap::new(),
        }
    }
    
    pub fn allocate_slot(&mut self) -> Option<usize> {
        self.free_slots.pop()
    }
    
    pub fn free_slot(&mut self, slot: usize) {
        self.used_slots.remove(&slot);
        self.free_slots.push(slot);
    }
    
    pub fn swap_out(&mut self, page: Page, data: &[u8; 4096]) -> Option<usize> {
        let slot = self.allocate_slot()?;
        
        // Ensure swap file is large enough
        while self.swap_file.len() <= slot {
            self.swap_file.push([0; 4096]);
        }
        
        self.swap_file[slot] = *data;
        self.used_slots.insert(slot, page);
        Some(slot)
    }
    
    pub fn swap_in(&mut self, slot: usize) -> Option<[u8; 4096]> {
        if slot < self.swap_file.len() {
            let data = self.swap_file[slot];
            Some(data)
        } else {
            None
        }
    }
}

// Demand paging manager
pub struct DemandPagingManager {
    page_table: BTreeMap<Page, PageInfo>,
    swap_manager: SwapManager,
    zero_frame: PhysFrame,
}

impl DemandPagingManager {
    pub fn new(swap_size: usize) -> Self {
        // Allocate a zero frame
        let zero_frame = super::frame_allocator::allocate_frame()
            .expect("Failed to allocate zero frame");
        
        // Clear the zero frame
        unsafe {
            let ptr = zero_frame.start_address().as_u64() as *mut u8;
            core::ptr::write_bytes(ptr, 0, 4096);
        }
        
        Self {
            page_table: BTreeMap::new(),
            swap_manager: SwapManager::new(swap_size),
            zero_frame,
        }
    }
    
    // Handle page fault
    pub fn handle_page_fault(
        &mut self,
        addr: VirtAddr,
        error_code: u64,
        mapper: &mut impl Mapper<Size4KiB>,
    ) -> Result<(), &'static str> {
        let page = Page::<Size4KiB>::containing_address(addr);
        
        // Check if this is a known page
        let page_info = self.page_table.get_mut(&page)
            .ok_or("Page fault on unmapped page")?;
        
        match page_info.state {
            PageState::NotPresent => {
                return Err("Page not present");
            }
            
            PageState::Zero => {
                // Allocate a new frame for zero page
                let frame = super::frame_allocator::allocate_frame()
                    .ok_or("Out of memory")?;
                
                // Clear the frame
                unsafe {
                    let ptr = frame.start_address().as_u64() as *mut u8;
                    core::ptr::write_bytes(ptr, 0, 4096);
                }
                
                // Map the page
                let flags = PageTableFlags::PRESENT 
                    | PageTableFlags::WRITABLE 
                    | PageTableFlags::USER_ACCESSIBLE;
                
                unsafe {
                    mapper.map_to(page, frame, flags, &mut *super::frame_allocator::FRAME_ALLOCATOR.lock())
                        .map_err(|_| "Failed to map page")?
                        .flush();
                }
                
                page_info.state = PageState::InMemory;
                page_info.frame = Some(frame);
                page_info.flags = flags;
                
                Ok(())
            }
            
            PageState::OnDisk => {
                // Page is swapped out, bring it back
                let slot = page_info.swap_slot
                    .ok_or("No swap slot for swapped page")?;
                
                let data = self.swap_manager.swap_in(slot)
                    .ok_or("Failed to swap in page")?;
                
                // Allocate a new frame
                let frame = super::frame_allocator::allocate_frame()
                    .ok_or("Out of memory")?;
                
                // Copy data to frame
                unsafe {
                    let ptr = frame.start_address().as_u64() as *mut [u8; 4096];
                    *ptr = data;
                }
                
                // Map the page
                unsafe {
                    mapper.map_to(page, frame, page_info.flags | PageTableFlags::PRESENT, 
                        &mut *super::frame_allocator::FRAME_ALLOCATOR.lock())
                        .map_err(|_| "Failed to map page")?
                        .flush();
                }
                
                // Update page info
                page_info.state = PageState::InMemory;
                page_info.frame = Some(frame);
                self.swap_manager.free_slot(slot);
                page_info.swap_slot = None;
                
                Ok(())
            }
            
            PageState::CopyOnWrite => {
                // Check if this is a write fault
                if error_code & PAGE_FAULT_WRITE == 0 {
                    // Read fault on COW page - just map it read-only
                    if let Some(source) = page_info.cow_source {
                        unsafe {
                            mapper.map_to(page, source, 
                                page_info.flags & !PageTableFlags::WRITABLE,
                                &mut *super::frame_allocator::FRAME_ALLOCATOR.lock())
                                .map_err(|_| "Failed to map COW page")?
                                .flush();
                        }
                        return Ok(());
                    }
                }
                
                // Write fault - need to copy the page
                let source = page_info.cow_source
                    .ok_or("No source for COW page")?;
                
                // Allocate a new frame
                let new_frame = super::frame_allocator::allocate_frame()
                    .ok_or("Out of memory")?;
                
                // Copy the page content
                unsafe {
                    let src_ptr = source.start_address().as_u64() as *const u8;
                    let dst_ptr = new_frame.start_address().as_u64() as *mut u8;
                    core::ptr::copy_nonoverlapping(src_ptr, dst_ptr, 4096);
                }
                
                // Map the new frame with write permission
                unsafe {
                    mapper.map_to(page, new_frame, 
                        page_info.flags | PageTableFlags::WRITABLE | PageTableFlags::PRESENT,
                        &mut *super::frame_allocator::FRAME_ALLOCATOR.lock())
                        .map_err(|_| "Failed to map copied page")?
                        .flush();
                }
                
                // Update page info
                page_info.state = PageState::InMemory;
                page_info.frame = Some(new_frame);
                page_info.cow_source = None;
                page_info.ref_count = 1;
                
                Ok(())
            }
            
            PageState::InMemory => {
                // Page should be present, this shouldn't happen
                Err("Page fault on present page")
            }
        }
    }
    
    // Allocate a zero page (lazy allocation)
    pub fn allocate_zero_page(&mut self, page: Page) -> Result<(), &'static str> {
        if self.page_table.contains_key(&page) {
            return Err("Page already allocated");
        }
        
        self.page_table.insert(page, PageInfo::new_zero());
        Ok(())
    }
    
    // Set up copy-on-write for a page
    pub fn setup_cow(
        &mut self,
        page: Page,
        source_frame: PhysFrame,
        flags: PageTableFlags,
    ) -> Result<(), &'static str> {
        let page_info = PageInfo::new_cow(source_frame, flags);
        self.page_table.insert(page, page_info);
        Ok(())
    }
    
    // Swap out a page to disk
    pub fn swap_out_page(
        &mut self,
        page: Page,
        mapper: &mut impl Mapper<Size4KiB>,
    ) -> Result<(), &'static str> {
        let page_info = self.page_table.get_mut(&page)
            .ok_or("Page not found")?;
        
        if page_info.state != PageState::InMemory {
            return Err("Page not in memory");
        }
        
        let frame = page_info.frame
            .ok_or("No frame for in-memory page")?;
        
        // Read page content
        let data = unsafe {
            let ptr = frame.start_address().as_u64() as *const [u8; 4096];
            *ptr
        };
        
        // Swap to disk
        let slot = self.swap_manager.swap_out(page, &data)
            .ok_or("Failed to allocate swap slot")?;
        
        // Unmap the page
        mapper.unmap(page)
            .map_err(|_| "Failed to unmap page")?
            .1.flush();
        
        // Free the frame
        super::frame_allocator::deallocate_frame(frame);
        
        // Update page info
        page_info.state = PageState::OnDisk;
        page_info.frame = None;
        page_info.swap_slot = Some(slot);
        
        Ok(())
    }
    
    // Fork a process's memory (for COW)
    pub fn fork_memory_space(
        &mut self,
        parent_pages: &BTreeMap<Page, PageInfo>,
    ) -> BTreeMap<Page, PageInfo> {
        let mut child_pages = BTreeMap::new();
        
        for (page, info) in parent_pages {
            match info.state {
                PageState::InMemory => {
                    // Set up COW for both parent and child
                    if let Some(frame) = info.frame {
                        let child_info = PageInfo::new_cow(frame, info.flags);
                        child_pages.insert(*page, child_info);
                        
                        // Note: In real implementation, would also update parent's page
                        // to be COW and increase reference count
                    }
                }
                PageState::Zero => {
                    // Share zero pages
                    child_pages.insert(*page, PageInfo::new_zero());
                }
                _ => {
                    // Copy other page states
                    child_pages.insert(*page, info.clone());
                }
            }
        }
        
        child_pages
    }
}

// Global demand paging manager
lazy_static! {
    pub static ref DEMAND_PAGING: Mutex<Option<DemandPagingManager>> = Mutex::new(None);
}

// Initialize demand paging
pub fn init_demand_paging(swap_size_mb: usize) {
    let swap_pages = swap_size_mb * 256; // 256 pages per MB
    let manager = DemandPagingManager::new(swap_pages);
    *DEMAND_PAGING.lock() = Some(manager);
    crate::serial_println!("Demand paging initialized with {}MB swap", swap_size_mb);
}

// Handle page fault from interrupt handler
pub fn handle_page_fault(addr: VirtAddr, error_code: u64) -> Result<(), &'static str> {
    let mut demand_paging = DEMAND_PAGING.lock();
    if let Some(ref mut manager) = *demand_paging {
        // Get active page table
        use x86_64::registers::control::Cr3;
        let (level_4_table_frame, _) = Cr3::read();
        let phys = level_4_table_frame.start_address();
        let virt = VirtAddr::new(phys.as_u64());
        let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
        
        unsafe {
            use x86_64::structures::paging::OffsetPageTable;
            let mut mapper = OffsetPageTable::new(
                &mut *page_table_ptr,
                VirtAddr::new(0),
            );
            
            manager.handle_page_fault(addr, error_code, &mut mapper)
        }
    } else {
        Err("Demand paging not initialized")
    }
}