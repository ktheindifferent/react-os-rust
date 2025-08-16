//! DMA and Memory Management APIs for Drivers

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    sync::Arc,
    vec::Vec,
};
use core::{
    marker::PhantomData,
    mem,
    ptr::NonNull,
    slice,
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
};
use spin::{Mutex, RwLock};

use super::{Device, DriverError, Result};

/// DMA direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDirection {
    /// Device to memory
    ToDevice,
    /// Memory to device
    FromDevice,
    /// Bidirectional
    Bidirectional,
    /// No DMA (coherent memory)
    None,
}

/// DMA attributes
#[derive(Debug, Clone, Copy, Default)]
pub struct DmaAttributes {
    /// Use coherent memory
    pub coherent: bool,
    /// Allow write combining
    pub write_combine: bool,
    /// Force contiguous allocation
    pub force_contiguous: bool,
    /// Skip CPU cache sync
    pub skip_cpu_sync: bool,
    /// Use streaming DMA
    pub streaming: bool,
}

/// DMA buffer
pub struct DmaBuffer {
    /// Virtual address
    virt_addr: NonNull<u8>,
    /// Physical address for DMA
    phys_addr: u64,
    /// Buffer size
    size: usize,
    /// DMA direction
    direction: DmaDirection,
    /// Owning device
    device: Arc<Device>,
    /// Attributes
    attributes: DmaAttributes,
}

impl DmaBuffer {
    /// Get virtual address
    pub fn virt_addr(&self) -> *mut u8 {
        self.virt_addr.as_ptr()
    }
    
    /// Get physical address
    pub fn phys_addr(&self) -> u64 {
        self.phys_addr
    }
    
    /// Get size
    pub fn size(&self) -> usize {
        self.size
    }
    
    /// Get as slice
    pub unsafe fn as_slice(&self) -> &[u8] {
        slice::from_raw_parts(self.virt_addr.as_ptr(), self.size)
    }
    
    /// Get as mutable slice
    pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        slice::from_raw_parts_mut(self.virt_addr.as_ptr(), self.size)
    }
    
    /// Sync for CPU access
    pub fn sync_for_cpu(&self) -> Result<()> {
        if !self.attributes.skip_cpu_sync {
            dma_manager().sync_for_cpu(self)?;
        }
        Ok(())
    }
    
    /// Sync for device access
    pub fn sync_for_device(&self) -> Result<()> {
        if !self.attributes.skip_cpu_sync {
            dma_manager().sync_for_device(self)?;
        }
        Ok(())
    }
}

impl Drop for DmaBuffer {
    fn drop(&mut self) {
        // Free DMA buffer
        let _ = dma_manager().free_buffer(self);
    }
}

/// DMA mapping for existing memory
pub struct DmaMapping {
    /// DMA address
    dma_addr: u64,
    /// Original virtual address
    virt_addr: usize,
    /// Mapping size
    size: usize,
    /// Direction
    direction: DmaDirection,
    /// Device
    device: Arc<Device>,
}

impl DmaMapping {
    /// Get DMA address
    pub fn dma_addr(&self) -> u64 {
        self.dma_addr
    }
    
    /// Sync for CPU
    pub fn sync_for_cpu(&self) -> Result<()> {
        dma_manager().sync_mapping_for_cpu(self)
    }
    
    /// Sync for device
    pub fn sync_for_device(&self) -> Result<()> {
        dma_manager().sync_mapping_for_device(self)
    }
}

impl Drop for DmaMapping {
    fn drop(&mut self) {
        let _ = dma_manager().unmap(self);
    }
}

/// Scatter-gather list entry
#[derive(Debug, Clone, Copy)]
pub struct ScatterGatherEntry {
    /// Physical address
    pub address: u64,
    /// Length
    pub length: u32,
    /// Flags
    pub flags: SgFlags,
}

/// Scatter-gather flags
#[derive(Debug, Clone, Copy, Default)]
pub struct SgFlags {
    /// Last entry in list
    pub end: bool,
    /// Chain to another SG list
    pub chain: bool,
}

/// Scatter-gather list
pub struct ScatterGatherList {
    entries: Vec<ScatterGatherEntry>,
    total_size: usize,
    device: Arc<Device>,
}

