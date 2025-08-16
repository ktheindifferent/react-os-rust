// GPU Driver Framework
use alloc::vec::Vec;
use alloc::string::String;
use alloc::boxed::Box;
use alloc::sync::Arc;
use spin::{Mutex, RwLock};
use x86_64::{PhysAddr, VirtAddr};
use crate::drivers::pci::{PciDevice, PciClass};
use crate::memory::PhysicalMemoryManager;
use lazy_static::lazy_static;

pub mod intel;
pub mod amd;
pub mod command;
pub mod memory;
pub mod fence;
pub mod drm;
pub mod kms;
pub mod opengl;
pub mod video;

// GPU Vendor IDs
pub const VENDOR_INTEL: u16 = 0x8086;
pub const VENDOR_AMD: u16 = 0x1002;
pub const VENDOR_NVIDIA: u16 = 0x10DE;

// Intel GPU Device IDs (common ones)
pub const INTEL_HD_GRAPHICS_520: u16 = 0x1916;
pub const INTEL_HD_GRAPHICS_620: u16 = 0x5916;
pub const INTEL_UHD_GRAPHICS_620: u16 = 0x5917;
pub const INTEL_IRIS_XE: u16 = 0x9A49;

// AMD GPU Device IDs (common ones)
pub const AMD_RADEON_RX580: u16 = 0x67DF;
pub const AMD_RADEON_RX5700: u16 = 0x731F;
pub const AMD_RADEON_RX6800: u16 = 0x73BF;

// GPU Capabilities
#[derive(Debug, Clone, Copy)]
pub struct GpuCapabilities {
    pub max_texture_size: u32,
    pub max_viewport_dims: (u32, u32),
    pub max_vertex_attributes: u32,
    pub max_uniform_vectors: u32,
    pub max_varying_vectors: u32,
    pub max_vertex_texture_units: u32,
    pub max_fragment_texture_units: u32,
    pub has_compute_shaders: bool,
    pub has_geometry_shaders: bool,
    pub has_tessellation: bool,
    pub has_raytracing: bool,
    pub has_mesh_shaders: bool,
    pub max_compute_work_groups: (u32, u32, u32),
    pub max_compute_work_group_size: (u32, u32, u32),
    pub max_compute_shared_memory: u64,
    pub video_memory_size: u64,
    pub dedicated_video_memory: bool,
}

// GPU Memory Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryType {
    SystemRam,           // Regular system memory
    VideoRam,           // Dedicated VRAM
    GttAperture,        // Graphics Translation Table mapped memory
    Stolen,             // Memory stolen from system RAM for GPU
    Local,              // GPU-local memory
    Coherent,           // CPU-GPU coherent memory
    CachedCoherent,     // Cached coherent memory
}

// GPU Memory Region
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub region_type: MemoryType,
    pub physical_address: PhysAddr,
    pub virtual_address: Option<VirtAddr>,
    pub size: u64,
    pub is_mappable: bool,
    pub is_cacheable: bool,
    pub is_write_combined: bool,
}

// GPU Buffer Object
#[derive(Debug)]
pub struct BufferObject {
    pub id: u64,
    pub size: u64,
    pub memory_type: MemoryType,
    pub virtual_address: Option<VirtAddr>,
    pub physical_address: Option<PhysAddr>,
    pub is_pinned: bool,
    pub is_tiled: bool,
    pub tiling_mode: TilingMode,
    pub cache_level: CacheLevel,
    pub usage_flags: BufferUsageFlags,
}

// Buffer Usage Flags
bitflags::bitflags! {
    pub struct BufferUsageFlags: u32 {
        const VERTEX_BUFFER = 1 << 0;
        const INDEX_BUFFER = 1 << 1;
        const UNIFORM_BUFFER = 1 << 2;
        const TEXTURE = 1 << 3;
        const RENDER_TARGET = 1 << 4;
        const DEPTH_STENCIL = 1 << 5;
        const COMMAND_BUFFER = 1 << 6;
        const SHADER_STORAGE = 1 << 7;
        const TRANSFER_SRC = 1 << 8;
        const TRANSFER_DST = 1 << 9;
        const SCANOUT = 1 << 10;  // For display
    }
}

