// Intel GPU Driver (i915/Xe compatible)
use alloc::vec::Vec;
use alloc::string::String;
use x86_64::{PhysAddr, VirtAddr};
use crate::drivers::pci::PciDevice;
use super::{GpuDriver, GpuCapabilities, MemoryRegion, MemoryType, BufferObject, BufferUsageFlags,
           CommandBuffer, EngineType, DisplayOutput, DisplayMode, ConnectorType};

// Intel GPU Generations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntelGen {
    Gen9,   // Skylake, Kaby Lake, Coffee Lake
    Gen11,  // Ice Lake
    Gen12,  // Tiger Lake, Rocket Lake, Alder Lake
    Xe,     // DG1, Arc
}

// Intel GPU Register Offsets
mod regs {
    // Display Engine Registers
    pub const PIPE_A_CONF: u32 = 0x70008;
    pub const PIPE_B_CONF: u32 = 0x71008;
    pub const PIPE_A_SRC: u32 = 0x6001C;
    pub const PIPE_B_SRC: u32 = 0x6101C;
    
    // Display Plane Registers
    pub const PLANE_A_CTRL: u32 = 0x70180;
    pub const PLANE_A_STRIDE: u32 = 0x70188;
    pub const PLANE_A_SURF: u32 = 0x7019C;
    
    // GTT Control
    pub const GFX_MODE: u32 = 0x2090;
    pub const GTT_BASE: u32 = 0x2024;
    
    // Ring Buffer Control
    pub const RING_HEAD: u32 = 0x2034;
    pub const RING_TAIL: u32 = 0x2030;
    pub const RING_START: u32 = 0x2038;
    pub const RING_CTL: u32 = 0x203C;
    
    // Render Engine
    pub const RCS_RING_BASE: u32 = 0x2000;
    pub const BCS_RING_BASE: u32 = 0x22000;  // Blitter
    pub const VCS_RING_BASE: u32 = 0x12000;  // Video
    pub const VECS_RING_BASE: u32 = 0x1A000; // Video Enhancement
    
    // Power Management
    pub const GEN6_RC_CONTROL: u32 = 0xA090;
    pub const GEN6_RC_STATE: u32 = 0xA094;
    pub const GEN6_RPNSWREQ: u32 = 0xA008;
    
    // Interrupt Control
    pub const GEN8_MASTER_IRQ: u32 = 0x44200;
    pub const GEN8_GT_IMR: u32 = 0x44304;
    pub const GEN8_GT_IIR: u32 = 0x44308;
    pub const GEN8_GT_IER: u32 = 0x4430C;
    
    // Fence Registers
    pub const FENCE_REG_BASE: u32 = 0x100000;
    pub const FENCE_REG_SIZE: u32 = 0x8;
}

// Intel GPU Hardware Context
#[derive(Debug, Clone)]
pub struct HwContext {
    pub id: u32,
    pub ring_buffer: PhysAddr,
    pub ring_size: u32,
    pub pml4: PhysAddr,  // Page table root
}

// Intel GPU Driver Implementation
pub struct IntelGpu {
    device: PciDevice,
    generation: IntelGen,
    mmio_base: VirtAddr,
    mmio_size: usize,
    gtt_base: VirtAddr,
    gtt_size: usize,
    stolen_memory_base: PhysAddr,
    stolen_memory_size: u64,
    capabilities: GpuCapabilities,
    memory_regions: Vec<MemoryRegion>,
    outputs: Vec<DisplayOutput>,
    contexts: Vec<HwContext>,
    initialized: bool,
}

impl IntelGpu {
    pub fn new(device: &PciDevice) -> Self {
        let generation = Self::detect_generation(device.device_id);
        
        let capabilities = GpuCapabilities {
            max_texture_size: 16384,
            max_viewport_dims: (16384, 16384),
            max_vertex_attributes: 32,
            max_uniform_vectors: 4096,
            max_varying_vectors: 128,
            max_vertex_texture_units: 32,
            max_fragment_texture_units: 32,
            has_compute_shaders: true,
            has_geometry_shaders: true,
            has_tessellation: true,
            has_raytracing: generation == IntelGen::Xe,
            has_mesh_shaders: generation == IntelGen::Xe,
            max_compute_work_groups: (65535, 65535, 65535),
            max_compute_work_group_size: (1024, 1024, 64),
            max_compute_shared_memory: 64 * 1024,
            video_memory_size: 0, // Will be detected
            dedicated_video_memory: false,
        };
        
        Self {
            device: device.clone(),
            generation,
            mmio_base: VirtAddr::new(0),
            mmio_size: 0,
            gtt_base: VirtAddr::new(0),
            gtt_size: 0,
            stolen_memory_base: PhysAddr::new(0),
            stolen_memory_size: 0,
            capabilities,
            memory_regions: Vec::new(),
            outputs: Vec::new(),
            contexts: Vec::new(),
            initialized: false,
        }
    }
    