impl ScatterGatherList {
    /// Create new scatter-gather list
    pub fn new(device: Arc<Device>) -> Self {
        Self {
            entries: Vec::new(),
            total_size: 0,
            device,
        }
    }
    
    /// Add entry
    pub fn add_entry(&mut self, address: u64, length: u32) {
        self.entries.push(ScatterGatherEntry {
            address,
            length,
            flags: SgFlags::default(),
        });
        self.total_size += length as usize;
    }
    
    /// Get entries
    pub fn entries(&self) -> &[ScatterGatherEntry] {
        &self.entries
    }
    
    /// Get total size
    pub fn total_size(&self) -> usize {
        self.total_size
    }
    
    /// Map for DMA
    pub fn map(&mut self, direction: DmaDirection) -> Result<()> {
        // Map all entries
        for entry in &mut self.entries {
            let mapping = dma_manager().map_page(
                self.device.clone(),
                entry.address,
                entry.length as usize,
                direction,
            )?;
            entry.address = mapping.dma_addr();
            mem::forget(mapping); // Keep mapping
        }
        Ok(())
    }
}

/// DMA pool for small allocations
pub struct DmaPool {
    name: String,
    size: usize,
    align: usize,
    device: Arc<Device>,
    free_list: Mutex<Vec<DmaPoolEntry>>,
    allocated: AtomicUsize,
}

struct DmaPoolEntry {
    virt_addr: NonNull<u8>,
    phys_addr: u64,
}

impl DmaPool {
    /// Create new DMA pool
    pub fn new(
        name: String,
        device: Arc<Device>,
        size: usize,
        align: usize,
    ) -> Result<Self> {
        Ok(Self {
            name,
            size,
            align,
            device,
            free_list: Mutex::new(Vec::new()),
            allocated: AtomicUsize::new(0),
        })
    }
    
    /// Allocate from pool
    pub fn alloc(&self) -> Result<(NonNull<u8>, u64)> {
        let mut free_list = self.free_list.lock();
        
        if let Some(entry) = free_list.pop() {
            self.allocated.fetch_add(1, Ordering::Relaxed);
            Ok((entry.virt_addr, entry.phys_addr))
        } else {
            // Allocate new buffer
            let buffer = dma_manager().alloc_coherent(
                self.device.clone(),
                self.size,
                DmaAttributes {
                    coherent: true,
                    ..Default::default()
                },
            )?;
            
            let virt = NonNull::new(buffer.virt_addr()).unwrap();
            let phys = buffer.phys_addr();
            
            mem::forget(buffer); // Keep allocation
            self.allocated.fetch_add(1, Ordering::Relaxed);
            
            Ok((virt, phys))
        }
    }
    
    /// Free to pool
    pub fn free(&self, virt_addr: NonNull<u8>, phys_addr: u64) {
        self.free_list.lock().push(DmaPoolEntry {
            virt_addr,
            phys_addr,
        });
        self.allocated.fetch_sub(1, Ordering::Relaxed);
    }
}

/// IOMMU support
pub struct IommuDomain {
    id: u32,
    device: Arc<Device>,
    mappings: RwLock<BTreeMap<u64, IommuMapping>>,
}

struct IommuMapping {
    iova: u64,  // I/O virtual address
    phys: u64,  // Physical address
    size: usize,
    prot: IommuProt,
}

/// IOMMU protection flags
#[derive(Debug, Clone, Copy)]
pub struct IommuProt {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
    pub cache: bool,
}

impl IommuDomain {
    /// Map IOVA to physical address
    pub fn map(&self, iova: u64, phys: u64, size: usize, prot: IommuProt) -> Result<()> {
        let mut mappings = self.mappings.write();
        
        // Check for overlap
        for (mapped_iova, mapping) in mappings.iter() {
            if iova < mapped_iova + mapping.size as u64 &&
               iova + size as u64 > *mapped_iova {
                return Err(DriverError::ResourceConflict);
            }
        }
        
        mappings.insert(iova, IommuMapping {
            iova,
            phys,
            size,
            prot,
        });
        
        // Would program actual IOMMU hardware
        
        Ok(())
    }
    
    /// Unmap IOVA
    pub fn unmap(&self, iova: u64) -> Result<()> {
        self.mappings.write().remove(&iova)
            .ok_or(DriverError::NotFound)?;
        
        // Would clear IOMMU hardware mapping
        
        Ok(())
    }
}

