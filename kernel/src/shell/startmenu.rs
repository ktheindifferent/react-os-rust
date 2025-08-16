// Start Menu Implementation
use super::*;
use alloc::vec::Vec;
use alloc::string::String;

pub struct StartMenu {
    items: Vec<StartMenuItem>,
    recent_programs: Vec<StartMenuItem>,
    pinned_items: Vec<StartMenuItem>,
    user_name: String,
    user_picture: Option<String>,
    search_box: SearchBox,
    show_recent: bool,
    show_run_command: bool,
}

#[derive(Debug, Clone)]
pub struct StartMenuItem {
    pub name: String,
    pub path: String,
    pub icon: Option<Handle>,
    pub item_type: StartMenuItemType,
    pub children: Vec<StartMenuItem>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StartMenuItemType {
    Program,
    Folder,
    Separator,
    SpecialFolder,
    ControlPanel,
    Settings,
    Search,
    Run,
    Shutdown,
}

pub struct SearchBox {
    query: String,
    results: Vec<SearchResult>,
    active: bool,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub path: String,
    pub category: SearchCategory,
    pub relevance: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchCategory {
    Programs,
    Documents,
    Settings,
    Files,
    Web,
}

impl StartMenu {
    pub fn new() -> Self {
        let mut menu = Self {
            items: Vec::new(),
            recent_programs: Vec::new(),
            pinned_items: Vec::new(),
            user_name: String::from("User"),
            user_picture: None,
            search_box: SearchBox::new(),
            show_recent: true,
            show_run_command: true,
        };
        
        menu.initialize_default_items();
        menu
    }
    
    fn initialize_default_items(&mut self) {
        // All Programs
        self.items.push(StartMenuItem {
            name: String::from("All Programs"),
            path: String::from(""),
            icon: None,
            item_type: StartMenuItemType::Folder,
            children: self.load_all_programs(),
        });
        
        // Separator
        self.items.push(StartMenuItem {
            name: String::new(),
            path: String::new(),
            icon: None,
            item_type: StartMenuItemType::Separator,
            children: Vec::new(),
        });
        
        // Documents
        self.items.push(StartMenuItem {
            name: String::from("Documents"),
            path: String::from("C:\\Users\\Default\\Documents"),
            icon: None,
            item_type: StartMenuItemType::SpecialFolder,
            children: Vec::new(),
        });
        
        // Pictures
        self.items.push(StartMenuItem {
            name: String::from("Pictures"),
            path: String::from("C:\\Users\\Default\\Pictures"),
            icon: None,
            item_type: StartMenuItemType::SpecialFolder,
            children: Vec::new(),
        });
        
        // Music
        self.items.push(StartMenuItem {
            name: String::from("Music"),
            path: String::from("C:\\Users\\Default\\Music"),
            icon: None,
            item_type: StartMenuItemType::SpecialFolder,
            children: Vec::new(),
        });
        
        // Computer
        self.items.push(StartMenuItem {
            name: String::from("Computer"),
            path: String::from("MyComputer"),
            icon: None,
            item_type: StartMenuItemType::SpecialFolder,
            children: Vec::new(),
        });
        
        // Control Panel
        self.items.push(StartMenuItem {
            name: String::from("Control Panel"),
            path: String::from("ControlPanel"),
            icon: None,
            item_type: StartMenuItemType::ControlPanel,
            children: Vec::new(),
        });
        
        // Separator
        self.items.push(StartMenuItem {
            name: String::new(),
            path: String::new(),
            icon: None,
            item_type: StartMenuItemType::Separator,
            children: Vec::new(),
        });
        
        // Search
        self.items.push(StartMenuItem {
            name: String::from("Search"),
            path: String::from("Search"),
            icon: None,
            item_type: StartMenuItemType::Search,
            children: Vec::new(),
        });
        
        // Run
        if self.show_run_command {
            self.items.push(StartMenuItem {
                name: String::from("Run..."),
                path: String::from("Run"),
                icon: None,
                item_type: StartMenuItemType::Run,
                children: Vec::new(),
            });
        }
        
        // Separator
        self.items.push(StartMenuItem {
            name: String::new(),
            path: String::new(),
            icon: None,
            item_type: StartMenuItemType::Separator,
            children: Vec::new(),
        });
        
        // Shut Down
        self.items.push(StartMenuItem {
            name: String::from("Shut Down"),
            path: String::from("Shutdown"),
            icon: None,
            item_type: StartMenuItemType::Shutdown,
            children: Vec::new(),
        });
        
        // Load pinned items
        self.load_pinned_items();
        
        // Load recent programs
        self.load_recent_programs();
    }
    
