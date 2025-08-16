//! User-space Driver Support (UIO/VFIO)

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::{
    mem,
    ptr::NonNull,
    sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};
use spin::{Mutex, RwLock};

use super::{
    Device, DeviceId, DeviceResource, DriverError, Result,
    interrupt::{Irq, InterruptReturn},
    dma::{DmaBuffer, DmaDirection},
};

/// UIO (Userspace I/O) device
pub struct UioDevice {
    /// Device ID
    id: u32,
    /// Device name
    name: String,
    /// Underlying device
    device: Arc<Device>,
    /// Memory regions
    mem_regions: Vec<UioMemRegion>,
    /// Port regions
    port_regions: Vec<UioPortRegion>,
    /// Interrupt info
    irq_info: Option<UioIrqInfo>,
    /// Open count
    open_count: AtomicU32,
    /// Event counter
    event_counter: AtomicU64,
}

/// UIO memory region
#[derive(Debug, Clone)]
pub struct UioMemRegion {
    /// Physical address
    pub phys_addr: u64,
    /// Size
    pub size: usize,
    /// Memory type
    pub mem_type: UioMemType,
    /// Mapped virtual address (in userspace)
    pub virt_addr: Option<usize>,
}

/// UIO memory type
#[derive(Debug, Clone, Copy)]
pub enum UioMemType {
    /// Physical memory
    Physical,
    /// Logical memory (kernel allocated)
    Logical,
    /// Virtual memory
    Virtual,
}

/// UIO port region
#[derive(Debug, Clone)]
pub struct UioPortRegion {
    /// Port start
    pub start: u16,
    /// Port size
    pub size: u16,
    /// Port type
    pub port_type: UioPortType,
}

/// UIO port type
#[derive(Debug, Clone, Copy)]
pub enum UioPortType {
    /// x86 I/O port
    X86,
    /// Memory-mapped I/O
    Mmio,
}

/// UIO interrupt info
#[derive(Debug, Clone)]
pub struct UioIrqInfo {
    /// IRQ number
    pub irq: Irq,
    /// IRQ type
    pub irq_type: UioIrqType,
    /// Handler installed
    pub handler_installed: AtomicBool,
}

/// UIO IRQ type
#[derive(Debug, Clone, Copy)]
pub enum UioIrqType {
    /// Edge triggered
    Edge,
    /// Level triggered
    Level,
    /// Message signaled
    Msi,
}

impl UioDevice {
    /// Create new UIO device
    pub fn new(id: u32, name: String, device: Arc<Device>) -> Self {
        let mut uio = Self {
            id,
            name,
            device: device.clone(),
            mem_regions: Vec::new(),
            port_regions: Vec::new(),
            irq_info: None,
            open_count: AtomicU32::new(0),
            event_counter: AtomicU64::new(0),
        };
        
        // Map device resources to UIO regions
        for resource in device.resources() {
            match resource {
                DeviceResource::Memory { base, size, .. } => {
                    uio.mem_regions.push(UioMemRegion {
                        phys_addr: base,
                        size,
                        mem_type: UioMemType::Physical,
                        virt_addr: None,
                    });
                }
                DeviceResource::Io { base, size } => {
                    uio.port_regions.push(UioPortRegion {
                        start: base,
                        size,
                        port_type: UioPortType::X86,
                    });
                }
                DeviceResource::Interrupt { irq, .. } => {
                    uio.irq_info = Some(UioIrqInfo {
                        irq: Irq::new(irq),
                        irq_type: UioIrqType::Level,
                        handler_installed: AtomicBool::new(false),
                    });
                }
                _ => {}
            }
        }
        
        uio
    }
    
    /// Open UIO device
    pub fn open(&self) -> Result<UioHandle> {
        self.open_count.fetch_add(1, Ordering::AcqRel);
        
        Ok(UioHandle {
            device_id: self.id,
            event_count: self.event_counter.load(Ordering::Acquire),
        })
    }
    
    /// Close UIO device
    pub fn close(&self) {
        self.open_count.fetch_sub(1, Ordering::AcqRel);
    }
    
