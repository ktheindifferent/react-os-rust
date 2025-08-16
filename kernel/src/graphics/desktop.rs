use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;
use super::{Color, Point, Rect, framebuffer::Framebuffer};

const DOUBLE_BUFFER_SIZE: usize = 1920 * 1080 * 4;

pub struct DesktopManager {
    width: u32,
    height: u32,
    back_buffer: Vec<u32>,
    front_buffer: Vec<u32>,
    dirty_regions: Vec<Rect>,
    windows: BTreeMap<u32, DesktopWindow>,
    next_window_id: u32,
    focused_window: Option<u32>,
    background_color: Color,
    cursor_position: Point,
    cursor_visible: bool,
}

#[derive(Clone)]
pub struct DesktopWindow {
    pub id: u32,
    pub rect: Rect,
    pub title: String,
    pub buffer: Vec<u32>,
    pub visible: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub resizable: bool,
    pub movable: bool,
    pub z_order: u32,
    pub dirty: bool,
    pub transparent: bool,
    pub opacity: u8,
}

impl DesktopWindow {
    pub fn new(id: u32, x: i32, y: i32, width: u32, height: u32, title: String) -> Self {
        let buffer_size = (width * height) as usize;
        let mut buffer = Vec::with_capacity(buffer_size);
        buffer.resize(buffer_size, 0xFFFFFFFF);
        
        DesktopWindow {
            id,
            rect: Rect::new(x, y, width, height),
            title,
            buffer,
            visible: true,
            minimized: false,
            maximized: false,
            resizable: true,
            movable: true,
            z_order: 0,
            dirty: true,
            transparent: false,
            opacity: 255,
        }
    }
    
    pub fn draw_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x < self.rect.width && y < self.rect.height {
            let index = (y * self.rect.width + x) as usize;
            if index < self.buffer.len() {
                self.buffer[index] = color.to_argb8888();
                self.dirty = true;
            }
        }
    }
    
    pub fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, color: Color) {
        let color_val = color.to_argb8888();
        for dy in 0..height {
            for dx in 0..width {
                let px = x + dx;
                let py = y + dy;
                if px < self.rect.width && py < self.rect.height {
                    let index = (py * self.rect.width + px) as usize;
                    if index < self.buffer.len() {
                        self.buffer[index] = color_val;
                    }
                }
            }
        }
        self.dirty = true;
    }
    
    pub fn clear(&mut self, color: Color) {
        let color_val = color.to_argb8888();
        for pixel in &mut self.buffer {
            *pixel = color_val;
        }
        self.dirty = true;
    }
}

impl DesktopManager {
    pub fn new(width: u32, height: u32) -> Self {
        let buffer_size = (width * height) as usize;
        let mut back_buffer = Vec::with_capacity(buffer_size);
        let mut front_buffer = Vec::with_capacity(buffer_size);
        
        back_buffer.resize(buffer_size, 0xFF2C3E50);
        front_buffer.resize(buffer_size, 0xFF2C3E50);
        
        DesktopManager {
            width,
            height,
            back_buffer,
            front_buffer,
            dirty_regions: Vec::new(),
            windows: BTreeMap::new(),
            next_window_id: 1,
            focused_window: None,
            background_color: Color::new(44, 62, 80),
            cursor_position: Point::new(width as i32 / 2, height as i32 / 2),
            cursor_visible: true,
        }
    }
    
    pub fn create_window(&mut self, x: i32, y: i32, width: u32, height: u32, title: String) -> u32 {
        let id = self.next_window_id;
        self.next_window_id += 1;
        
        let mut window = DesktopWindow::new(id, x, y, width, height, title);
        window.z_order = self.windows.len() as u32;
        
        self.windows.insert(id, window);
        self.focused_window = Some(id);
        self.mark_dirty(Rect::new(x, y, width, height));
        
        id
    }
    
    pub fn destroy_window(&mut self, id: u32) {
        if let Some(window) = self.windows.remove(&id) {
            self.mark_dirty(window.rect);
            
            if self.focused_window == Some(id) {
                self.focused_window = self.windows.keys().next().copied();
            }
        }
    }
    