// Tiling modes for optimal memory access patterns
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TilingMode {
    Linear,             // Linear layout
    XTiled,            // X-tiling (Intel)
    YTiled,            // Y-tiling (Intel)
    YfTiled,           // Yf-tiling (Intel)
    TiledDcc,          // Display compression (AMD)
    TiledDccRetile,    // Retiled DCC (AMD)
}

// Cache levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CacheLevel {
    None,              // Uncached
    WriteThrough,      // Write-through cache
    WriteBack,         // Write-back cache
    WriteCombining,    // Write-combining
}

// GPU Engine Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineType {
    Render,            // 3D rendering engine
    Blitter,           // 2D blitting engine
    Video,             // Video decode/encode engine
    VideoEnhance,      // Video enhancement engine
    Compute,           // Compute engine
    Copy,              // DMA copy engine
}

// GPU Command Buffer
pub struct CommandBuffer {
    pub id: u64,
    pub engine: EngineType,
    pub buffer: BufferObject,
    pub size: u64,
    pub head: u64,
    pub tail: u64,
    pub is_ring: bool,
}

// Display Mode
#[derive(Debug, Clone, Copy)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
    pub pixel_clock: u32,
    pub hsync_start: u32,
    pub hsync_end: u32,
    pub htotal: u32,
    pub vsync_start: u32,
    pub vsync_end: u32,
    pub vtotal: u32,
    pub flags: DisplayModeFlags,
}

bitflags::bitflags! {
    pub struct DisplayModeFlags: u32 {
        const INTERLACED = 1 << 0;
        const DOUBLE_SCAN = 1 << 1;
        const HSYNC_POSITIVE = 1 << 2;
        const VSYNC_POSITIVE = 1 << 3;
        const PREFERRED = 1 << 4;
    }
}

// Display Connector Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectorType {
    VGA,
    DVII,
    DVID,
    DVIA,
    Composite,
    SVideo,
    LVDS,
    Component,
    DisplayPort,
    HDMIA,
    HDMIB,
    TV,
    eDP,
    Virtual,
    DSI,
    DPI,
}

// Display Output
pub struct DisplayOutput {
    pub id: u32,
    pub name: String,
    pub connector_type: ConnectorType,
    pub is_connected: bool,
    pub modes: Vec<DisplayMode>,
    pub current_mode: Option<DisplayMode>,
    pub edid_data: Option<Vec<u8>>,
}

// GPU Driver Trait
pub trait GpuDriver: Send + Sync {
    fn name(&self) -> &str;
    fn vendor_id(&self) -> u16;
    fn device_id(&self) -> u16;
    
    fn init(&mut self, device: &PciDevice) -> Result<(), &'static str>;
    fn reset(&mut self) -> Result<(), &'static str>;
    fn suspend(&mut self) -> Result<(), &'static str>;
    fn resume(&mut self) -> Result<(), &'static str>;
    
    fn get_capabilities(&self) -> &GpuCapabilities;
    fn get_memory_regions(&self) -> &[MemoryRegion];
    
    fn allocate_buffer(&mut self, size: u64, usage: BufferUsageFlags) -> Result<BufferObject, &'static str>;
    fn free_buffer(&mut self, buffer: BufferObject) -> Result<(), &'static str>;
    fn map_buffer(&mut self, buffer: &BufferObject) -> Result<VirtAddr, &'static str>;
    fn unmap_buffer(&mut self, buffer: &BufferObject) -> Result<(), &'static str>;
    
    fn create_command_buffer(&mut self, engine: EngineType, size: u64) -> Result<CommandBuffer, &'static str>;
    fn submit_command_buffer(&mut self, cmd_buf: &CommandBuffer) -> Result<(), &'static str>;
    fn wait_idle(&mut self) -> Result<(), &'static str>;
    