    /// Map memory region to userspace
    pub fn mmap(&self, region: usize, virt_addr: usize) -> Result<()> {
        if region >= self.mem_regions.len() {
            return Err(DriverError::NotFound);
        }
        
        // Would perform actual memory mapping
        self.mem_regions[region].virt_addr = Some(virt_addr);
        
        Ok(())
    }
    
    /// Enable interrupts
    pub fn enable_irq(&self) -> Result<()> {
        if let Some(ref irq_info) = self.irq_info {
            if !irq_info.handler_installed.load(Ordering::Acquire) {
                // Install interrupt handler
                super::interrupt::interrupt_manager().request_irq(
                    irq_info.irq,
                    Box::new(move || {
                        // Signal userspace
                        InterruptReturn::Handled
                    }),
                    super::interrupt::IrqFlags::default(),
                    self.name.clone(),
                    self.device.clone(),
                )?;
                
                irq_info.handler_installed.store(true, Ordering::Release);
            }
        }
        
        Ok(())
    }
    
    /// Disable interrupts
    pub fn disable_irq(&self) -> Result<()> {
        if let Some(ref irq_info) = self.irq_info {
            if irq_info.handler_installed.load(Ordering::Acquire) {
                super::interrupt::interrupt_manager().free_irq(irq_info.irq, &self.device)?;
                irq_info.handler_installed.store(false, Ordering::Release);
            }
        }
        
        Ok(())
    }
    
    /// Wait for interrupt
    pub fn wait_for_interrupt(&self) -> u64 {
        // Would block until interrupt occurs
        self.event_counter.fetch_add(1, Ordering::AcqRel)
    }
}

/// UIO device handle
pub struct UioHandle {
    device_id: u32,
    event_count: u64,
}

/// VFIO (Virtual Function I/O) for device passthrough
pub struct VfioDevice {
    /// Device ID
    id: u32,
    /// Group ID
    group_id: u32,
    /// Device
    device: Arc<Device>,
    /// IOMMU domain
    iommu_domain: Option<Arc<IommuDomain>>,
    /// Mapped regions
    mapped_regions: RwLock<Vec<VfioRegion>>,
    /// Interrupt mappings
    interrupt_mappings: RwLock<Vec<VfioInterrupt>>,
    /// DMA mappings
    dma_mappings: RwLock<BTreeMap<u64, VfioDmaMapping>>,
}

/// VFIO memory region
#[derive(Debug, Clone)]
pub struct VfioRegion {
    /// Region index
    pub index: u32,
    /// Physical address
    pub phys_addr: u64,
    /// Size
    pub size: usize,
    /// Offset in device
    pub offset: u64,
    /// Flags
    pub flags: VfioRegionFlags,
}

/// VFIO region flags
#[derive(Debug, Clone, Copy, Default)]
pub struct VfioRegionFlags {
    pub readable: bool,
    pub writable: bool,
    pub mappable: bool,
}

/// VFIO interrupt mapping
#[derive(Debug, Clone)]
pub struct VfioInterrupt {
    /// Interrupt index
    pub index: u32,
    /// IRQ number
    pub irq: Irq,
    /// Guest vector
    pub guest_vector: u32,
    /// Masked
    pub masked: AtomicBool,
}

/// VFIO DMA mapping
#[derive(Debug, Clone)]
pub struct VfioDmaMapping {
    /// IOVA (I/O Virtual Address)
    pub iova: u64,
    /// Size
    pub size: usize,
    /// User address
    pub user_addr: usize,
    /// Permissions
    pub prot: DmaProtection,
}

/// DMA protection flags
#[derive(Debug, Clone, Copy)]
pub struct DmaProtection {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
}

/// IOMMU domain for VFIO
pub struct IommuDomain {
    /// Domain ID
    id: u32,
    /// Type
    domain_type: IommuType,
    /// Page table root
    page_table: AtomicU64,
    /// Mappings
    mappings: RwLock<BTreeMap<u64, IommuMapEntry>>,
}

