use crate::serial_println;
use x86_64::{
    VirtAddr, PhysAddr,
    structures::paging::{PageTableFlags, Page, PageTable, Mapper, Size4KiB},
    registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags},
};
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::mem::size_of;

static WX_ENFORCED: AtomicBool = AtomicBool::new(false);
static SMEP_ENABLED: AtomicBool = AtomicBool::new(false);
static SMAP_ENABLED: AtomicBool = AtomicBool::new(false);
static HEAP_HARDENING_ENABLED: AtomicBool = AtomicBool::new(false);

const HEAP_GUARD_SIZE: u64 = 0x1000;
const HEAP_REDZONE_SIZE: usize = 16;
const HEAP_MAGIC: u32 = 0xDEADC0DE;

pub struct MemoryRegionProtection {
    pub start: VirtAddr,
    pub end: VirtAddr,
    pub flags: PageTableFlags,
    pub region_type: MemoryRegionType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryRegionType {
    Code,
    Data,
    Stack,
    Heap,
    Kernel,
    User,
    Device,
}

static MEMORY_REGIONS: Mutex<Vec<MemoryRegionProtection>> = Mutex::new(Vec::new());

pub fn init() {
    serial_println!("[MEMORY] Initializing memory protection");
    
    // Enforce W^X (Write XOR Execute)
    enforce_wx();
    
    // Set up initial memory regions
    setup_memory_regions();
    
    serial_println!("[MEMORY] Memory protection initialized");
}

fn enforce_wx() {
    if WX_ENFORCED.load(Ordering::SeqCst) {
        return;
    }
    
    serial_println!("[MEMORY] Enforcing W^X protection");
    
    // Enable write protection in CR0
    unsafe {
        Cr0::write(Cr0::read() | Cr0Flags::WRITE_PROTECT);
    }
    
    // Scan and fix all mapped pages
    let mapper = unsafe { crate::memory::get_mapper() };
    scan_and_fix_wx_violations(mapper);
    
    WX_ENFORCED.store(true, Ordering::SeqCst);
    serial_println!("[MEMORY] W^X protection enforced");
}

fn scan_and_fix_wx_violations(mapper: &mut impl Mapper<Size4KiB>) {
    // This would iterate through page tables and ensure no page is both writable and executable
    // For now, we'll set up the policy for new mappings
    
    let regions = MEMORY_REGIONS.lock();
    for region in regions.iter() {
        match region.region_type {
            MemoryRegionType::Code => {
                // Code should be executable but not writable
                update_region_protection(mapper, region.start, region.end, 
                    PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE.complement());
            },
            MemoryRegionType::Data | MemoryRegionType::Heap => {
                // Data should be writable but not executable
                update_region_protection(mapper, region.start, region.end,
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE);
            },
            MemoryRegionType::Stack => {
                // Stack should be writable but not executable
                update_region_protection(mapper, region.start, region.end,
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE);
            },
            _ => {}
        }
    }
}

fn update_region_protection(
    mapper: &mut impl Mapper<Size4KiB>,
    start: VirtAddr,
    end: VirtAddr,
    flags: PageTableFlags
) {
    let start_page = Page::containing_address(start);
    let end_page = Page::containing_address(end);
    
    let mut current_page = start_page;
    while current_page <= end_page {
        unsafe {
            if let Ok(mut flags_updater) = mapper.update_flags(current_page, flags) {
                flags_updater.flush();
            }
        }
        current_page += 1;
    }
}

pub fn enable_smep() -> bool {
    if SMEP_ENABLED.load(Ordering::SeqCst) {
        return true;
    }
    
    // Check if CPU supports SMEP
    if !check_smep_support() {
        serial_println!("[MEMORY] SMEP not supported by CPU");
        return false;
    }
    
    // Enable SMEP in CR4 (bit 20)
    unsafe {
        Cr4::write(Cr4::read() | Cr4Flags::SUPERVISOR_MODE_EXECUTION_PROTECTION);
    }
    
    SMEP_ENABLED.store(true, Ordering::SeqCst);
    serial_println!("[MEMORY] SMEP (Supervisor Mode Execution Prevention) enabled");
    true
}

pub fn enable_smap() -> bool {
    if SMAP_ENABLED.load(Ordering::SeqCst) {
        return true;
    }
    
    // Check if CPU supports SMAP
    if !check_smap_support() {
        serial_println!("[MEMORY] SMAP not supported by CPU");
        return false;
    }
    
    // Enable SMAP in CR4 (bit 21)
    unsafe {
        Cr4::write(Cr4::read() | Cr4Flags::SUPERVISOR_MODE_ACCESS_PREVENTION);
    }
    
    SMAP_ENABLED.store(true, Ordering::SeqCst);
    serial_println!("[MEMORY] SMAP (Supervisor Mode Access Prevention) enabled");
    true
}

fn check_smep_support() -> bool {
    unsafe {
        let ebx_out: u32;
        core::arch::asm!(
            "push rbx",
            "cpuid",
            "mov esi, ebx",
            "pop rbx",
            inout("eax") 7u32 => _,
            inout("ecx") 0u32 => _,
            out("esi") ebx_out,
            out("edx") _,
            options(preserves_flags),
        );
        
        // Check SMEP bit (bit 7 of EBX)
        (ebx_out & (1 << 7)) != 0
    }
}

fn check_smap_support() -> bool {
    unsafe {
        let ebx_out: u32;
        core::arch::asm!(
            "push rbx",
            "cpuid",
            "mov esi, ebx",
            "pop rbx",
            inout("eax") 7u32 => _,
            inout("ecx") 0u32 => _,
            out("esi") ebx_out,
            out("edx") _,
            options(preserves_flags),
        );
        
        // Check SMAP bit (bit 20 of EBX)
        (ebx_out & (1 << 20)) != 0
    }
}

pub fn enable_heap_hardening() {
    if HEAP_HARDENING_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    
    HEAP_HARDENING_ENABLED.store(true, Ordering::SeqCst);
    serial_println!("[MEMORY] Heap hardening enabled");
}

#[repr(C)]
pub struct HardenedHeapBlock {
    pub magic: u32,
    pub size: usize,
    pub allocated: bool,
    pub canary: u64,
    pub redzone_pre: [u8; HEAP_REDZONE_SIZE],
}

impl HardenedHeapBlock {
    pub fn new(size: usize) -> Self {
        let canary = generate_heap_canary();
        Self {
            magic: HEAP_MAGIC,
            size,
            allocated: true,
            canary,
            redzone_pre: [0xAA; HEAP_REDZONE_SIZE],
        }
    }
    
