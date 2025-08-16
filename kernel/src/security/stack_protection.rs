use crate::serial_println;
use x86_64::{VirtAddr, structures::paging::{PageTableFlags, Page, Size4KiB, FrameAllocator, Mapper}};
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use spin::Mutex;
use alloc::vec::Vec;
use core::arch::asm;

const STACK_CANARY_MAGIC: u64 = 0xDEADBEEF_CAFEBABE;
const GUARD_PAGE_SIZE: u64 = 0x1000; // 4KB
const SHADOW_STACK_SIZE: u64 = 0x10000; // 64KB per thread

static CANARIES_ENABLED: AtomicBool = AtomicBool::new(false);
static GUARD_PAGES_ENABLED: AtomicBool = AtomicBool::new(false);
static SHADOW_STACK_ENABLED: AtomicBool = AtomicBool::new(false);
static GLOBAL_CANARY: AtomicU64 = AtomicU64::new(0);

#[repr(C)]
pub struct StackFrame {
    pub canary: u64,
    pub return_address: u64,
    pub base_pointer: u64,
}

#[derive(Clone)]
pub struct ThreadStackInfo {
    pub stack_base: VirtAddr,
    pub stack_size: u64,
    pub guard_page_addr: Option<VirtAddr>,
    pub shadow_stack_addr: Option<VirtAddr>,
    pub canary_value: u64,
}

static THREAD_STACKS: Mutex<Vec<ThreadStackInfo>> = Mutex::new(Vec::new());

pub fn init_canaries() {
    if CANARIES_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    
    // Generate global canary value
    let canary = generate_canary();
    GLOBAL_CANARY.store(canary, Ordering::SeqCst);
    
    // Set up canary in GS segment for fast access
    unsafe {
        setup_gs_canary(canary);
    }
    
    CANARIES_ENABLED.store(true, Ordering::SeqCst);
    serial_println!("[STACK] Stack canaries initialized");
}

fn generate_canary() -> u64 {
    let mut canary = STACK_CANARY_MAGIC;
    
    // Mix with RDTSC
    unsafe {
        let tsc: u64;
        asm!("rdtsc", out("rax") tsc, out("rdx") _);
        canary ^= tsc;
    }
    
    // Mix with RDRAND if available
    if crate::cpu::get_info().has_rdrand() {
        unsafe {
            let mut random: u64;
            asm!(
                "rdrand {}",
                out(reg) random,
                options(nomem, nostack)
            );
            canary ^= random;
        }
    }
    
    // Ensure canary has terminator bytes to prevent string operations
    canary |= 0x00FF_0000_0000_0000; // Add null byte
    canary
}

unsafe fn setup_gs_canary(canary: u64) {
    // Store canary at GS:0x28 (standard location)
    asm!(
        "mov gs:[0x28], {}",
        in(reg) canary,
        options(nostack)
    );
}

#[inline(always)]
pub fn check_canary() -> bool {
    if !CANARIES_ENABLED.load(Ordering::SeqCst) {
        return true;
    }
    
    let stored_canary: u64;
    unsafe {
        asm!(
            "mov {}, gs:[0x28]",
            out(reg) stored_canary,
            options(nomem, nostack, pure)
        );
    }
    
    let expected = GLOBAL_CANARY.load(Ordering::SeqCst);
    if stored_canary != expected {
        panic!("Stack canary violation detected! Possible buffer overflow.");
    }
    
    true
}

#[macro_export]
macro_rules! stack_protect {
    ($body:block) => {{
        let _canary = $crate::security::stack_protection::push_canary();
        let result = $body;
        $crate::security::stack_protection::pop_canary(_canary);
        result
    }};
}

pub fn push_canary() -> u64 {
    if !CANARIES_ENABLED.load(Ordering::SeqCst) {
        return 0;
    }
    
    let canary = GLOBAL_CANARY.load(Ordering::SeqCst);
    unsafe {
        asm!(
            "push {}",
            in(reg) canary,
            options(nostack)
        );
    }
    canary
}

pub fn pop_canary(expected: u64) {
    if !CANARIES_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    
    let actual: u64;
    unsafe {
        asm!(
            "pop {}",
            out(reg) actual,
            options(nostack)
        );
    }
    
    if actual != expected {
        panic!("Stack canary mismatch! Stack corruption detected.");
    }
}

pub fn init_guard_pages() {
    if GUARD_PAGES_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    
    GUARD_PAGES_ENABLED.store(true, Ordering::SeqCst);
    serial_println!("[STACK] Guard pages initialized");
}

pub fn create_stack_with_guard(size: u64) -> Result<ThreadStackInfo, &'static str> {
    // Allocate stack memory
    let stack_pages = (size + 0xFFF) / 0x1000;
    let total_pages = stack_pages + 1; // +1 for guard page
    
    // Get physical frames for the stack
    let mut mapper = unsafe { crate::memory::get_mapper() };
    let frame_allocator = unsafe { crate::memory::get_frame_allocator() };
    
    // Calculate addresses
    let guard_page_addr = crate::security::kaslr::randomize_stack_base();
    let stack_base = guard_page_addr + GUARD_PAGE_SIZE;
    
    // Map stack pages
    for i in 0..stack_pages {
        let page = Page::<Size4KiB>::containing_address(stack_base + (i * 0x1000));
        let frame = frame_allocator
            .allocate_frame()
            .ok_or("Failed to allocate frame for stack")?;
        
        let flags = PageTableFlags::PRESENT 
            | PageTableFlags::WRITABLE 
            | PageTableFlags::NO_EXECUTE;
        
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)
                .map_err(|_| "Failed to map stack page")?
                .flush();
        }
    }
    
    // Guard page remains unmapped (accessing it will cause page fault)
    
    let mut info = ThreadStackInfo {
        stack_base,
        stack_size: size,
        guard_page_addr: Some(guard_page_addr),
        shadow_stack_addr: None,
        canary_value: generate_canary(),
    };
    
    // Initialize shadow stack if enabled
    if SHADOW_STACK_ENABLED.load(Ordering::SeqCst) {
        info.shadow_stack_addr = Some(create_shadow_stack()?);
    }
    
    THREAD_STACKS.lock().push(info.clone());
    
    Ok(info)
}

