// GPU Memory Management
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use x86_64::{PhysAddr, VirtAddr};
use super::{MemoryType, BufferObject, TilingMode, CacheLevel, BufferUsageFlags};

// GTT (Graphics Translation Table) Entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GttEntry {
    pub valid: bool,
    pub writable: bool,
    pub snooped: bool,
    pub cached: bool,
    pub address: PhysAddr,
}

impl GttEntry {
    pub fn to_raw(&self) -> u64 {
        let mut entry = self.address.as_u64() & 0xFFFF_FFFF_F000;
        
        if self.valid {
            entry |= 1;
        }
        if self.writable {
            entry |= 1 << 1;
        }
        if self.snooped {
            entry |= 1 << 11;
        }
        if self.cached {
            entry |= 3 << 3; // LLC cached
        }
        
        entry
    }
    
    pub fn from_raw(raw: u64) -> Self {
        Self {
            valid: (raw & 1) != 0,
            writable: (raw & (1 << 1)) != 0,
            snooped: (raw & (1 << 11)) != 0,
            cached: ((raw >> 3) & 3) == 3,
            address: PhysAddr::new(raw & 0xFFFF_FFFF_F000),
        }
    }
}

// Graphics Translation Table
pub struct Gtt {
    base_address: VirtAddr,
    entries: usize,
    page_size: usize,
}

impl Gtt {
    pub fn new(base_address: VirtAddr, size: usize, page_size: usize) -> Self {
        let entries = size / page_size;
        Self {
            base_address,
            entries,
            page_size,
        }
    }
    
    pub fn map_page(&mut self, gtt_index: usize, phys_addr: PhysAddr, flags: GttEntry) {
        if gtt_index >= self.entries {
            return;
        }
        
        let mut entry = flags;
        entry.address = phys_addr;
        entry.valid = true;
        
        unsafe {
            let gtt_ptr = (self.base_address.as_u64() + (gtt_index * 8) as u64) as *mut u64;
            *gtt_ptr = entry.to_raw();
        }
    }
    
    pub fn unmap_page(&mut self, gtt_index: usize) {
        if gtt_index >= self.entries {
            return;
        }
        
        unsafe {
            let gtt_ptr = (self.base_address.as_u64() + (gtt_index * 8) as u64) as *mut u64;
            *gtt_ptr = 0; // Clear the entry
        }
    }
    
    pub fn flush(&self) {
        // Force TLB flush for GTT changes
        unsafe {
            core::arch::asm!("wbinvd");
        }
    }
}

// Memory Allocator for GPU
pub struct GpuMemoryAllocator {
    total_size: u64,
    page_size: u64,
    free_pages: Vec<u64>,
    allocated_objects: BTreeMap<u64, BufferObject>,
    next_object_id: u64,
}

impl GpuMemoryAllocator {
    pub fn new(total_size: u64, page_size: u64) -> Self {
        let num_pages = total_size / page_size;
        let mut free_pages = Vec::with_capacity(num_pages as usize);
        
        for i in 0..num_pages {
            free_pages.push(i * page_size);
        }
        
        Self {
            total_size,
            page_size,
            free_pages,
            allocated_objects: BTreeMap::new(),
            next_object_id: 1,
        }
    }
    
    pub fn allocate(&mut self, size: u64, memory_type: MemoryType, 
                   usage: BufferUsageFlags) -> Result<BufferObject, &'static str> {
        let pages_needed = (size + self.page_size - 1) / self.page_size;
        
        if self.free_pages.len() < pages_needed as usize {
            return Err("Not enough free memory");
        }
        
        let mut pages = Vec::new();
        for _ in 0..pages_needed {
            if let Some(page) = self.free_pages.pop() {
                pages.push(page);
            } else {
                // Return pages on failure
                self.free_pages.extend(pages);
                return Err("Failed to allocate pages");
            }
        }
        
        let base_offset = pages[0];
        let object = BufferObject {
            id: self.next_object_id,
            size,
            memory_type,
            virtual_address: None,
            physical_address: Some(PhysAddr::new(base_offset)),
            is_pinned: false,
            is_tiled: false,
            tiling_mode: TilingMode::Linear,
            cache_level: CacheLevel::None,
            usage_flags: usage,
        };
        
        self.allocated_objects.insert(self.next_object_id, object.clone());
        self.next_object_id += 1;
        
        Ok(object)
    }
    
    pub fn free(&mut self, object_id: u64) -> Result<(), &'static str> {
        if let Some(object) = self.allocated_objects.remove(&object_id) {
            if let Some(phys_addr) = object.physical_address {
                let pages_count = (object.size + self.page_size - 1) / self.page_size;
                let base_page = phys_addr.as_u64();
                
                for i in 0..pages_count {
                    self.free_pages.push(base_page + i * self.page_size);
                }
            }
            Ok(())
        } else {
            Err("Object not found")
        }
    }
    
    pub fn get_object(&self, object_id: u64) -> Option<&BufferObject> {
        self.allocated_objects.get(&object_id)
    }
    
    pub fn get_free_memory(&self) -> u64 {
        (self.free_pages.len() as u64) * self.page_size
    }
    
    pub fn get_used_memory(&self) -> u64 {
        self.total_size - self.get_free_memory()
    }
}

// Memory Pool for specific usage types
pub struct MemoryPool {
    name: &'static str,
    memory_type: MemoryType,
    allocator: GpuMemoryAllocator,
    high_water_mark: u64,
}

