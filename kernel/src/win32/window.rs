// Window Manager implementation for Win32 subsystem
use super::*;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

// Window structure
#[derive(Debug, Clone)]
pub struct Window {
    pub handle: HANDLE,
    pub parent: Option<HANDLE>,
    pub owner: Option<HANDLE>,
    pub class_name: String,
    pub window_name: String,
    pub style: DWORD,
    pub ex_style: DWORD,
    pub rect: WindowRect,
    pub client_rect: WindowRect,
    pub visible: bool,
    pub enabled: bool,
    pub active: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub menu: Option<HANDLE>,
    pub instance: Option<HANDLE>,
    pub wnd_proc: Option<WindowProc>,
    pub thread_id: DWORD,
    pub process_id: DWORD,
    pub children: Vec<HANDLE>,
    pub z_order: i32,
}

// Window rectangle
#[derive(Debug, Clone, Copy)]
pub struct WindowRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl WindowRect {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { left, top, right, bottom }
    }
    
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
    
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }
}

// Window procedure type
pub type WindowProc = extern "C" fn(HANDLE, u32, usize, isize) -> isize;

// Window class structure
#[derive(Debug, Clone)]
pub struct WindowClass {
    pub name: String,
    pub style: DWORD,
    pub wnd_proc: WindowProc,
    pub class_extra: i32,
    pub window_extra: i32,
    pub instance: Option<HANDLE>,
    pub icon: Option<HANDLE>,
    pub cursor: Option<HANDLE>,
    pub background: Option<HANDLE>,
    pub menu_name: Option<String>,
}

// Window Manager
pub struct WindowManager {
    windows: BTreeMap<u64, Window>,
    classes: BTreeMap<String, WindowClass>,
    next_handle: u64,
    desktop_window: HANDLE,
    active_window: Option<HANDLE>,
    capture_window: Option<HANDLE>,
    focus_window: Option<HANDLE>,
    message_queue: Vec<Message>,
}

// Window message
#[derive(Debug, Clone)]
pub struct Message {
    pub hwnd: HANDLE,
    pub message: u32,
    pub wparam: usize,
    pub lparam: isize,
    pub time: u32,
    pub point: Point,
}

// Point structure
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

lazy_static! {
    pub static ref WINDOW_MANAGER: Mutex<WindowManager> = Mutex::new(WindowManager::new());
}

impl WindowManager {
    pub fn new() -> Self {
        let mut manager = Self {
            windows: BTreeMap::new(),
            classes: BTreeMap::new(),
            next_handle: 0x10000,
            desktop_window: Handle(0xFFFF),
            active_window: None,
            capture_window: None,
            focus_window: None,
            message_queue: Vec::new(),
        };
        
        // Create desktop window
        manager.create_desktop_window();
        
        // Register default classes
        manager.register_default_classes();
        
        manager
    }
    
    fn create_desktop_window(&mut self) {
        let desktop = Window {
            handle: self.desktop_window,
            parent: None,
            owner: None,
            class_name: String::from("#32769"), // Desktop class
            window_name: String::from("Desktop"),
            style: WS_VISIBLE | WS_CLIPCHILDREN,
            ex_style: 0,
            rect: WindowRect::new(0, 0, 1024, 768), // Default desktop size
            client_rect: WindowRect::new(0, 0, 1024, 768),
            visible: true,
            enabled: true,
            active: false,
            minimized: false,
            maximized: false,
            menu: None,
            instance: None,
            wnd_proc: None,
            thread_id: 0,
            process_id: 0,
            children: Vec::new(),
            z_order: -1,
        };
        self.windows.insert(self.desktop_window.0, desktop);
    }
    
    fn register_default_classes(&mut self) {
        // Register button class
        self.register_class(WindowClass {
            name: String::from("BUTTON"),
            style: CS_GLOBALCLASS,
            wnd_proc: default_window_proc,
            class_extra: 0,
            window_extra: 0,
            instance: None,
            icon: None,
            cursor: None,
            background: None,
            menu_name: None,
        });
        
        // Register edit class
        self.register_class(WindowClass {
            name: String::from("EDIT"),
            style: CS_GLOBALCLASS,
            wnd_proc: default_window_proc,
            class_extra: 0,
            window_extra: 0,
            instance: None,
            icon: None,
            cursor: None,
            background: None,
            menu_name: None,
        });
        
        // Register static class
        self.register_class(WindowClass {
            name: String::from("STATIC"),
            style: CS_GLOBALCLASS,
            wnd_proc: default_window_proc,
            class_extra: 0,
            window_extra: 0,
            instance: None,
            icon: None,
            cursor: None,
            background: None,
            menu_name: None,
        });
    }
    
