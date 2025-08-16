// File Explorer Implementation
use super::*;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::VecDeque;

pub struct Explorer {
    current_path: String,
    navigation_history: NavigationHistory,
    view_mode: ViewMode,
    sort_by: SortBy,
    show_hidden: bool,
    show_extensions: bool,
    selection: Vec<usize>,
    items: Vec<ShellItem>,
    address_bar: String,
    search_query: Option<String>,
    folder_tree: FolderTree,
}

pub struct NavigationHistory {
    back_stack: VecDeque<String>,
    forward_stack: VecDeque<String>,
    max_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Icons,
    List,
    Details,
    Tiles,
    Thumbnails,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortBy {
    Name,
    Size,
    Type,
    Modified,
}

pub struct FolderTree {
    root: TreeNode,
    expanded: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub children: Vec<TreeNode>,
    pub is_expanded: bool,
}

impl Explorer {
    pub fn new() -> Self {
        Self {
            current_path: String::from("C:\\"),
            navigation_history: NavigationHistory::new(),
            view_mode: ViewMode::Icons,
            sort_by: SortBy::Name,
            show_hidden: false,
            show_extensions: true,
            selection: Vec::new(),
            items: Vec::new(),
            address_bar: String::from("C:\\"),
            search_query: None,
            folder_tree: FolderTree::new(),
        }
    }
    
    pub fn navigate_to(&mut self, path: &str) {
        // Save current path to history
        self.navigation_history.push_back(self.current_path.clone());
        
        self.current_path = String::from(path);
        self.address_bar = self.current_path.clone();
        self.selection.clear();
        
        // Load items for new path
        self.load_items();
        
        crate::println!("Navigated to: {}", path);
    }
    
    pub fn navigate_back(&mut self) -> bool {
        if let Some(path) = self.navigation_history.go_back(&self.current_path) {
            self.current_path = path.clone();
            self.address_bar = path;
            self.load_items();
            true
        } else {
            false
        }
    }
    
    pub fn navigate_forward(&mut self) -> bool {
        if let Some(path) = self.navigation_history.go_forward(&self.current_path) {
            self.current_path = path.clone();
            self.address_bar = path;
            self.load_items();
            true
        } else {
            false
        }
    }
    
    pub fn navigate_up(&mut self) {
        if let Some(parent) = self.get_parent_path(&self.current_path) {
            self.navigate_to(&parent);
        }
    }
    
    fn get_parent_path(&self, path: &str) -> Option<String> {
        if path == "C:\\" || path.len() <= 3 {
            return None;
        }
        
        if let Some(pos) = path.rfind('\\') {
            if pos > 2 {
                Some(String::from(&path[..pos]))
            } else {
                Some(String::from("C:\\"))
            }
        } else {
            None
        }
    }
    
    fn load_items(&mut self) {
        self.items.clear();
        
        // Simulate loading directory contents
        if self.current_path == "C:\\" {
            self.items.push(ShellItem {
                name: String::from("Windows"),
                path: String::from("C:\\Windows"),
                icon_index: 3,
                item_type: ShellItemType::Folder,
                attributes: ShellItemAttributes {
                    directory: true,
                    system: true,
                    ..Default::default()
                },
                size: 0,
                modified: 0,
            });
            
            self.items.push(ShellItem {
                name: String::from("Program Files"),
                path: String::from("C:\\Program Files"),
                icon_index: 3,
                item_type: ShellItemType::Folder,
                attributes: ShellItemAttributes {
                    directory: true,
                    ..Default::default()
                },
                size: 0,
                modified: 0,
            });
            
            self.items.push(ShellItem {
                name: String::from("Users"),
                path: String::from("C:\\Users"),
                icon_index: 3,
                item_type: ShellItemType::Folder,
                attributes: ShellItemAttributes {
                    directory: true,
                    ..Default::default()
                },
                size: 0,
                modified: 0,
            });
            
            self.items.push(ShellItem {
                name: String::from("autoexec.bat"),
                path: String::from("C:\\autoexec.bat"),
                icon_index: 71,
                item_type: ShellItemType::File,
                attributes: ShellItemAttributes {
                    hidden: true,
                    system: true,
                    ..Default::default()
                },
                size: 256,
                modified: 0,
            });
        }
        
        // Sort items
        self.sort_items();
    }
    