    pub fn move_window(&mut self, id: u32, new_x: i32, new_y: i32) {
        let (old_rect, new_rect) = if let Some(window) = self.windows.get_mut(&id) {
            let old_rect = window.rect;
            window.rect.x = new_x;
            window.rect.y = new_y;
            (old_rect, window.rect)
        } else {
            return;
        };
        
        self.mark_dirty(old_rect);
        self.mark_dirty(new_rect);
    }
    
    pub fn resize_window(&mut self, id: u32, new_width: u32, new_height: u32) {
        let (old_rect, new_rect) = if let Some(window) = self.windows.get_mut(&id) {
            let old_rect = window.rect;
            window.rect.width = new_width;
            window.rect.height = new_height;
            
            let new_buffer_size = (new_width * new_height) as usize;
            window.buffer.resize(new_buffer_size, 0xFFFFFFFF);
            
            (old_rect, window.rect)
        } else {
            return;
        };
        
        self.mark_dirty(old_rect);
        self.mark_dirty(new_rect);
    }
    
    pub fn focus_window(&mut self, id: u32) {
        if self.windows.contains_key(&id) {
            self.focused_window = Some(id);
            
            let max_z = self.windows.values().map(|w| w.z_order).max().unwrap_or(0);
            let rect = if let Some(window) = self.windows.get_mut(&id) {
                window.z_order = max_z + 1;
                window.rect
            } else {
                return;
            };
            
            self.mark_dirty(rect);
        }
    }
    
    pub fn get_window_at_point(&self, point: Point) -> Option<u32> {
        let mut windows: Vec<_> = self.windows.values().collect();
        windows.sort_by_key(|w| -(w.z_order as i32));
        
        for window in windows {
            if window.visible && !window.minimized && window.rect.contains_point(point) {
                return Some(window.id);
            }
        }
        
        None
    }
    
    fn mark_dirty(&mut self, rect: Rect) {
        self.dirty_regions.push(rect);
    }
    
    fn composite(&mut self) {
        for pixel in &mut self.back_buffer {
            *pixel = self.background_color.to_argb8888();
        }
        
        // Clone window data to avoid borrow conflicts
        let mut window_data: Vec<_> = self.windows.values()
            .filter(|w| w.visible && !w.minimized)
            .map(|w| (w.rect, w.buffer.clone(), w.transparent, w.opacity))
            .collect();
        
        window_data.sort_by_key(|(rect, _, _, _)| rect.x); // Sort by some criteria
        
        for (rect, buffer, transparent, opacity) in window_data {
            self.composite_window_buffer(rect, &buffer, transparent, opacity);
        }
        
        if self.cursor_visible {
            self.draw_cursor();
        }
    }
    
    fn composite_window_buffer(&mut self, rect: Rect, buffer: &[u32], transparent: bool, opacity: u8) {
        let start_x = rect.x.max(0) as u32;
        let start_y = rect.y.max(0) as u32;
        let end_x = ((rect.x + rect.width as i32).min(self.width as i32)) as u32;
        let end_y = ((rect.y + rect.height as i32).min(self.height as i32)) as u32;
        
        for y in start_y..end_y {
            for x in start_x..end_x {
                let window_x = (x as i32 - rect.x) as u32;
                let window_y = (y as i32 - rect.y) as u32;
                
                let window_index = (window_y * rect.width + window_x) as usize;
                let screen_index = (y * self.width + x) as usize;
                
                if window_index < buffer.len() && screen_index < self.back_buffer.len() {
                    let pixel = buffer[window_index];
                    
                    if transparent {
                        let alpha = ((pixel >> 24) & 0xFF) as u16 * opacity as u16 / 255;
                        if alpha > 0 {
                            self.back_buffer[screen_index] = blend_pixels(
                                self.back_buffer[screen_index],
                                pixel,
                                alpha as u8
                            );
                        }
                    } else {
                        self.back_buffer[screen_index] = pixel;
                    }
                }
            }
        }
    }
    
