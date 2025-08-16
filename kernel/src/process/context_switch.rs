// Context switching implementation for x86_64 with modern CPU optimizations
use super::pcb::{CpuContext, ExtendedState};
use crate::cpu::{get_info, CpuFeatures};
use core::arch::asm;
use core::mem::MaybeUninit;
use bitflags::bitflags;

bitflags! {
    pub struct XSaveFeatures: u64 {
        const X87 = 1 << 0;          // x87 FPU state
        const SSE = 1 << 1;          // SSE state
        const AVX = 1 << 2;          // AVX state
        const MPX_BNDREGS = 1 << 3;  // MPX bounds registers
        const MPX_BNDCSR = 1 << 4;   // MPX bounds config
        const AVX512_OPMASK = 1 << 5; // AVX-512 opmask
        const AVX512_ZMM_HI256 = 1 << 6; // AVX-512 ZMM upper 256 bits
        const AVX512_ZMM_HI16 = 1 << 7;  // AVX-512 ZMM16-31
        const PKRU = 1 << 9;          // Protection keys
    }
}

// XSAVE area structure for FPU/SSE/AVX state
#[repr(C, align(64))]
pub struct XSaveArea {
    legacy_region: [u8; 512],      // Legacy x87/SSE region
    xsave_header: XSaveHeader,      // XSAVE header
    extended_region: [u8; 2560],   // Extended state (AVX, etc.)
}

#[repr(C)]
struct XSaveHeader {
    xstate_bv: u64,     // State components in use
    xcomp_bv: u64,      // Compaction mode
    reserved: [u64; 6],
}

impl XSaveArea {
    pub const fn new() -> Self {
        Self {
            legacy_region: [0; 512],
            xsave_header: XSaveHeader {
                xstate_bv: 0,
                xcomp_bv: 0,
                reserved: [0; 6],
            },
            extended_region: [0; 2560],
        }
    }
}

// Thread-local storage for lazy FPU state
static mut CURRENT_FPU_OWNER: Option<usize> = None;

// Fast context switch flags
pub struct ContextSwitchFlags {
    pub save_fpu: bool,
    pub use_xsave: bool,
    pub use_fsgsbase: bool,
    pub use_pcid: bool,
}

// Optimized context save using modern CPU features
#[no_mangle]
pub unsafe extern "C" fn save_context_optimized(context: *mut CpuContext, flags: &ContextSwitchFlags) {
    let cpu_info = get_info();
    
    // Use FSGSBASE if available for faster FS/GS handling
    if flags.use_fsgsbase && cpu_info.features.contains(CpuFeatures::FSGSBASE) {
        asm!(
            // Save FS and GS base using RDFSBASE/RDGSBASE
            "rdfsbase rax",
            "mov [rdi + 0xA0], rax",  // Save FS base
            "rdgsbase rax",
            "mov [rdi + 0xA8], rax",  // Save GS base
            in("rdi") context,
            out("rax") _,
            options(nostack)
        );
    }
    
    // Main context save
    asm!(
        // Optimized register save using fewer memory operations
        "push rbx",
        "push rbp",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        
        // Save all general purpose registers efficiently
        "mov [rdi + 0x00], rax",
        "mov [rdi + 0x08], rbx",
        "mov [rdi + 0x10], rcx",
        "mov [rdi + 0x18], rdx",
        "mov [rdi + 0x20], rsi",
        "mov [rdi + 0x28], rdi",
        "mov [rdi + 0x30], rbp",
        "mov [rdi + 0x38], rsp",
        "mov [rdi + 0x40], r8",
        "mov [rdi + 0x48], r9",
        "mov [rdi + 0x50], r10",
        "mov [rdi + 0x58], r11",
        "mov [rdi + 0x60], r12",
        "mov [rdi + 0x68], r13",
        "mov [rdi + 0x70], r14",
        "mov [rdi + 0x78], r15",
        
        // Restore callee-saved registers
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbp",
        "pop rbx",
        
        // Save instruction pointer (return address)
        "mov rax, [rsp]",
        "mov [rdi + 0x80], rax",
        
        // Save flags
        "pushfq",
        "pop rax",
        "mov [rdi + 0x88], rax",
        
        // Save segment registers
        "mov ax, cs",
        "mov [rdi + 0x90], ax",
        "mov ax, ds",
        "mov [rdi + 0x92], ax",
        "mov ax, es",
        "mov [rdi + 0x94], ax",
        "mov ax, fs",
        "mov [rdi + 0x96], ax",
        "mov ax, gs",
        "mov [rdi + 0x98], ax",
        "mov ax, ss",
        "mov [rdi + 0x9A], ax",
        
        // Save CR3 with PCID if available
        "mov rax, cr3",
        "mov [rdi + 0x9C], rax",
        
        in("rdi") context,
        out("rax") _,
        options(nostack)
    );
    
    // Lazy FPU save - only save if this thread owns FPU
    if flags.save_fpu {
        let thread_id = (*context).thread_id;
        if CURRENT_FPU_OWNER == Some(thread_id) {
            save_fpu_state(context, flags.use_xsave);
        }
    }
}