    fn sort_items(&mut self) {
        match self.sort_by {
            SortBy::Name => {
                self.items.sort_by(|a, b| a.name.cmp(&b.name));
            }
            SortBy::Size => {
                self.items.sort_by(|a, b| a.size.cmp(&b.size));
            }
            SortBy::Type => {
                self.items.sort_by(|a, b| {
                    let a_ext = get_extension(&a.name);
                    let b_ext = get_extension(&b.name);
                    a_ext.cmp(&b_ext)
                });
            }
            SortBy::Modified => {
                self.items.sort_by(|a, b| a.modified.cmp(&b.modified));
            }
        }
        
        // Folders first
        self.items.sort_by(|a, b| {
            let a_is_dir = a.attributes.directory;
            let b_is_dir = b.attributes.directory;
            b_is_dir.cmp(&a_is_dir)
        });
    }
    
    fn get_extension<'a>(&self, name: &'a str) -> &'a str {
        get_extension(name)
    }
    
    pub fn select_item(&mut self, index: usize) {
        if index < self.items.len() {
            self.selection.clear();
            self.selection.push(index);
        }
    }
    
    pub fn select_all(&mut self) {
        self.selection = (0..self.items.len()).collect();
    }
    
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }
    
    pub fn open_selected(&mut self) {
        let selection_copy = self.selection.clone();
        for index in selection_copy {
            if let Some(item) = self.items.get(index) {
                if item.attributes.directory {
                    let path = item.path.clone();
                    self.navigate_to(&path);
                    break;
                } else {
                    shell_execute("open", &item.path, "", "", 1);
                }
            }
        }
    }
    
    pub fn delete_selected(&mut self) {
        let selection_copy = self.selection.clone();
        for index in selection_copy {
            if let Some(item) = self.items.get(index) {
                crate::println!("Delete: {}", item.path);
                // Would delete file/folder
            }
        }
        self.load_items();
    }
    
    pub fn copy_selected(&mut self) {
        let selection_copy = self.selection.clone();
        for index in selection_copy {
            if let Some(item) = self.items.get(index) {
                crate::println!("Copy: {}", item.path);
                // Would copy to clipboard
            }
        }
    }
    
    pub fn paste(&mut self) {
        crate::println!("Paste to: {}", self.current_path);
        // Would paste from clipboard
        self.load_items();
    }
    
    pub fn new_folder(&mut self, name: &str) {
        let mut path = self.current_path.clone();
        path.push_str("\\");
        path.push_str(name);
        crate::println!("Create folder: {}", path);
        // Would create folder
        self.load_items();
    }
    
    pub fn rename_selected(&mut self, new_name: &str) {
        if let Some(&index) = self.selection.first() {
            let old_path = if let Some(item) = self.items.get(index) {
                item.path.clone()
            } else {
                return;
            };
            
            let parent = self.get_parent_path(&old_path).unwrap_or_default();
            let mut new_path = parent;
            new_path.push_str("\\");
            new_path.push_str(new_name);
            
            if let Some(item) = self.items.get_mut(index) {
                item.path = new_path.clone();
                item.name = String::from(new_name);
                crate::println!("Rename: {} -> {}", old_path, new_path);
            }
        }
    }
    
    pub fn search(&mut self, query: &str) {
        self.search_query = Some(String::from(query));
        crate::println!("Searching for: {}", query);
        // Would perform search
    }
    
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
        self.refresh();
    }
    
    pub fn set_sort_by(&mut self, sort: SortBy) {
        self.sort_by = sort;
        self.sort_items();
        self.refresh();
    }
    
    pub fn toggle_hidden_files(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.load_items();
    }
    
    pub fn refresh(&mut self) {
        self.load_items();
        crate::println!("Explorer refreshed");
    }
    
    pub fn paint(&self) {
        crate::println!("Explorer - {}", self.current_path);
        crate::println!("View: {:?}, Sort: {:?}", self.view_mode, self.sort_by);
        
        match self.view_mode {
            ViewMode::Details => self.paint_details_view(),
            ViewMode::List => self.paint_list_view(),
            _ => self.paint_icons_view(),
        }
    }
    
    fn paint_details_view(&self) {
        crate::println!("Name                    Size        Type        Modified");
        crate::println!("---------------------------------------------------------");
        
        for (i, item) in self.items.iter().enumerate() {
            let selected = if self.selection.contains(&i) { "*" } else { " " };
            let size = if item.attributes.directory {
                String::from("<DIR>")
            } else {
                {
                    let mut s = String::new();
                    // Simplified number to string conversion
                    s.push_str("Size");
                    s
                }
            };
            
            crate::println!("{}{:<20} {:>10}  {:<10}  {}",
                selected,
                item.name,
                size,
                if item.attributes.directory { "Folder" } else { "File" },
                "12/15/2024"
            );
        }
    }
    
    fn paint_list_view(&self) {
        for (i, item) in self.items.iter().enumerate() {
            let selected = if self.selection.contains(&i) { "*" } else { " " };
            let icon = if item.attributes.directory { "[D]" } else { "[F]" };
            crate::println!("{}{} {}", selected, icon, item.name);
        }
    }
    
    fn paint_icons_view(&self) {
        let mut row = Vec::new();
        let items_per_row = 5;
        
        for (i, item) in self.items.iter().enumerate() {
            let selected = if self.selection.contains(&i) { "*" } else { " " };
            let icon = if item.attributes.directory { "[D]" } else { "[F]" };
            let mut item_str = String::new();
            item_str.push_str(selected);
            item_str.push_str(icon);
            item_str.push_str(&item.name);
            row.push(item_str);
            
            if row.len() >= items_per_row {
                crate::println!("{}", row.join("  "));
                row.clear();
            }
        }
        
        if !row.is_empty() {
            crate::println!("{}", row.join("  "));
        }
    }
}

