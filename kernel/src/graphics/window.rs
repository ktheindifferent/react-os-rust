// Window Manager
use super::{Color, Rect, Point, Framebuffer, FramebufferOps};
use alloc::vec::Vec;
use alloc::string::String;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;

// Window ID type
pub type WindowId = usize;

// Window flags
bitflags::bitflags! {
    pub struct WindowFlags: u32 {
        const VISIBLE = 0x0001;
        const RESIZABLE = 0x0002;
        const MOVABLE = 0x0004;
        const CLOSABLE = 0x0008;
        const MINIMIZABLE = 0x0010;
        const MAXIMIZABLE = 0x0020;
        const HAS_TITLE_BAR = 0x0040;
        const HAS_BORDER = 0x0080;
        const ALWAYS_ON_TOP = 0x0100;
        const MODAL = 0x0200;
        const TRANSPARENT = 0x0400;
    }
}

// Window state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

// Window events
#[derive(Debug, Clone)]
pub enum WindowEvent {
    Close,
    Minimize,
    Maximize,
    Restore,
    Move(Point),
    Resize(u32, u32),
    Focus,
    Blur,
    Paint,
    MouseEnter,
    MouseLeave,
    MouseMove(Point),
    MouseDown(MouseButton, Point),
    MouseUp(MouseButton, Point),
    KeyDown(u8),
    KeyUp(u8),
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

// Window structure
pub struct Window {
    pub id: WindowId,
    pub title: String,
    pub rect: Rect,
    pub client_rect: Rect,
    pub flags: WindowFlags,
    pub state: WindowState,
    pub z_order: i32,
    pub parent: Option<WindowId>,
    pub children: Vec<WindowId>,
    pub framebuffer: Framebuffer,
    pub background_color: Color,
    pub border_color: Color,
    pub title_bar_color: Color,
    pub title_text_color: Color,
    pub is_focused: bool,
    pub is_dirty: bool,
}

impl Window {
    pub fn new(id: WindowId, title: String, rect: Rect, flags: WindowFlags) -> Self {
        let mut window = Self {
            id,
            title,
            rect,
            client_rect: rect,
            flags,
            state: WindowState::Normal,
            z_order: 0,
            parent: None,
            children: Vec::new(),
            framebuffer: Framebuffer::new(rect.width as usize, rect.height as usize),
            background_color: Color::new(240, 240, 240), // Light gray
            border_color: Color::new(128, 128, 128),      // Gray
            title_bar_color: Color::new(0, 120, 215),     // Windows 10 blue
            title_text_color: Color::WHITE,
            is_focused: false,
            is_dirty: true,
        };
        
        // Calculate client rect (excluding title bar and borders)
        window.update_client_rect();
        window
    }
    
    fn update_client_rect(&mut self) {
        let mut client_x = 0;
        let mut client_y = 0;
        let mut client_width = self.rect.width;
        let mut client_height = self.rect.height;
        
        if self.flags.contains(WindowFlags::HAS_BORDER) {
            client_x += 2;
            client_y += 2;
            client_width = client_width.saturating_sub(4);
            client_height = client_height.saturating_sub(4);
        }
        
        if self.flags.contains(WindowFlags::HAS_TITLE_BAR) {
            client_y += 24; // Title bar height
            client_height = client_height.saturating_sub(24);
        }
        
        self.client_rect = Rect::new(
            self.rect.x + client_x as i32,
            self.rect.y + client_y as i32,
            client_width,
            client_height
        );
    }
    
    pub fn move_to(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        self.update_client_rect();
        self.is_dirty = true;
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.rect.width = width;
        self.rect.height = height;
        self.framebuffer = Framebuffer::new(width as usize, height as usize);
        self.update_client_rect();
        self.is_dirty = true;
    }
    
    pub fn set_title(&mut self, title: String) {
        self.title = title;
        self.is_dirty = true;
    }
    
    pub fn paint(&mut self) {
        // Clear with background color
        self.framebuffer.clear(self.background_color);
        
        // Draw border
        if self.flags.contains(WindowFlags::HAS_BORDER) {
            self.framebuffer.draw_rect(
                Rect::new(0, 0, self.rect.width, self.rect.height),
                self.border_color
            );
        }
        
        // Draw title bar
        if self.flags.contains(WindowFlags::HAS_TITLE_BAR) {
            let title_bar_color = if self.is_focused {
                self.title_bar_color
            } else {
                Color::new(128, 128, 128) // Gray for unfocused
            };
            
            self.framebuffer.fill_rect(
                Rect::new(2, 2, self.rect.width - 4, 22),
                title_bar_color
            );
            
            // Draw title text
            super::font::draw_text(
                &mut self.framebuffer,
                &self.title,
                6,
                7,
                self.title_text_color
            );
            
            // Draw window controls
            if self.flags.contains(WindowFlags::CLOSABLE) {
                self.draw_close_button();
            }
            if self.flags.contains(WindowFlags::MAXIMIZABLE) {
                self.draw_maximize_button();
            }
            if self.flags.contains(WindowFlags::MINIMIZABLE) {
                self.draw_minimize_button();
            }
        }
        
        self.is_dirty = false;
    }
    