/// Global DMA manager
pub struct DmaManager {
    /// DMA zones
    zones: RwLock<Vec<DmaZone>>,
    /// IOMMU domains
    iommu_domains: RwLock<BTreeMap<u32, Arc<IommuDomain>>>,
    /// Statistics
    stats: DmaStats,
}

/// DMA zone for allocation
struct DmaZone {
    start: u64,
    size: usize,
    free_pages: Mutex<Vec<u64>>,
}

/// DMA statistics
struct DmaStats {
    allocations: AtomicU64,
    mappings: AtomicU64,
    total_allocated: AtomicU64,
    total_mapped: AtomicU64,
}

impl DmaManager {
    /// Create new DMA manager
    pub const fn new() -> Self {
        Self {
            zones: RwLock::new(Vec::new()),
            iommu_domains: RwLock::new(BTreeMap::new()),
            stats: DmaStats {
                allocations: AtomicU64::new(0),
                mappings: AtomicU64::new(0),
                total_allocated: AtomicU64::new(0),
                total_mapped: AtomicU64::new(0),
            },
        }
    }
    
    /// Allocate coherent DMA buffer
    pub fn alloc_coherent(
        &self,
        device: Arc<Device>,
        size: usize,
        attrs: DmaAttributes,
    ) -> Result<DmaBuffer> {
        // Round up to page size
        let size = (size + 4095) & !4095;
        
        // Allocate physical memory
        let phys_addr = self.alloc_physical(size)?;
        
        // Map to virtual address
        let virt_addr = self.map_physical(phys_addr, size)?;
        
        self.stats.allocations.fetch_add(1, Ordering::Relaxed);
        self.stats.total_allocated.fetch_add(size as u64, Ordering::Relaxed);
        
        Ok(DmaBuffer {
            virt_addr: NonNull::new(virt_addr as *mut u8).unwrap(),
            phys_addr,
            size,
            direction: DmaDirection::Bidirectional,
            device,
            attributes: attrs,
        })
    }
    
    /// Allocate streaming DMA buffer
    pub fn alloc_streaming(
        &self,
        device: Arc<Device>,
        size: usize,
        direction: DmaDirection,
    ) -> Result<DmaBuffer> {
        self.alloc_coherent(device, size, DmaAttributes {
            streaming: true,
            ..Default::default()
        })
    }
    
    /// Free DMA buffer
    pub fn free_buffer(&self, buffer: &DmaBuffer) -> Result<()> {
        // Unmap and free physical memory
        self.unmap_physical(buffer.virt_addr.as_ptr() as usize, buffer.size)?;
        self.free_physical(buffer.phys_addr, buffer.size)?;
        
        self.stats.total_allocated.fetch_sub(buffer.size as u64, Ordering::Relaxed);
        
        Ok(())
    }
    
    /// Map memory for DMA
    pub fn map(
        &self,
        device: Arc<Device>,
        virt_addr: usize,
        size: usize,
        direction: DmaDirection,
    ) -> Result<DmaMapping> {
        // Get physical address
        let phys_addr = self.virt_to_phys(virt_addr)?;
        
        // Check if IOMMU is enabled for device
        let dma_addr = if let Some(_domain) = self.get_iommu_domain(&device) {
            // Map through IOMMU
            self.iommu_map(device.clone(), phys_addr, size)?
        } else {
            // Direct mapping
            phys_addr
        };
        
        self.stats.mappings.fetch_add(1, Ordering::Relaxed);
        self.stats.total_mapped.fetch_add(size as u64, Ordering::Relaxed);
        
        Ok(DmaMapping {
            dma_addr,
            virt_addr,
            size,
            direction,
            device,
        })
    }
    
    /// Map single page
    pub fn map_page(
        &self,
        device: Arc<Device>,
        page_addr: u64,
        size: usize,
        direction: DmaDirection,
    ) -> Result<DmaMapping> {
        self.map(device, page_addr as usize, size, direction)
    }
    
    /// Unmap DMA mapping
    pub fn unmap(&self, mapping: &DmaMapping) -> Result<()> {
        if let Some(_domain) = self.get_iommu_domain(&mapping.device) {
            self.iommu_unmap(mapping.device.clone(), mapping.dma_addr, mapping.size)?;
        }
        
        self.stats.total_mapped.fetch_sub(mapping.size as u64, Ordering::Relaxed);
        
        Ok(())
    }
    