impl NavigationHistory {
    pub fn new() -> Self {
        Self {
            back_stack: VecDeque::new(),
            forward_stack: VecDeque::new(),
            max_size: 50,
        }
    }
    
    pub fn push_back(&mut self, path: String) {
        self.back_stack.push_back(path);
        if self.back_stack.len() > self.max_size {
            self.back_stack.pop_front();
        }
        self.forward_stack.clear();
    }
    
    pub fn go_back(&mut self, current: &str) -> Option<String> {
        if let Some(path) = self.back_stack.pop_back() {
            self.forward_stack.push_front(String::from(current));
            Some(path)
        } else {
            None
        }
    }
    
    pub fn go_forward(&mut self, current: &str) -> Option<String> {
        if let Some(path) = self.forward_stack.pop_front() {
            self.back_stack.push_back(String::from(current));
            Some(path)
        } else {
            None
        }
    }
}

impl FolderTree {
    pub fn new() -> Self {
        let root = TreeNode {
            name: String::from("Computer"),
            path: String::from(""),
            children: {
                let mut children = Vec::new();
                children.push(TreeNode {
                    name: String::from("C:"),
                    path: String::from("C:\\"),
                    children: Vec::new(),
                    is_expanded: false,
                });
                children
            },
            is_expanded: true,
        };
        
        Self {
            root,
            expanded: Vec::new(),
        }
    }
    
    pub fn expand(&mut self, path: &str) {
        if !self.expanded.contains(&String::from(path)) {
            self.expanded.push(String::from(path));
            // Load children for the path
        }
    }
    
    pub fn collapse(&mut self, path: &str) {
        self.expanded.retain(|p| p != path);
    }
}

fn get_extension(name: &str) -> &str {
    if let Some(pos) = name.rfind('.') {
        &name[pos + 1..]
    } else {
        ""
    }
}