    fn draw_close_button(&mut self) {
        let x = self.rect.width as i32 - 20;
        let y = 6;
        
        // Draw X
        self.framebuffer.draw_line(x + 4, y + 4, x + 11, y + 11, Color::WHITE);
        self.framebuffer.draw_line(x + 11, y + 4, x + 4, y + 11, Color::WHITE);
    }
    
    fn draw_maximize_button(&mut self) {
        let x = self.rect.width as i32 - 40;
        let y = 6;
        
        // Draw square
        self.framebuffer.draw_rect(
            Rect::new(x + 4, y + 4, 8, 8),
            Color::WHITE
        );
    }
    
    fn draw_minimize_button(&mut self) {
        let x = self.rect.width as i32 - 60;
        let y = 10;
        
        // Draw line
        self.framebuffer.draw_line(x + 4, y + 4, x + 11, y + 4, Color::WHITE);
    }
    
    pub fn hit_test(&self, point: Point) -> WindowHitTest {
        if !self.rect.contains_point(point) {
            return WindowHitTest::None;
        }
        
        let local_x = point.x - self.rect.x;
        let local_y = point.y - self.rect.y;
        
        // Check title bar buttons
        if self.flags.contains(WindowFlags::HAS_TITLE_BAR) && local_y < 24 {
            if self.flags.contains(WindowFlags::CLOSABLE) {
                if local_x >= self.rect.width as i32 - 24 {
                    return WindowHitTest::CloseButton;
                }
            }
            if self.flags.contains(WindowFlags::MAXIMIZABLE) {
                if local_x >= self.rect.width as i32 - 48 && 
                   local_x < self.rect.width as i32 - 24 {
                    return WindowHitTest::MaximizeButton;
                }
            }
            if self.flags.contains(WindowFlags::MINIMIZABLE) {
                if local_x >= self.rect.width as i32 - 72 && 
                   local_x < self.rect.width as i32 - 48 {
                    return WindowHitTest::MinimizeButton;
                }
            }
            
            return WindowHitTest::TitleBar;
        }
        
        // Check resize borders
        if self.flags.contains(WindowFlags::RESIZABLE) {
            let border_size = 4;
            
            if local_x < border_size {
                if local_y < border_size {
                    return WindowHitTest::TopLeftResize;
                } else if local_y >= self.rect.height as i32 - border_size {
                    return WindowHitTest::BottomLeftResize;
                } else {
                    return WindowHitTest::LeftResize;
                }
            } else if local_x >= self.rect.width as i32 - border_size {
                if local_y < border_size {
                    return WindowHitTest::TopRightResize;
                } else if local_y >= self.rect.height as i32 - border_size {
                    return WindowHitTest::BottomRightResize;
                } else {
                    return WindowHitTest::RightResize;
                }
            } else if local_y < border_size {
                return WindowHitTest::TopResize;
            } else if local_y >= self.rect.height as i32 - border_size {
                return WindowHitTest::BottomResize;
            }
        }
        
        WindowHitTest::Client
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowHitTest {
    None,
    Client,
    TitleBar,
    CloseButton,
    MinimizeButton,
    MaximizeButton,
    LeftResize,
    RightResize,
    TopResize,
    BottomResize,
    TopLeftResize,
    TopRightResize,
    BottomLeftResize,
    BottomRightResize,
}

// Window Manager
pub struct WindowManager {
    windows: Vec<Box<Window>>,
    next_window_id: WindowId,
    focused_window: Option<WindowId>,
    desktop_rect: Rect,
    z_order_counter: i32,
}

impl WindowManager {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            windows: Vec::new(),
            next_window_id: 1,
            focused_window: None,
            desktop_rect: Rect::new(0, 0, width, height),
            z_order_counter: 0,
        }
    }
    
    pub fn create_window(
        &mut self,
        title: String,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        flags: WindowFlags,
    ) -> WindowId {
        let id = self.next_window_id;
        self.next_window_id += 1;
        
        let rect = Rect::new(x, y, width, height);
        let mut window = Box::new(Window::new(id, title, rect, flags));
        
        // Set initial z-order
        self.z_order_counter += 1;
        window.z_order = self.z_order_counter;
        
        self.windows.push(window);
        
        // Auto-focus new window
        self.focus_window(id);
        
        id
    }
    
