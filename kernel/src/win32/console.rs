// Console Subsystem implementation for Win32
use super::*;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::{VecDeque, BTreeMap};
use spin::Mutex;
use lazy_static::lazy_static;
use core::fmt::Write;

// Console structure
#[derive(Debug, Clone)]
pub struct Console {
    pub handle: HANDLE,
    pub input_handle: HANDLE,
    pub output_handle: HANDLE,
    pub error_handle: HANDLE,
    pub screen_buffer: ScreenBuffer,
    pub input_buffer: VecDeque<InputRecord>,
    pub input_mode: DWORD,
    pub output_mode: DWORD,
    pub title: String,
    pub cursor_info: ConsoleCursorInfo,
    pub window_info: ConsoleWindowInfo,
    pub process_list: Vec<DWORD>,
}

// Screen buffer for console output
#[derive(Debug, Clone)]
pub struct ScreenBuffer {
    pub width: u16,
    pub height: u16,
    pub cursor_x: u16,
    pub cursor_y: u16,
    pub attributes: u16,
    pub buffer: Vec<CharInfo>,
    pub active: bool,
}

// Character information in screen buffer
#[derive(Debug, Clone, Copy)]
pub struct CharInfo {
    pub char: u16,
    pub attributes: u16,
}

impl Default for CharInfo {
    fn default() -> Self {
        Self {
            char: b' ' as u16,
            attributes: FOREGROUND_WHITE,
        }
    }
}

// Console cursor information
#[derive(Debug, Clone, Copy)]
pub struct ConsoleCursorInfo {
    pub size: DWORD,
    pub visible: bool,
}

// Console window information
#[derive(Debug, Clone, Copy)]
pub struct ConsoleWindowInfo {
    pub left: i16,
    pub top: i16,
    pub right: i16,
    pub bottom: i16,
}

// Input record for console input
#[derive(Debug, Clone)]
pub enum InputRecord {
    KeyEvent(KeyEventRecord),
    MouseEvent(MouseEventRecord),
    WindowBufferSizeEvent(WindowBufferSizeRecord),
    MenuEvent(MenuEventRecord),
    FocusEvent(FocusEventRecord),
}

// Key event record
#[derive(Debug, Clone)]
pub struct KeyEventRecord {
    pub key_down: bool,
    pub repeat_count: u16,
    pub virtual_key_code: u16,
    pub virtual_scan_code: u16,
    pub unicode_char: u16,
    pub control_key_state: DWORD,
}

// Mouse event record
#[derive(Debug, Clone)]
pub struct MouseEventRecord {
    pub mouse_position: Coord,
    pub button_state: DWORD,
    pub control_key_state: DWORD,
    pub event_flags: DWORD,
}

// Window buffer size record
#[derive(Debug, Clone)]
pub struct WindowBufferSizeRecord {
    pub size: Coord,
}

// Menu event record
#[derive(Debug, Clone)]
pub struct MenuEventRecord {
    pub command_id: u32,
}

// Focus event record
#[derive(Debug, Clone)]
pub struct FocusEventRecord {
    pub set_focus: bool,
}

// Coordinate structure
#[derive(Debug, Clone, Copy)]
pub struct Coord {
    pub x: i16,
    pub y: i16,
}

// Console Manager
pub struct ConsoleManager {
    consoles: BTreeMap<u64, Console>,
    next_handle: u64,
    active_console: Option<HANDLE>,
}

lazy_static! {
    pub static ref CONSOLE_MANAGER: Mutex<ConsoleManager> = Mutex::new(ConsoleManager::new());
}

impl ConsoleManager {
    pub fn new() -> Self {
        let mut manager = Self {
            consoles: BTreeMap::new(),
            next_handle: 0x20000,
            active_console: None,
        };
        
        // Create default console
        manager.create_default_console();
        
        manager
    }
    
