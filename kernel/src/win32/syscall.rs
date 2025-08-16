// Windows NT System Call Interface
use super::*;
use crate::process::executor::EXECUTOR;
use x86_64::registers::model_specific::Msr;

// Windows system call numbers (simplified subset)
pub const SYSCALL_OPEN_PROCESS: u64 = 0x23;
pub const SYSCALL_CLOSE_HANDLE: u64 = 0x0C;
pub const SYSCALL_CREATE_FILE: u64 = 0x42;
pub const SYSCALL_READ_FILE: u64 = 0x03;
pub const SYSCALL_WRITE_FILE: u64 = 0x04;
pub const SYSCALL_ALLOCATE_VIRTUAL_MEMORY: u64 = 0x15;
pub const SYSCALL_FREE_VIRTUAL_MEMORY: u64 = 0x16;
pub const SYSCALL_QUERY_INFORMATION_PROCESS: u64 = 0x19;
pub const SYSCALL_CREATE_THREAD: u64 = 0x4B;
pub const SYSCALL_TERMINATE_PROCESS: u64 = 0x29;
pub const SYSCALL_WAIT_FOR_SINGLE_OBJECT: u64 = 0x01;
pub const SYSCALL_QUERY_SYSTEM_INFORMATION: u64 = 0x36;

