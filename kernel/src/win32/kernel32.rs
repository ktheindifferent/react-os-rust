use super::*;
use core::ffi::CStr;
use crate::process::executor::EXECUTOR;

/// CreateProcessA - Create a new process (ANSI version)
#[no_mangle]
pub extern "C" fn CreateProcessA(
    application_name: LPCSTR,
    command_line: LPSTR,
    _process_attributes: *const u8,
    _thread_attributes: *const u8,
    inherit_handles: BOOL,
    _creation_flags: DWORD,
    _environment: *const u8,
    _current_directory: LPCSTR,
    _startup_info: *const StartupInfo,
    process_information: *mut ProcessInformation,
) -> BOOL {
    // Convert C strings to Rust strings
    let app_name = if application_name.is_null() {
        if command_line.is_null() {
            unsafe { SetLastError(87); } // ERROR_INVALID_PARAMETER
            return 0;
        }
        // Parse app name from command line
        match unsafe { CStr::from_ptr(command_line as *const i8) }.to_str() {
            Ok(s) => s.split_whitespace().next().unwrap_or("unknown"),
            Err(_) => {
                unsafe { SetLastError(87); } // ERROR_INVALID_PARAMETER
                return 0;
            }
        }
    } else {
        match unsafe { CStr::from_ptr(application_name as *const i8) }.to_str() {
            Ok(s) => s,
            Err(_) => {
                unsafe { SetLastError(87); } // ERROR_INVALID_PARAMETER
                return 0;
            }
        }
    };

    // Log the process creation attempt
    crate::println!("CreateProcessA: Starting {}", app_name);
    
    // Create a dummy process for now
    // In a real implementation, this would:
    // 1. Load the executable file
    // 2. Parse PE/ELF format
    // 3. Create process via EXECUTOR
    // 4. Set up initial thread
    // 5. Handle environment and startup info
    
    let process_id = {
        static mut NEXT_PID: u32 = 1000;
        unsafe {
            let pid = NEXT_PID;
            NEXT_PID += 1;
            pid
        }
    };
    
    let thread_id = process_id + 1000; // Simple thread ID generation
    
    if !process_information.is_null() {
        unsafe {
            (*process_information).process = Handle(process_id as u64);
            (*process_information).thread = Handle(thread_id as u64);
            (*process_information).process_id = process_id;
            (*process_information).thread_id = thread_id;
        }
    }
    
    crate::println!("CreateProcessA: Process {} created (PID: {})", app_name, process_id);
    1 // TRUE - success
}

// Thread-local storage for last error (simplified with static)
static mut LAST_ERROR: DWORD = 0;

/// GetLastError - Get the last error code
#[no_mangle]
pub extern "C" fn GetLastError() -> DWORD {
    unsafe { LAST_ERROR }
}

/// SetLastError - Set the last error code
#[no_mangle]
pub extern "C" fn SetLastError(error: DWORD) {
    unsafe { LAST_ERROR = error; }
}

/// CloseHandle - Close an object handle
#[no_mangle]
pub extern "C" fn CloseHandle(handle: HANDLE) -> BOOL {
    if handle == Handle::INVALID || handle == Handle::NULL {
        return 0; // FALSE
    }
    // Placeholder implementation
    1 // TRUE
}

/// GetCurrentProcessId - Get current process identifier
#[no_mangle]
pub extern "C" fn GetCurrentProcessId() -> DWORD {
    // Return a dummy process ID for now
    1
}

/// GetCurrentThreadId - Get current thread identifier
#[no_mangle]
pub extern "C" fn GetCurrentThreadId() -> DWORD {
    // Return a dummy thread ID for now
    1
}

/// ExitProcess - Terminate the current process
#[no_mangle]
pub extern "C" fn ExitProcess(exit_code: DWORD) -> ! {
    crate::println!("Process exiting with code: {}", exit_code);
    crate::hlt_loop();
}

/// Sleep - Suspend thread execution
#[no_mangle]
pub extern "C" fn Sleep(milliseconds: DWORD) {
    // Placeholder implementation
    crate::println!("Sleep called for {} ms", milliseconds);
    // In a real implementation, this would yield to the scheduler
}

