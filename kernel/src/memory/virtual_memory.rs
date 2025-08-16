use super::{PageProtection, AllocationType, FreeType, MemoryError, USER_SPACE_END, SYSTEM_SPACE_START};
use x86_64::VirtAddr;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct VirtualAddressDescriptor {
    pub start_address: VirtAddr,
    pub end_address: VirtAddr,
    pub protection: PageProtection,
    pub allocation_type: AllocationType,
    pub committed: bool,
}

#[derive(Debug)]
pub struct VirtualAllocator {
    // Track allocated regions
    allocated_regions: BTreeMap<u64, VirtualAddressDescriptor>,
    // Track free regions for efficient allocation
    free_regions: Vec<(VirtAddr, u64)>, // (start, size)
}

impl VirtualAllocator {
    pub fn new() -> Self {
        let mut allocator = Self {
            allocated_regions: BTreeMap::new(),
            free_regions: Vec::new(),
        };
        
        // Initialize with the entire user space as free
        allocator.free_regions.push((VirtAddr::new(0x10000), USER_SPACE_END - 0x10000));
        
        allocator
    }

    pub fn allocate(
        &mut self,
        preferred_address: Option<VirtAddr>,
        size: u64,
        allocation_type: AllocationType,
        protect: PageProtection,
    ) -> Result<VirtAddr, MemoryError> {
        let aligned_size = align_up(size, 4096);
        
        let base_address = if let Some(addr) = preferred_address {
            if self.is_address_available(addr, aligned_size) {
                addr
            } else {
                return Err(MemoryError::InvalidAddress);
            }
        } else {
            self.find_free_region(aligned_size)?
        };

        // Remove the allocated region from free regions
        self.remove_from_free_regions(base_address, aligned_size);

        // Create the VAD entry
        let vad = VirtualAddressDescriptor {
            start_address: base_address,
            end_address: VirtAddr::new(base_address.as_u64() + aligned_size),
            protection: protect,
            allocation_type,
            committed: allocation_type == AllocationType::Commit,
        };

        self.allocated_regions.insert(base_address.as_u64(), vad);

        // If this is a commit operation, we would map physical pages here
        if allocation_type == AllocationType::Commit {
            self.commit_pages(base_address, aligned_size, protect)?;
        }

        Ok(base_address)
    }

    pub fn free(
        &mut self,
        address: VirtAddr,
        size: u64,
        free_type: FreeType,
    ) -> Result<(), MemoryError> {
        let region = self.allocated_regions.get(&address.as_u64())
            .ok_or(MemoryError::InvalidAddress)?
            .clone();

        match free_type {
            FreeType::Decommit => {
                // Decommit pages but keep the reservation
                self.decommit_pages(address, size)?;
                
                // Update VAD to mark as not committed
                if let Some(vad) = self.allocated_regions.get_mut(&address.as_u64()) {
                    vad.committed = false;
                }
            }
            FreeType::Release => {
                // Release the entire reservation
                self.allocated_regions.remove(&address.as_u64());
                
                // Add back to free regions
                let region_size = region.end_address.as_u64() - region.start_address.as_u64();
                self.add_to_free_regions(region.start_address, region_size);
                
                // Unmap all pages in the region
                self.unmap_pages(region.start_address, region_size)?;
            }
        }

        Ok(())
    }

    pub fn protect(
        &mut self,
        address: VirtAddr,
        size: u64,
        new_protect: PageProtection,
    ) -> Result<PageProtection, MemoryError> {
        let old_protect = self.allocated_regions.get(&address.as_u64())
            .map(|vad| vad.protection)
            .ok_or(MemoryError::InvalidAddress)?;

        // Update protection in VAD
        if let Some(vad) = self.allocated_regions.get_mut(&address.as_u64()) {
            vad.protection = new_protect;
        }

        // Update page table entries with new protection
        self.update_page_protection(address, size, new_protect)?;

        Ok(old_protect)
    }

    fn is_address_available(&self, address: VirtAddr, size: u64) -> bool {
        let end_address = address.as_u64() + size;
        
        // Check if it overlaps with any allocated region
        for (_, vad) in &self.allocated_regions {
            if address.as_u64() < vad.end_address.as_u64() && 
               end_address > vad.start_address.as_u64() {
                return false;
            }
        }
        
        // Check if it's within user space
        address.as_u64() >= 0x10000 && end_address <= USER_SPACE_END
    }

    fn find_free_region(&self, size: u64) -> Result<VirtAddr, MemoryError> {
        for (start_addr, region_size) in &self.free_regions {
            if *region_size >= size {
                return Ok(*start_addr);
            }
        }
        Err(MemoryError::OutOfMemory)
    }

    fn remove_from_free_regions(&mut self, address: VirtAddr, size: u64) {
        let mut regions_to_add = Vec::new();
        
        self.free_regions.retain(|(start, region_size)| {
            let region_end = start.as_u64() + region_size;
            let alloc_end = address.as_u64() + size;
            
            // Check if this free region overlaps with the allocation
            if address.as_u64() < region_end && alloc_end > start.as_u64() {
                // Split the region if necessary
                if start.as_u64() < address.as_u64() {
                    // Add region before allocation
                    regions_to_add.push((*start, address.as_u64() - start.as_u64()));
                }
                if alloc_end < region_end {
                    // Add region after allocation
                    regions_to_add.push((VirtAddr::new(alloc_end), region_end - alloc_end));
                }
                false // Remove this region
            } else {
                true // Keep this region
            }
        });

        // Add the split regions
        self.free_regions.extend(regions_to_add);
    }

    fn add_to_free_regions(&mut self, address: VirtAddr, size: u64) {
        // For simplicity, just add the region. In a real implementation,
        // we would merge adjacent regions.
        self.free_regions.push((address, size));
    }

    // Placeholder implementations for page table operations
    fn commit_pages(&mut self, _address: VirtAddr, _size: u64, _protect: PageProtection) -> Result<(), MemoryError> {
        // In a real implementation, this would allocate physical frames
        // and map them in the page tables
        Ok(())
    }

    fn decommit_pages(&mut self, _address: VirtAddr, _size: u64) -> Result<(), MemoryError> {
        // In a real implementation, this would unmap pages but keep the VAD
        Ok(())
    }

    fn unmap_pages(&mut self, _address: VirtAddr, _size: u64) -> Result<(), MemoryError> {
        // In a real implementation, this would remove page table entries
        // and free physical frames
        Ok(())
    }

    fn update_page_protection(&mut self, _address: VirtAddr, _size: u64, _protect: PageProtection) -> Result<(), MemoryError> {
        // In a real implementation, this would update page table entry flags
        Ok(())
    }
}

fn align_up(value: u64, alignment: u64) -> u64 {
    (value + alignment - 1) & !(alignment - 1)
}