    fn create_default_console(&mut self) {
        let handle = self.allocate_handle();
        let input_handle = self.allocate_handle();
        let output_handle = self.allocate_handle();
        let error_handle = self.allocate_handle();
        
        let mut buffer = Vec::with_capacity((80 * 25) as usize);
        buffer.resize((80 * 25) as usize, CharInfo::default());
        
        let console = Console {
            handle,
            input_handle,
            output_handle,
            error_handle,
            screen_buffer: ScreenBuffer {
                width: 80,
                height: 25,
                cursor_x: 0,
                cursor_y: 0,
                attributes: FOREGROUND_WHITE,
                buffer,
                active: true,
            },
            input_buffer: VecDeque::new(),
            input_mode: ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT,
            output_mode: ENABLE_PROCESSED_OUTPUT | ENABLE_WRAP_AT_EOL_OUTPUT,
            title: String::from("ReactOS Console"),
            cursor_info: ConsoleCursorInfo {
                size: 25,
                visible: true,
            },
            window_info: ConsoleWindowInfo {
                left: 0,
                top: 0,
                right: 79,
                bottom: 24,
            },
            process_list: {
                let mut list = Vec::new();
                list.push(1); // Default process ID
                list
            },
        };
        
        self.consoles.insert(handle.0, console);
        self.active_console = Some(handle);
    }
    
    pub fn allocate_handle(&mut self) -> HANDLE {
        let handle = Handle(self.next_handle);
        self.next_handle += 1;
        handle
    }
    
    pub fn get_std_handle(&self, std_handle: i32) -> HANDLE {
        if let Some(console_handle) = self.active_console {
            if let Some(console) = self.consoles.get(&console_handle.0) {
                match std_handle {
                    STD_INPUT_HANDLE => return console.input_handle,
                    STD_OUTPUT_HANDLE => return console.output_handle,
                    STD_ERROR_HANDLE => return console.error_handle,
                    _ => {}
                }
            }
        }
        Handle::INVALID
    }
    
    pub fn alloc_console(&mut self) -> bool {
        if self.active_console.is_some() {
            return false; // Process already has a console
        }
        
        self.create_default_console();
        true
    }
    
    pub fn free_console(&mut self) -> bool {
        if let Some(handle) = self.active_console {
            self.consoles.remove(&handle.0);
            self.active_console = None;
            true
        } else {
            false
        }
    }
    
    pub fn write_console(&mut self, handle: HANDLE, data: &[u8]) -> Option<u32> {
        // Find the console that owns this handle and extract the screen buffer
        let console_key = self.consoles.iter()
            .find(|(_, console)| console.output_handle == handle || console.error_handle == handle)
            .map(|(key, _)| *key)?;
        
        // Now get the console mutably and write to it
        if let Some(console) = self.consoles.get_mut(&console_key) {
            return Some(Self::write_to_screen_buffer(&mut console.screen_buffer, data));
        }
        None
    }
    
    fn write_to_screen_buffer(buffer: &mut ScreenBuffer, data: &[u8]) -> u32 {
        let mut written = 0;
        
        for &byte in data {
            match byte {
                b'\n' => {
                    buffer.cursor_y += 1;
                    buffer.cursor_x = 0;
                    if buffer.cursor_y >= buffer.height {
                        Self::scroll_screen_buffer(buffer);
                        buffer.cursor_y = buffer.height - 1;
                    }
                }
                b'\r' => {
                    buffer.cursor_x = 0;
                }
                b'\t' => {
                    let tab_size = 8 - (buffer.cursor_x % 8);
                    for _ in 0..tab_size {
                        if buffer.cursor_x < buffer.width {
                            let index = (buffer.cursor_y * buffer.width + buffer.cursor_x) as usize;
                            if index < buffer.buffer.len() {
                                buffer.buffer[index] = CharInfo {
                                    char: b' ' as u16,
                                    attributes: buffer.attributes,
                                };
                            }
                            buffer.cursor_x += 1;
                        }
                    }
                }
                b'\x08' => { // Backspace
                    if buffer.cursor_x > 0 {
                        buffer.cursor_x -= 1;
                        let index = (buffer.cursor_y * buffer.width + buffer.cursor_x) as usize;
                        if index < buffer.buffer.len() {
                            buffer.buffer[index] = CharInfo {
                                char: b' ' as u16,
                                attributes: buffer.attributes,
                            };
                        }
                    }
                }
                _ => {
                    if buffer.cursor_x < buffer.width {
                        let index = (buffer.cursor_y * buffer.width + buffer.cursor_x) as usize;
                        if index < buffer.buffer.len() {
                            buffer.buffer[index] = CharInfo {
                                char: byte as u16,
                                attributes: buffer.attributes,
                            };
                        }
                        buffer.cursor_x += 1;
                        
                        if buffer.cursor_x >= buffer.width {
                            buffer.cursor_x = 0;
                            buffer.cursor_y += 1;
                            if buffer.cursor_y >= buffer.height {
                                Self::scroll_screen_buffer(buffer);
                                buffer.cursor_y = buffer.height - 1;
                            }
                        }
                    }
                }
            }
            
            written += 1;
            
            // Output to VGA buffer for visual feedback
            if byte.is_ascii_graphic() || byte == b' ' {
                crate::print!("{}", byte as char);
            } else if byte == b'\n' {
                crate::println!();
            }
        }
        
        written
    }
    
