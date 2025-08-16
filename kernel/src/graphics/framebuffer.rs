// Framebuffer management and double buffering
use super::{Color, Rect, Point};
use alloc::vec::Vec;
use alloc::boxed::Box;
use spin::Mutex;
use core::ptr;

// Framebuffer trait for different buffer types
pub trait FramebufferOps {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn bpp(&self) -> u8;
    fn set_pixel(&mut self, x: usize, y: usize, color: Color);
    fn get_pixel(&self, x: usize, y: usize) -> Color;
    fn clear(&mut self, color: Color);
    fn blit(&mut self, src: &dyn FramebufferOps, src_rect: Rect, dst_point: Point);
}

// Software framebuffer for double buffering
pub struct Framebuffer {
    width: usize,
    height: usize,
    bpp: u8,
    buffer: Vec<u32>,  // ARGB8888 format for simplicity
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let buffer_size = width * height;
        let mut buffer = Vec::new();
        buffer.resize(buffer_size, 0xFF000000); // Black with full alpha
        
        Self {
            width,
            height,
            bpp: 32,
            buffer,
        }
    }
    
    // Create from existing buffer
    pub fn from_buffer(width: usize, height: usize, buffer: Vec<u32>) -> Self {
        Self {
            width,
            height,
            bpp: 32,
            buffer,
        }
    }
    
    // Get raw buffer
    pub fn buffer(&self) -> &[u32] {
        &self.buffer
    }
    
    // Get mutable buffer
    pub fn buffer_mut(&mut self) -> &mut [u32] {
        &mut self.buffer
    }
    
    // Fast fill with single color
    pub fn fill(&mut self, color: Color) {
        let pixel = color.to_argb8888();
        for p in self.buffer.iter_mut() {
            *p = pixel;
        }
    }
    
    // Draw filled rectangle
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let pixel = color.to_argb8888();
        let x_start = rect.x.max(0) as usize;
        let y_start = rect.y.max(0) as usize;
        let x_end = ((rect.x + rect.width as i32).min(self.width as i32) as usize).min(self.width);
        let y_end = ((rect.y + rect.height as i32).min(self.height as i32) as usize).min(self.height);
        
        for y in y_start..y_end {
            let row_start = y * self.width + x_start;
            let row_end = y * self.width + x_end;
            for i in row_start..row_end {
                self.buffer[i] = pixel;
            }
        }
    }
    
    // Draw rectangle outline
    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        let pixel = color.to_argb8888();
        let x_start = rect.x.max(0) as usize;
        let y_start = rect.y.max(0) as usize;
        let x_end = ((rect.x + rect.width as i32).min(self.width as i32) as usize).min(self.width);
        let y_end = ((rect.y + rect.height as i32).min(self.height as i32) as usize).min(self.height);
        
        // Top and bottom edges
        if y_start < self.height {
            for x in x_start..x_end {
                self.buffer[y_start * self.width + x] = pixel;
            }
        }
        if y_end > 0 && y_end - 1 < self.height {
            for x in x_start..x_end {
                self.buffer[(y_end - 1) * self.width + x] = pixel;
            }
        }
        
        // Left and right edges
        for y in y_start..y_end {
            if x_start < self.width {
                self.buffer[y * self.width + x_start] = pixel;
            }
            if x_end > 0 && x_end - 1 < self.width {
                self.buffer[y * self.width + (x_end - 1)] = pixel;
            }
        }
    }
    
    // Draw line using Bresenham's algorithm
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let pixel = color.to_argb8888();
        let mut x = x0;
        let mut y = y0;
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        
        loop {
            if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
                self.buffer[(y as usize) * self.width + (x as usize)] = pixel;
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
    
    // Draw circle using midpoint algorithm
    pub fn draw_circle(&mut self, cx: i32, cy: i32, radius: i32, color: Color) {
        let pixel = color.to_argb8888();
        let mut x = 0;
        let mut y = radius;
        let mut d = 1 - radius;
        
        // Draw the initial points
        self.set_pixel_safe(cx, cy + radius, pixel);
        self.set_pixel_safe(cx, cy - radius, pixel);
        self.set_pixel_safe(cx + radius, cy, pixel);
        self.set_pixel_safe(cx - radius, cy, pixel);
        
        while x < y {
            if d < 0 {
                d += 2 * x + 3;
            } else {
                d += 2 * (x - y) + 5;
                y -= 1;
            }
            x += 1;
            
            // Draw 8 octants
            self.set_pixel_safe(cx + x, cy + y, pixel);
            self.set_pixel_safe(cx - x, cy + y, pixel);
            self.set_pixel_safe(cx + x, cy - y, pixel);
            self.set_pixel_safe(cx - x, cy - y, pixel);
            self.set_pixel_safe(cx + y, cy + x, pixel);
            self.set_pixel_safe(cx - y, cy + x, pixel);
            self.set_pixel_safe(cx + y, cy - x, pixel);
            self.set_pixel_safe(cx - y, cy - x, pixel);
        }
    }
    
    // Draw filled circle
    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: i32, color: Color) {
        let pixel = color.to_argb8888();
        
        for y in -radius..=radius {
            let dy2 = y * y;
            let r2_minus_dy2 = radius * radius - dy2;
            
            // Integer square root approximation
            let mut max_x = 0;
            while max_x * max_x <= r2_minus_dy2 {
                max_x += 1;
            }
            max_x -= 1;
            
            for x in -max_x..=max_x {
                self.set_pixel_safe(cx + x, cy + y, pixel);
            }
        }
    }
    
    // Safe pixel setting with bounds checking
    fn set_pixel_safe(&mut self, x: i32, y: i32, pixel: u32) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            self.buffer[(y as usize) * self.width + (x as usize)] = pixel;
        }
    }
    
    // Alpha blending
    pub fn blend_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }
        
        let index = y * self.width + x;
        let dst = self.buffer[index];
        
        // Extract destination components
        let dst_a = ((dst >> 24) & 0xFF) as u32;
        let dst_r = ((dst >> 16) & 0xFF) as u32;
        let dst_g = ((dst >> 8) & 0xFF) as u32;
        let dst_b = (dst & 0xFF) as u32;
        
        // Source alpha
        let src_a = color.a as u32;
        let inv_src_a = 255 - src_a;
        
        // Blend
        let out_a = src_a + (dst_a * inv_src_a) / 255;
        let out_r = (color.r as u32 * src_a + dst_r * inv_src_a) / 255;
        let out_g = (color.g as u32 * src_a + dst_g * inv_src_a) / 255;
        let out_b = (color.b as u32 * src_a + dst_b * inv_src_a) / 255;
        
        self.buffer[index] = (out_a << 24) | (out_r << 16) | (out_g << 8) | out_b;
    }
}