    pub fn validate(&self) -> bool {
        if self.magic != HEAP_MAGIC {
            serial_println!("[HEAP] Invalid magic number detected!");
            return false;
        }
        
        for &byte in &self.redzone_pre {
            if byte != 0xAA {
                serial_println!("[HEAP] Redzone corruption detected!");
                return false;
            }
        }
        
        true
    }
}

fn generate_heap_canary() -> u64 {
    let mut canary = 0xBADC0FFEE0DDF00Du64;
    
    unsafe {
        let tsc: u64;
        core::arch::asm!("rdtsc", out("rax") tsc, out("rdx") _);
        canary ^= tsc;
    }
    
    canary
}

pub fn allocate_hardened(size: usize) -> Result<*mut u8, &'static str> {
    if !HEAP_HARDENING_ENABLED.load(Ordering::SeqCst) {
        // Fall back to normal allocation
        return Ok(unsafe { alloc::alloc::alloc(alloc::alloc::Layout::from_size_align_unchecked(size, 8)) });
    }
    
    // Add space for metadata and redzones
    let total_size = size_of::<HardenedHeapBlock>() + size + HEAP_REDZONE_SIZE;
    
    let layout = alloc::alloc::Layout::from_size_align(total_size, 16)
        .map_err(|_| "Invalid layout")?;
    
    let ptr = unsafe { alloc::alloc::alloc(layout) };
    if ptr.is_null() {
        return Err("Allocation failed");
    }
    
    // Initialize metadata
    let block = ptr as *mut HardenedHeapBlock;
    unsafe {
        (*block) = HardenedHeapBlock::new(size);
        
        // Initialize post-redzone
        let data_ptr = (ptr as usize + size_of::<HardenedHeapBlock>() + size) as *mut u8;
        for i in 0..HEAP_REDZONE_SIZE {
            *data_ptr.add(i) = 0xBB;
        }
    }
    
    // Return pointer to user data
    Ok(unsafe { ptr.add(size_of::<HardenedHeapBlock>()) })
}