    pub fn destroy_window(&mut self, id: WindowId) {
        self.windows.retain(|w| w.id != id);
        
        if self.focused_window == Some(id) {
            // Focus next available window
            self.focused_window = self.windows.last().map(|w| w.id);
        }
    }
    
    pub fn get_window(&self, id: WindowId) -> Option<&Window> {
        self.windows.iter().find(|w| w.id == id).map(|w| w.as_ref())
    }
    
    pub fn get_window_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.id == id).map(|w| w.as_mut())
    }
    
    pub fn focus_window(&mut self, id: WindowId) {
        // Update z-order counter first
        self.z_order_counter += 1;
        let new_z_order = self.z_order_counter;
        
        // Find and update the window
        for window in &mut self.windows {
            if window.id == id {
                window.z_order = new_z_order;
                window.is_focused = true;
                window.is_dirty = true;
                break;
            }
        }
        
        // Unfocus previous window
        if let Some(prev_id) = self.focused_window {
            if prev_id != id {
                for window in &mut self.windows {
                    if window.id == prev_id {
                        window.is_focused = false;
                        window.is_dirty = true;
                        break;
                    }
                }
            }
        }
        
        self.focused_window = Some(id);
    }
    
    pub fn window_at_point(&self, point: Point) -> Option<WindowId> {
        // Find topmost window at point (highest z-order)
        self.windows
            .iter()
            .filter(|w| w.rect.contains_point(point) && w.flags.contains(WindowFlags::VISIBLE))
            .max_by_key(|w| w.z_order)
            .map(|w| w.id)
    }
    
    pub fn handle_mouse_down(&mut self, point: Point, button: MouseButton) {
        if let Some(window_id) = self.window_at_point(point) {
            self.focus_window(window_id);
            
            if let Some(window) = self.get_window(window_id) {
                let hit_test = window.hit_test(point);
                
                match hit_test {
                    WindowHitTest::CloseButton => {
                        self.destroy_window(window_id);
                    }
                    WindowHitTest::MaximizeButton => {
                        // Store desktop dimensions first
                        let desktop_width = self.desktop_rect.width;
                        let desktop_height = self.desktop_rect.height;
                        
                        // Toggle maximize
                        for window in &mut self.windows {
                            if window.id == window_id {
                                if window.state == WindowState::Maximized {
                                    window.state = WindowState::Normal;
                                    // Restore previous size
                                } else {
                                    window.state = WindowState::Maximized;
                                    window.move_to(0, 0);
                                    window.resize(desktop_width, desktop_height);
                                }
                                break;
                            }
                        }
                    }
                    WindowHitTest::MinimizeButton => {
                        if let Some(window) = self.get_window_mut(window_id) {
                            window.state = WindowState::Minimized;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    pub fn render(&mut self, framebuffer: &mut dyn FramebufferOps) {
        // Sort windows by z-order
        self.windows.sort_by_key(|w| w.z_order);
        
        // Draw each visible window
        for window in &mut self.windows {
            if !window.flags.contains(WindowFlags::VISIBLE) {
                continue;
            }
            
            if window.state == WindowState::Minimized {
                continue;
            }
            
            if window.is_dirty {
                window.paint();
            }
            
            // Blit window to screen
            framebuffer.blit(
                &window.framebuffer,
                Rect::new(0, 0, window.rect.width, window.rect.height),
                Point::new(window.rect.x, window.rect.y)
            );
        }
    }
}

// Global window manager
lazy_static! {
    pub static ref WINDOW_MANAGER: Mutex<Option<WindowManager>> = Mutex::new(None);
}

pub fn init() {
    let mut wm = WINDOW_MANAGER.lock();
    *wm = Some(WindowManager::new(800, 600)); // Default resolution
    crate::serial_println!("Window manager initialized");
}

pub fn create_window(
    title: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> WindowId {
    let mut wm = WINDOW_MANAGER.lock();
    if let Some(manager) = wm.as_mut() {
        let flags = WindowFlags::VISIBLE | 
                   WindowFlags::MOVABLE | 
                   WindowFlags::RESIZABLE | 
                   WindowFlags::CLOSABLE | 
                   WindowFlags::MINIMIZABLE | 
                   WindowFlags::MAXIMIZABLE | 
                   WindowFlags::HAS_TITLE_BAR | 
                   WindowFlags::HAS_BORDER;
        
        manager.create_window(String::from(title), x, y, width, height, flags)
    } else {
        0
    }
}