    pub fn allocate_handle(&mut self) -> HANDLE {
        let handle = Handle(self.next_handle);
        self.next_handle += 1;
        handle
    }
    
    pub fn register_class(&mut self, class: WindowClass) -> bool {
        if self.classes.contains_key(&class.name) {
            false
        } else {
            self.classes.insert(class.name.clone(), class);
            true
        }
    }
    
    pub fn create_window(
        &mut self,
        class_name: &str,
        window_name: &str,
        style: DWORD,
        ex_style: DWORD,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Option<HANDLE>,
        menu: Option<HANDLE>,
        instance: Option<HANDLE>,
    ) -> Option<HANDLE> {
        // Find the window class and get wnd_proc
        let wnd_proc = self.classes.get(class_name)?.wnd_proc;
        
        let handle = self.allocate_handle();
        
        let window = Window {
            handle,
            parent,
            owner: None,
            class_name: String::from(class_name),
            window_name: String::from(window_name),
            style,
            ex_style,
            rect: WindowRect::new(x, y, x + width, y + height),
            client_rect: WindowRect::new(0, 0, width, height),
            visible: (style & WS_VISIBLE) != 0,
            enabled: (style & WS_DISABLED) == 0,
            active: false,
            minimized: (style & WS_MINIMIZE) != 0,
            maximized: (style & WS_MAXIMIZE) != 0,
            menu,
            instance,
            wnd_proc: Some(wnd_proc),
            thread_id: get_current_thread_id(),
            process_id: get_current_process_id(),
            children: Vec::new(),
            z_order: 0,
        };
        
        // Add to parent's children list
        if let Some(parent_handle) = parent {
            if let Some(parent_window) = self.windows.get_mut(&parent_handle.0) {
                parent_window.children.push(handle);
            }
        }
        
        self.windows.insert(handle.0, window.clone());
        
        // Send WM_CREATE message
        self.send_message(handle, WM_CREATE, 0, 0);
        
        Some(handle)
    }
    
    pub fn destroy_window(&mut self, hwnd: HANDLE) -> bool {
        // Send WM_DESTROY message
        self.send_message(hwnd, WM_DESTROY, 0, 0);
        
        // Remove from parent's children list
        if let Some(window) = self.windows.get(&hwnd.0) {
            if let Some(parent_handle) = window.parent {
                if let Some(parent) = self.windows.get_mut(&parent_handle.0) {
                    parent.children.retain(|&h| h != hwnd);
                }
            }
        }
        
        // Destroy all children
        if let Some(window) = self.windows.get(&hwnd.0) {
            let children = window.children.clone();
            for child in children {
                self.destroy_window(child);
            }
        }
        
        self.windows.remove(&hwnd.0).is_some()
    }
    
    pub fn show_window(&mut self, hwnd: HANDLE, cmd_show: i32) -> bool {
        // First update the window state
        let visible = if let Some(window) = self.windows.get_mut(&hwnd.0) {
            match cmd_show {
                SW_HIDE => window.visible = false,
                SW_SHOW | SW_SHOWNORMAL => {
                    window.visible = true;
                    window.minimized = false;
                    window.maximized = false;
                }
                SW_MINIMIZE => {
                    window.visible = true;
                    window.minimized = true;
                    window.maximized = false;
                }
                SW_MAXIMIZE => {
                    window.visible = true;
                    window.minimized = false;
                    window.maximized = true;
                }
                _ => {}
            }
            window.visible
        } else {
            return false;
        };
        
        // Then send the message
        self.send_message(hwnd, WM_SHOWWINDOW, visible as usize, 0);
        
        true
    }
    
