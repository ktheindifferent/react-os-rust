use super::{NtStatus, object::Handle};
use super::process::{ProcessCreateFlags, ProcessId, ThreadId};
use super::object::ObjectAttributes;
use alloc::string::String;
use core::arch::asm;

// System call numbers - matching Windows NT
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemCall {
    // Process management
    NtCreateProcess = 0x0020,
    NtCreateProcessEx = 0x004D,
    NtCreateUserProcess = 0x00C8,
    NtTerminateProcess = 0x002C,
    NtQueryInformationProcess = 0x0019,
    NtSetInformationProcess = 0x001C,
    
    // Thread management
    NtCreateThread = 0x004E,
    NtCreateThreadEx = 0x00A1,
    NtTerminateThread = 0x0053,
    NtQueryInformationThread = 0x0025,
    NtSetInformationThread = 0x000D,
    
    // Object management
    NtCreateDirectoryObject = 0x0042,
    NtOpenDirectoryObject = 0x0088,
    NtQueryDirectoryObject = 0x0055,
    NtCreateSymbolicLinkObject = 0x0094,
    NtOpenSymbolicLinkObject = 0x00B6,
    
    // Handle management
    NtClose = 0x000F,
    NtDuplicateObject = 0x003C,
    NtQueryObject = 0x0010,
    NtSetInformationObject = 0x005C,
    
    // Memory management
    NtAllocateVirtualMemory = 0x0018,
    NtFreeVirtualMemory = 0x001E,
    NtQueryVirtualMemory = 0x0023,
    NtProtectVirtualMemory = 0x0050,
    NtMapViewOfSection = 0x0028,
    NtUnmapViewOfSection = 0x002A,
    
    // File I/O
    NtCreateFile = 0x0056,
    NtOpenFile = 0x0074,
    NtReadFile = 0x0006,
    NtWriteFile = 0x0008,
    NtQueryInformationFile = 0x0011,
    NtSetInformationFile = 0x0027,
    NtDeleteFile = 0x0073,
    
    // Registry
    NtCreateKey = 0x001D,
    NtOpenKey = 0x0012,
    NtQueryKey = 0x0016,
    NtSetValueKey = 0x0096,
    NtQueryValueKey = 0x0017,
    NtDeleteKey = 0x007C,
    NtDeleteValueKey = 0x007F,
    
    // Synchronization
    NtCreateEvent = 0x0048,
    NtOpenEvent = 0x0040,
    NtSetEvent = 0x000E,
    NtClearEvent = 0x003D,
    NtWaitForSingleObject = 0x0001,
    NtWaitForMultipleObjects = 0x002D,
    
    // Time and system info
    NtQuerySystemTime = 0x005A,
    NtSetSystemTime = 0x00CE,
    NtQuerySystemInformation = 0x0036,
    NtSetSystemInformation = 0x00AD,
    
    // Debug and trace
    NtDebugActiveProcess = 0x00AB,
    NtDebugContinue = 0x0061,
    
    // Security
    NtOpenProcessToken = 0x0122,
    NtOpenThreadToken = 0x0024,
    NtAdjustPrivilegesToken = 0x0041,
    
    // Power management
    NtSetSystemPowerState = 0x01AB,
    NtInitiatePowerAction = 0x01B3,
}

// System call parameter structure
#[repr(C)]
#[derive(Debug)]
pub struct SyscallParams {
    pub rax: u64, // System call number
    pub rcx: u64, // First parameter
    pub rdx: u64, // Second parameter
    pub r8: u64,  // Third parameter
    pub r9: u64,  // Fourth parameter
    pub r10: u64, // Fifth parameter (from stack)
    pub r11: u64, // Sixth parameter (from stack)
    pub rsp: u64, // Stack pointer for additional params
}

// System call dispatcher
pub fn dispatch_syscall(params: &SyscallParams) -> u64 {
    let syscall_num = params.rax as u32;
    
    match SystemCall::try_from(syscall_num) {
        Ok(syscall) => handle_syscall(syscall, params),
        Err(_) => NtStatus::NotImplemented as u64,
    }
}