    /// Sync buffer for CPU access
    pub fn sync_for_cpu(&self, buffer: &DmaBuffer) -> Result<()> {
        // Invalidate CPU cache for FROM_DEVICE
        if buffer.direction == DmaDirection::FromDevice ||
           buffer.direction == DmaDirection::Bidirectional {
            self.invalidate_cache(buffer.virt_addr.as_ptr() as usize, buffer.size)?;
        }
        Ok(())
    }
    
    /// Sync buffer for device access
    pub fn sync_for_device(&self, buffer: &DmaBuffer) -> Result<()> {
        // Flush CPU cache for TO_DEVICE
        if buffer.direction == DmaDirection::ToDevice ||
           buffer.direction == DmaDirection::Bidirectional {
            self.flush_cache(buffer.virt_addr.as_ptr() as usize, buffer.size)?;
        }
        Ok(())
    }
    
    /// Sync mapping for CPU
    pub fn sync_mapping_for_cpu(&self, mapping: &DmaMapping) -> Result<()> {
        if mapping.direction == DmaDirection::FromDevice ||
           mapping.direction == DmaDirection::Bidirectional {
            self.invalidate_cache(mapping.virt_addr, mapping.size)?;
        }
        Ok(())
    }
    
    /// Sync mapping for device
    pub fn sync_mapping_for_device(&self, mapping: &DmaMapping) -> Result<()> {
        if mapping.direction == DmaDirection::ToDevice ||
           mapping.direction == DmaDirection::Bidirectional {
            self.flush_cache(mapping.virt_addr, mapping.size)?;
        }
        Ok(())
    }
    
    // Helper functions (would be implemented with actual memory management)
    
    fn alloc_physical(&self, size: usize) -> Result<u64> {
        // Would allocate from DMA zone
        Ok(0x100000) // Dummy address
    }
    
    fn free_physical(&self, addr: u64, size: usize) -> Result<()> {
        // Would free to DMA zone
        Ok(())
    }
    
    fn map_physical(&self, phys: u64, size: usize) -> Result<usize> {
        // Would create virtual mapping
        Ok(phys as usize)
    }
    
    fn unmap_physical(&self, virt: usize, size: usize) -> Result<()> {
        // Would remove virtual mapping
        Ok(())
    }
    
    fn virt_to_phys(&self, virt: usize) -> Result<u64> {
        // Would translate virtual to physical
        Ok(virt as u64)
    }
    
    fn get_iommu_domain(&self, device: &Device) -> Option<Arc<IommuDomain>> {
        // Would look up IOMMU domain for device
        None
    }
    
    fn iommu_map(&self, device: Arc<Device>, phys: u64, size: usize) -> Result<u64> {
        // Would map through IOMMU
        Ok(phys)
    }
    
    fn iommu_unmap(&self, device: Arc<Device>, iova: u64, size: usize) -> Result<()> {
        // Would unmap from IOMMU
        Ok(())
    }
    
    fn flush_cache(&self, addr: usize, size: usize) -> Result<()> {
        // Would flush CPU cache
        Ok(())
    }
    
    fn invalidate_cache(&self, addr: usize, size: usize) -> Result<()> {
        // Would invalidate CPU cache
        Ok(())
    }
}

/// Global DMA manager instance
static DMA_MANAGER: DmaManager = DmaManager::new();

/// Get global DMA manager
pub fn dma_manager() -> &'static DmaManager {
    &DMA_MANAGER
}

/// Memory barrier operations
pub mod barriers {
    use core::sync::atomic::{compiler_fence, fence, Ordering};
    
    /// Read memory barrier
    #[inline]
    pub fn rmb() {
        fence(Ordering::Acquire);
    }
    
    /// Write memory barrier
    #[inline]
    pub fn wmb() {
        fence(Ordering::Release);
    }
    
    /// Full memory barrier
    #[inline]
    pub fn mb() {
        fence(Ordering::SeqCst);
    }
    
    /// Compiler barrier
    #[inline]
    pub fn barrier() {
        compiler_fence(Ordering::SeqCst);
    }
}