    pub fn set_window_text(&mut self, hwnd: HANDLE, text: &str) -> bool {
        if let Some(window) = self.windows.get_mut(&hwnd.0) {
            window.window_name = String::from(text);
            
            // Send WM_SETTEXT message
            self.send_message(hwnd, WM_SETTEXT, 0, 0);
            
            true
        } else {
            false
        }
    }
    
    pub fn get_window_text(&self, hwnd: HANDLE) -> Option<String> {
        self.windows.get(&hwnd.0).map(|w| w.window_name.clone())
    }
    
    pub fn find_window(&self, class_name: Option<&str>, window_name: Option<&str>) -> Option<HANDLE> {
        for (_, window) in &self.windows {
            let class_match = class_name.map_or(true, |cn| window.class_name == cn);
            let name_match = window_name.map_or(true, |wn| window.window_name == wn);
            
            if class_match && name_match {
                return Some(window.handle);
            }
        }
        None
    }
    
    pub fn send_message(&mut self, hwnd: HANDLE, msg: u32, wparam: usize, lparam: isize) -> isize {
        // Call the window procedure directly if it exists
        if let Some(window) = self.windows.get(&hwnd.0) {
            if let Some(wnd_proc) = window.wnd_proc {
                return wnd_proc(hwnd, msg, wparam, lparam);
            }
        }
        
        // Default processing
        default_window_proc(hwnd, msg, wparam, lparam)
    }
    
    pub fn post_message(&mut self, hwnd: HANDLE, msg: u32, wparam: usize, lparam: isize) {
        let message = Message {
            hwnd,
            message: msg,
            wparam,
            lparam,
            time: 0, // TODO: Get tick count
            point: Point { x: 0, y: 0 },
        };
        self.message_queue.push(message);
    }
    
    pub fn get_message(&mut self) -> Option<Message> {
        self.message_queue.pop()
    }
    
    pub fn set_active_window(&mut self, hwnd: HANDLE) -> Option<HANDLE> {
        let old = self.active_window;
        
        // Deactivate old window
        if let Some(old_hwnd) = old {
            if let Some(window) = self.windows.get_mut(&old_hwnd.0) {
                window.active = false;
            }
            self.send_message(old_hwnd, WM_ACTIVATE, 0, 0);
        }
        
        // Activate new window
        if let Some(window) = self.windows.get_mut(&hwnd.0) {
            window.active = true;
            self.active_window = Some(hwnd);
            self.send_message(hwnd, WM_ACTIVATE, 1, 0);
        }
        
        old
    }
    
    pub fn set_focus(&mut self, hwnd: HANDLE) -> Option<HANDLE> {
        let old = self.focus_window;
        
        // Remove focus from old window
        if let Some(old_hwnd) = old {
            self.send_message(old_hwnd, WM_KILLFOCUS, 0, 0);
        }
        
        // Set focus to new window
        self.focus_window = Some(hwnd);
        self.send_message(hwnd, WM_SETFOCUS, 0, 0);
        
        old
    }
}

// Default window procedure
extern "C" fn default_window_proc(hwnd: HANDLE, msg: u32, wparam: usize, lparam: isize) -> isize {
    match msg {
        WM_CREATE => {
            crate::println!("Window {:?} created", hwnd);
            0
        }
        WM_DESTROY => {
            crate::println!("Window {:?} destroyed", hwnd);
            0
        }
        WM_PAINT => {
            crate::println!("Window {:?} paint", hwnd);
            0
        }
        WM_CLOSE => {
            crate::println!("Window {:?} close", hwnd);
            WINDOW_MANAGER.lock().destroy_window(hwnd);
            0
        }
        _ => 0,
    }
}

