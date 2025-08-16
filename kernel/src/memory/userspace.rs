use x86_64::{
    structures::paging::{
        PageTableFlags, PhysFrame,
        mapper::{MapperAllSizes, MappedFrame}
    },
    VirtAddr,
};
use spin::Mutex;
use lazy_static::lazy_static;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::ops::Range;

pub const USER_SPACE_START: u64 = 0x0000_0000_0000_1000;
pub const USER_SPACE_END: u64 = 0x0000_7FFF_FFFF_F000;
pub const KERNEL_SPACE_START: u64 = 0xFFFF_8000_0000_0000;
pub const KERNEL_SPACE_END: u64 = 0xFFFF_FFFF_FFFF_F000;

pub const USER_STACK_SIZE: u64 = 1024 * 1024;
pub const USER_HEAP_SIZE: u64 = 16 * 1024 * 1024;
pub const USER_STACK_TOP: u64 = USER_SPACE_END;
pub const USER_STACK_BOTTOM: u64 = USER_STACK_TOP - USER_STACK_SIZE;
pub const USER_HEAP_START: u64 = 0x0000_0000_4000_0000;
pub const USER_HEAP_END: u64 = USER_HEAP_START + USER_HEAP_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    Code,
    Data,
    Stack,
    Heap,
    Shared,
    MappedFile,
}

#[derive(Debug)]
pub struct MemoryRegion {
    pub start: VirtAddr,
    pub end: VirtAddr,
    pub region_type: MemoryRegionType,
    pub flags: PageTableFlags,
    pub mapped: bool,
}

impl MemoryRegion {
    pub fn new(start: VirtAddr, end: VirtAddr, region_type: MemoryRegionType) -> Self {
        let flags = match region_type {
            MemoryRegionType::Code => {
                PageTableFlags::PRESENT 
                | PageTableFlags::USER_ACCESSIBLE
            }
            MemoryRegionType::Data | MemoryRegionType::Heap => {
                PageTableFlags::PRESENT 
                | PageTableFlags::WRITABLE 
                | PageTableFlags::USER_ACCESSIBLE
                | PageTableFlags::NO_EXECUTE
            }
            MemoryRegionType::Stack => {
                PageTableFlags::PRESENT 
                | PageTableFlags::WRITABLE 
                | PageTableFlags::USER_ACCESSIBLE
                | PageTableFlags::NO_EXECUTE
            }
            MemoryRegionType::Shared => {
                PageTableFlags::PRESENT 
                | PageTableFlags::WRITABLE 
                | PageTableFlags::USER_ACCESSIBLE
            }
            MemoryRegionType::MappedFile => {
                PageTableFlags::PRESENT 
                | PageTableFlags::USER_ACCESSIBLE
            }
        };

        MemoryRegion {
            start,
            end,
            region_type,
            flags,
            mapped: false,
        }
    }

    pub fn size(&self) -> u64 {
        self.end.as_u64() - self.start.as_u64()
    }

    pub fn contains(&self, addr: VirtAddr) -> bool {
        addr >= self.start && addr < self.end
    }
}

pub struct AddressSpace {
    pub page_table: PhysFrame,
    pub regions: BTreeMap<u64, MemoryRegion>,
    pub brk: VirtAddr,
}

impl AddressSpace {
    pub fn new(page_table: PhysFrame) -> Self {
        AddressSpace {
            page_table,
            regions: BTreeMap::new(),
            brk: VirtAddr::new(USER_HEAP_START),
        }
    }

    pub fn add_region(&mut self, region: MemoryRegion) -> Result<(), &'static str> {
        if region.start.as_u64() < USER_SPACE_START || region.end.as_u64() > USER_SPACE_END {
            return Err("Region outside user space");
        }

        for (_, existing) in &self.regions {
            if region.start < existing.end && region.end > existing.start {
                return Err("Region overlaps with existing region");
            }
        }

