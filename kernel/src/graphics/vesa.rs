// VESA/VBE Graphics Mode Support
use x86_64::{VirtAddr, PhysAddr};
use core::ptr;
use spin::Mutex;
use lazy_static::lazy_static;
use alloc::vec::Vec;

// VESA BIOS Extensions (VBE) constants
const VBE_SIGNATURE: u32 = 0x41534556; // "VESA"
const VBE_VERSION_2_0: u16 = 0x0200;
const VBE_VERSION_3_0: u16 = 0x0300;

// VBE function numbers
const VBE_GET_INFO: u16 = 0x4F00;
const VBE_GET_MODE_INFO: u16 = 0x4F01;
const VBE_SET_MODE: u16 = 0x4F02;
const VBE_GET_CURRENT_MODE: u16 = 0x4F03;

// Standard VESA modes
pub const MODE_640X480X16: u16 = 0x111;   // 640x480, 16-bit color
pub const MODE_640X480X24: u16 = 0x112;   // 640x480, 24-bit color
pub const MODE_800X600X16: u16 = 0x114;   // 800x600, 16-bit color
pub const MODE_800X600X24: u16 = 0x115;   // 800x600, 24-bit color
pub const MODE_1024X768X16: u16 = 0x117;  // 1024x768, 16-bit color
pub const MODE_1024X768X24: u16 = 0x118;  // 1024x768, 24-bit color
pub const MODE_1280X1024X16: u16 = 0x11A; // 1280x1024, 16-bit color
pub const MODE_1280X1024X24: u16 = 0x11B; // 1280x1024, 24-bit color

// VBE Info Block structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VbeInfoBlock {
    signature: [u8; 4],           // "VESA" signature
    version: u16,                 // VBE version
    oem_string_ptr: u32,          // Pointer to OEM string
    capabilities: u32,            // Graphics capabilities
    video_mode_ptr: u32,          // Pointer to video mode list
    total_memory: u16,            // Video memory in 64KB blocks
    oem_software_rev: u16,        // OEM software revision
    oem_vendor_name_ptr: u32,     // OEM vendor name pointer
    oem_product_name_ptr: u32,    // OEM product name pointer
    oem_product_rev_ptr: u32,     // OEM product revision pointer
    reserved: [u8; 222],          // Reserved
    oem_data: [u8; 256],          // OEM data
}

// Mode Info Block structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ModeInfoBlock {
    // Mandatory information for all VBE revisions
    mode_attributes: u16,
    win_a_attributes: u8,
    win_b_attributes: u8,
    win_granularity: u16,
    win_size: u16,
    win_a_segment: u16,
    win_b_segment: u16,
    win_func_ptr: u32,
    bytes_per_scan_line: u16,
    
    // Mandatory for VBE 1.2+
    x_resolution: u16,
    y_resolution: u16,
    x_char_size: u8,
    y_char_size: u8,
    number_of_planes: u8,
    bits_per_pixel: u8,
    number_of_banks: u8,
    memory_model: u8,
    bank_size: u8,
    number_of_image_pages: u8,
    reserved1: u8,
    
    // Direct color fields
    red_mask_size: u8,
    red_field_position: u8,
    green_mask_size: u8,
    green_field_position: u8,
    blue_mask_size: u8,
    blue_field_position: u8,
    reserved_mask_size: u8,
    reserved_field_position: u8,
    direct_color_mode_info: u8,
    
    // Mandatory for VBE 2.0+
    phys_base_ptr: u32,           // Physical address of framebuffer
    reserved2: u32,
    reserved3: u16,
    
    // Mandatory for VBE 3.0+
    lin_bytes_per_scan_line: u16,
    bnk_number_of_image_pages: u8,
    lin_number_of_image_pages: u8,
    lin_red_mask_size: u8,
    lin_red_field_position: u8,
    lin_green_mask_size: u8,
    lin_green_field_position: u8,
    lin_blue_mask_size: u8,
    lin_blue_field_position: u8,
    lin_reserved_mask_size: u8,
    lin_reserved_field_position: u8,
    max_pixel_clock: u32,
    
    reserved4: [u8; 189],
}