// Original save_context for compatibility
#[no_mangle]
pub unsafe extern "C" fn save_context(context: *mut CpuContext) {
    let flags = ContextSwitchFlags {
        save_fpu: false,
        use_xsave: false,
        use_fsgsbase: false,
        use_pcid: false,
    };
    save_context_optimized(context, &flags);
}

// Save FPU/SSE/AVX state using XSAVE or FXSAVE
unsafe fn save_fpu_state(context: *mut CpuContext, use_xsave: bool) {
    let cpu_info = get_info();
    
    if use_xsave && cpu_info.features.contains(CpuFeatures::XSAVE) {
        // Use XSAVE for modern processors
        if let Some(xsave_area) = (*context).xsave_area.as_mut() {
            asm!(
                "xsave64 [{}]",
                in(reg) xsave_area,
                in("eax") 0xFFFFFFFFu32,  // Save all components
                in("edx") 0xFFFFFFFFu32,
            );
        }
    } else {
        // Fall back to FXSAVE for older processors
        if let Some(fpu_state) = (*context).fpu_state.as_mut() {
            asm!(
                "fxsave64 [{}]",
                in(reg) fpu_state,
            );
        }
    }
}

// Restore FPU/SSE/AVX state
unsafe fn restore_fpu_state(context: *const CpuContext, use_xsave: bool) {
    let cpu_info = get_info();
    let thread_id = (*context).thread_id;
    
    // Update FPU owner
    CURRENT_FPU_OWNER = Some(thread_id);
    
    if use_xsave && cpu_info.features.contains(CpuFeatures::XSAVE) {
        // Use XRSTOR for modern processors
        if let Some(xsave_area) = (*context).xsave_area.as_ref() {
            asm!(
                "xrstor64 [{}]",
                in(reg) xsave_area,
                in("eax") 0xFFFFFFFFu32,
                in("edx") 0xFFFFFFFFu32,
            );
        }
    } else {
        // Fall back to FXRSTOR
        if let Some(fpu_state) = (*context).fpu_state.as_ref() {
            asm!(
                "fxrstor64 [{}]",
                in(reg) fpu_state,
            );
        }
    }
}

