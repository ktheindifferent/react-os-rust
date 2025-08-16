// Fast syscall implementation using SYSCALL/SYSRET instructions
// Replaces slower INT 0x80 mechanism with optimized CPU instructions

use x86_64::VirtAddr;
use x86_64::registers::model_specific::{Efer, EferFlags, LStar, Star, SFMask};
use x86_64::registers::rflags::RFlags;
use core::arch::asm;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

// vDSO (virtual Dynamic Shared Object) for userspace-only syscalls
#[repr(C)]
pub struct VDsoData {
    pub version: u32,
    pub clock_gettime: unsafe extern "C" fn(clockid: i32, ts: *mut TimeSpec) -> i32,
    pub gettimeofday: unsafe extern "C" fn(tv: *mut TimeVal, tz: *mut TimeZone) -> i32,
    pub getpid: unsafe extern "C" fn() -> i32,
    pub getcpu: unsafe extern "C" fn(cpu: *mut u32, node: *mut u32) -> i32,
    pub tsc_frequency: u64,
    pub tsc_offset: u64,
    pub kernel_version: [u8; 64],
}

#[repr(C)]
pub struct TimeSpec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

#[repr(C)]
pub struct TimeVal {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

#[repr(C)]
pub struct TimeZone {
    pub tz_minuteswest: i32,
    pub tz_dsttime: i32,
}

// Syscall batch structure for reducing kernel transitions
#[derive(Debug, Clone)]
pub struct SyscallBatch {
    pub syscalls: Vec<SyscallRequest>,
    pub results: Vec<isize>,
}

#[derive(Debug, Clone)]
pub struct SyscallRequest {
    pub number: usize,
    pub args: [usize; 6],
}

lazy_static! {
    static ref SYSCALL_BATCH_BUFFER: Mutex<Vec<SyscallBatch>> = Mutex::new(Vec::new());
}

// Initialize fast syscall mechanism
pub fn init() {
    unsafe {
        // Enable SYSCALL/SYSRET in EFER MSR
        Efer::update(|flags| {
            *flags |= EferFlags::SYSTEM_CALL_EXTENSIONS;
            *flags |= EferFlags::NO_EXECUTE_ENABLE; // NX bit support
        });
        
        // Set up segment selectors for SYSCALL/SYSRET
        // STAR MSR: [63:48] = User CS, [47:32] = Kernel CS
        let star_value = ((0x23u64 << 48) | (0x08u64 << 32)) as u64;
        Star::write_raw(star_value);
        
        // Set syscall entry point
        LStar::write(VirtAddr::new(syscall_entry as usize as u64));
        
        // Set flags mask (clear interrupts and direction flag on syscall)
        SFMask::write(RFlags::INTERRUPT_FLAG | RFlags::DIRECTION_FLAG);
        
        // Initialize vDSO
        init_vdso();
        
        crate::serial_println!("Fast syscall (SYSCALL/SYSRET) initialized");
    }
}

// Fast syscall entry point - called by SYSCALL instruction
#[naked]
unsafe extern "C" fn syscall_entry() {
    asm!(
        // SYSCALL instruction loads:
        // RCX = RIP (return address)
        // R11 = RFLAGS
        // We need to save these for SYSRET
        
        // Switch to kernel stack
        "swapgs",                    // Swap GS to kernel GS
        "mov gs:[0x8], rsp",        // Save user RSP
        "mov rsp, gs:[0x0]",        // Load kernel RSP
        
        // Create minimal kernel frame
        "push rcx",                  // Save user RIP
        "push r11",                  // Save user RFLAGS
        
        // Save callee-saved registers
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        
        // Check for batch syscall
        "cmp rax, 0x1000",          // Special batch syscall number
        "je handle_batch_syscall",
        
        // Regular syscall - call handler
        // Arguments are already in correct registers:
        // RAX = syscall number
        // RDI, RSI, RDX, R10, R8, R9 = arguments
        "call handle_fast_syscall",
        
        "jmp syscall_return",
        
        "handle_batch_syscall:",
        "call handle_batch_syscall_impl",
        
        "syscall_return:",
        // RAX contains return value
        
        // Restore callee-saved registers
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        
        // Restore user RIP and RFLAGS
        "pop r11",                   // User RFLAGS
        "pop rcx",                   // User RIP
        
        // Restore user stack
        "mov rsp, gs:[0x8]",        // Restore user RSP
        "swapgs",                    // Swap back to user GS
        
        // Return to userspace
        "sysretq",
        options(noreturn)
    );
}

// Fast syscall handler - optimized version
#[no_mangle]
extern "C" fn handle_fast_syscall(
    number: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> isize {
    // Fast path for common syscalls
    match number {
        // Frequently used syscalls with optimized implementations
        0x27 => sys_getpid_fast(),           // getpid - no locks needed
        0x60 => sys_gettimeofday_fast(arg1, arg2), // gettimeofday
        0xE4 => sys_clock_gettime_fast(arg1, arg2), // clock_gettime
        _ => {
            // Fall back to regular syscall handler
            crate::syscall::handle_syscall(number, arg1, arg2, arg3, arg4, arg5, arg6)
        }
    }
}

// Fast getpid - no locks, just return cached value
fn sys_getpid_fast() -> isize {
    // In real implementation, would get from per-CPU data
    1 // Placeholder
}

// Fast gettimeofday using TSC
fn sys_gettimeofday_fast(tv_ptr: usize, _tz_ptr: usize) -> isize {
    if tv_ptr == 0 {
        return -14; // EFAULT
    }
    
    unsafe {
        let tv = tv_ptr as *mut TimeVal;
        let tsc = core::arch::x86_64::_rdtsc();
        let freq = 2_000_000_000u64; // 2GHz, should be calibrated
        
        let seconds = tsc / freq;
        let remainder = tsc % freq;
        let microseconds = (remainder * 1_000_000) / freq;
        
        (*tv).tv_sec = seconds as i64;
        (*tv).tv_usec = microseconds as i64;
    }
    
    0
}

// Fast clock_gettime using TSC
fn sys_clock_gettime_fast(clockid: usize, ts_ptr: usize) -> isize {
    if ts_ptr == 0 {
        return -14; // EFAULT
    }
    
    const CLOCK_MONOTONIC: usize = 1;
    const CLOCK_REALTIME: usize = 0;
    
    if clockid != CLOCK_MONOTONIC && clockid != CLOCK_REALTIME {
        return -22; // EINVAL
    }
    
    unsafe {
        let ts = ts_ptr as *mut TimeSpec;
        let tsc = core::arch::x86_64::_rdtsc();
        let freq = 2_000_000_000u64; // 2GHz, should be calibrated
        
        let seconds = tsc / freq;
        let remainder = tsc % freq;
        let nanoseconds = (remainder * 1_000_000_000) / freq;
        
        (*ts).tv_sec = seconds as i64;
        (*ts).tv_nsec = nanoseconds as i64;
    }
    
    0
}

// Handle batch syscalls
#[no_mangle]
extern "C" fn handle_batch_syscall_impl(batch_ptr: usize) -> isize {
    if batch_ptr == 0 {
        return -14; // EFAULT
    }
    
    unsafe {
        let batch = &mut *(batch_ptr as *mut SyscallBatch);
        
        for (i, request) in batch.syscalls.iter().enumerate() {
            let result = crate::syscall::handle_syscall(
                request.number,
                request.args[0],
                request.args[1],
                request.args[2],
                request.args[3],
                request.args[4],
                request.args[5],
            );
            
            if i < batch.results.len() {
                batch.results[i] = result;
            }
        }
        
        batch.syscalls.len() as isize
    }
}

// Initialize vDSO (virtual Dynamic Shared Object)
fn init_vdso() {
    // Allocate a page for vDSO
    let vdso_page = VirtAddr::new(0x7FFF_FFFF_F000);
    
    // Map vDSO page to all processes
    // This would contain userspace implementations of common syscalls
    
    // Set up vDSO data structure
    let vdso_data = VDsoData {
        version: 1,
        clock_gettime: vdso_clock_gettime,
        gettimeofday: vdso_gettimeofday,
        getpid: vdso_getpid,
        getcpu: vdso_getcpu,
        tsc_frequency: crate::timer::get_tsc_frequency(),
        tsc_offset: 0,
        kernel_version: [0; 64],
    };
    
    // Copy vDSO code and data to the page
    // This would be done during process creation
}

// vDSO implementations - run entirely in userspace
unsafe extern "C" fn vdso_clock_gettime(clockid: i32, ts: *mut TimeSpec) -> i32 {
    if ts.is_null() {
        return -14; // EFAULT
    }
    
    // Read TSC directly in userspace
    let tsc = core::arch::x86_64::_rdtsc();
    let freq = 2_000_000_000u64; // Should read from vDSO data
    
    let seconds = tsc / freq;
    let remainder = tsc % freq;
    let nanoseconds = (remainder * 1_000_000_000) / freq;
    
    (*ts).tv_sec = seconds as i64;
    (*ts).tv_nsec = nanoseconds as i64;
    
    0
}

unsafe extern "C" fn vdso_gettimeofday(tv: *mut TimeVal, _tz: *mut TimeZone) -> i32 {
    if tv.is_null() {
        return -14; // EFAULT
    }
    
    let tsc = core::arch::x86_64::_rdtsc();
    let freq = 2_000_000_000u64;
    
    let seconds = tsc / freq;
    let remainder = tsc % freq;
    let microseconds = (remainder * 1_000_000) / freq;
    
    (*tv).tv_sec = seconds as i64;
    (*tv).tv_usec = microseconds as i64;
    
    0
}

unsafe extern "C" fn vdso_getpid() -> i32 {
    // Would read from TLS or vDSO data area
    1 // Placeholder
}

unsafe extern "C" fn vdso_getcpu(cpu: *mut u32, node: *mut u32) -> i32 {
    // Read CPU ID using RDTSCP or from GS segment
    if !cpu.is_null() {
        *cpu = 0; // Placeholder
    }
    if !node.is_null() {
        *node = 0; // Placeholder
    }
    0
}

// vsyscall page for legacy compatibility
pub fn init_vsyscall() {
    // Map vsyscall page at fixed address
    let vsyscall_page = VirtAddr::new(0xFFFF_FFFF_FF60_0000);
    
    // Set up trampoline to vDSO functions
    // This provides backward compatibility for old binaries
}

// Syscall statistics for profiling
#[derive(Default)]
pub struct SyscallStats {
    pub count: u64,
    pub cycles: u64,
    pub fast_path_hits: u64,
    pub batch_count: u64,
}

static mut SYSCALL_STATS: [SyscallStats; 512] = [const { SyscallStats {
    count: 0,
    cycles: 0,
    fast_path_hits: 0,
    batch_count: 0,
}}; 512];

pub fn get_syscall_stats(syscall_num: usize) -> &'static SyscallStats {
    unsafe {
        &SYSCALL_STATS[syscall_num.min(511)]
    }
}

pub fn print_syscall_stats() {
    println!("Syscall Statistics:");
    println!("Number | Count      | Avg Cycles | Fast Path  | Batched");
    println!("-------|------------|------------|------------|--------");
    
    unsafe {
        for (i, stats) in SYSCALL_STATS.iter().enumerate() {
            if stats.count > 0 {
                let avg_cycles = stats.cycles / stats.count;
                println!("{:6} | {:10} | {:10} | {:10} | {:7}",
                    i, stats.count, avg_cycles, stats.fast_path_hits, stats.batch_count);
            }
        }
    }
}