// Window styles
pub const WS_OVERLAPPED: DWORD = 0x00000000;
pub const WS_POPUP: DWORD = 0x80000000;
pub const WS_CHILD: DWORD = 0x40000000;
pub const WS_MINIMIZE: DWORD = 0x20000000;
pub const WS_VISIBLE: DWORD = 0x10000000;
pub const WS_DISABLED: DWORD = 0x08000000;
pub const WS_CLIPSIBLINGS: DWORD = 0x04000000;
pub const WS_CLIPCHILDREN: DWORD = 0x02000000;
pub const WS_MAXIMIZE: DWORD = 0x01000000;
pub const WS_CAPTION: DWORD = 0x00C00000;
pub const WS_BORDER: DWORD = 0x00800000;
pub const WS_DLGFRAME: DWORD = 0x00400000;
pub const WS_VSCROLL: DWORD = 0x00200000;
pub const WS_HSCROLL: DWORD = 0x00100000;
pub const WS_SYSMENU: DWORD = 0x00080000;
pub const WS_THICKFRAME: DWORD = 0x00040000;
pub const WS_GROUP: DWORD = 0x00020000;
pub const WS_TABSTOP: DWORD = 0x00010000;
pub const WS_MINIMIZEBOX: DWORD = 0x00020000;
pub const WS_MAXIMIZEBOX: DWORD = 0x00010000;

// Extended window styles
pub const WS_EX_DLGMODALFRAME: DWORD = 0x00000001;
pub const WS_EX_TOPMOST: DWORD = 0x00000008;
pub const WS_EX_ACCEPTFILES: DWORD = 0x00000010;
pub const WS_EX_TRANSPARENT: DWORD = 0x00000020;
pub const WS_EX_MDICHILD: DWORD = 0x00000040;
pub const WS_EX_TOOLWINDOW: DWORD = 0x00000080;
pub const WS_EX_WINDOWEDGE: DWORD = 0x00000100;
pub const WS_EX_CLIENTEDGE: DWORD = 0x00000200;

// Class styles
pub const CS_VREDRAW: DWORD = 0x0001;
pub const CS_HREDRAW: DWORD = 0x0002;
pub const CS_DBLCLKS: DWORD = 0x0008;
pub const CS_OWNDC: DWORD = 0x0020;
pub const CS_CLASSDC: DWORD = 0x0040;
pub const CS_PARENTDC: DWORD = 0x0080;
pub const CS_GLOBALCLASS: DWORD = 0x4000;

// Show window commands
pub const SW_HIDE: i32 = 0;
pub const SW_SHOWNORMAL: i32 = 1;
pub const SW_SHOW: i32 = 5;
pub const SW_MINIMIZE: i32 = 6;
pub const SW_MAXIMIZE: i32 = 3;

// Window messages
pub const WM_NULL: u32 = 0x0000;
pub const WM_CREATE: u32 = 0x0001;
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_MOVE: u32 = 0x0003;
pub const WM_SIZE: u32 = 0x0005;
pub const WM_ACTIVATE: u32 = 0x0006;
pub const WM_SETFOCUS: u32 = 0x0007;
pub const WM_KILLFOCUS: u32 = 0x0008;
pub const WM_ENABLE: u32 = 0x000A;
pub const WM_SETTEXT: u32 = 0x000C;
pub const WM_GETTEXT: u32 = 0x000D;
pub const WM_PAINT: u32 = 0x000F;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_QUIT: u32 = 0x0012;
pub const WM_SHOWWINDOW: u32 = 0x0018;
pub const WM_ACTIVATEAPP: u32 = 0x001C;
pub const WM_COMMAND: u32 = 0x0111;
pub const WM_SYSCOMMAND: u32 = 0x0112;
pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_KEYUP: u32 = 0x0101;
pub const WM_CHAR: u32 = 0x0102;
pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_RBUTTONUP: u32 = 0x0205;

// Window API Functions

/// CreateWindowExA - Create a window with extended style
#[no_mangle]
pub extern "C" fn CreateWindowExA(
    ex_style: DWORD,
    class_name: LPCSTR,
    window_name: LPCSTR,
    style: DWORD,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    parent: HANDLE,
    menu: HANDLE,
    instance: HANDLE,
    _param: *const u8,
) -> HANDLE {
    use core::ffi::CStr;
    
    let class = if class_name.is_null() {
        "Default"
    } else {
        match unsafe { CStr::from_ptr(class_name as *const i8) }.to_str() {
            Ok(s) => s,
            Err(_) => return Handle::NULL,
        }
    };
    
    let window = if window_name.is_null() {
        ""
    } else {
        match unsafe { CStr::from_ptr(window_name as *const i8) }.to_str() {
            Ok(s) => s,
            Err(_) => return Handle::NULL,
        }
    };
    
    let parent_opt = if parent == Handle::NULL { None } else { Some(parent) };
    let menu_opt = if menu == Handle::NULL { None } else { Some(menu) };
    let instance_opt = if instance == Handle::NULL { None } else { Some(instance) };
    
    WINDOW_MANAGER.lock().create_window(
        class,
        window,
        style,
        ex_style,
        x,
        y,
        width,
        height,
        parent_opt,
        menu_opt,
        instance_opt,
    ).unwrap_or(Handle::NULL)
}