/// IOMMU type
#[derive(Debug, Clone, Copy)]
pub enum IommuType {
    /// No IOMMU
    NoIommu,
    /// Type 1 IOMMU (x86)
    Type1,
    /// SMMU (ARM)
    Smmu,
}

/// IOMMU mapping entry
#[derive(Debug, Clone)]
struct IommuMapEntry {
    iova: u64,
    phys: u64,
    size: usize,
    prot: DmaProtection,
}

impl VfioDevice {
    /// Create new VFIO device
    pub fn new(id: u32, group_id: u32, device: Arc<Device>) -> Self {
        Self {
            id,
            group_id,
            device,
            iommu_domain: None,
            mapped_regions: RwLock::new(Vec::new()),
            interrupt_mappings: RwLock::new(Vec::new()),
            dma_mappings: RwLock::new(BTreeMap::new()),
        }
    }
    
    /// Attach IOMMU domain
    pub fn attach_iommu(&mut self, domain: Arc<IommuDomain>) -> Result<()> {
        self.iommu_domain = Some(domain);
        Ok(())
    }
    
    /// Map DMA region
    pub fn map_dma(&self, mapping: VfioDmaMapping) -> Result<()> {
        if let Some(ref domain) = self.iommu_domain {
            // Map in IOMMU
            domain.map(mapping.iova, mapping.user_addr as u64, mapping.size, mapping.prot)?;
        }
        
        self.dma_mappings.write().insert(mapping.iova, mapping);
        
        Ok(())
    }
    
    /// Unmap DMA region
    pub fn unmap_dma(&self, iova: u64) -> Result<()> {
        if let Some(ref domain) = self.iommu_domain {
            // Unmap from IOMMU
            domain.unmap(iova)?;
        }
        
        self.dma_mappings.write().remove(&iova)
            .ok_or(DriverError::NotFound)?;
        
        Ok(())
    }
    
    /// Setup interrupt remapping
    pub fn setup_interrupt(&self, index: u32, guest_vector: u32) -> Result<()> {
        // Would setup interrupt remapping from device to guest
        
        let mapping = VfioInterrupt {
            index,
            irq: Irq::new(index),
            guest_vector,
            masked: AtomicBool::new(false),
        };
        
        self.interrupt_mappings.write().push(mapping);
        
        Ok(())
    }
    
    /// Reset device
    pub fn reset(&self) -> Result<()> {
        // Would perform device reset
        Ok(())
    }
}

impl IommuDomain {
    /// Create new IOMMU domain
    pub fn new(id: u32, domain_type: IommuType) -> Self {
        Self {
            id,
            domain_type,
            page_table: AtomicU64::new(0),
            mappings: RwLock::new(BTreeMap::new()),
        }
    }
    
    /// Map IOVA to physical address
    pub fn map(&self, iova: u64, phys: u64, size: usize, prot: DmaProtection) -> Result<()> {
        // Check for overlaps
        let mappings = self.mappings.read();
        for (mapped_iova, entry) in mappings.iter() {
            if iova < mapped_iova + entry.size as u64 &&
               iova + size as u64 > *mapped_iova {
                return Err(DriverError::ResourceConflict);
            }
        }
        drop(mappings);
        
        // Add mapping
        self.mappings.write().insert(iova, IommuMapEntry {
            iova,
            phys,
            size,
            prot,
        });
        
        // Would update IOMMU page tables
        
        Ok(())
    }
    
    /// Unmap IOVA
    pub fn unmap(&self, iova: u64) -> Result<()> {
        self.mappings.write().remove(&iova)
            .ok_or(DriverError::NotFound)?;
        
        // Would update IOMMU page tables
        
        Ok(())
    }
}

/// Global userspace driver manager
pub struct UserspaceDriverManager {
    /// UIO devices
    uio_devices: RwLock<BTreeMap<u32, Arc<UioDevice>>>,
    /// VFIO devices
    vfio_devices: RwLock<BTreeMap<u32, Arc<VfioDevice>>>,
    /// VFIO groups
    vfio_groups: RwLock<BTreeMap<u32, VfioGroup>>,
    /// IOMMU domains
    iommu_domains: RwLock<BTreeMap<u32, Arc<IommuDomain>>>,
    /// Next IDs
    next_uio_id: AtomicU32,
    next_vfio_id: AtomicU32,
    next_group_id: AtomicU32,
    next_domain_id: AtomicU32,
}

