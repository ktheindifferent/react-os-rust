use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::ptr;
use spin::Mutex;
use x86_64::{PhysAddr, VirtAddr};

use super::HypervisorError;

const PAGE_SIZE: usize = 4096;
const HUGE_PAGE_SIZE: usize = 2 * 1024 * 1024;
const GIGAPAGE_SIZE: usize = 1024 * 1024 * 1024;

#[derive(Debug)]
pub struct GuestMemory {
    regions: Vec<MemoryRegion>,
    total_size: usize,
}

#[derive(Debug)]
struct MemoryRegion {
    guest_addr: u64,
    host_addr: u64,
    size: usize,
    flags: MemoryFlags,
}

bitflags::bitflags! {
    struct MemoryFlags: u32 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXECUTE = 1 << 2;
        const MMIO = 1 << 3;
        const CACHEABLE = 1 << 4;
    }
}

impl GuestMemory {
    pub fn new(size: usize) -> Result<Self, HypervisorError> {
        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        
        let host_memory = unsafe {
            let layout = alloc::alloc::Layout::from_size_align(aligned_size, PAGE_SIZE)
                .map_err(|_| HypervisorError::MemoryAllocationFailed)?;
            let ptr = alloc::alloc::alloc_zeroed(layout);
            if ptr.is_null() {
                return Err(HypervisorError::MemoryAllocationFailed);
            }
            ptr as u64
        };
        
        let region = MemoryRegion {
            guest_addr: 0,
            host_addr: host_memory,
            size: aligned_size,
            flags: MemoryFlags::READ | MemoryFlags::WRITE | MemoryFlags::EXECUTE | MemoryFlags::CACHEABLE,
        };
        
        Ok(Self {
            regions: vec![region],
            total_size: aligned_size,
        })
    }
    
    pub fn add_region(&mut self, guest_addr: u64, size: usize, flags: u32) -> Result<(), HypervisorError> {
        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        
        let host_memory = unsafe {
            let layout = alloc::alloc::Layout::from_size_align(aligned_size, PAGE_SIZE)
                .map_err(|_| HypervisorError::MemoryAllocationFailed)?;
            let ptr = alloc::alloc::alloc_zeroed(layout);
            if ptr.is_null() {
                return Err(HypervisorError::MemoryAllocationFailed);
            }
            ptr as u64
        };
        
        let region = MemoryRegion {
            guest_addr,
            host_addr: host_memory,
            size: aligned_size,
            flags: MemoryFlags::from_bits_truncate(flags),
        };
        
        self.regions.push(region);
        self.total_size += aligned_size;
        
        Ok(())
    }
    
    pub fn translate_gpa(&self, guest_addr: u64) -> Option<u64> {
        for region in &self.regions {
            if guest_addr >= region.guest_addr && 
               guest_addr < region.guest_addr + region.size as u64 {
                let offset = guest_addr - region.guest_addr;
                return Some(region.host_addr + offset);
            }
        }
        None
    }
    
    pub fn read(&self, guest_addr: u64, buf: &mut [u8]) -> Result<(), HypervisorError> {
        if let Some(host_addr) = self.translate_gpa(guest_addr) {
            unsafe {
                ptr::copy_nonoverlapping(
                    host_addr as *const u8,
                    buf.as_mut_ptr(),
                    buf.len()
                );
            }
            Ok(())
        } else {
            Err(HypervisorError::InvalidParameter)
        }
    }
    
    pub fn write(&self, guest_addr: u64, buf: &[u8]) -> Result<(), HypervisorError> {
        if let Some(host_addr) = self.translate_gpa(guest_addr) {
            unsafe {
                ptr::copy_nonoverlapping(
                    buf.as_ptr(),
                    host_addr as *mut u8,
                    buf.len()
                );
            }
            Ok(())
        } else {
            Err(HypervisorError::InvalidParameter)
        }
    }
}

#[repr(C, align(4096))]
pub struct EptPml4 {
    entries: [EptPml4e; 512],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EptPml4e {
    value: u64,
}

#[repr(C, align(4096))]
struct EptPdpt {
    entries: [EptPdpte; 512],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EptPdpte {
    value: u64,
}

#[repr(C, align(4096))]
struct EptPd {
    entries: [EptPde; 512],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EptPde {
    value: u64,
}

#[repr(C, align(4096))]
struct EptPt {
    entries: [EptPte; 512],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EptPte {
    value: u64,
}

pub struct EptManager {
    pml4: Box<EptPml4>,
    pdpts: Vec<Box<EptPdpt>>,
    pds: Vec<Box<EptPd>>,
    pts: Vec<Box<EptPt>>,
}

impl EptManager {
    pub fn new() -> Result<Self, HypervisorError> {
        let mut pml4 = Box::new(EptPml4 {
            entries: [EptPml4e { value: 0 }; 512],
        });
        
        let pdpts = Vec::new();
        let pds = Vec::new();
        let pts = Vec::new();
        
        Ok(Self {
            pml4,
            pdpts,
            pds,
            pts,
        })
    }
    
    pub fn map_page(&mut self, guest_phys: u64, host_phys: u64, flags: u64) -> Result<(), HypervisorError> {
        let pml4_idx = ((guest_phys >> 39) & 0x1FF) as usize;
        let pdpt_idx = ((guest_phys >> 30) & 0x1FF) as usize;
        let pd_idx = ((guest_phys >> 21) & 0x1FF) as usize;
        let pt_idx = ((guest_phys >> 12) & 0x1FF) as usize;
        
        if self.pml4.entries[pml4_idx].value == 0 {
            let pdpt = Box::new(EptPdpt {
                entries: [EptPdpte { value: 0 }; 512],
            });
            let pdpt_addr = &*pdpt as *const _ as u64;
            self.pml4.entries[pml4_idx].value = pdpt_addr | 0x7;
            self.pdpts.push(pdpt);
        }
        
        Ok(())
    }
    
    pub fn handle_violation(&self) -> Result<(), HypervisorError> {
        Ok(())
    }
    
    pub fn get_eptp(&self) -> u64 {
        let pml4_addr = &*self.pml4 as *const _ as u64;
        (pml4_addr & !0xFFF) | 0x1E
    }
}

pub struct NptManager {
    ncr3: u64,
    page_tables: BTreeMap<u64, Vec<u8>>,
}

impl NptManager {
    pub fn new() -> Result<Self, HypervisorError> {
        Ok(Self {
            ncr3: 0,
            page_tables: BTreeMap::new(),
        })
    }
    
    pub fn map_page(&mut self, guest_phys: u64, host_phys: u64, flags: u64) -> Result<(), HypervisorError> {
        Ok(())
    }
    
    pub fn handle_violation(&self) -> Result<(), HypervisorError> {
        Ok(())
    }
    
    pub fn get_ncr3(&self) -> u64 {
        self.ncr3
    }
}