// Optimized context restore with modern CPU features
#[no_mangle]
pub unsafe extern "C" fn restore_context_optimized(context: *const CpuContext, flags: &ContextSwitchFlags) -> ! {
    let cpu_info = get_info();
    
    // Restore FS/GS base if FSGSBASE is available
    if flags.use_fsgsbase && cpu_info.features.contains(CpuFeatures::FSGSBASE) {
        asm!(
            "mov rax, [rdi + 0xA0]",
            "wrfsbase rax",
            "mov rax, [rdi + 0xA8]",
            "wrgsbase rax",
            in("rdi") context,
            out("rax") _,
            options(nostack)
        );
    }
    
    asm!(
        // Load CR3 with PCID optimization if available
        "mov rax, [rdi + 0x9C]",
        "mov cr3, rax",
        
        // Restore segment registers (except CS - done by iretq)
        "mov ax, [rdi + 0x92]",
        "mov ds, ax",
        "mov ax, [rdi + 0x94]",
        "mov es, ax",
        "mov ax, [rdi + 0x96]",
        "mov fs, ax",
        "mov ax, [rdi + 0x98]",
        "mov gs, ax",
        "mov ax, [rdi + 0x9A]",
        "mov ss, ax",
        
        // Restore general purpose registers (except RSP)
        "mov rax, [rdi + 0x00]",
        "mov rbx, [rdi + 0x08]",
        "mov rcx, [rdi + 0x10]",
        "mov rdx, [rdi + 0x18]",
        "mov rsi, [rdi + 0x20]",
        // Skip RDI for now (it contains context pointer)
        "mov rbp, [rdi + 0x30]",
        // Skip RSP (restored last)
        "mov r8,  [rdi + 0x40]",
        "mov r9,  [rdi + 0x48]",
        "mov r10, [rdi + 0x50]",
        "mov r11, [rdi + 0x58]",
        "mov r12, [rdi + 0x60]",
        "mov r13, [rdi + 0x68]",
        "mov r14, [rdi + 0x70]",
        "mov r15, [rdi + 0x78]",
        
        // Prepare stack for iretq
        "mov rsp, [rdi + 0x38]",  // Restore stack pointer
        
        // Push values for iretq (in reverse order)
        "push qword ptr [rdi + 0x9A]",  // SS
        "push qword ptr [rdi + 0x38]",  // RSP
        "push qword ptr [rdi + 0x88]",  // RFLAGS
        "push qword ptr [rdi + 0x90]",  // CS
        "push qword ptr [rdi + 0x80]",  // RIP
        
        // Restore RDI last
        "mov rdi, [rdi + 0x28]",
        
        // Return to saved context
        "iretq",
        in("rdi") context,
        options(noreturn, nostack)
    );
}

// Original restore_context for compatibility
#[no_mangle]
pub unsafe extern "C" fn restore_context(context: *const CpuContext) -> ! {
    let flags = ContextSwitchFlags {
        save_fpu: false,
        use_xsave: false,
        use_fsgsbase: false,
        use_pcid: false,
    };
    restore_context_optimized(context, &flags);
}

// Optimized context switch with lazy FPU and modern CPU features
pub unsafe fn switch_context_fast(from: *mut CpuContext, to: *const CpuContext) {
    // Start performance monitoring
    let probe = crate::perf::ContextSwitchProbe::start();
    
    let cpu_info = get_info();
    
    let flags = ContextSwitchFlags {
        save_fpu: true,  // Enable lazy FPU
        use_xsave: cpu_info.features.contains(CpuFeatures::XSAVE),
        use_fsgsbase: cpu_info.features.contains(CpuFeatures::FSGSBASE),
        use_pcid: cpu_info.features.contains(CpuFeatures::PCID),
    };
    
    // Save minimal context (lazy FPU)
    save_context_optimized(from, &flags);
    
    // Update performance counters
    (*from).perf_counters.context_switches += 1;
    
    // Check if we need to flush TLB (PCID optimization)
    if !flags.use_pcid || (*from).cr3 != (*to).cr3 {
        // TLB flush required
        asm!("invlpg [0]", options(nostack));
        (*to).perf_counters.tlb_flushes += 1;
    }
    
    // Track NUMA migration if needed
    let from_cpu = crate::cpu::get_cpu_id();
    let to_cpu = (*to).thread_id as u32 % 16; // Approximate CPU from thread ID
    if from_cpu != to_cpu {
        let from_node = crate::numa::NUMA_TOPOLOGY.get_node_for_cpu(from_cpu);
        let to_node = crate::numa::NUMA_TOPOLOGY.get_node_for_cpu(to_cpu);
        if from_node != to_node {
            crate::numa::NUMA_STATS.record_migration();
        }
    }
    
    restore_context_optimized(to, &flags);
    
    // End performance monitoring
    probe.end();
}

