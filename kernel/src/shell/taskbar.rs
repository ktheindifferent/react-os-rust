// Taskbar Implementation
use super::*;
use alloc::vec::Vec;
use alloc::string::String;

pub struct Taskbar {
    position: TaskbarPosition,
    auto_hide: bool,
    always_on_top: bool,
    show_clock: bool,
    show_quick_launch: bool,
    locked: bool,
    height: i32,
    buttons: Vec<TaskbarButton>,
    start_button: StartButton,
    system_tray: SystemTray,
    quick_launch: QuickLaunch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskbarPosition {
    Bottom,
    Top,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct TaskbarButton {
    pub window: Handle,
    pub title: String,
    pub icon: Option<Handle>,
    pub flashing: bool,
    pub progress: Option<u8>,
    pub rect: WindowRect,
}

pub struct StartButton {
    rect: WindowRect,
    pressed: bool,
    hover: bool,
}

pub struct SystemTray {
    rect: WindowRect,
    icons: Vec<TrayIcon>,
    clock: Clock,
    show_hidden: bool,
}

pub struct QuickLaunch {
    rect: WindowRect,
    items: Vec<QuickLaunchItem>,
    show_text: bool,
}

#[derive(Debug, Clone)]
pub struct QuickLaunchItem {
    pub name: String,
    pub path: String,
    pub icon: Handle,
    pub rect: WindowRect,
}

pub struct Clock {
    format_24h: bool,
    show_date: bool,
    show_seconds: bool,
    rect: WindowRect,
}

impl Taskbar {
    pub fn new() -> Self {
        Self {
            position: TaskbarPosition::Bottom,
            auto_hide: false,
            always_on_top: true,
            show_clock: true,
            show_quick_launch: true,
            locked: false,
            height: 40,
            buttons: Vec::new(),
            start_button: StartButton::new(),
            system_tray: SystemTray::new(),
            quick_launch: QuickLaunch::new(),
        }
    }
    
    pub fn add_window(&mut self, window: Handle, title: String, icon: Option<Handle>) {
        let button_width = 150;
        let x = 60 + self.quick_launch.rect.width() + (self.buttons.len() as i32 * button_width);
        
        self.buttons.push(TaskbarButton {
            window,
            title,
            icon,
            flashing: false,
            progress: None,
            rect: WindowRect::new(x, 0, x + button_width, self.height),
        });
        
        self.refresh();
    }
    
    pub fn remove_window(&mut self, window: Handle) {
        self.buttons.retain(|btn| btn.window != window);
        self.recalculate_layout();
        self.refresh();
    }
    
    pub fn set_window_progress(&mut self, window: Handle, progress: u8) {
        if let Some(button) = self.buttons.iter_mut().find(|btn| btn.window == window) {
            button.progress = Some(progress.min(100));
            self.refresh();
        }
    }
    
    pub fn flash_window(&mut self, window: Handle, flash: bool) {
        if let Some(button) = self.buttons.iter_mut().find(|btn| btn.window == window) {
            button.flashing = flash;
            self.refresh();
        }
    }
    
    pub fn on_click(&mut self, x: i32, y: i32) -> TaskbarAction {
        // Check start button
        if self.start_button.rect.contains(x, y) {
            self.start_button.pressed = true;
            return TaskbarAction::StartMenu;
        }
        
        // Check taskbar buttons
        for button in &self.buttons {
            if button.rect.contains(x, y) {
                return TaskbarAction::ActivateWindow(button.window);
            }
        }
        
        // Check quick launch
        for item in &self.quick_launch.items {
            if item.rect.contains(x, y) {
                return TaskbarAction::LaunchApp(item.path.clone());
            }
        }
        
        // Check system tray
        if self.system_tray.clock.rect.contains(x, y) {
            return TaskbarAction::ShowClock;
        }
        
        for icon in &self.system_tray.icons {
            // Check if click is on tray icon
            // Would calculate icon positions
        }
        
        TaskbarAction::None
    }
    
    pub fn on_right_click(&mut self, x: i32, y: i32) {
        if self.start_button.rect.contains(x, y) {
            self.show_start_context_menu();
        } else if self.is_on_empty_area(x, y) {
            self.show_taskbar_context_menu();
        }
    }
    
    fn is_on_empty_area(&self, x: i32, y: i32) -> bool {
        // Check if click is on empty taskbar area
        for button in &self.buttons {
            if button.rect.contains(x, y) {
                return false;
            }
        }
        true
    }
    
    fn show_start_context_menu(&self) {
        crate::println!("Start button context menu:");
        crate::println!("  - Open");
        crate::println!("  - Explore");
        crate::println!("  - Search");
        crate::println!("  - Properties");
    }
    
    fn show_taskbar_context_menu(&self) {
        crate::println!("Taskbar context menu:");
        crate::println!("  - Toolbars");
        crate::println!("  - Cascade windows");
        crate::println!("  - Show windows side by side");
        crate::println!("  - Show the desktop");
        crate::println!("  - Task Manager");
        crate::println!("  - Lock the taskbar");
        crate::println!("  - Properties");
    }
    
    fn recalculate_layout(&mut self) {
        let button_width = 150;
        let start_x = 60 + self.quick_launch.rect.width();
        
        for (i, button) in self.buttons.iter_mut().enumerate() {
            let x = start_x + (i as i32 * button_width);
            button.rect = WindowRect::new(x, 0, x + button_width, self.height);
        }
    }
    
    pub fn refresh(&self) {
        crate::println!("Taskbar refreshed");
        self.paint();
    }
    
    pub fn paint(&self) {
        // Paint taskbar background
        crate::println!("Painting taskbar at {:?}", self.position);
        
        // Paint start button
        self.start_button.paint();
        
        // Paint quick launch
        if self.show_quick_launch {
            self.quick_launch.paint();
        }
        
        // Paint window buttons
        for button in &self.buttons {
            button.paint();
        }
        
        // Paint system tray
        self.system_tray.paint();
    }
    
    pub fn set_position(&mut self, position: TaskbarPosition) {
        self.position = position;
        self.recalculate_layout();
        self.refresh();
    }
    
    pub fn set_auto_hide(&mut self, auto_hide: bool) {
        self.auto_hide = auto_hide;
    }
    
    pub fn lock(&mut self, locked: bool) {
        self.locked = locked;
    }
}

impl StartButton {
    pub fn new() -> Self {
        Self {
            rect: WindowRect::new(0, 0, 60, 40),
            pressed: false,
            hover: false,
        }
    }
    
    pub fn paint(&self) {
        let state = if self.pressed {
            "pressed"
        } else if self.hover {
            "hover"
        } else {
            "normal"
        };
        crate::println!("Start button ({})", state);
    }
}

impl SystemTray {
    pub fn new() -> Self {
        Self {
            rect: WindowRect::new(900, 0, 1024, 40),
            icons: Vec::new(),
            clock: Clock::new(),
            show_hidden: false,
        }
    }
    
    pub fn add_icon(&mut self, icon: TrayIcon) {
        self.icons.push(icon);
    }
    
    pub fn remove_icon(&mut self, id: u32) {
        self.icons.retain(|icon| icon.id != id);
    }
    
    pub fn paint(&self) {
        // Paint tray icons
        for icon in &self.icons {
            if icon.visible || self.show_hidden {
                crate::println!("Tray icon: {}", icon.tooltip);
            }
        }
        
        // Paint clock
        self.clock.paint();
    }
}

impl QuickLaunch {
    pub fn new() -> Self {
        Self {
            rect: WindowRect::new(60, 0, 200, 40),
            items: Vec::new(),
            show_text: false,
        }
    }
    
    pub fn add_item(&mut self, name: String, path: String, icon: Handle) {
        let x = self.rect.left + (self.items.len() as i32 * 24);
        self.items.push(QuickLaunchItem {
            name,
            path,
            icon,
            rect: WindowRect::new(x, 8, x + 24, 32),
        });
    }
    
    pub fn paint(&self) {
        for item in &self.items {
            crate::println!("Quick launch: {}", item.name);
        }
    }
}

impl Clock {
    pub fn new() -> Self {
        Self {
            format_24h: false,
            show_date: false,
            show_seconds: false,
            rect: WindowRect::new(960, 0, 1024, 40),
        }
    }
    
    pub fn paint(&self) {
        // Get current time
        let time_str = if self.format_24h {
            "14:30"
        } else {
            "2:30 PM"
        };
        
        if self.show_date {
            crate::println!("Clock: {} 12/15/2024", time_str);
        } else {
            crate::println!("Clock: {}", time_str);
        }
    }
}

impl TaskbarButton {
    pub fn paint(&self) {
        let mut title = self.title.clone();
        if title.len() > 20 {
            title.truncate(17);
            title.push_str("...");
        }
        
        let mut status = String::new();
        if self.flashing {
            status.push_str(" [FLASH]");
        }
        if let Some(progress) = self.progress {
            status.push_str(" [");
            // Simplified progress display
            status.push_str("Progress");
            status.push_str("%]");
        }
        
        crate::println!("Button: {}{}", title, status);
    }
}

// WindowRect methods are already defined in win32::window module

#[derive(Debug)]
pub enum TaskbarAction {
    None,
    StartMenu,
    ActivateWindow(Handle),
    LaunchApp(String),
    ShowClock,
    TrayIcon(u32),
}