impl TryFrom<u32> for SystemCall {
    type Error = ();
    
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0x0020 => Ok(SystemCall::NtCreateProcess),
            0x004D => Ok(SystemCall::NtCreateProcessEx),
            0x00C8 => Ok(SystemCall::NtCreateUserProcess),
            0x002C => Ok(SystemCall::NtTerminateProcess),
            0x0019 => Ok(SystemCall::NtQueryInformationProcess),
            0x001C => Ok(SystemCall::NtSetInformationProcess),
            0x004E => Ok(SystemCall::NtCreateThread),
            0x00A1 => Ok(SystemCall::NtCreateThreadEx),
            0x0053 => Ok(SystemCall::NtTerminateThread),
            0x0025 => Ok(SystemCall::NtQueryInformationThread),
            0x000D => Ok(SystemCall::NtSetInformationThread),
            0x0042 => Ok(SystemCall::NtCreateDirectoryObject),
            0x0088 => Ok(SystemCall::NtOpenDirectoryObject),
            0x0055 => Ok(SystemCall::NtQueryDirectoryObject),
            0x0094 => Ok(SystemCall::NtCreateSymbolicLinkObject),
            0x00B6 => Ok(SystemCall::NtOpenSymbolicLinkObject),
            0x000F => Ok(SystemCall::NtClose),
            0x003C => Ok(SystemCall::NtDuplicateObject),
            0x0010 => Ok(SystemCall::NtQueryObject),
            0x005C => Ok(SystemCall::NtSetInformationObject),
            0x0018 => Ok(SystemCall::NtAllocateVirtualMemory),
            0x001E => Ok(SystemCall::NtFreeVirtualMemory),
            0x0023 => Ok(SystemCall::NtQueryVirtualMemory),
            0x0050 => Ok(SystemCall::NtProtectVirtualMemory),
            0x0028 => Ok(SystemCall::NtMapViewOfSection),
            0x002A => Ok(SystemCall::NtUnmapViewOfSection),
            _ => Err(()),
        }
    }
}

fn handle_syscall(syscall: SystemCall, params: &SyscallParams) -> u64 {
    use crate::serial_println;
    
    serial_println!("Syscall: {:?} with params: {:?}", syscall, params);
    
    match syscall {
        SystemCall::NtCreateProcess => {
            handle_nt_create_process(params)
        }
        SystemCall::NtCreateProcessEx => {
            handle_nt_create_process_ex(params)
        }
        SystemCall::NtCreateUserProcess => {
            handle_nt_create_user_process(params)
        }
        SystemCall::NtTerminateProcess => {
            handle_nt_terminate_process(params)
        }
        SystemCall::NtQueryInformationProcess => {
            handle_nt_query_information_process(params)
        }
        SystemCall::NtCreateDirectoryObject => {
            handle_nt_create_directory_object(params)
        }
        SystemCall::NtClose => {
            handle_nt_close(params)
        }
        SystemCall::NtAllocateVirtualMemory => {
            handle_nt_allocate_virtual_memory(params)
        }
        SystemCall::NtFreeVirtualMemory => {
            handle_nt_free_virtual_memory(params)
        }
        _ => {
            serial_println!("Unimplemented syscall: {:?}", syscall);
            NtStatus::NotImplemented as u64
        }
    }
}

// Individual system call handlers
fn handle_nt_create_process(params: &SyscallParams) -> u64 {
    let process_handle_ptr = params.rcx as *mut Handle;
    let desired_access = params.rdx as u32;
    
    // Simplified implementation - create a basic process
    use super::process::PROCESS_MANAGER;
    let mut pm = PROCESS_MANAGER.lock();
    
    match pm.create_process(
        String::from("syscall_process.exe"),
        String::from(""),
        None,
        ProcessCreateFlags::empty(),
    ) {
        Ok((_, handle)) => {
            // In a real implementation, we'd safely write to user space
            // For now, we'll just return success
            unsafe {
                if !process_handle_ptr.is_null() {
                    *process_handle_ptr = handle;
                }
            }
            NtStatus::Success as u64
        }
        Err(status) => status as u64,
    }
}

fn handle_nt_create_process_ex(params: &SyscallParams) -> u64 {
    // More advanced process creation
    handle_nt_create_process(params)
}