// Simple context switch between two contexts (compatibility)
pub unsafe fn switch_context(from: *mut CpuContext, to: *const CpuContext) {
    switch_context_fast(from, to);
}

// Initialize a new context for a process
pub fn init_context(
    context: &mut CpuContext,
    entry_point: u64,
    stack_pointer: u64,
    is_kernel: bool,
) {
    // Set instruction pointer to entry point
    context.rip = entry_point;
    
    // Set stack pointer
    context.rsp = stack_pointer;
    context.rbp = stack_pointer;
    
    // Set up segments
    if is_kernel {
        context.cs = 0x08;  // Kernel code segment
        context.ss = 0x10;  // Kernel data segment
    } else {
        context.cs = 0x23;  // User code segment (0x20 | 3)
        context.ss = 0x1B;  // User data segment (0x18 | 3)
    }
    
    context.ds = context.ss;
    context.es = context.ss;
    context.fs = context.ss;
    context.gs = context.ss;
    
    // Enable interrupts
    context.rflags = 0x202;
    
    // Clear other registers
    context.rax = 0;
    context.rbx = 0;
    context.rcx = 0;
    context.rdx = 0;
    context.rsi = 0;
    context.rdi = 0;
    context.r8 = 0;
    context.r9 = 0;
    context.r10 = 0;
    context.r11 = 0;
    context.r12 = 0;
    context.r13 = 0;
    context.r14 = 0;
    context.r15 = 0;
}

// Save current context and switch to scheduler
pub unsafe fn yield_to_scheduler(current_context: *mut CpuContext) {
    // This would save current context and jump to scheduler
    // The scheduler would then pick next process and restore its context
    save_context(current_context);
    
    // Call scheduler to pick next process
    // scheduler::schedule_next();
}

// Fork-like context copy for creating child processes
pub fn fork_context(parent: &CpuContext, child: &mut CpuContext) {
    // Copy parent context to child
    *child = *parent;
    
    // Child gets return value 0 from fork
    child.rax = 0;
    
    // Parent keeps its PID as return value (set by caller)
    
    // Mark FPU state as needing reload for child
    unsafe {
        if CURRENT_FPU_OWNER == Some(parent.thread_id) {
            // Child will need to reload FPU state when it first uses it
            CURRENT_FPU_OWNER = None;
        }
    }
}

// Handle FPU not available exception (Device Not Available - #NM)
pub unsafe extern "x86-interrupt" fn fpu_not_available_handler(
    _stack_frame: x86_64::structures::idt::InterruptStackFrame
) {
    // Clear TS flag in CR0 to enable FPU
    let mut cr0: u64;
    asm!("mov {}, cr0", out(reg) cr0);
    cr0 &= !(1 << 3);  // Clear TS (Task Switched) flag
    asm!("mov cr0, {}", in(reg) cr0);
    
    // Load FPU state for current thread if needed
    // This would be called when a thread first uses FPU after context switch
}

// TSS optimization for faster interrupt handling
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved1: u32,
    pub rsp0: u64,  // Stack pointer for privilege level 0
    pub rsp1: u64,  // Stack pointer for privilege level 1
    pub rsp2: u64,  // Stack pointer for privilege level 2
    reserved2: u64,
    pub ist: [u64; 7],  // Interrupt Stack Table
    reserved3: u64,
    reserved4: u16,
    pub iomap_base: u16,
}

impl TaskStateSegment {
    pub const fn new() -> Self {
        Self {
            reserved1: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            reserved2: 0,
            ist: [0; 7],
            reserved3: 0,
            reserved4: 0,
            iomap_base: 0,
        }
    }
    
    pub fn set_kernel_stack(&mut self, stack: u64) {
        self.rsp0 = stack;
    }
    
    pub fn set_interrupt_stack(&mut self, index: usize, stack: u64) {
        if index < 7 {
            self.ist[index] = stack;
        }
    }
}