// Framebuffer information
pub struct Framebuffer {
    pub base_address: VirtAddr,
    pub physical_address: PhysAddr,
    pub width: usize,
    pub height: usize,
    pub pitch: usize,              // Bytes per scan line
    pub bpp: u8,                   // Bits per pixel
    pub bytes_per_pixel: usize,
    pub size: usize,               // Total framebuffer size
}

// Color structure for different pixel formats
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
    pub const RED: Color = Color { r: 255, g: 0, b: 0, a: 255 };
    pub const GREEN: Color = Color { r: 0, g: 255, b: 0, a: 255 };
    pub const BLUE: Color = Color { r: 0, g: 0, b: 255, a: 255 };
    pub const YELLOW: Color = Color { r: 255, g: 255, b: 0, a: 255 };
    pub const CYAN: Color = Color { r: 0, g: 255, b: 255, a: 255 };
    pub const MAGENTA: Color = Color { r: 255, g: 0, b: 255, a: 255 };
    
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }
    
    pub fn with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }
    
    // Convert to 16-bit RGB565 format
    pub fn to_rgb565(&self) -> u16 {
        let r = (self.r as u16 >> 3) & 0x1F;
        let g = (self.g as u16 >> 2) & 0x3F;
        let b = (self.b as u16 >> 3) & 0x1F;
        (r << 11) | (g << 5) | b
    }
    
    // Convert to 24-bit RGB888 format
    pub fn to_rgb888(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
    
    // Convert to 32-bit ARGB8888 format
    pub fn to_argb8888(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | 
        ((self.g as u32) << 8) | (self.b as u32)
    }
}

// VESA graphics driver
pub struct VesaDriver {
    framebuffer: Option<Framebuffer>,
    current_mode: Option<u16>,
    available_modes: Vec<u16>,
}

impl VesaDriver {
    pub fn new() -> Self {
        Self {
            framebuffer: None,
            current_mode: None,
            available_modes: Vec::new(),
        }
    }
    
    // Initialize VESA driver and detect available modes
    pub fn init(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would:
        // 1. Call VBE BIOS functions to get info
        // 2. Parse available modes
        // 3. Set up framebuffer mapping
        
        // For now, we'll simulate with common modes
        let mut modes = Vec::new();
        modes.push(MODE_640X480X16);
        modes.push(MODE_640X480X24);
        modes.push(MODE_800X600X16);
        modes.push(MODE_800X600X24);
        modes.push(MODE_1024X768X16);
        modes.push(MODE_1024X768X24);
        self.available_modes = modes;
        
        crate::serial_println!("VESA driver initialized with {} modes", 
            self.available_modes.len());
        
        Ok(())
    }
    
    // Set graphics mode
    pub fn set_mode(&mut self, mode: u16) -> Result<(), &'static str> {
        // Check if mode is available
        if !self.available_modes.contains(&mode) {
            return Err("Mode not supported");
        }
        
        // Get mode information
        let (width, height, bpp) = match mode {
            MODE_640X480X16 => (640, 480, 16),
            MODE_640X480X24 => (640, 480, 24),
            MODE_800X600X16 => (800, 600, 16),
            MODE_800X600X24 => (800, 600, 24),
            MODE_1024X768X16 => (1024, 768, 16),
            MODE_1024X768X24 => (1024, 768, 24),
            MODE_1280X1024X16 => (1280, 1024, 16),
            MODE_1280X1024X24 => (1280, 1024, 24),
            _ => return Err("Unknown mode"),
        };
        
        let bytes_per_pixel = (bpp + 7) / 8;
        let pitch = width * bytes_per_pixel;
        let size = pitch * height;
        
        // In a real implementation, we would:
        // 1. Call VBE SET_MODE BIOS function
        // 2. Map the framebuffer to virtual memory
        // For now, use a dummy address
        let phys_addr = PhysAddr::new(0xE0000000); // Typical framebuffer location
        let virt_addr = VirtAddr::new(0xFFFF_8000_E000_0000);
        
