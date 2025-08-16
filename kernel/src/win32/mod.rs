pub mod kernel32;
pub mod user32;
pub mod gdi;
pub mod advapi32;
pub mod syscall;
pub mod window;
pub mod console;
pub mod winmm;
pub mod winsock;
pub mod printing;
pub mod ole32;
pub mod graphics;


// Windows-style handles
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Handle(pub u64);

impl Handle {
    pub const INVALID: Handle = Handle(0xFFFFFFFFFFFFFFFF);
    pub const NULL: Handle = Handle(0);
}

// Windows error codes
pub const ERROR_SUCCESS: u32 = 0;
pub const ERROR_FILE_NOT_FOUND: u32 = 2;
pub const ERROR_ACCESS_DENIED: u32 = 5;
pub const ERROR_INVALID_HANDLE: u32 = 6;
pub const ERROR_NOT_ENOUGH_MEMORY: u32 = 8;

// Windows types
pub type DWORD = u32;
pub type BOOL = i32;
pub type HANDLE = Handle;
pub type LPSTR = *mut u8;
pub type LPCSTR = *const u8;
pub type LPWSTR = *mut u16;
pub type LPCWSTR = *const u16;
pub type HRESULT = i32;

// Process information structure
#[repr(C)]
pub struct ProcessInformation {
    pub process: HANDLE,
    pub thread: HANDLE,
    pub process_id: DWORD,
    pub thread_id: DWORD,
}

// Startup info structure
#[repr(C)]
pub struct StartupInfo {
    pub cb: DWORD,
    pub reserved: LPSTR,
    pub desktop: LPSTR,
    pub title: LPSTR,
    pub x: DWORD,
    pub y: DWORD,
    pub x_size: DWORD,
    pub y_size: DWORD,
    pub x_count_chars: DWORD,
    pub y_count_chars: DWORD,
    pub fill_attribute: DWORD,
    pub flags: DWORD,
    pub show_window: u16,
    pub cb_reserved2: u16,
    pub reserved2: *mut u8,
    pub std_input: HANDLE,
    pub std_output: HANDLE,
    pub std_error: HANDLE,
}

impl Default for StartupInfo {
    fn default() -> Self {
        Self {
            cb: core::mem::size_of::<StartupInfo>() as DWORD,
            reserved: core::ptr::null_mut(),
            desktop: core::ptr::null_mut(),
            title: core::ptr::null_mut(),
            x: 0,
            y: 0,
            x_size: 0,
            y_size: 0,
            x_count_chars: 0,
            y_count_chars: 0,
            fill_attribute: 0,
            flags: 0,
            show_window: 0,
            cb_reserved2: 0,
            reserved2: core::ptr::null_mut(),
            std_input: Handle::NULL,
            std_output: Handle::NULL,
            std_error: Handle::NULL,
        }
    }
}