// System call handler entry point
#[no_mangle]
pub extern "C" fn syscall_handler(
    syscall_number: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> i64 {
    match syscall_number {
        SYSCALL_OPEN_PROCESS => {
            // NtOpenProcess
            let desired_access = arg1 as u32;
            let process_id = arg2 as u32;
            
            crate::serial_println!("SYSCALL: NtOpenProcess(access=0x{:x}, pid={})", 
                desired_access, process_id);
            
            // Return dummy handle
            process_id as i64
        }
        
        SYSCALL_CLOSE_HANDLE => {
            // NtClose
            let handle = arg1;
            
            crate::serial_println!("SYSCALL: NtClose(handle=0x{:x})", handle);
            
            0 // STATUS_SUCCESS
        }
        
        SYSCALL_CREATE_FILE => {
            // NtCreateFile
            let desired_access = arg2 as u32;
            let file_attributes = arg3 as u32;
            
            crate::serial_println!("SYSCALL: NtCreateFile(access=0x{:x}, attrs=0x{:x})", 
                desired_access, file_attributes);
            
            // Return dummy file handle
            1000
        }
        
        SYSCALL_READ_FILE => {
            // NtReadFile
            let file_handle = arg1;
            let buffer = arg3 as *mut u8;
            let length = arg4 as u32;
            
            crate::serial_println!("SYSCALL: NtReadFile(handle={}, len={})", 
                file_handle, length);
            
            // Return bytes read (0 for now)
            0
        }
        
        SYSCALL_WRITE_FILE => {
            // NtWriteFile
            let file_handle = arg1;
            let buffer = arg3 as *const u8;
            let length = arg4 as u32;
            
            crate::serial_println!("SYSCALL: NtWriteFile(handle={}, len={})", 
                file_handle, length);
            
            // If writing to stdout/stderr, output to console
            if file_handle == 1 || file_handle == 2 {
                let data = unsafe {
                    core::slice::from_raw_parts(buffer, length as usize)
                };
                for &byte in data {
                    crate::print!("{}", byte as char);
                }
                return length as i64;
            }
            
            // Return bytes written
            length as i64
        }
        
        SYSCALL_ALLOCATE_VIRTUAL_MEMORY => {
            // NtAllocateVirtualMemory
            let size = arg2;
            let allocation_type = arg3 as u32;
            let protect = arg4 as u32;
            
            crate::serial_println!("SYSCALL: NtAllocateVirtualMemory(size={}, type=0x{:x})", 
                size, allocation_type);
            
            // Allocate memory (simplified)
            if size > 0 {
                let layout = core::alloc::Layout::from_size_align(size as usize, 4096).unwrap();
                let ptr = unsafe { alloc::alloc::alloc(layout) };
                ptr as i64
            } else {
                -1 // STATUS_INVALID_PARAMETER
            }
        }
        
        SYSCALL_FREE_VIRTUAL_MEMORY => {
            // NtFreeVirtualMemory
            let base_address = arg1 as *mut u8;
            let size = arg2;
            
            crate::serial_println!("SYSCALL: NtFreeVirtualMemory(addr=0x{:x}, size={})", 
                base_address as u64, size);
            
            if !base_address.is_null() && size > 0 {
                let layout = core::alloc::Layout::from_size_align(size as usize, 4096).unwrap();
                unsafe { alloc::alloc::dealloc(base_address, layout); }
            }
            
            0 // STATUS_SUCCESS
        }
        
        SYSCALL_QUERY_INFORMATION_PROCESS => {
            // NtQueryInformationProcess
            let process_handle = arg1;
            let info_class = arg2 as u32;
            
            crate::serial_println!("SYSCALL: NtQueryInformationProcess(handle={}, class={})", 
                process_handle, info_class);
            
            0 // STATUS_SUCCESS
        }
        
        SYSCALL_CREATE_THREAD => {
            // NtCreateThread
            let start_address = arg2;
            let parameter = arg3;
            
            crate::serial_println!("SYSCALL: NtCreateThread(start=0x{:x}, param=0x{:x})", 
                start_address, parameter);
            
            // Return dummy thread handle
            2000
        }
        
        SYSCALL_TERMINATE_PROCESS => {
            // NtTerminateProcess
            let process_handle = arg1;
            let exit_status = arg2 as i32;
            
            crate::serial_println!("SYSCALL: NtTerminateProcess(handle={}, status={})", 
                process_handle, exit_status);
            
            // Terminate current process if handle is -1 (current process)
            if process_handle == 0xFFFFFFFFFFFFFFFF {
                let mut executor = EXECUTOR.lock();
                if let Some(pid) = executor.get_current_pid() {
                    executor.terminate_process(pid, exit_status);
                }
            }
            
            0 // STATUS_SUCCESS
        }
        
        SYSCALL_WAIT_FOR_SINGLE_OBJECT => {
            // NtWaitForSingleObject
            let handle = arg1;
            let alertable = arg2 != 0;
            let timeout = arg3 as *const i64;
            
            crate::serial_println!("SYSCALL: NtWaitForSingleObject(handle={}, alertable={})", 
                handle, alertable);
            
            // For now, just return immediately
            0 // STATUS_SUCCESS
        }
        
        SYSCALL_QUERY_SYSTEM_INFORMATION => {
            // NtQuerySystemInformation
            let info_class = arg1 as u32;
            let buffer = arg2 as *mut u8;
            let length = arg3 as u32;
            
            crate::serial_println!("SYSCALL: NtQuerySystemInformation(class={}, len={})", 
                info_class, length);
            
            // Return some dummy information based on class
            match info_class {
                0 => { // SystemBasicInformation
                    if !buffer.is_null() && length >= 44 {
                        unsafe {
                            // Number of processors
                            *(buffer as *mut u32) = 1;
                            // Page size
                            *(buffer.add(4) as *mut u32) = 4096;
                        }
                    }
                }
                5 => { // SystemProcessInformation
                    // Return process list
                }
                _ => {}
            }
            
            0 // STATUS_SUCCESS
        }
        
        _ => {
            crate::serial_println!("SYSCALL: Unknown system call 0x{:x}", syscall_number);
            -1 // STATUS_NOT_IMPLEMENTED
        }
    }
}

// Initialize system call handler
pub fn init_syscall_handler() {
    // Set up MSRs for SYSCALL/SYSRET instructions
    unsafe {
        // IA32_EFER - Enable SYSCALL/SYSRET
        let mut efer = Msr::new(0xC0000080);
        let efer_value = efer.read();
        efer.write(efer_value | 1); // Set SCE bit
        
        // IA32_STAR - Segment selectors for SYSCALL
        let mut star = Msr::new(0xC0000081);
        // Kernel CS = 0x08, Kernel SS = 0x10
        // User CS = 0x1B, User SS = 0x23
        star.write((0x08u64 << 32) | (0x1Bu64 << 48));
        
        // IA32_LSTAR - SYSCALL entry point
        let mut lstar = Msr::new(0xC0000082);
        lstar.write(syscall_entry as u64);
        
        // IA32_FMASK - RFLAGS mask for SYSCALL
        let mut fmask = Msr::new(0xC0000084);
        fmask.write(0x200); // Clear interrupt flag
    }
    
    crate::serial_println!("Windows-compatible system call handler initialized");
}

// Assembly entry point for SYSCALL instruction
#[unsafe(naked)]
extern "C" fn syscall_entry() -> ! {
    unsafe {
        core::arch::naked_asm!(
            // Save user context
            "push rcx",      // RIP
            "push r11",      // RFLAGS
            "push rbp",
            "push rbx",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            
            // Call the handler
            // RAX = syscall number
            // RDI, RSI, RDX, R10, R8, R9 = arguments
            "mov rdi, rax",  // syscall number
            "mov rsi, rdi",  // arg1 (was in RDI)
            "mov rdx, rsi",  // arg2 (was in RSI)
            "mov rcx, rdx",  // arg3 (was in RDX)
            "mov r8, r10",   // arg4 (was in R10)
            // R8, R9 already have arg5, arg6
            
            "call syscall_handler",
            
            // Restore user context
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop rbx",
            "pop rbp",
            "pop r11",       // RFLAGS
            "pop rcx",       // RIP
            
            // Return to user mode
            "sysretq"
        );
    }
}