impl MemoryPool {
    pub fn new(name: &'static str, memory_type: MemoryType, 
               size: u64, page_size: u64) -> Self {
        Self {
            name,
            memory_type,
            allocator: GpuMemoryAllocator::new(size, page_size),
            high_water_mark: 0,
        }
    }
    
    pub fn allocate(&mut self, size: u64, usage: BufferUsageFlags) -> Result<BufferObject, &'static str> {
        let result = self.allocator.allocate(size, self.memory_type, usage)?;
        
        let used = self.allocator.get_used_memory();
        if used > self.high_water_mark {
            self.high_water_mark = used;
        }
        
        Ok(result)
    }
    
    pub fn free(&mut self, object_id: u64) -> Result<(), &'static str> {
        self.allocator.free(object_id)
    }
    
    pub fn get_stats(&self) -> MemoryPoolStats {
        MemoryPoolStats {
            name: self.name,
            total_size: self.allocator.total_size,
            used_size: self.allocator.get_used_memory(),
            free_size: self.allocator.get_free_memory(),
            high_water_mark: self.high_water_mark,
            allocation_count: self.allocator.allocated_objects.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryPoolStats {
    pub name: &'static str,
    pub total_size: u64,
    pub used_size: u64,
    pub free_size: u64,
    pub high_water_mark: u64,
    pub allocation_count: usize,
}

// Memory Manager for all GPU memory
pub struct GpuMemoryManager {
    pools: Vec<MemoryPool>,
    gtt: Option<Gtt>,
    stolen_memory_base: PhysAddr,
    stolen_memory_size: u64,
}

impl GpuMemoryManager {
    pub fn new() -> Self {
        Self {
            pools: Vec::new(),
            gtt: None,
            stolen_memory_base: PhysAddr::new(0),
            stolen_memory_size: 0,
        }
    }
    
    pub fn init(&mut self, stolen_base: PhysAddr, stolen_size: u64,
                gtt_base: VirtAddr, gtt_size: usize) {
        self.stolen_memory_base = stolen_base;
        self.stolen_memory_size = stolen_size;
        
        // Initialize GTT
        self.gtt = Some(Gtt::new(gtt_base, gtt_size, 4096));
        
        // Create memory pools
        let page_size = 4096;
        
        // System memory pool (for command buffers, etc.)
        self.pools.push(MemoryPool::new(
            "System",
            MemoryType::SystemRam,
            256 * 1024 * 1024, // 256MB
            page_size
        ));
        
        // Stolen memory pool (for framebuffers, etc.)
        if stolen_size > 0 {
            self.pools.push(MemoryPool::new(
                "Stolen",
                MemoryType::Stolen,
                stolen_size,
                page_size
            ));
        }
        
        // GTT aperture pool
        self.pools.push(MemoryPool::new(
            "GTT",
            MemoryType::GttAperture,
            gtt_size as u64,
            page_size
        ));
    }
    
    pub fn allocate_buffer(&mut self, size: u64, memory_type: MemoryType,
                          usage: BufferUsageFlags) -> Result<BufferObject, &'static str> {
        // Find the appropriate pool
        for pool in &mut self.pools {
            if pool.memory_type == memory_type {
                return pool.allocate(size, usage);
            }
        }
        
        Err("No suitable memory pool found")
    }
    
    pub fn free_buffer(&mut self, object_id: u64) -> Result<(), &'static str> {
        for pool in &mut self.pools {
            if pool.allocator.get_object(object_id).is_some() {
                return pool.free(object_id);
            }
        }
        
        Err("Buffer not found in any pool")
    }
    
    pub fn map_to_gtt(&mut self, buffer: &BufferObject, gtt_offset: usize) -> Result<(), &'static str> {
        if let Some(gtt) = &mut self.gtt {
            if let Some(phys_addr) = buffer.physical_address {
                let pages = (buffer.size as usize + 4095) / 4096;
                
                for i in 0..pages {
                    let page_addr = PhysAddr::new(phys_addr.as_u64() + (i * 4096) as u64);
                    let entry = GttEntry {
                        valid: true,
                        writable: true,
                        snooped: buffer.cache_level != CacheLevel::None,
                        cached: buffer.cache_level == CacheLevel::WriteBack,
                        address: page_addr,
                    };
                    
                    gtt.map_page(gtt_offset + i, page_addr, entry);
                }
                
                gtt.flush();
                Ok(())
            } else {
                Err("Buffer has no physical address")
            }
        } else {
            Err("GTT not initialized")
        }
    }
    
    pub fn unmap_from_gtt(&mut self, gtt_offset: usize, pages: usize) -> Result<(), &'static str> {
        if let Some(gtt) = &mut self.gtt {
            for i in 0..pages {
                gtt.unmap_page(gtt_offset + i);
            }
            gtt.flush();
            Ok(())
        } else {
            Err("GTT not initialized")
        }
    }
    
    pub fn get_memory_stats(&self) -> Vec<MemoryPoolStats> {
        self.pools.iter().map(|pool| pool.get_stats()).collect()
    }
}

// DMA Buffer Management
pub struct DmaBuffer {
    pub physical_address: PhysAddr,
    pub virtual_address: VirtAddr,
    pub size: usize,
    pub coherent: bool,
}

impl DmaBuffer {
    pub fn allocate(size: usize, coherent: bool) -> Result<Self, &'static str> {
        // This would allocate DMA-capable memory
        // For now, return a placeholder
        Ok(Self {
            physical_address: PhysAddr::new(0),
            virtual_address: VirtAddr::new(0),
            size,
            coherent,
        })
    }
    
    pub fn free(self) {
        // Free the DMA buffer
    }
}