    fn detect_generation(device_id: u16) -> IntelGen {
        match device_id {
            0x1900..=0x19FF => IntelGen::Gen9,  // Skylake
            0x5900..=0x59FF => IntelGen::Gen9,  // Kaby Lake
            0x3E00..=0x3EFF => IntelGen::Gen9,  // Coffee Lake
            0x8A00..=0x8AFF => IntelGen::Gen11, // Ice Lake
            0x9A00..=0x9AFF => IntelGen::Gen12, // Tiger Lake
            0x4600..=0x46FF => IntelGen::Gen12, // Alder Lake
            0x4C00..=0x4CFF => IntelGen::Xe,    // DG1
            0x5600..=0x56FF => IntelGen::Xe,    // Arc A-series
            _ => IntelGen::Gen9, // Default to Gen9
        }
    }
    
    fn read_reg32(&self, offset: u32) -> u32 {
        unsafe {
            let addr = (self.mmio_base.as_u64() + offset as u64) as *const u32;
            addr.read_volatile()
        }
    }
    
    fn write_reg32(&self, offset: u32, value: u32) {
        unsafe {
            let addr = (self.mmio_base.as_u64() + offset as u64) as *mut u32;
            addr.write_volatile(value);
        }
    }
    
    fn detect_outputs(&mut self) {
        // Detect display outputs
        // This would probe DDC/EDID and detect connected displays
        
        // For now, assume a basic eDP output
        self.outputs.push(DisplayOutput {
            id: 0,
            name: String::from("eDP-1"),
            connector_type: ConnectorType::eDP,
            is_connected: true,
            modes: vec![
                DisplayMode {
                    width: 1920,
                    height: 1080,
                    refresh_rate: 60,
                    pixel_clock: 148500,
                    hsync_start: 2008,
                    hsync_end: 2052,
                    htotal: 2200,
                    vsync_start: 1084,
                    vsync_end: 1089,
                    vtotal: 1125,
                    flags: super::DisplayModeFlags::empty(),
                },
            ],
            current_mode: None,
            edid_data: None,
        });
    }
    
    fn init_ring_buffer(&mut self, engine: EngineType) -> Result<(), &'static str> {
        let ring_base = match engine {
            EngineType::Render => regs::RCS_RING_BASE,
            EngineType::Blitter => regs::BCS_RING_BASE,
            EngineType::Video => regs::VCS_RING_BASE,
            EngineType::VideoEnhance => regs::VECS_RING_BASE,
            _ => return Err("Unsupported engine type"),
        };
        
        // Allocate ring buffer (64KB)
        let ring_size = 64 * 1024;
        
        // Setup ring buffer registers
        self.write_reg32(ring_base + 0x38, 0); // RING_START
        self.write_reg32(ring_base + 0x3C, ring_size); // RING_CTL
        self.write_reg32(ring_base + 0x34, 0); // RING_HEAD
        self.write_reg32(ring_base + 0x30, 0); // RING_TAIL
        
        // Enable ring
        let ctl = self.read_reg32(ring_base + 0x3C);
        self.write_reg32(ring_base + 0x3C, ctl | 1);
        
        Ok(())
    }
    
    fn init_power_management(&mut self) {
        // Enable RC6 (GPU power saving)
        if self.generation >= IntelGen::Gen9 {
            self.write_reg32(regs::GEN6_RC_CONTROL, 0x88040000);
        }
    }
}

impl GpuDriver for IntelGpu {
    fn name(&self) -> &str {
        "Intel GPU"
    }
    
    fn vendor_id(&self) -> u16 {
        self.device.vendor_id
    }
    
    fn device_id(&self) -> u16 {
        self.device.device_id
    }
    
    fn init(&mut self, device: &PciDevice) -> Result<(), &'static str> {
        if self.initialized {
            return Ok(());
        }
        
        crate::println!("Intel GPU: Initializing {:?} generation GPU", self.generation);
        
        // Map MMIO registers (BAR 0)
        let bar0 = device.base_addresses[0];
        if bar0 == 0 {
            return Err("MMIO BAR not configured");
        }
        
        self.mmio_base = VirtAddr::new(bar0 as u64);
        self.mmio_size = 2 * 1024 * 1024; // 2MB typical
        
        // Map GTT (BAR 2)
        let bar2 = device.base_addresses[2];
        if bar2 != 0 {
            self.gtt_base = VirtAddr::new(bar2 as u64);
            self.gtt_size = 512 * 1024 * 1024; // 512MB typical aperture
        }
        
        // Detect stolen memory
        // This would read from BIOS/UEFI configured registers
        self.stolen_memory_size = 64 * 1024 * 1024; // 64MB default
        
        // Setup memory regions
        self.memory_regions.push(MemoryRegion {
            region_type: MemoryType::Stolen,
            physical_address: self.stolen_memory_base,
            virtual_address: None,
            size: self.stolen_memory_size,
            is_mappable: false,
            is_cacheable: false,
            is_write_combined: false,
        });
        
        self.memory_regions.push(MemoryRegion {
            region_type: MemoryType::GttAperture,
            physical_address: PhysAddr::new(bar2 as u64),
            virtual_address: Some(self.gtt_base),
            size: self.gtt_size as u64,
            is_mappable: true,
            is_cacheable: false,
            is_write_combined: true,
        });
        