    fn load_all_programs(&self) -> Vec<StartMenuItem> {
        let mut programs = Vec::new();
        
        // Accessories
        let mut accessories = StartMenuItem {
            name: String::from("Accessories"),
            path: String::from(""),
            icon: None,
            item_type: StartMenuItemType::Folder,
            children: Vec::new(),
        };
        
        accessories.children.push(StartMenuItem {
            name: String::from("Calculator"),
            path: String::from("C:\\Windows\\System32\\calc.exe"),
            icon: None,
            item_type: StartMenuItemType::Program,
            children: Vec::new(),
        });
        
        accessories.children.push(StartMenuItem {
            name: String::from("Notepad"),
            path: String::from("C:\\Windows\\System32\\notepad.exe"),
            icon: None,
            item_type: StartMenuItemType::Program,
            children: Vec::new(),
        });
        
        accessories.children.push(StartMenuItem {
            name: String::from("Paint"),
            path: String::from("C:\\Windows\\System32\\mspaint.exe"),
            icon: None,
            item_type: StartMenuItemType::Program,
            children: Vec::new(),
        });
        
        accessories.children.push(StartMenuItem {
            name: String::from("Command Prompt"),
            path: String::from("C:\\Windows\\System32\\cmd.exe"),
            icon: None,
            item_type: StartMenuItemType::Program,
            children: Vec::new(),
        });
        
        programs.push(accessories);
        
        // Internet
        programs.push(StartMenuItem {
            name: String::from("Internet Explorer"),
            path: String::from("C:\\Program Files\\Internet Explorer\\iexplore.exe"),
            icon: None,
            item_type: StartMenuItemType::Program,
            children: Vec::new(),
        });
        
        // Games
        let mut games = StartMenuItem {
            name: String::from("Games"),
            path: String::from(""),
            icon: None,
            item_type: StartMenuItemType::Folder,
            children: Vec::new(),
        };
        
        games.children.push(StartMenuItem {
            name: String::from("Solitaire"),
            path: String::from("C:\\Program Files\\Games\\Solitaire.exe"),
            icon: None,
            item_type: StartMenuItemType::Program,
            children: Vec::new(),
        });
        
        games.children.push(StartMenuItem {
            name: String::from("Minesweeper"),
            path: String::from("C:\\Program Files\\Games\\Minesweeper.exe"),
            icon: None,
            item_type: StartMenuItemType::Program,
            children: Vec::new(),
        });
        
        programs.push(games);
        
        programs
    }
    
    fn load_pinned_items(&mut self) {
        // Load frequently used items
        self.pinned_items.push(StartMenuItem {
            name: String::from("Internet Explorer"),
            path: String::from("C:\\Program Files\\Internet Explorer\\iexplore.exe"),
            icon: None,
            item_type: StartMenuItemType::Program,
            children: Vec::new(),
        });
        
        self.pinned_items.push(StartMenuItem {
            name: String::from("Notepad"),
            path: String::from("C:\\Windows\\System32\\notepad.exe"),
            icon: None,
            item_type: StartMenuItemType::Program,
            children: Vec::new(),
        });
    }
    
    fn load_recent_programs(&mut self) {
        // Load recently used programs
        if self.show_recent {
            self.recent_programs.push(StartMenuItem {
                name: String::from("Calculator"),
                path: String::from("C:\\Windows\\System32\\calc.exe"),
                icon: None,
                item_type: StartMenuItemType::Program,
                children: Vec::new(),
            });
        }
    }
    
    pub fn search(&mut self, query: &str) {
        self.search_box.search(query);
    }
    