        self.regions.insert(region.start.as_u64(), region);
        Ok(())
    }

    pub fn remove_region(&mut self, start: VirtAddr) -> Option<MemoryRegion> {
        self.regions.remove(&start.as_u64())
    }

    pub fn find_region(&self, addr: VirtAddr) -> Option<&MemoryRegion> {
        for (_, region) in &self.regions {
            if region.contains(addr) {
                return Some(region);
            }
        }
        None
    }

    pub fn find_region_mut(&mut self, addr: VirtAddr) -> Option<&mut MemoryRegion> {
        for (_, region) in &mut self.regions {
            if region.contains(addr) {
                return Some(region);
            }
        }
        None
    }

    pub fn find_free_region(&self, size: u64, alignment: u64) -> Option<VirtAddr> {
        let mut current = VirtAddr::new(USER_SPACE_START);
        let size_aligned = (size + alignment - 1) & !(alignment - 1);

        let mut sorted_regions: Vec<_> = self.regions.values().collect();
        sorted_regions.sort_by_key(|r| r.start);

        for region in sorted_regions {
            let gap_end = region.start;
            let gap_size = gap_end.as_u64() - current.as_u64();

            if gap_size >= size_aligned {
                let aligned = (current.as_u64() + alignment - 1) & !(alignment - 1);
                if aligned + size_aligned <= gap_end.as_u64() {
                    return Some(VirtAddr::new(aligned));
                }
            }

            current = region.end;
        }

        let remaining = USER_SPACE_END - current.as_u64();
        if remaining >= size_aligned {
            let aligned = (current.as_u64() + alignment - 1) & !(alignment - 1);
            if aligned + size_aligned <= USER_SPACE_END {
                return Some(VirtAddr::new(aligned));
            }
        }

        None
    }

    pub fn set_brk(&mut self, new_brk: VirtAddr) -> Result<VirtAddr, &'static str> {
        if new_brk < VirtAddr::new(USER_HEAP_START) || new_brk > VirtAddr::new(USER_HEAP_END) {
            return Err("Invalid brk address");
        }

        let old_brk = self.brk;
        self.brk = new_brk;
        Ok(old_brk)
    }
}

pub struct UserSpaceManager {
    address_spaces: BTreeMap<u64, AddressSpace>,
    next_asid: u64,
}

impl UserSpaceManager {
    pub fn new() -> Self {
        UserSpaceManager {
            address_spaces: BTreeMap::new(),
            next_asid: 1,
        }
    }

    pub fn create_address_space(&mut self, page_table: PhysFrame) -> u64 {
        let asid = self.next_asid;
        self.next_asid += 1;

        let mut space = AddressSpace::new(page_table);

        space.add_region(MemoryRegion::new(
            VirtAddr::new(USER_STACK_BOTTOM),
            VirtAddr::new(USER_STACK_TOP),
            MemoryRegionType::Stack,
        )).unwrap();

        space.add_region(MemoryRegion::new(
            VirtAddr::new(USER_HEAP_START),
            VirtAddr::new(USER_HEAP_START),
            MemoryRegionType::Heap,
        )).unwrap();

        self.address_spaces.insert(asid, space);
        asid
    }

    pub fn destroy_address_space(&mut self, asid: u64) -> Option<AddressSpace> {
        self.address_spaces.remove(&asid)
    }

    pub fn get_address_space(&self, asid: u64) -> Option<&AddressSpace> {
        self.address_spaces.get(&asid)
    }

    pub fn get_address_space_mut(&mut self, asid: u64) -> Option<&mut AddressSpace> {
        self.address_spaces.get_mut(&asid)
    }
}

lazy_static! {
    pub static ref USER_SPACE_MANAGER: Mutex<UserSpaceManager> = Mutex::new(UserSpaceManager::new());
}

pub fn init() {
    crate::serial_println!("Userspace memory manager initialized");
    crate::serial_println!("  User space: 0x{:016X} - 0x{:016X}", USER_SPACE_START, USER_SPACE_END);
    crate::serial_println!("  Kernel space: 0x{:016X} - 0x{:016X}", KERNEL_SPACE_START, KERNEL_SPACE_END);
}

pub fn is_user_accessible(addr: VirtAddr) -> bool {
    let addr_u64 = addr.as_u64();
    addr_u64 >= USER_SPACE_START && addr_u64 < USER_SPACE_END
}

pub fn is_kernel_space(addr: VirtAddr) -> bool {
    let addr_u64 = addr.as_u64();
    addr_u64 >= KERNEL_SPACE_START && addr_u64 < KERNEL_SPACE_END
}

pub fn validate_user_buffer(addr: VirtAddr, size: usize) -> bool {
    let start = addr.as_u64();
    let end = start.saturating_add(size as u64);
    
    start >= USER_SPACE_START && end <= USER_SPACE_END
}