    fn scroll_screen_buffer(buffer: &mut ScreenBuffer) {
        let width = buffer.width as usize;
        let height = buffer.height as usize;
        
        // Move all lines up by one
        for y in 0..(height - 1) {
            for x in 0..width {
                let src_index = (y + 1) * width + x;
                let dst_index = y * width + x;
                if src_index < buffer.buffer.len() && dst_index < buffer.buffer.len() {
                    buffer.buffer[dst_index] = buffer.buffer[src_index];
                }
            }
        }
        
        // Clear the last line
        for x in 0..width {
            let index = (height - 1) * width + x;
            if index < buffer.buffer.len() {
                buffer.buffer[index] = CharInfo::default();
            }
        }
    }
    
    pub fn read_console(&mut self, handle: HANDLE, buffer: &mut [u8], max_count: u32) -> Option<u32> {
        // Find the console that owns this handle
        for (_, console) in &mut self.consoles {
            if console.input_handle == handle {
                // Simple implementation: read from input buffer
                let mut read = 0;
                while read < max_count && !console.input_buffer.is_empty() {
                    if let Some(InputRecord::KeyEvent(key)) = console.input_buffer.pop_front() {
                        if key.key_down && key.unicode_char != 0 {
                            if (read as usize) < buffer.len() {
                                buffer[read as usize] = key.unicode_char as u8;
                                read += 1;
                            }
                        }
                    }
                }
                return Some(read);
            }
        }
        None
    }
    
    pub fn set_console_title(&mut self, title: &str) -> bool {
        if let Some(handle) = self.active_console {
            if let Some(console) = self.consoles.get_mut(&handle.0) {
                console.title = String::from(title);
                return true;
            }
        }
        false
    }
    
    pub fn get_console_title(&self) -> Option<String> {
        if let Some(handle) = self.active_console {
            if let Some(console) = self.consoles.get(&handle.0) {
                return Some(console.title.clone());
            }
        }
        None
    }
    
    pub fn set_console_text_attribute(&mut self, handle: HANDLE, attributes: u16) -> bool {
        for (_, console) in &mut self.consoles {
            if console.output_handle == handle {
                console.screen_buffer.attributes = attributes;
                return true;
            }
        }
        false
    }
    
    pub fn set_console_cursor_position(&mut self, handle: HANDLE, coord: Coord) -> bool {
        for (_, console) in &mut self.consoles {
            if console.output_handle == handle {
                if coord.x >= 0 && coord.x < console.screen_buffer.width as i16 &&
                   coord.y >= 0 && coord.y < console.screen_buffer.height as i16 {
                    console.screen_buffer.cursor_x = coord.x as u16;
                    console.screen_buffer.cursor_y = coord.y as u16;
                    return true;
                }
            }
        }
        false
    }
}

// Console color attributes
pub const FOREGROUND_BLUE: u16 = 0x0001;
pub const FOREGROUND_GREEN: u16 = 0x0002;
pub const FOREGROUND_RED: u16 = 0x0004;
pub const FOREGROUND_INTENSITY: u16 = 0x0008;
pub const BACKGROUND_BLUE: u16 = 0x0010;
pub const BACKGROUND_GREEN: u16 = 0x0020;
pub const BACKGROUND_RED: u16 = 0x0040;
pub const BACKGROUND_INTENSITY: u16 = 0x0080;

pub const FOREGROUND_WHITE: u16 = FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE;
pub const BACKGROUND_WHITE: u16 = BACKGROUND_RED | BACKGROUND_GREEN | BACKGROUND_BLUE;

// Console mode flags
pub const ENABLE_PROCESSED_INPUT: DWORD = 0x0001;
pub const ENABLE_LINE_INPUT: DWORD = 0x0002;
pub const ENABLE_ECHO_INPUT: DWORD = 0x0004;
pub const ENABLE_WINDOW_INPUT: DWORD = 0x0008;
pub const ENABLE_MOUSE_INPUT: DWORD = 0x0010;
pub const ENABLE_INSERT_MODE: DWORD = 0x0020;
pub const ENABLE_QUICK_EDIT_MODE: DWORD = 0x0040;