fn handle_nt_create_user_process(params: &SyscallParams) -> u64 {
    let process_handle_ptr = params.rcx as *mut Handle;
    let thread_handle_ptr = params.rdx as *mut Handle;
    let process_flags = ProcessCreateFlags::from_bits_truncate(params.r8 as u32);
    
    use super::process::PROCESS_MANAGER;
    let mut pm = PROCESS_MANAGER.lock();
    
    match pm.create_process(
        String::from("user_process.exe"),
        String::from(""),
        None,
        process_flags,
    ) {
        Ok((_, proc_handle)) => {
            unsafe {
                if !process_handle_ptr.is_null() {
                    *process_handle_ptr = proc_handle;
                }
                if !thread_handle_ptr.is_null() {
                    *thread_handle_ptr = Handle::new(); // Create initial thread handle
                }
            }
            NtStatus::Success as u64
        }
        Err(status) => status as u64,
    }
}

fn handle_nt_terminate_process(params: &SyscallParams) -> u64 {
    let process_handle = Handle::from_raw(params.rcx);
    let exit_status = params.rdx as u32;
    
    super::process::nt_terminate_process(process_handle, exit_status) as u64
}

fn handle_nt_query_information_process(params: &SyscallParams) -> u64 {
    let process_handle = Handle::from_raw(params.rcx);
    let process_information_class = params.rdx as u32;
    let process_information = params.r8 as *mut u8;
    let process_information_length = params.r9 as u32;
    
    super::process::nt_query_information_process(
        process_handle,
        process_information_class,
        process_information,
        process_information_length,
        None,
    ) as u64
}

fn handle_nt_create_directory_object(params: &SyscallParams) -> u64 {
    let directory_handle_ptr = params.rcx as *mut Handle;
    let desired_access = params.rdx as u32;
    
    let mut handle = Handle::NULL;
    let obj_attrs = ObjectAttributes::new();
    
    let status = super::object::nt_create_directory_object(&mut handle, desired_access, &obj_attrs);
    
    unsafe {
        if !directory_handle_ptr.is_null() {
            *directory_handle_ptr = handle;
        }
    }
    
    status as u64
}

fn handle_nt_close(params: &SyscallParams) -> u64 {
    let handle = Handle::from_raw(params.rcx);
    
    // In a real implementation, we'd close the handle and release resources
    use super::object::OBJECT_MANAGER;
    let mut om = OBJECT_MANAGER.lock();
    
    om.close_handle(handle) as u64
}

fn handle_nt_allocate_virtual_memory(params: &SyscallParams) -> u64 {
    let process_handle = Handle::from_raw(params.rcx);
    let base_address_ptr = params.rdx as *mut *mut u8;
    let zero_bits = params.r8;
    let region_size_ptr = params.r9 as *mut usize;
    
    // For now, return not implemented
    // In a real implementation, we'd:
    // 1. Get the process from the handle
    // 2. Call the process's virtual memory allocator
    // 3. Update the base address and size pointers
    
    NtStatus::NotImplemented as u64
}

fn handle_nt_free_virtual_memory(params: &SyscallParams) -> u64 {
    let process_handle = Handle::from_raw(params.rcx);
    let base_address_ptr = params.rdx as *mut *mut u8;
    let region_size_ptr = params.r8 as *mut usize;
    let free_type = params.r9 as u32;
    
    // For now, return not implemented
    NtStatus::NotImplemented as u64
}

// System call entry point - would be called from interrupt handler
pub fn syscall_entry() -> ! {
    // This would be the actual syscall entry point
    // For now, we'll just halt
    loop {
        unsafe { asm!("hlt") };
    }
}

// Fast system call support (SYSCALL/SYSRET instructions)
pub fn setup_fast_syscall() {
    // This would set up MSR registers for fast system calls
    // MSR_LSTAR (0xC0000082) - System call entry point
    // MSR_STAR (0xC0000081) - Segment selectors
    // MSR_SFMASK (0xC0000084) - RFLAGS mask
    
    use crate::serial_println;
    serial_println!("Fast syscall support initialized");
}

// System call stub for user mode
pub fn make_syscall(syscall_num: u32, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64 {
    let mut result: u64;
    
    unsafe {
        asm!(
            "syscall",
            inout("rax") syscall_num as u64 => result,
            in("rcx") arg1,
            in("rdx") arg2,
            in("r8") arg3,
            in("r9") arg4,
            out("r10") _,
            out("r11") _,
            options(nostack, preserves_flags)
        );
    }
    
    result
}