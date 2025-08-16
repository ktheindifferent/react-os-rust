// Compositor - Manages screen composition and rendering
use super::{Color, Rect, Point, Framebuffer, FramebufferOps, window::WINDOW_MANAGER};
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

// Desktop wallpaper pattern
fn draw_desktop_pattern(fb: &mut dyn FramebufferOps) {
    // Gradient background
    let height = fb.height();
    let width = fb.width();
    
    for y in 0..height {
        let intensity = (y * 128 / height) as u8;
        let color = Color::new(0, intensity / 2, intensity);
        
        for x in 0..width {
            fb.set_pixel(x, y, color);
        }
    }
    
    // Draw grid pattern
    let grid_color = Color::new(0, 64, 128);
    for y in (0..height).step_by(32) {
        for x in 0..width {
            fb.set_pixel(x, y, grid_color);
        }
    }
    
    for x in (0..width).step_by(32) {
        for y in 0..height {
            fb.set_pixel(x, y, grid_color);
        }
    }
}

// Compositor manages the final screen composition
pub struct Compositor {
    screen_buffer: Framebuffer,
    desktop_buffer: Framebuffer,
    width: usize,
    height: usize,
    needs_redraw: bool,
    show_fps: bool,
    frame_count: u64,
    last_fps_time: u64,
    current_fps: u32,
}

impl Compositor {
    pub fn new(width: usize, height: usize) -> Self {
        let mut desktop_buffer = Framebuffer::new(width, height);
        draw_desktop_pattern(&mut desktop_buffer);
        
        Self {
            screen_buffer: Framebuffer::new(width, height),
            desktop_buffer,
            width,
            height,
            needs_redraw: true,
            show_fps: false,
            frame_count: 0,
            last_fps_time: 0,
            current_fps: 0,
        }
    }
    
    pub fn compose(&mut self) {
        // Start with desktop background
        self.screen_buffer.blit(
            &self.desktop_buffer,
            Rect::new(0, 0, self.width as u32, self.height as u32),
            Point::new(0, 0)
        );
        
        // Draw windows
        if let Some(wm) = WINDOW_MANAGER.lock().as_mut() {
            wm.render(&mut self.screen_buffer);
        }
        
        // Draw taskbar
        self.draw_taskbar();
        
        // Draw FPS counter if enabled
        if self.show_fps {
            self.draw_fps();
        }
        
        // Update frame counter
        self.frame_count += 1;
        
        self.needs_redraw = false;
    }
    
    fn draw_taskbar(&mut self) {
        let taskbar_height = 40;
        let taskbar_y = self.height - taskbar_height;
        
        // Draw taskbar background
        let taskbar_color = Color::new(48, 48, 48);
        self.screen_buffer.fill_rect(
            Rect::new(0, taskbar_y as i32, self.width as u32, taskbar_height as u32),
            taskbar_color
        );
        
        // Draw start button
        let start_button_width = 48;
        let start_color = Color::new(0, 120, 215);
        self.screen_buffer.fill_rect(
            Rect::new(0, taskbar_y as i32, start_button_width, taskbar_height as u32),
            start_color
        );
        
        // Draw Windows logo (simplified)
        let logo_x = 12;
        let logo_y = taskbar_y + 10;
        let logo_color = Color::WHITE;
        
        // Four squares for Windows logo
        self.screen_buffer.fill_rect(
            Rect::new(logo_x, logo_y as i32, 10, 10),
            logo_color
        );
        self.screen_buffer.fill_rect(
            Rect::new(logo_x + 12, logo_y as i32, 10, 10),
            logo_color
        );
        self.screen_buffer.fill_rect(
            Rect::new(logo_x, (logo_y + 12) as i32, 10, 10),
            logo_color
        );
        self.screen_buffer.fill_rect(
            Rect::new(logo_x + 12, (logo_y + 12) as i32, 10, 10),
            logo_color
        );
        
        // Draw clock
        let clock_text = "12:00 PM";
        let clock_width = 80;
        let clock_x = self.width - clock_width;
        
        super::font::draw_text(
            &mut self.screen_buffer,
            clock_text,
            clock_x + 10,
            taskbar_y + 12,
            Color::WHITE
        );
        
        // Draw system tray icons (simplified)
        let tray_x = self.width - 120;
        let icon_size = 16;
        let icon_y = taskbar_y + 12;
        
        // Network icon
        self.screen_buffer.draw_rect(
            Rect::new(tray_x as i32, icon_y as i32, icon_size, icon_size),
            Color::WHITE
        );
        
        // Volume icon
        self.screen_buffer.draw_rect(
            Rect::new((tray_x - 20) as i32, icon_y as i32, icon_size, icon_size),
            Color::WHITE
        );
        
        // Battery icon
        self.screen_buffer.draw_rect(
            Rect::new((tray_x - 40) as i32, icon_y as i32, icon_size, icon_size),
            Color::WHITE
        );
    }
    
    fn draw_fps(&mut self) {
        let fps_text = alloc::format!("FPS: {}", self.current_fps);
        
        // Draw with background for visibility
        super::font::draw_text_with_bg(
            &mut self.screen_buffer,
            &fps_text,
            10,
            10,
            Color::GREEN,
            Color::new(0, 0, 0)
        );
    }
    
    pub fn present(&mut self) {
        // In a real implementation, this would copy to the actual framebuffer
        // For now, we'll use the VESA driver
        if let Some(mut vesa) = super::vesa::VESA_DRIVER.try_lock() {
            if let Some(fb) = vesa.get_framebuffer() {
                // Copy screen buffer to VESA framebuffer
                // This would need proper pixel format conversion
                for y in 0..self.height.min(fb.height) {
                    for x in 0..self.width.min(fb.width) {
                        let color = self.screen_buffer.get_pixel(x, y);
                        vesa.set_pixel(x, y, color);
                    }
                }
            }
        }
    }
    
    pub fn request_redraw(&mut self) {
        self.needs_redraw = true;
    }
    
    pub fn set_show_fps(&mut self, show: bool) {
        self.show_fps = show;
        self.needs_redraw = true;
    }
    
    pub fn update_fps(&mut self, current_time: u64) {
        if current_time - self.last_fps_time >= 1000 {
            self.current_fps = self.frame_count as u32;
            self.frame_count = 0;
            self.last_fps_time = current_time;
        }
    }
}

// Global compositor
lazy_static! {
    pub static ref COMPOSITOR: Mutex<Option<Compositor>> = Mutex::new(None);
}

pub fn init() {
    let mut comp = COMPOSITOR.lock();
    *comp = Some(Compositor::new(800, 600)); // Default resolution
    crate::serial_println!("Compositor initialized");
}

pub fn compose() {
    if let Some(compositor) = COMPOSITOR.lock().as_mut() {
        if compositor.needs_redraw {
            compositor.compose();
            compositor.present();
        }
    }
}

pub fn request_redraw() {
    if let Some(compositor) = COMPOSITOR.lock().as_mut() {
        compositor.request_redraw();
    }
}

pub fn set_show_fps(show: bool) {
    if let Some(compositor) = COMPOSITOR.lock().as_mut() {
        compositor.set_show_fps(show);
    }
}