    fn composite_window(&mut self, window: &DesktopWindow) {
        let start_x = window.rect.x.max(0) as u32;
        let start_y = window.rect.y.max(0) as u32;
        let end_x = ((window.rect.x + window.rect.width as i32).min(self.width as i32)) as u32;
        let end_y = ((window.rect.y + window.rect.height as i32).min(self.height as i32)) as u32;
        
        for y in start_y..end_y {
            for x in start_x..end_x {
                let window_x = (x as i32 - window.rect.x) as u32;
                let window_y = (y as i32 - window.rect.y) as u32;
                
                let window_index = (window_y * window.rect.width + window_x) as usize;
                let screen_index = (y * self.width + x) as usize;
                
                if window_index < window.buffer.len() && screen_index < self.back_buffer.len() {
                    let pixel = window.buffer[window_index];
                    
                    if window.transparent {
                        let alpha = ((pixel >> 24) & 0xFF) as u16 * window.opacity as u16 / 255;
                        if alpha > 0 {
                            self.back_buffer[screen_index] = blend_pixels(
                                self.back_buffer[screen_index],
                                pixel,
                                alpha as u8
                            );
                        }
                    } else {
                        self.back_buffer[screen_index] = pixel;
                    }
                }
            }
        }
    }
    
    fn draw_cursor(&mut self) {
        const CURSOR_SIZE: i32 = 12;
        const CURSOR_COLOR: u32 = 0xFFFFFFFF;
        
        let cx = self.cursor_position.x;
        let cy = self.cursor_position.y;
        
        for y in (cy - CURSOR_SIZE / 2)..(cy + CURSOR_SIZE / 2) {
            if y >= 0 && y < self.height as i32 {
                let index = (y * self.width as i32 + cx) as usize;
                if index < self.back_buffer.len() && cx >= 0 && cx < self.width as i32 {
                    self.back_buffer[index] = CURSOR_COLOR;
                }
            }
        }
        
        for x in (cx - CURSOR_SIZE / 2)..(cx + CURSOR_SIZE / 2) {
            if x >= 0 && x < self.width as i32 {
                let index = (cy * self.width as i32 + x) as usize;
                if index < self.back_buffer.len() && cy >= 0 && cy < self.height as i32 {
                    self.back_buffer[index] = CURSOR_COLOR;
                }
            }
        }
    }
    
    pub fn render(&mut self) {
        self.composite();
        
        core::mem::swap(&mut self.front_buffer, &mut self.back_buffer);
        
        self.dirty_regions.clear();
    }
    
    pub fn get_front_buffer(&self) -> &[u32] {
        &self.front_buffer
    }
    
    pub fn set_cursor_position(&mut self, x: i32, y: i32) {
        let old_pos = self.cursor_position;
        self.cursor_position = Point::new(
            x.max(0).min(self.width as i32 - 1),
            y.max(0).min(self.height as i32 - 1)
        );
        
        self.mark_dirty(Rect::new(old_pos.x - 10, old_pos.y - 10, 20, 20));
        self.mark_dirty(Rect::new(self.cursor_position.x - 10, self.cursor_position.y - 10, 20, 20));
    }
    
    pub fn show_cursor(&mut self, visible: bool) {
        self.cursor_visible = visible;
        self.mark_dirty(Rect::new(
            self.cursor_position.x - 10,
            self.cursor_position.y - 10,
            20,
            20
        ));
    }
}

fn blend_pixels(dst: u32, src: u32, alpha: u8) -> u32 {
    let inv_alpha = 255 - alpha;
    
    let dst_r = (dst >> 16) & 0xFF;
    let dst_g = (dst >> 8) & 0xFF;
    let dst_b = dst & 0xFF;
    
    let src_r = (src >> 16) & 0xFF;
    let src_g = (src >> 8) & 0xFF;
    let src_b = src & 0xFF;
    
    let r = ((src_r * alpha as u32 + dst_r * inv_alpha as u32) / 255) & 0xFF;
    let g = ((src_g * alpha as u32 + dst_g * inv_alpha as u32) / 255) & 0xFF;
    let b = ((src_b * alpha as u32 + dst_b * inv_alpha as u32) / 255) & 0xFF;
    
    0xFF000000 | (r << 16) | (g << 8) | b
}


lazy_static! {
    pub static ref DESKTOP_MANAGER: Mutex<Option<DesktopManager>> = Mutex::new(None);
}

pub fn init(width: u32, height: u32) {
    let mut dm = DESKTOP_MANAGER.lock();
    *dm = Some(DesktopManager::new(width, height));
    crate::serial_println!("Desktop manager initialized with {}x{} resolution", width, height);
}