/// GetTickCount - Get system uptime in milliseconds
#[no_mangle]
pub extern "C" fn GetTickCount() -> DWORD {
    // Placeholder implementation - return a dummy value
    12345
}

/// VirtualAlloc - Reserve or commit memory pages
#[no_mangle]
pub extern "C" fn VirtualAlloc(
    address: *mut u8,
    size: usize,
    allocation_type: DWORD,
    protect: DWORD,
) -> *mut u8 {
    use crate::memory;
    
    // Simplified implementation
    if size == 0 {
        unsafe { SetLastError(87); } // ERROR_INVALID_PARAMETER
        return core::ptr::null_mut();
    }
    
    // Allocate memory (simplified - just use heap)
    let layout = match core::alloc::Layout::from_size_align(size, 4096) {
        Ok(layout) => layout,
        Err(_) => {
            unsafe { SetLastError(87); } // ERROR_INVALID_PARAMETER
            return core::ptr::null_mut();
        }
    };
    unsafe {
        let ptr = alloc::alloc::alloc(layout);
        if ptr.is_null() {
            SetLastError(8); // ERROR_NOT_ENOUGH_MEMORY
        }
        ptr
    }
}

/// VirtualFree - Release or decommit memory pages
#[no_mangle]
pub extern "C" fn VirtualFree(
    address: *mut u8,
    size: usize,
    free_type: DWORD,
) -> BOOL {
    if address.is_null() {
        unsafe { SetLastError(87); } // ERROR_INVALID_PARAMETER
        return 0;
    }
    
    // Simplified - just deallocate
    if size > 0 {
        let layout = core::alloc::Layout::from_size_align(size, 4096).unwrap();
        unsafe {
            alloc::alloc::dealloc(address, layout);
        }
    }
    
    1 // TRUE
}

/// GetModuleHandleA - Get module handle
#[no_mangle]
pub extern "C" fn GetModuleHandleA(module_name: LPCSTR) -> Handle {
    if module_name.is_null() {
        // Return handle to current process
        return Handle(0x400000); // Standard base address
    }
    
    // For now, return a dummy handle
    Handle(0x400000)
}

/// GetProcAddress - Get function address from module
#[no_mangle]
pub extern "C" fn GetProcAddress(module: Handle, proc_name: LPCSTR) -> *const u8 {
    if proc_name.is_null() {
        return core::ptr::null();
    }
    
    let name = unsafe {
        match CStr::from_ptr(proc_name as *const i8).to_str() {
            Ok(s) => s,
            Err(_) => return core::ptr::null(),
        }
    };
    
    // Return addresses of our implemented functions
    match name {
        "CreateProcessA" => CreateProcessA as *const u8,
        "GetLastError" => GetLastError as *const u8,
        "SetLastError" => SetLastError as *const u8,
        "CloseHandle" => CloseHandle as *const u8,
        "ExitProcess" => ExitProcess as *const u8,
        "Sleep" => Sleep as *const u8,
        "GetTickCount" => GetTickCount as *const u8,
        "VirtualAlloc" => VirtualAlloc as *const u8,
        "VirtualFree" => VirtualFree as *const u8,
        "GetModuleHandleA" => GetModuleHandleA as *const u8,
        "GetProcAddress" => GetProcAddress as *const u8,
        _ => core::ptr::null(),
    }
}

/// LoadLibraryA - Load a DLL
#[no_mangle]
pub extern "C" fn LoadLibraryA(filename: LPCSTR) -> Handle {
    if filename.is_null() {
        unsafe { SetLastError(87); } // ERROR_INVALID_PARAMETER
        return Handle::NULL;
    }
    
    let name = unsafe {
        match CStr::from_ptr(filename as *const i8).to_str() {
            Ok(s) => s,
            Err(_) => {
                SetLastError(87); // ERROR_INVALID_PARAMETER
                return Handle::NULL;
            }
        }
    };
    
    crate::println!("LoadLibrary: {}", name);
    
    // For now, return a dummy handle for known DLLs
    match name.to_lowercase().as_str() {
        "kernel32.dll" | "kernel32" => Handle(0x77000000),
        "ntdll.dll" | "ntdll" => Handle(0x77100000),
        "user32.dll" | "user32" => Handle(0x77200000),
        "gdi32.dll" | "gdi32" => Handle(0x77300000),
        _ => {
            unsafe { SetLastError(2); } // ERROR_FILE_NOT_FOUND
            Handle::NULL
        }
    }
}