impl FramebufferOps for Framebuffer {
    fn width(&self) -> usize {
        self.width
    }
    
    fn height(&self) -> usize {
        self.height
    }
    
    fn bpp(&self) -> u8 {
        self.bpp
    }
    
    fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = color.to_argb8888();
        }
    }
    
    fn get_pixel(&self, x: usize, y: usize) -> Color {
        if x < self.width && y < self.height {
            let pixel = self.buffer[y * self.width + x];
            Color {
                a: ((pixel >> 24) & 0xFF) as u8,
                r: ((pixel >> 16) & 0xFF) as u8,
                g: ((pixel >> 8) & 0xFF) as u8,
                b: (pixel & 0xFF) as u8,
            }
        } else {
            Color::BLACK
        }
    }
    
    fn clear(&mut self, color: Color) {
        self.fill(color);
    }
    
    fn blit(&mut self, src: &dyn FramebufferOps, src_rect: Rect, dst_point: Point) {
        let src_x = src_rect.x.max(0) as usize;
        let src_y = src_rect.y.max(0) as usize;
        let width = src_rect.width.min((src.width() - src_x) as u32) as usize;
        let height = src_rect.height.min((src.height() - src_y) as u32) as usize;
        
        let dst_x = dst_point.x.max(0) as usize;
        let dst_y = dst_point.y.max(0) as usize;
        
        for y in 0..height {
            if dst_y + y >= self.height {
                break;
            }
            
            for x in 0..width {
                if dst_x + x >= self.width {
                    break;
                }
                
                let color = src.get_pixel(src_x + x, src_y + y);
                if color.a > 0 {
                    if color.a == 255 {
                        self.set_pixel(dst_x + x, dst_y + y, color);
                    } else {
                        self.blend_pixel(dst_x + x, dst_y + y, color);
                    }
                }
            }
        }
    }
}

// Double buffering manager
pub struct DoubleBuffer {
    front: Mutex<Framebuffer>,
    back: Mutex<Framebuffer>,
    width: usize,
    height: usize,
}

impl DoubleBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            front: Mutex::new(Framebuffer::new(width, height)),
            back: Mutex::new(Framebuffer::new(width, height)),
            width,
            height,
        }
    }
    
    // Get back buffer for drawing
    pub fn back_buffer(&self) -> &Mutex<Framebuffer> {
        &self.back
    }
    
    // Swap buffers
    pub fn swap(&self) {
        let mut front = self.front.lock();
        let mut back = self.back.lock();
        
        // Simple swap - in a real implementation, we'd swap pointers
        for i in 0..front.buffer.len() {
            core::mem::swap(&mut front.buffer[i], &mut back.buffer[i]);
        }
    }
    
    // Present front buffer to screen
    pub fn present(&self) {
        // This would copy the front buffer to the actual framebuffer
        // For now, it's a placeholder
        let front = self.front.lock();
        // Copy to VESA framebuffer here
    }
}