/// VFIO group
struct VfioGroup {
    id: u32,
    devices: Vec<u32>,
    container: Option<u32>,
}

impl UserspaceDriverManager {
    /// Create new manager
    pub const fn new() -> Self {
        Self {
            uio_devices: RwLock::new(BTreeMap::new()),
            vfio_devices: RwLock::new(BTreeMap::new()),
            vfio_groups: RwLock::new(BTreeMap::new()),
            iommu_domains: RwLock::new(BTreeMap::new()),
            next_uio_id: AtomicU32::new(0),
            next_vfio_id: AtomicU32::new(0),
            next_group_id: AtomicU32::new(0),
            next_domain_id: AtomicU32::new(0),
        }
    }
    
    /// Register UIO device
    pub fn register_uio(&self, device: Arc<Device>) -> Result<u32> {
        let id = self.next_uio_id.fetch_add(1, Ordering::Relaxed);
        let name = format!("uio{}", id);
        
        let uio = Arc::new(UioDevice::new(id, name, device));
        
        self.uio_devices.write().insert(id, uio);
        
        Ok(id)
    }
    
    /// Unregister UIO device
    pub fn unregister_uio(&self, id: u32) -> Result<()> {
        self.uio_devices.write().remove(&id)
            .ok_or(DriverError::NotFound)?;
        
        Ok(())
    }
    
    /// Get UIO device
    pub fn get_uio(&self, id: u32) -> Option<Arc<UioDevice>> {
        self.uio_devices.read().get(&id).cloned()
    }
    
    /// Register VFIO device
    pub fn register_vfio(&self, device: Arc<Device>, group_id: u32) -> Result<u32> {
        let id = self.next_vfio_id.fetch_add(1, Ordering::Relaxed);
        
        let vfio = Arc::new(VfioDevice::new(id, group_id, device));
        
        self.vfio_devices.write().insert(id, vfio);
        
        // Add to group
        let mut groups = self.vfio_groups.write();
        if let Some(group) = groups.get_mut(&group_id) {
            group.devices.push(id);
        }
        
        Ok(id)
    }
    
    /// Create VFIO group
    pub fn create_vfio_group(&self) -> u32 {
        let id = self.next_group_id.fetch_add(1, Ordering::Relaxed);
        
        let group = VfioGroup {
            id,
            devices: Vec::new(),
            container: None,
        };
        
        self.vfio_groups.write().insert(id, group);
        
        id
    }
    
    /// Create IOMMU domain
    pub fn create_iommu_domain(&self, domain_type: IommuType) -> Result<u32> {
        let id = self.next_domain_id.fetch_add(1, Ordering::Relaxed);
        
        let domain = Arc::new(IommuDomain::new(id, domain_type));
        
        self.iommu_domains.write().insert(id, domain);
        
        Ok(id)
    }
    
    /// Attach group to container
    pub fn attach_group_to_container(&self, group_id: u32, container_id: u32) -> Result<()> {
        let mut groups = self.vfio_groups.write();
        
        if let Some(group) = groups.get_mut(&group_id) {
            group.container = Some(container_id);
            Ok(())
        } else {
            Err(DriverError::NotFound)
        }
    }
}

/// Global userspace driver manager instance
static USERSPACE_MANAGER: UserspaceDriverManager = UserspaceDriverManager::new();

/// Get userspace driver manager
pub fn userspace_manager() -> &'static UserspaceDriverManager {
    &USERSPACE_MANAGER
}

/// Helper macros
#[macro_export]
macro_rules! register_uio_device {
    ($device:expr) => {
        $crate::userspace_manager().register_uio($device)
    };
}

#[macro_export]
macro_rules! register_vfio_device {
    ($device:expr, $group:expr) => {
        $crate::userspace_manager().register_vfio($device, $group)
    };
}