pub fn init_shadow_stack() -> bool {
    // Check for Intel CET support
    if !check_cet_support() {
        serial_println!("[STACK] Shadow stack not supported (no Intel CET)");
        return false;
    }
    
    SHADOW_STACK_ENABLED.store(true, Ordering::SeqCst);
    
    // Enable shadow stack in CR4
    unsafe {
        enable_cet();
    }
    
    serial_println!("[STACK] Shadow stack (Intel CET) enabled");
    true
}

fn check_cet_support() -> bool {
    // Check CPUID for CET support
    unsafe {
        let result: u32;
        let ecx_out: u32;
        asm!(
            "push rbx",
            "cpuid",
            "pop rbx",
            inout("eax") 7u32 => _,
            inout("ecx") 0u32 => ecx_out,
            out("edx") _,
            options(preserves_flags),
        );
        result = ecx_out;
        
        // Check CET_SS bit (bit 7 of ECX)
        (result & (1 << 7)) != 0
    }
}

unsafe fn enable_cet() {
    // Enable CET in CR4 (bit 23)
    let mut cr4: u64;
    asm!("mov {}, cr4", out(reg) cr4);
    cr4 |= 1 << 23;
    asm!("mov cr4, {}", in(reg) cr4);
    
    // Enable shadow stack in IA32_U_CET MSR
    let cet_msr = 0x6A0;
    let mut cet_value: u64 = 0;
    
    // Read current value
    asm!(
        "rdmsr",
        in("ecx") cet_msr,
        out("eax") cet_value,
        out("edx") _,
    );
    
    // Set SH_STK_EN (bit 0) and WR_SHSTK_EN (bit 1)
    cet_value |= 0x3;
    
    // Write back
    asm!(
        "wrmsr",
        in("ecx") cet_msr,
        in("eax") cet_value as u32,
        in("edx") (cet_value >> 32) as u32,
    );
}

fn create_shadow_stack() -> Result<VirtAddr, &'static str> {
    let pages = (SHADOW_STACK_SIZE + 0xFFF) / 0x1000;
    let mut mapper = unsafe { crate::memory::get_mapper() };
    let frame_allocator = unsafe { crate::memory::get_frame_allocator() };
    
    // Shadow stack grows down, so allocate from a high address
    let shadow_stack_base = VirtAddr::new(0xFFFF_FE00_0000_0000);
    
    for i in 0..pages {
        let page = Page::<Size4KiB>::containing_address(shadow_stack_base + (i * 0x1000));
        let frame = frame_allocator
            .allocate_frame()
            .ok_or("Failed to allocate frame for shadow stack")?;
        
        // Shadow stack pages need special flags
        let flags = PageTableFlags::PRESENT 
            | PageTableFlags::WRITABLE
            | PageTableFlags::NO_EXECUTE
            | PageTableFlags::BIT_11; // Shadow stack flag
        
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)
                .map_err(|_| "Failed to map shadow stack page")?
                .flush();
        }
    }
    
    // Initialize shadow stack pointer
    let ssp = shadow_stack_base + SHADOW_STACK_SIZE - 8u64;
    unsafe {
        asm!(
            "wrssq [{}], {}",
            in(reg) ssp.as_u64(),
            in(reg) 0u64,
        );
    }
    
    Ok(shadow_stack_base)
}

pub fn verify_return_address(expected_return: u64) {
    if !SHADOW_STACK_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    
    // Read from shadow stack
    let shadow_return: u64;
    unsafe {
        asm!(
            "rdsspq {}",
            out(reg) shadow_return,
        );
    }
    
    if shadow_return != expected_return {
        panic!("Return address mismatch! Possible ROP attack detected.");
    }
}

pub fn handle_stack_overflow() {
    serial_println!("[STACK] Stack overflow detected!");
    
    // Log the violation
    if let Some(audit) = crate::security::audit::get_auditor() {
        audit.log_security_event(
            crate::security::audit::SecurityEvent::StackOverflow,
            "Stack guard page violation detected"
        );
    }
    
    // Terminate the offending thread/process
    panic!("Stack overflow - terminating process");
}

pub fn validate_stack_pointer(rsp: VirtAddr) -> bool {
    let stacks = THREAD_STACKS.lock();
    
    for stack in stacks.iter() {
        if rsp >= stack.stack_base && rsp < (stack.stack_base + stack.stack_size) {
            // Check if we're not in the guard page
            if let Some(guard) = stack.guard_page_addr {
                if rsp >= guard && rsp < (guard + GUARD_PAGE_SIZE) {
                    return false; // In guard page!
                }
            }
            return true;
        }
    }
    
    false // Not in any known stack
}