pub const ENABLE_PROCESSED_OUTPUT: DWORD = 0x0001;
pub const ENABLE_WRAP_AT_EOL_OUTPUT: DWORD = 0x0002;

// Standard handle constants
pub const STD_INPUT_HANDLE: i32 = -10;
pub const STD_OUTPUT_HANDLE: i32 = -11;
pub const STD_ERROR_HANDLE: i32 = -12;

// Console API Functions

/// GetStdHandle - Get standard handle
#[no_mangle]
pub extern "C" fn GetStdHandle(std_handle: i32) -> HANDLE {
    CONSOLE_MANAGER.lock().get_std_handle(std_handle)
}

/// AllocConsole - Allocate a console for the process
#[no_mangle]
pub extern "C" fn AllocConsole() -> BOOL {
    if CONSOLE_MANAGER.lock().alloc_console() {
        1
    } else {
        0
    }
}

/// FreeConsole - Free the console
#[no_mangle]
pub extern "C" fn FreeConsole() -> BOOL {
    if CONSOLE_MANAGER.lock().free_console() {
        1
    } else {
        0
    }
}

/// WriteConsoleA - Write to console output
#[no_mangle]
pub extern "C" fn WriteConsoleA(
    handle: HANDLE,
    buffer: *const u8,
    chars_to_write: DWORD,
    chars_written: *mut DWORD,
    _reserved: *const u8,
) -> BOOL {
    if buffer.is_null() {
        return 0;
    }
    
    let data = unsafe { core::slice::from_raw_parts(buffer, chars_to_write as usize) };
    
    if let Some(written) = CONSOLE_MANAGER.lock().write_console(handle, data) {
        if !chars_written.is_null() {
            unsafe {
                *chars_written = written;
            }
        }
        1
    } else {
        0
    }
}

/// ReadConsoleA - Read from console input
#[no_mangle]
pub extern "C" fn ReadConsoleA(
    handle: HANDLE,
    buffer: *mut u8,
    chars_to_read: DWORD,
    chars_read: *mut DWORD,
    _input_control: *const u8,
) -> BOOL {
    if buffer.is_null() {
        return 0;
    }
    
    let mut data = unsafe { core::slice::from_raw_parts_mut(buffer, chars_to_read as usize) };
    
    if let Some(read) = CONSOLE_MANAGER.lock().read_console(handle, &mut data, chars_to_read) {
        if !chars_read.is_null() {
            unsafe {
                *chars_read = read;
            }
        }
        1
    } else {
        0
    }
}

/// SetConsoleTitleA - Set console window title
#[no_mangle]
pub extern "C" fn SetConsoleTitleA(title: LPCSTR) -> BOOL {
    use core::ffi::CStr;
    
    let title_str = if title.is_null() {
        ""
    } else {
        match unsafe { CStr::from_ptr(title as *const i8) }.to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };
    
    if CONSOLE_MANAGER.lock().set_console_title(title_str) {
        1
    } else {
        0
    }
}

/// GetConsoleTitleA - Get console window title
#[no_mangle]
pub extern "C" fn GetConsoleTitleA(title: LPSTR, size: DWORD) -> DWORD {
    if title.is_null() || size == 0 {
        return 0;
    }
    
    if let Some(console_title) = CONSOLE_MANAGER.lock().get_console_title() {
        let bytes = console_title.as_bytes();
        let copy_len = core::cmp::min(bytes.len(), (size - 1) as usize);
        
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), title, copy_len);
            *title.add(copy_len) = 0; // Null terminate
        }
        
        copy_len as DWORD
    } else {
        0
    }
}

/// SetConsoleTextAttribute - Set console text attributes
#[no_mangle]
pub extern "C" fn SetConsoleTextAttribute(handle: HANDLE, attributes: u16) -> BOOL {
    if CONSOLE_MANAGER.lock().set_console_text_attribute(handle, attributes) {
        1
    } else {
        0
    }
}

/// SetConsoleCursorPosition - Set cursor position
#[no_mangle]
pub extern "C" fn SetConsoleCursorPosition(handle: HANDLE, coord: Coord) -> BOOL {
    if CONSOLE_MANAGER.lock().set_console_cursor_position(handle, coord) {
        1
    } else {
        0
    }
}