/// FreeLibrary - Unload a DLL
#[no_mangle]
pub extern "C" fn FreeLibrary(module: Handle) -> BOOL {
    if module == Handle::NULL || module == Handle::INVALID {
        unsafe { SetLastError(6); } // ERROR_INVALID_HANDLE
        return 0;
    }
    
    // Placeholder implementation
    1 // TRUE
}

/// WriteFile - Write to file or device
#[no_mangle]
pub extern "C" fn WriteFile(
    file: Handle,
    buffer: *const u8,
    bytes_to_write: DWORD,
    bytes_written: *mut DWORD,
    overlapped: *mut u8,
) -> BOOL {
    if buffer.is_null() {
        unsafe { SetLastError(87); } // ERROR_INVALID_PARAMETER
        return 0;
    }
    
    // Handle console output
    if file == Handle(2) || file == Handle(3) { // stdout or stderr
        let data = unsafe {
            core::slice::from_raw_parts(buffer, bytes_to_write as usize)
        };
        
        // Write to console
        for &byte in data {
            crate::print!("{}", byte as char);
        }
        
        if !bytes_written.is_null() {
            unsafe { *bytes_written = bytes_to_write; }
        }
        
        return 1; // TRUE
    }
    
    // File system write would go here
    unsafe { SetLastError(6); } // ERROR_INVALID_HANDLE
    0 // FALSE
}

/// ReadFile - Read from file or device  
#[no_mangle]
pub extern "C" fn ReadFile(
    file: Handle,
    buffer: *mut u8,
    bytes_to_read: DWORD,
    bytes_read: *mut DWORD,
    overlapped: *mut u8,
) -> BOOL {
    if buffer.is_null() {
        unsafe { SetLastError(87); } // ERROR_INVALID_PARAMETER
        return 0;
    }
    
    // Handle console input
    if file == Handle(1) { // stdin
        // Simplified - just return 0 bytes read for now
        if !bytes_read.is_null() {
            unsafe { *bytes_read = 0; }
        }
        return 1; // TRUE
    }
    
    // File system read would go here
    unsafe { SetLastError(6); } // ERROR_INVALID_HANDLE
    0 // FALSE
}

/// CreateFileA - Create or open file
#[no_mangle]
pub extern "C" fn CreateFileA(
    filename: LPCSTR,
    desired_access: DWORD,
    share_mode: DWORD,
    security_attributes: *mut u8,
    creation_disposition: DWORD,
    flags_and_attributes: DWORD,
    template_file: Handle,
) -> Handle {
    if filename.is_null() {
        unsafe { SetLastError(87); } // ERROR_INVALID_PARAMETER
        return Handle::INVALID;
    }
    
    let name = unsafe {
        match CStr::from_ptr(filename as *const i8).to_str() {
            Ok(s) => s,
            Err(_) => {
                SetLastError(87); // ERROR_INVALID_PARAMETER
                return Handle::INVALID;
            }
        }
    };
    
    // For now, return a dummy file handle
    // In a real implementation, this would open the file through VFS
    crate::println!("CreateFileA: {}", name);
    Handle(1000) // Dummy file handle
}

/// GetCommandLineA - Get command line string
#[no_mangle]
pub extern "C" fn GetCommandLineA() -> LPSTR {
    // Return a static command line string
    static COMMAND_LINE: &[u8] = b"rust-os.exe\0";
    COMMAND_LINE.as_ptr() as LPSTR
}

/// GetEnvironmentVariableA - Get environment variable
#[no_mangle]
pub extern "C" fn GetEnvironmentVariableA(
    name: LPCSTR,
    buffer: LPSTR,
    size: DWORD,
) -> DWORD {
    if name.is_null() || buffer.is_null() {
        unsafe { SetLastError(87); } // ERROR_INVALID_PARAMETER
        return 0;
    }
    
    // For now, return 0 (variable not found)
    0
}