    pub fn execute_item(&self, item: &StartMenuItem) {
        match item.item_type {
            StartMenuItemType::Program => {
                shell_execute("open", &item.path, "", "", 1);
            }
            StartMenuItemType::SpecialFolder => {
                shell_execute("explore", &item.path, "", "", 1);
            }
            StartMenuItemType::ControlPanel => {
                self.open_control_panel();
            }
            StartMenuItemType::Search => {
                self.open_search();
            }
            StartMenuItemType::Run => {
                self.open_run_dialog();
            }
            StartMenuItemType::Shutdown => {
                self.show_shutdown_dialog();
            }
            _ => {}
        }
    }
    
    fn open_control_panel(&self) {
        crate::println!("Opening Control Panel");
        shell_execute("open", "control.exe", "", "", 1);
    }
    
    fn open_search(&self) {
        crate::println!("Opening Search");
    }
    
    fn open_run_dialog(&self) {
        crate::println!("Opening Run dialog");
    }
    
    fn show_shutdown_dialog(&self) {
        crate::println!("Shutdown options:");
        crate::println!("  - Shut Down");
        crate::println!("  - Restart");
        crate::println!("  - Sleep");
        crate::println!("  - Hibernate");
        crate::println!("  - Log Off");
        crate::println!("  - Switch User");
    }
    
    pub fn paint(&self) {
        crate::println!("╔══════════════════════════════╗");
        crate::println!("║ {} {:>20} ║", self.user_name, "");
        crate::println!("╠══════════════════════════════╣");
        
        // Paint pinned items
        if !self.pinned_items.is_empty() {
            for item in &self.pinned_items {
                crate::println!("║ ★ {:<26} ║", item.name);
            }
            crate::println!("╠──────────────────────────────╣");
        }
        
        // Paint recent programs
        if self.show_recent && !self.recent_programs.is_empty() {
            for item in &self.recent_programs {
                crate::println!("║   {:<27} ║", item.name);
            }
            crate::println!("╠──────────────────────────────╣");
        }
        
        // Paint menu items
        for item in &self.items {
            match item.item_type {
                StartMenuItemType::Separator => {
                    crate::println!("╠──────────────────────────────╣");
                }
                _ => {
                    let arrow = if !item.children.is_empty() { "▶" } else { " " };
                    crate::println!("║ {:<26} {} ║", item.name, arrow);
                }
            }
        }
        
        crate::println!("╚══════════════════════════════╝");
    }
}

impl SearchBox {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            active: false,
        }
    }
    
    pub fn search(&mut self, query: &str) {
        self.query = String::from(query);
        self.results.clear();
        
        if query.is_empty() {
            return;
        }
        
        // Search for programs
        self.search_programs(query);
        
        // Search for documents
        self.search_documents(query);
        
        // Search for settings
        self.search_settings(query);
        
        // Sort by relevance
        self.results.sort_by(|a, b| b.relevance.cmp(&a.relevance));
    }
    
    fn search_programs(&mut self, query: &str) {
        let mut programs = Vec::new();
        programs.push(("Notepad", "C:\\Windows\\System32\\notepad.exe"));
        programs.push(("Calculator", "C:\\Windows\\System32\\calc.exe"));
        programs.push(("Paint", "C:\\Windows\\System32\\mspaint.exe"));
        programs.push(("Command Prompt", "C:\\Windows\\System32\\cmd.exe"));
        
        for (name, path) in programs {
            if name.to_lowercase().contains(&query.to_lowercase()) {
                self.results.push(SearchResult {
                    name: String::from(name),
                    path: String::from(path),
                    category: SearchCategory::Programs,
                    relevance: 100,
                });
            }
        }
    }
    
    fn search_documents(&mut self, query: &str) {
        // Search for documents matching query
        crate::println!("Searching documents for: {}", query);
    }
    
    fn search_settings(&mut self, query: &str) {
        let mut settings = Vec::new();
        settings.push(("Display Settings", "control.exe desk.cpl"));
        settings.push(("Network Settings", "control.exe ncpa.cpl"));
        settings.push(("System Properties", "control.exe sysdm.cpl"));
        
        for (name, path) in settings {
            if name.to_lowercase().contains(&query.to_lowercase()) {
                self.results.push(SearchResult {
                    name: String::from(name),
                    path: String::from(path),
                    category: SearchCategory::Settings,
                    relevance: 80,
                });
            }
        }
    }
}