    fn get_display_outputs(&self) -> &[DisplayOutput];
    fn set_display_mode(&mut self, output_id: u32, mode: &DisplayMode) -> Result<(), &'static str>;
    fn create_framebuffer(&mut self, width: u32, height: u32) -> Result<BufferObject, &'static str>;
    fn present_framebuffer(&mut self, buffer: &BufferObject) -> Result<(), &'static str>;
    
    fn enable_acceleration(&mut self) -> Result<(), &'static str>;
    fn blit_2d(&mut self, src: &BufferObject, dst: &BufferObject, 
               src_x: u32, src_y: u32, dst_x: u32, dst_y: u32,
               width: u32, height: u32) -> Result<(), &'static str>;
    fn fill_2d(&mut self, dst: &BufferObject, x: u32, y: u32, 
               width: u32, height: u32, color: u32) -> Result<(), &'static str>;
}

// GPU Manager
pub struct GpuManager {
    drivers: Vec<Arc<Mutex<Box<dyn GpuDriver>>>>,
    primary_gpu: Option<usize>,
    initialized: bool,
}

impl GpuManager {
    pub fn new() -> Self {
        Self {
            drivers: Vec::new(),
            primary_gpu: None,
            initialized: false,
        }
    }
    
    pub fn init(&mut self) -> Result<(), &'static str> {
        if self.initialized {
            return Ok(());
        }
        
        crate::println!("GPU: Initializing GPU manager...");
        
        // Scan PCI bus for GPU devices
        self.scan_pci_devices()?;
        
        if self.drivers.is_empty() {
            crate::println!("GPU: No supported GPU devices found");
            return Err("No GPU devices found");
        }
        
        // Set the first GPU as primary
        self.primary_gpu = Some(0);
        self.initialized = true;
        
        crate::println!("GPU: Initialized {} GPU device(s)", self.drivers.len());
        Ok(())
    }
    
    fn scan_pci_devices(&mut self) -> Result<(), &'static str> {
        // This would interface with the PCI subsystem to find GPU devices
        // For now, we'll simulate finding devices
        crate::println!("GPU: Scanning PCI bus for GPU devices...");
        
        // In a real implementation, iterate through PCI devices
        // and check for display controllers (class 0x03)
        
        Ok(())
    }
    
    pub fn register_driver(&mut self, driver: Box<dyn GpuDriver>) {
        self.drivers.push(Arc::new(Mutex::new(driver)));
    }
    
    pub fn get_primary_gpu(&self) -> Option<Arc<Mutex<Box<dyn GpuDriver>>>> {
        self.primary_gpu.and_then(|idx| self.drivers.get(idx).cloned())
    }
    
    pub fn get_gpu_count(&self) -> usize {
        self.drivers.len()
    }
    
    pub fn get_gpu(&self, index: usize) -> Option<Arc<Mutex<Box<dyn GpuDriver>>>> {
        self.drivers.get(index).cloned()
    }
}

// Global GPU Manager instance
lazy_static! {
    pub static ref GPU_MANAGER: RwLock<GpuManager> = RwLock::new(GpuManager::new());
}

// Initialize GPU subsystem
pub fn init() -> Result<(), &'static str> {
    let mut manager = GPU_MANAGER.write();
    manager.init()?;
    Ok(())
}

// Helper function to detect and initialize a GPU from PCI device
pub fn probe_gpu_device(device: &PciDevice) -> Option<Box<dyn GpuDriver>> {
    match device.vendor_id {
        VENDOR_INTEL => {
            crate::println!("GPU: Found Intel GPU (device: 0x{:04X})", device.device_id);
            Some(Box::new(intel::IntelGpu::new(device)))
        }
        VENDOR_AMD => {
            crate::println!("GPU: Found AMD GPU (device: 0x{:04X})", device.device_id);
            Some(Box::new(amd::AmdGpu::new(device)))
        }
        VENDOR_NVIDIA => {
            crate::println!("GPU: Found NVIDIA GPU (device: 0x{:04X}) - not yet supported", device.device_id);
            None
        }
        _ => None,
    }
}