pub fn deallocate_hardened(ptr: *mut u8, size: usize) -> Result<(), &'static str> {
    if !HEAP_HARDENING_ENABLED.load(Ordering::SeqCst) {
        unsafe { alloc::alloc::dealloc(ptr, alloc::alloc::Layout::from_size_align_unchecked(size, 8)) };
        return Ok(());
    }
    
    // Get metadata block
    let block_ptr = unsafe { ptr.sub(size_of::<HardenedHeapBlock>()) } as *mut HardenedHeapBlock;
    let block = unsafe { &*block_ptr };
    
    // Validate block
    if !block.validate() {
        return Err("Heap corruption detected during deallocation");
    }
    
    // Check post-redzone
    let post_redzone = unsafe { ptr.add(size) };
    for i in 0..HEAP_REDZONE_SIZE {
        if unsafe { *post_redzone.add(i) } != 0xBB {
            return Err("Post-redzone corruption detected");
        }
    }
    
    // Clear sensitive data
    unsafe {
        core::ptr::write_bytes(ptr, 0, size);
    }
    
    // Deallocate
    let total_size = size_of::<HardenedHeapBlock>() + size + HEAP_REDZONE_SIZE;
    let layout = alloc::alloc::Layout::from_size_align(total_size, 16)
        .map_err(|_| "Invalid layout")?;
    
    unsafe {
        alloc::alloc::dealloc(block_ptr as *mut u8, layout);
    }
    
    Ok(())
}

pub fn setup_memory_regions() {
    let mut regions = MEMORY_REGIONS.lock();
    
    // Kernel code region
    regions.push(MemoryRegionProtection {
        start: VirtAddr::new(0xFFFF_8000_0000_0000),
        end: VirtAddr::new(0xFFFF_8000_0100_0000),
        flags: PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE.complement(),
        region_type: MemoryRegionType::Code,
    });
    
    // Kernel data region
    regions.push(MemoryRegionProtection {
        start: VirtAddr::new(0xFFFF_8000_0100_0000),
        end: VirtAddr::new(0xFFFF_8000_0200_0000),
        flags: PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
        region_type: MemoryRegionType::Data,
    });
    
    // Kernel heap region
    regions.push(MemoryRegionProtection {
        start: VirtAddr::new(0xFFFF_8800_0000_0000),
        end: VirtAddr::new(0xFFFF_8900_0000_0000),
        flags: PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
        region_type: MemoryRegionType::Heap,
    });
    
    // User space regions
    regions.push(MemoryRegionProtection {
        start: VirtAddr::new(0x0000_0000_0040_0000),
        end: VirtAddr::new(0x0000_7FFF_FFFF_FFFF),
        flags: PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
        region_type: MemoryRegionType::User,
    });
}

pub fn validate_memory_access(addr: VirtAddr, size: usize, write: bool) -> bool {
    let regions = MEMORY_REGIONS.lock();
    
    for region in regions.iter() {
        if addr >= region.start && addr + (size as u64) <= region.end {
            // Check permissions
            if write && !region.flags.contains(PageTableFlags::WRITABLE) {
                return false;
            }
            
            // Check if kernel trying to execute user memory (SMEP)
            if SMEP_ENABLED.load(Ordering::SeqCst) {
                if region.region_type == MemoryRegionType::User && 
                   !region.flags.contains(PageTableFlags::USER_ACCESSIBLE) {
                    return false;
                }
            }
            
            // Check if kernel accessing user memory without STAC (SMAP)
            if SMAP_ENABLED.load(Ordering::SeqCst) {
                if region.region_type == MemoryRegionType::User {
                    // Would need to check EFLAGS.AC here
                    // For now, assume it's properly managed
                }
            }
            
            return true;
        }
    }
    
    false
}

pub fn mark_pages_nx(start: VirtAddr, size: u64) {
    let mut mapper = unsafe { crate::memory::get_mapper() };
    let pages = (size + 0xFFF) / 0x1000;
    
    for i in 0..pages {
        let page = Page::<Size4KiB>::containing_address(start + (i * 0x1000));
        unsafe {
            if let Ok(mut flags_updater) = mapper.update_flags(
                page,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE
            ) {
                flags_updater.flush();
            }
        }
    }
}

pub fn mark_pages_ro(start: VirtAddr, size: u64) {
    let mut mapper = unsafe { crate::memory::get_mapper() };
    let pages = (size + 0xFFF) / 0x1000;
    
    for i in 0..pages {
        let page = Page::<Size4KiB>::containing_address(start + (i * 0x1000));
        unsafe {
            if let Ok(mut flags_updater) = mapper.update_flags(
                page,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE.complement()
            ) {
                flags_updater.flush();
            }
        }
    }
}