        // Detect display outputs
        self.detect_outputs();
        
        // Initialize render engine
        self.init_ring_buffer(EngineType::Render)?;
        
        // Initialize blitter engine
        self.init_ring_buffer(EngineType::Blitter)?;
        
        // Setup power management
        self.init_power_management();
        
        self.initialized = true;
        crate::println!("Intel GPU: Initialization complete");
        
        Ok(())
    }
    
    fn reset(&mut self) -> Result<(), &'static str> {
        // Trigger GPU reset
        // This would write to specific reset registers
        Ok(())
    }
    
    fn suspend(&mut self) -> Result<(), &'static str> {
        // Save GPU state and power down
        Ok(())
    }
    
    fn resume(&mut self) -> Result<(), &'static str> {
        // Restore GPU state and power up
        Ok(())
    }
    
    fn get_capabilities(&self) -> &GpuCapabilities {
        &self.capabilities
    }
    
    fn get_memory_regions(&self) -> &[MemoryRegion] {
        &self.memory_regions
    }
    
    fn allocate_buffer(&mut self, size: u64, usage: BufferUsageFlags) -> Result<BufferObject, &'static str> {
        // Allocate from appropriate memory pool
        Ok(BufferObject {
            id: 0,
            size,
            memory_type: MemoryType::SystemRam,
            virtual_address: None,
            physical_address: None,
            is_pinned: false,
            is_tiled: false,
            tiling_mode: super::TilingMode::Linear,
            cache_level: super::CacheLevel::None,
            usage_flags: usage,
        })
    }
    
    fn free_buffer(&mut self, _buffer: BufferObject) -> Result<(), &'static str> {
        Ok(())
    }
    
    fn map_buffer(&mut self, _buffer: &BufferObject) -> Result<VirtAddr, &'static str> {
        Ok(VirtAddr::new(0))
    }
    
    fn unmap_buffer(&mut self, _buffer: &BufferObject) -> Result<(), &'static str> {
        Ok(())
    }
    
    fn create_command_buffer(&mut self, engine: EngineType, size: u64) -> Result<CommandBuffer, &'static str> {
        let buffer = self.allocate_buffer(size, BufferUsageFlags::COMMAND_BUFFER)?;
        
        Ok(CommandBuffer {
            id: 0,
            engine,
            buffer,
            size,
            head: 0,
            tail: 0,
            is_ring: true,
        })
    }
    
    fn submit_command_buffer(&mut self, cmd_buf: &CommandBuffer) -> Result<(), &'static str> {
        let ring_base = match cmd_buf.engine {
            EngineType::Render => regs::RCS_RING_BASE,
            EngineType::Blitter => regs::BCS_RING_BASE,
            _ => return Err("Unsupported engine"),
        };
        
        // Update ring tail pointer
        self.write_reg32(ring_base + 0x30, cmd_buf.tail as u32);
        
        Ok(())
    }
    
    fn wait_idle(&mut self) -> Result<(), &'static str> {
        // Wait for all engines to be idle
        Ok(())
    }
    
    fn get_display_outputs(&self) -> &[DisplayOutput] {
        &self.outputs
    }
    
    fn set_display_mode(&mut self, output_id: u32, mode: &DisplayMode) -> Result<(), &'static str> {
        if output_id as usize >= self.outputs.len() {
            return Err("Invalid output ID");
        }
        
        // Program display timing
        let pipe_conf = regs::PIPE_A_CONF;
        let pipe_src = regs::PIPE_A_SRC;
        
        // Set pipe source size
        self.write_reg32(pipe_src, ((mode.height - 1) << 16) | (mode.width - 1));
        
        // Enable pipe
        let conf = self.read_reg32(pipe_conf);
        self.write_reg32(pipe_conf, conf | 0x80000000);
        
        self.outputs[output_id as usize].current_mode = Some(*mode);
        
        Ok(())
    }
    
    fn create_framebuffer(&mut self, width: u32, height: u32) -> Result<BufferObject, &'static str> {
        let size = (width * height * 4) as u64; // 32-bit color
        self.allocate_buffer(size, BufferUsageFlags::SCANOUT)
    }
    
    fn present_framebuffer(&mut self, buffer: &BufferObject) -> Result<(), &'static str> {
        // Set plane surface address
        if let Some(phys_addr) = buffer.physical_address {
            self.write_reg32(regs::PLANE_A_SURF, phys_addr.as_u64() as u32);
        }
        
        Ok(())
    }
    
    fn enable_acceleration(&mut self) -> Result<(), &'static str> {
        // Enable GPU acceleration features
        Ok(())
    }
    
    fn blit_2d(&mut self, _src: &BufferObject, _dst: &BufferObject,
               _src_x: u32, _src_y: u32, _dst_x: u32, _dst_y: u32,
               _width: u32, _height: u32) -> Result<(), &'static str> {
        // Submit 2D blit command
        Ok(())
    }
    
    fn fill_2d(&mut self, _dst: &BufferObject, _x: u32, _y: u32,
               _width: u32, _height: u32, _color: u32) -> Result<(), &'static str> {
        // Submit 2D fill command
        Ok(())
    }
}