/// DestroyWindow - Destroy a window
#[no_mangle]
pub extern "C" fn DestroyWindow(hwnd: HANDLE) -> BOOL {
    if WINDOW_MANAGER.lock().destroy_window(hwnd) {
        1
    } else {
        0
    }
}

/// RegisterClassA - Register a window class
#[no_mangle]
pub extern "C" fn RegisterClassA(wnd_class: *const WNDCLASSA) -> u16 {
    if wnd_class.is_null() {
        return 0;
    }
    
    unsafe {
        let class = &*wnd_class;
        
        use core::ffi::CStr;
        let class_name = if class.lpszClassName.is_null() {
            return 0;
        } else {
            match CStr::from_ptr(class.lpszClassName as *const i8).to_str() {
                Ok(s) => String::from(s),
                Err(_) => return 0,
            }
        };
        
        let window_class = WindowClass {
            name: class_name,
            style: class.style,
            wnd_proc: class.lpfnWndProc,
            class_extra: class.cbClsExtra,
            window_extra: class.cbWndExtra,
            instance: if class.hInstance == Handle::NULL { None } else { Some(class.hInstance) },
            icon: if class.hIcon == Handle::NULL { None } else { Some(class.hIcon) },
            cursor: if class.hCursor == Handle::NULL { None } else { Some(class.hCursor) },
            background: if class.hbrBackground == Handle::NULL { None } else { Some(class.hbrBackground) },
            menu_name: None,
        };
        
        if WINDOW_MANAGER.lock().register_class(window_class) {
            1
        } else {
            0
        }
    }
}

// WNDCLASSA structure
#[repr(C)]
pub struct WNDCLASSA {
    pub style: DWORD,
    pub lpfnWndProc: WindowProc,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HANDLE,
    pub hIcon: HANDLE,
    pub hCursor: HANDLE,
    pub hbrBackground: HANDLE,
    pub lpszMenuName: LPCSTR,
    pub lpszClassName: LPCSTR,
}

/// SendMessageA - Send a message to a window
#[no_mangle]
pub extern "C" fn SendMessageA(
    hwnd: HANDLE,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    WINDOW_MANAGER.lock().send_message(hwnd, msg, wparam, lparam)
}

/// PostMessageA - Post a message to a window
#[no_mangle]
pub extern "C" fn PostMessageA(
    hwnd: HANDLE,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> BOOL {
    WINDOW_MANAGER.lock().post_message(hwnd, msg, wparam, lparam);
    1
}

/// SetActiveWindow - Set the active window
#[no_mangle]
pub extern "C" fn SetActiveWindow(hwnd: HANDLE) -> HANDLE {
    WINDOW_MANAGER.lock().set_active_window(hwnd).unwrap_or(Handle::NULL)
}

/// SetFocus - Set keyboard focus
#[no_mangle]
pub extern "C" fn SetFocus(hwnd: HANDLE) -> HANDLE {
    WINDOW_MANAGER.lock().set_focus(hwnd).unwrap_or(Handle::NULL)
}

// Helper functions to get current thread and process IDs
fn get_current_thread_id() -> DWORD {
    use crate::process::thread::THREAD_MANAGER;
    
    if let Some(thread_id) = THREAD_MANAGER.lock().get_current_thread() {
        thread_id.0
    } else {
        // Default to thread ID 1 if no current thread
        1
    }
}

fn get_current_process_id() -> DWORD {
    use crate::process::PROCESS_MANAGER;
    
    if let Some(process_id) = PROCESS_MANAGER.lock().current_process {
        process_id.0
    } else {
        // Default to process ID 1 if no current process
        1
    }
}