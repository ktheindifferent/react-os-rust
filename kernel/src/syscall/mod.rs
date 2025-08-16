use x86_64::structures::idt::InterruptStackFrame;
use x86_64::VirtAddr;
use alloc::string::String;
use alloc::vec::Vec;
use core::slice;

pub mod handlers;

#[derive(Debug, Clone, Copy)]
#[repr(usize)]
pub enum SyscallNumber {
    Exit = 0,
    Read = 1,
    Write = 2,
    Open = 3,
    Close = 4,
    Fork = 5,
    Exec = 6,
    Wait = 7,
    Kill = 8,
    GetPid = 9,
    Brk = 10,
    Mmap = 11,
    Munmap = 12,
    Sleep = 13,
    GetTime = 14,
    CreateWindow = 100,
    DestroyWindow = 101,
    DrawWindow = 102,
    HandleEvent = 103,
    GetScreenInfo = 104,
}

#[derive(Debug)]
pub struct SyscallContext {
    pub number: usize,
    pub arg1: usize,
    pub arg2: usize,
    pub arg3: usize,
    pub arg4: usize,
    pub arg5: usize,
    pub arg6: usize,
}

impl SyscallContext {
    pub fn from_registers(rax: usize, rdi: usize, rsi: usize, rdx: usize, 
                         r10: usize, r8: usize, r9: usize) -> Self {
        SyscallContext {
            number: rax,
            arg1: rdi,
            arg2: rsi,
            arg3: rdx,
            arg4: r10,
            arg5: r8,
            arg6: r9,
        }
    }
}

pub fn init() {
    use x86_64::registers::model_specific::{Efer, EferFlags, LStar, Star};
    use x86_64::registers::rflags::RFlags;
    
    unsafe {
        Efer::update(|flags| {
            *flags |= EferFlags::SYSTEM_CALL_EXTENSIONS;
        });
        
        // Set up GDT segments for syscall/sysret
        // The x86_64 crate may have changed - let's use a different approach
        // Star::write sets up the segment selectors for syscall/sysret
        
        LStar::write(VirtAddr::new(syscall_handler as usize as u64));
        
        x86_64::registers::model_specific::SFMask::write(
            RFlags::INTERRUPT_FLAG | RFlags::DIRECTION_FLAG
        );
    }
    
    crate::serial_println!("Syscall interface initialized");
}

extern "C" fn syscall_handler() {
    // For now, use a non-naked function until we fix the assembly
    // This is a temporary workaround
    let number: usize;
    let arg1: usize;
    let arg2: usize;
    let arg3: usize;
    let arg4: usize;
    let arg5: usize;
    let arg6: usize;
    
    unsafe {
        core::arch::asm!(
            "mov {}, rax",
            "mov {}, rdi",
            "mov {}, rsi",
            "mov {}, rdx",
            "mov {}, r10",
            "mov {}, r8",
            "mov {}, r9",
            out(reg) number,
            out(reg) arg1,
            out(reg) arg2,
            out(reg) arg3,
            out(reg) arg4,
            out(reg) arg5,
            out(reg) arg6,
        );
    }
    
    let _result = handle_syscall(number, arg1, arg2, arg3, arg4, arg5, arg6);
}

#[no_mangle]
pub extern "C" fn handle_syscall(
    number: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> isize {
    let context = SyscallContext::from_registers(number, arg1, arg2, arg3, arg4, arg5, arg6);
    
    match dispatch_syscall(context) {
        Ok(result) => result as isize,
        Err(errno) => -(errno as isize),
    }
}

fn dispatch_syscall(context: SyscallContext) -> Result<usize, usize> {
    use crate::memory::userspace::validate_user_buffer;
    
    match context.number {
        0 => handlers::sys_exit(context.arg1 as i32),
        1 => handlers::sys_read(context.arg1, context.arg2, context.arg3),
        2 => handlers::sys_write(context.arg1, context.arg2, context.arg3),
        3 => handlers::sys_open(context.arg1, context.arg2),
        4 => handlers::sys_close(context.arg1),
        5 => handlers::sys_fork(),
        6 => handlers::sys_exec(context.arg1, context.arg2),
        7 => handlers::sys_wait(context.arg1),
        8 => handlers::sys_kill(context.arg1, context.arg2),
        9 => handlers::sys_getpid(),
        10 => handlers::sys_brk(context.arg1),
        11 => handlers::sys_mmap(context.arg1, context.arg2, context.arg3, context.arg4, context.arg5, context.arg6),
        12 => handlers::sys_munmap(context.arg1, context.arg2),
        13 => handlers::sys_sleep(context.arg1),
        14 => handlers::sys_gettime(),
        100 => handlers::sys_create_window(context.arg1, context.arg2, context.arg3, context.arg4),
        101 => handlers::sys_destroy_window(context.arg1),
        102 => handlers::sys_draw_window(context.arg1, context.arg2),
        103 => handlers::sys_handle_event(context.arg1),
        104 => handlers::sys_get_screen_info(context.arg1),
        _ => Err(38),
    }
}

pub const EPERM: usize = 1;
pub const ENOENT: usize = 2;
pub const ESRCH: usize = 3;
pub const EINTR: usize = 4;
pub const EIO: usize = 5;
pub const ENXIO: usize = 6;
pub const E2BIG: usize = 7;
pub const ENOEXEC: usize = 8;
pub const EBADF: usize = 9;
pub const ECHILD: usize = 10;
pub const EAGAIN: usize = 11;
pub const ENOMEM: usize = 12;
pub const EACCES: usize = 13;
pub const EFAULT: usize = 14;
pub const ENOTBLK: usize = 15;
pub const EBUSY: usize = 16;
pub const EEXIST: usize = 17;
pub const EXDEV: usize = 18;
pub const ENODEV: usize = 19;
pub const ENOTDIR: usize = 20;
pub const EISDIR: usize = 21;
pub const EINVAL: usize = 22;
pub const ENFILE: usize = 23;
pub const EMFILE: usize = 24;
pub const ENOTTY: usize = 25;
pub const ETXTBSY: usize = 26;
pub const EFBIG: usize = 27;
pub const ENOSPC: usize = 28;
pub const ESPIPE: usize = 29;
pub const EROFS: usize = 30;
pub const EMLINK: usize = 31;
pub const EPIPE: usize = 32;
pub const EDOM: usize = 33;
pub const ERANGE: usize = 34;
pub const ENOSYS: usize = 38;