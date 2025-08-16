// Desktop Window Manager
use super::*;
use alloc::vec::Vec;
use alloc::string::String;

pub struct Desktop {
    wallpaper: Option<Wallpaper>,
    icons: Vec<DesktopIcon>,
    grid_size: (i32, i32),
    auto_arrange: bool,
    align_to_grid: bool,
    show_icons: bool,
    icon_size: IconSize,
    background_color: u32,
}

#[derive(Debug, Clone)]
pub struct Wallpaper {
    pub path: String,
    pub style: WallpaperStyle,
    pub color: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum WallpaperStyle {
    Center,
    Tile,
    Stretch,
    Fit,
    Fill,
}

#[derive(Debug, Clone)]
pub struct DesktopIcon {
    pub item: ShellItem,
    pub position: (i32, i32),
    pub selected: bool,
    pub label_rect: WindowRect,
}

#[derive(Debug, Clone, Copy)]
pub enum IconSize {
    Small,   // 16x16
    Medium,  // 32x32
    Large,   // 48x48
    ExtraLarge, // 64x64
}

impl Desktop {
    pub fn new() -> Self {
        Self {
            wallpaper: None,
            icons: Vec::new(),
            grid_size: (75, 75),
            auto_arrange: false,
            align_to_grid: true,
            show_icons: true,
            icon_size: IconSize::Medium,
            background_color: RGB(58, 110, 165), // ReactOS blue
        }
    }
    
    pub fn set_wallpaper(&mut self, path: String, style: WallpaperStyle) {
        self.wallpaper = Some(Wallpaper {
            path,
            style,
            color: self.background_color,
        });
        self.refresh();
    }
    
    pub fn add_icon(&mut self, item: ShellItem, position: (i32, i32)) {
        let adjusted_pos = if self.align_to_grid {
            self.snap_to_grid(position)
        } else {
            position
        };
        
        self.icons.push(DesktopIcon {
            item,
            position: adjusted_pos,
            selected: false,
            label_rect: WindowRect::new(
                adjusted_pos.0,
                adjusted_pos.1 + 48,
                adjusted_pos.0 + 75,
                adjusted_pos.1 + 75,
            ),
        });
        
        if self.auto_arrange {
            self.arrange_icons();
        }
    }
    
    pub fn remove_icon(&mut self, name: &str) {
        self.icons.retain(|icon| icon.item.name != name);
        if self.auto_arrange {
            self.arrange_icons();
        }
    }
    
    pub fn arrange_icons(&mut self) {
        let mut x = 20;
        let mut y = 20;
        
        for icon in &mut self.icons {
            icon.position = (x, y);
            icon.label_rect = WindowRect::new(
                x,
                y + 48,
                x + 75,
                y + 75,
            );
            
            y += self.grid_size.1;
            if y > 600 {
                y = 20;
                x += self.grid_size.0;
            }
        }
    }
    
    fn snap_to_grid(&self, position: (i32, i32)) -> (i32, i32) {
        let grid_x = (position.0 / self.grid_size.0) * self.grid_size.0;
        let grid_y = (position.1 / self.grid_size.1) * self.grid_size.1;
        (grid_x, grid_y)
    }
    
    pub fn select_icon(&mut self, position: (i32, i32)) -> Option<&DesktopIcon> {
        for icon in &mut self.icons {
            let in_bounds = position.0 >= icon.position.0 
                && position.0 <= icon.position.0 + 48
                && position.1 >= icon.position.1
                && position.1 <= icon.position.1 + 75;
                
            icon.selected = in_bounds;
            if in_bounds {
                return Some(icon);
            }
        }
        None
    }
    
    pub fn get_icon_at_position(&self, position: (i32, i32)) -> Option<&DesktopIcon> {
        for icon in &self.icons {
            let in_bounds = position.0 >= icon.position.0 
                && position.0 <= icon.position.0 + 48
                && position.1 >= icon.position.1
                && position.1 <= icon.position.1 + 75;
                
            if in_bounds {
                return Some(icon);
            }
        }
        None
    }
    
    pub fn open_icon(&self, icon: &DesktopIcon) {
        match icon.item.item_type {
            ShellItemType::Folder | ShellItemType::SpecialFolder => {
                shell_execute("explore", &icon.item.path, "", "", 1);
            }
            ShellItemType::Application => {
                shell_execute("open", &icon.item.path, "", "", 1);
            }
            ShellItemType::Shortcut => {
                // Resolve shortcut and open target
                shell_execute("open", &icon.item.path, "", "", 1);
            }
            _ => {
                shell_execute("open", &icon.item.path, "", "", 1);
            }
        }
    }
    
    pub fn refresh(&mut self) {
        crate::println!("Desktop refreshed");
        // Redraw desktop
        self.paint();
    }
    
    pub fn paint(&self) {
        // Paint wallpaper
        if let Some(ref wallpaper) = self.wallpaper {
            self.draw_wallpaper(wallpaper);
        } else {
            self.fill_background();
        }
        
        // Paint icons
        if self.show_icons {
            for icon in &self.icons {
                self.draw_icon(icon);
            }
        }
    }
    
    fn draw_wallpaper(&self, wallpaper: &Wallpaper) {
        crate::println!("Drawing wallpaper: {} ({:?})", wallpaper.path, wallpaper.style);
        // Would load and draw wallpaper image
    }
    
    fn fill_background(&self) {
        // Fill with background color
        crate::println!("Filling desktop with color: {:08x}", self.background_color);
    }
    
    fn draw_icon(&self, icon: &DesktopIcon) {
        let size = match self.icon_size {
            IconSize::Small => 16,
            IconSize::Medium => 32,
            IconSize::Large => 48,
            IconSize::ExtraLarge => 64,
        };
        
        // Draw icon image
        crate::println!("Drawing icon: {} at ({}, {}) size {}",
            icon.item.name, icon.position.0, icon.position.1, size);
        
        // Draw selection highlight if selected
        if icon.selected {
            crate::println!("  [Selected]");
        }
        
        // Draw icon label
        crate::println!("  Label: {}", icon.item.name);
    }
    
    pub fn set_icon_size(&mut self, size: IconSize) {
        self.icon_size = size;
        self.refresh();
    }
    
    pub fn toggle_auto_arrange(&mut self) {
        self.auto_arrange = !self.auto_arrange;
        if self.auto_arrange {
            self.arrange_icons();
        }
    }
    
    pub fn toggle_align_to_grid(&mut self) {
        self.align_to_grid = !self.align_to_grid;
    }
    
    pub fn context_menu(&self, position: (i32, i32)) {
        if self.get_icon_at_position(position).is_some() {
            // Icon context menu
            crate::println!("Icon context menu:");
            crate::println!("  - Open");
            crate::println!("  - Cut");
            crate::println!("  - Copy");
            crate::println!("  - Delete");
            crate::println!("  - Rename");
            crate::println!("  - Properties");
        } else {
            // Desktop context menu
            crate::println!("Desktop context menu:");
            crate::println!("  - View");
            crate::println!("  - Sort by");
            crate::println!("  - Refresh");
            crate::println!("  - Paste");
            crate::println!("  - New");
            crate::println!("  - Display settings");
            crate::println!("  - Personalize");
        }
    }
}