        self.framebuffer = Some(Framebuffer {
            base_address: virt_addr,
            physical_address: phys_addr,
            width,
            height,
            pitch,
            bpp: bpp as u8,
            bytes_per_pixel,
            size,
        });
        
        self.current_mode = Some(mode);
        
        crate::serial_println!("Set VESA mode 0x{:X} ({}x{} {}bpp)", 
            mode, width, height, bpp);
        
        Ok(())
    }
    
    // Get current framebuffer
    pub fn get_framebuffer(&self) -> Option<&Framebuffer> {
        self.framebuffer.as_ref()
    }
    
    // Draw a pixel
    pub fn set_pixel(&self, x: usize, y: usize, color: Color) {
        if let Some(fb) = &self.framebuffer {
            if x >= fb.width || y >= fb.height {
                return;
            }
            
            let offset = y * fb.pitch + x * fb.bytes_per_pixel;
            let ptr = fb.base_address.as_u64() as *mut u8;
            
            unsafe {
                match fb.bpp {
                    16 => {
                        let pixel = color.to_rgb565();
                        let pixel_ptr = ptr.add(offset) as *mut u16;
                        *pixel_ptr = pixel;
                    }
                    24 => {
                        *ptr.add(offset) = color.b;
                        *ptr.add(offset + 1) = color.g;
                        *ptr.add(offset + 2) = color.r;
                    }
                    32 => {
                        let pixel = color.to_argb8888();
                        let pixel_ptr = ptr.add(offset) as *mut u32;
                        *pixel_ptr = pixel;
                    }
                    _ => {}
                }
            }
        }
    }
    
    // Clear screen with color
    pub fn clear(&self, color: Color) {
        if let Some(fb) = &self.framebuffer {
            for y in 0..fb.height {
                for x in 0..fb.width {
                    self.set_pixel(x, y, color);
                }
            }
        }
    }
    
    // Draw a filled rectangle
    pub fn fill_rect(&self, x: usize, y: usize, width: usize, height: usize, color: Color) {
        for dy in 0..height {
            for dx in 0..width {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }
    
    // Draw a line using Bresenham's algorithm
    pub fn draw_line(&self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let mut x = x0;
        let mut y = y0;
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        
        loop {
            if x >= 0 && y >= 0 {
                self.set_pixel(x as usize, y as usize, color);
            }
            
            if x == x1 && y == y1 {
                break;
            }
            
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }
    
    // Draw a rectangle outline
    pub fn draw_rect(&self, x: usize, y: usize, width: usize, height: usize, color: Color) {
        // Top and bottom
        for dx in 0..width {
            self.set_pixel(x + dx, y, color);
            self.set_pixel(x + dx, y + height - 1, color);
        }
        
        // Left and right
        for dy in 1..height - 1 {
            self.set_pixel(x, y + dy, color);
            self.set_pixel(x + width - 1, y + dy, color);
        }
    }
}

// Global VESA driver instance
lazy_static! {
    pub static ref VESA_DRIVER: Mutex<VesaDriver> = Mutex::new(VesaDriver::new());
}

// Initialize VESA graphics
pub fn init() -> Result<(), &'static str> {
    let mut driver = VESA_DRIVER.lock();
    driver.init()?;
    
    // Try to set a default mode (800x600 16-bit)
    if let Err(e) = driver.set_mode(MODE_800X600X16) {
        crate::serial_println!("Failed to set default VESA mode: {}", e);
        // Fall back to text mode
        return Ok(());
    }
    
    // Clear screen to black
    driver.clear(Color::BLACK);
    
    // Draw a test pattern
    driver.fill_rect(10, 10, 100, 100, Color::RED);
    driver.fill_rect(120, 10, 100, 100, Color::GREEN);
    driver.fill_rect(230, 10, 100, 100, Color::BLUE);
    
    driver.draw_rect(10, 120, 320, 200, Color::WHITE);
    driver.draw_line(10, 120, 330, 320, Color::YELLOW);
    
    crate::serial_println!("VESA graphics initialized successfully");
    
    Ok(())
}