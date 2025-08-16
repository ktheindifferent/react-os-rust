// Context switching implementation for x86_64
use super::pcb::CpuContext;
use core::arch::asm;

// Save current CPU context
#[no_mangle]
pub unsafe extern "C" fn save_context(context: *mut CpuContext) {
    core::arch::asm!(
        // Save general purpose registers
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
        
        // Save CR3 (page table base)
        "mov rax, cr3",
        "mov [rdi + 0x9C], rax",
        
        "ret"
    );
}

// Restore CPU context and jump to saved instruction pointer
#[no_mangle]
pub unsafe extern "C" fn restore_context(context: *const CpuContext) -> ! {
    core::arch::asm!(
        // Load CR3 (switch page tables)
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
        "iretq"
    );
    unreachable!();
}

// Simple context switch between two contexts
pub unsafe fn switch_context(from: *mut CpuContext, to: *const CpuContext) {
    save_context(from);
    restore_context(to);
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
}