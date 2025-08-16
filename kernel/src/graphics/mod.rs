// Graphics subsystem
pub mod vesa;
pub mod framebuffer;
pub mod font;
pub mod window;
pub mod compositor;

use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

// Re-export commonly used types
pub use vesa::{Color, VesaDriver, VESA_DRIVER};
pub use framebuffer::{Framebuffer, FramebufferOps};
pub use window::{Window, WindowId};

// Graphics subsystem initialization
pub fn init() -> Result<(), &'static str> {
    crate::serial_println!("Initializing graphics subsystem...");
    
    // Initialize VESA driver
    vesa::init()?;
    
    // Initialize font system
    font::init();
    
    // Initialize window manager
    window::init();
    
    // Initialize compositor
    compositor::init();
    
    crate::serial_println!("Graphics subsystem initialized");
    Ok(())
}

// Screen resolution information
#[derive(Debug, Clone, Copy)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub const VGA: Resolution = Resolution { width: 640, height: 480 };
    pub const SVGA: Resolution = Resolution { width: 800, height: 600 };
    pub const XGA: Resolution = Resolution { width: 1024, height: 768 };
    pub const SXGA: Resolution = Resolution { width: 1280, height: 1024 };
    pub const HD: Resolution = Resolution { width: 1920, height: 1080 };
}

// Point structure for 2D coordinates
#[derive(Debug, Clone, Copy, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }
}

// Rectangle structure
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Rect { x, y, width, height }
    }
    
    pub fn contains_point(&self, point: Point) -> bool {
        point.x >= self.x && 
        point.x < (self.x + self.width as i32) &&
        point.y >= self.y && 
        point.y < (self.y + self.height as i32)
    }
    
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < (other.x + other.width as i32) &&
        (self.x + self.width as i32) > other.x &&
        self.y < (other.y + other.height as i32) &&
        (self.y + self.height as i32) > other.y
    }
    
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }
        
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width as i32).min(other.x + other.width as i32);
        let bottom = (self.y + self.height as i32).min(other.y + other.height as i32);
        
        Some(Rect::new(x, y, (right - x) as u32, (bottom - y) as u32))
    }
}