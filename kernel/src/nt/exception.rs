use super::{NtStatus, object::{Handle, ObjectHeader, ObjectTrait, ObjectType}};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::VirtAddr;

// Windows exception codes - matching Windows NT exactly
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExceptionCode {
    // Standard exception codes
    AccessViolation = 0xC0000005,
    ArrayBoundsExceeded = 0xC000008C,
    BreakPoint = 0x80000003,
    DataTypeMisalignment = 0x80000002,
    FloatDenormalOperand = 0xC000008D,
    FloatDivideByZero = 0xC000008E,
    FloatInexactResult = 0xC000008F,
    FloatInvalidOperation = 0xC0000090,
    FloatOverflow = 0xC0000091,
    FloatStackCheck = 0xC0000092,
    FloatUnderflow = 0xC0000093,
    IllegalInstruction = 0xC000001D,
    InPageError = 0xC0000006,
    IntegerDivideByZero = 0xC0000094,
    IntegerOverflow = 0xC0000095,
    InvalidDisposition = 0xC0000026,
    InvalidHandle = 0xC0000008,
    NonContinuableException = 0xC0000025,
    PrivilegedInstruction = 0xC0000096,
    SingleStep = 0x80000004,
    StackOverflow = 0xC00000FD,
    
    // Kernel-specific exceptions
    KernelModeExceptionNotHandled = 0xC000009D,
    SystemServiceException = 0xC000009E,
    SystemThreadException = 0xC000009F,
    UnhandledException = 0xC00000A0,
    
    // Security exceptions
    SecurityViolation = 0xC0000022,
    TokenAlreadyInUse = 0xC000012B,
    
    // Memory exceptions
    InvalidPageProtection = 0xC0000045,
    NoMemory = 0xC0000017,
    PagefileQuota = 0xC0000007,
    CommitmentLimit = 0xC000012D,
    
    // Registry exceptions
    RegistryCorrupt = 0xC000014C,
    RegistryIOFailed = 0xC000014D,
    RegistryRecovered = 0x40000009,
}

// Exception disposition values
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionDisposition {
    ExceptionContinueExecution = 0,
    ExceptionContinueSearch = 1,
    ExceptionNestedException = 2,
    ExceptionCollidedUnwind = 3,
}

// Exception flags
bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct ExceptionFlags: u32 {
        const NONCONTINUABLE = 0x1;
        const UNWINDING = 0x2;
        const EXIT_UNWIND = 0x4;
        const STACK_INVALID = 0x8;
        const NESTED_CALL = 0x10;
        const TARGET_UNWIND = 0x20;
        const COLLIDED_UNWIND = 0x40;
        const UNWIND = 0x66; // UNWINDING | EXIT_UNWIND | TARGET_UNWIND
    }
}

// Exception record - Windows compatible
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ExceptionRecord {
    pub exception_code: ExceptionCode,
    pub exception_flags: ExceptionFlags,
    pub exception_record: Option<Box<ExceptionRecord>>, // Nested exception
    pub exception_address: VirtAddr,
    pub number_parameters: u32,
    pub exception_information: [u64; 15], // EXCEPTION_MAXIMUM_PARAMETERS
}

impl ExceptionRecord {
    pub fn new(code: ExceptionCode, address: VirtAddr) -> Self {
        Self {
            exception_code: code,
            exception_flags: ExceptionFlags::empty(),
            exception_record: None,
            exception_address: address,
            number_parameters: 0,
            exception_information: [0; 15],
        }
    }
    
    pub fn with_parameters(mut self, params: &[u64]) -> Self {
        let len = core::cmp::min(params.len(), 15);
        self.exception_information[..len].copy_from_slice(&params[..len]);
        self.number_parameters = len as u32;
        self
    }
    
    pub fn with_flags(mut self, flags: ExceptionFlags) -> Self {
        self.exception_flags = flags;
        self
    }
}

// Context record for x86_64 - Windows compatible
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ContextRecord {
    // Control flags
    pub context_flags: u32,
    
    // Debug registers
    pub dr0: u64,
    pub dr1: u64,
    pub dr2: u64,
    pub dr3: u64,
    pub dr6: u64,
    pub dr7: u64,
    
    // Floating point state
    pub flt_save: FloatingSaveArea,
    
    // Segment registers
    pub seg_gs: u16,
    pub seg_fs: u16,
    pub seg_es: u16,
    pub seg_ds: u16,
    
    // Integer registers
    pub rdi: u64,
    pub rsi: u64,
    pub rbx: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rax: u64,
    pub rbp: u64,
    pub rip: u64,
    pub seg_cs: u16,
    pub eflags: u32,
    pub rsp: u64,
    pub seg_ss: u16,
    
    // Extended registers
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

impl Default for ContextRecord {
    fn default() -> Self {
        Self {
            context_flags: 0,
            dr0: 0, dr1: 0, dr2: 0, dr3: 0, dr6: 0, dr7: 0,
            flt_save: FloatingSaveArea::default(),
            seg_gs: 0, seg_fs: 0, seg_es: 0, seg_ds: 0,
            rdi: 0, rsi: 0, rbx: 0, rdx: 0, rcx: 0, rax: 0,
            rbp: 0, rip: 0, seg_cs: 0, eflags: 0, rsp: 0, seg_ss: 0,
            r8: 0, r9: 0, r10: 0, r11: 0, r12: 0, r13: 0, r14: 0, r15: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct FloatingSaveArea {
    pub control_word: u16,
    pub status_word: u16,
    pub tag_word: u8,
    pub error_opcode: u8,
    pub error_offset: u32,
    pub error_selector: u16,
    pub data_offset: u32,
    pub data_selector: u16,
    pub register_area: [u8; 80],
    pub spare0: u32,
}

impl Default for FloatingSaveArea {
    fn default() -> Self {
        Self {
            control_word: 0,
            status_word: 0,
            tag_word: 0,
            error_opcode: 0,
            error_offset: 0,
            error_selector: 0,
            data_offset: 0,
            data_selector: 0,
            register_area: [0; 80],
            spare0: 0,
        }
    }
}

// Exception handler function type
pub type ExceptionHandler = fn(&mut ExceptionRecord, &mut ContextRecord) -> ExceptionDisposition;

// Exception frame for structured exception handling
#[repr(C)]
#[derive(Debug)]
pub struct ExceptionFrame {
    pub next: Option<*mut ExceptionFrame>,
    pub handler: ExceptionHandler,
}

// Exception dispatcher and manager
pub struct ExceptionManager {
    exception_handlers: BTreeMap<ExceptionCode, Vec<ExceptionHandler>>,
    global_handlers: Vec<ExceptionHandler>,
    kernel_debugger_enabled: bool,
    debug_break_on_exception: bool,
    exception_statistics: ExceptionStatistics,
}

#[derive(Debug, Clone)]
pub struct ExceptionStatistics {
    pub total_exceptions: u64,
    pub handled_exceptions: u64,
    pub unhandled_exceptions: u64,
    pub exception_counts: BTreeMap<ExceptionCode, u64>,
}

impl ExceptionManager {
    pub fn new() -> Self {
        Self {
            exception_handlers: BTreeMap::new(),
            global_handlers: Vec::new(),
            kernel_debugger_enabled: false,
            debug_break_on_exception: false,
            exception_statistics: ExceptionStatistics {
                total_exceptions: 0,
                handled_exceptions: 0,
                unhandled_exceptions: 0,
                exception_counts: BTreeMap::new(),
            },
        }
    }
    
    pub fn register_exception_handler(&mut self, code: ExceptionCode, handler: ExceptionHandler) {
        self.exception_handlers.entry(code).or_insert_with(Vec::new).push(handler);
    }
    
    pub fn register_global_handler(&mut self, handler: ExceptionHandler) {
        self.global_handlers.push(handler);
    }
    
    pub fn dispatch_exception(
        &mut self,
        exception_record: &mut ExceptionRecord,
        context: &mut ContextRecord,
    ) -> ExceptionDisposition {
        use crate::serial_println;
        
        self.exception_statistics.total_exceptions += 1;
        *self.exception_statistics.exception_counts
            .entry(exception_record.exception_code)
            .or_insert(0) += 1;
        
        serial_println!("Exception: Dispatching {:?} at {:?}", 
                       exception_record.exception_code, 
                       exception_record.exception_address);
        
        // First, try kernel debugger if enabled
        if self.kernel_debugger_enabled {
            if let Some(disposition) = self.handle_kernel_debugger_exception(exception_record, context) {
                return disposition;
            }
        }
        
        // Try specific handlers for this exception code
        if let Some(handlers) = self.exception_handlers.get(&exception_record.exception_code) {
            for handler in handlers {
                match handler(exception_record, context) {
                    ExceptionDisposition::ExceptionContinueExecution => {
                        self.exception_statistics.handled_exceptions += 1;
                        return ExceptionDisposition::ExceptionContinueExecution;
                    }
                    ExceptionDisposition::ExceptionContinueSearch => continue,
                    other => return other,
                }
            }
        }
        
        // Try global handlers
        for handler in &self.global_handlers {
            match handler(exception_record, context) {
                ExceptionDisposition::ExceptionContinueExecution => {
                    self.exception_statistics.handled_exceptions += 1;
                    return ExceptionDisposition::ExceptionContinueExecution;
                }
                ExceptionDisposition::ExceptionContinueSearch => continue,
                other => return other,
            }
        }
        
        // Default kernel exception handling
        self.handle_unhandled_exception(exception_record, context)
    }
    
    fn handle_kernel_debugger_exception(
        &self,
        exception_record: &ExceptionRecord,
        _context: &ContextRecord,
    ) -> Option<ExceptionDisposition> {
        use crate::serial_println;
        
        match exception_record.exception_code {
            ExceptionCode::BreakPoint => {
                serial_println!("KD: Breakpoint hit at {:?}", exception_record.exception_address);
                // In a real implementation, this would break to debugger
                Some(ExceptionDisposition::ExceptionContinueExecution)
            }
            ExceptionCode::SingleStep => {
                serial_println!("KD: Single step at {:?}", exception_record.exception_address);
                Some(ExceptionDisposition::ExceptionContinueExecution)
            }
            _ => {
                if self.debug_break_on_exception {
                    serial_println!("KD: Breaking on exception {:?}", exception_record.exception_code);
                    Some(ExceptionDisposition::ExceptionContinueSearch)
                } else {
                    None
                }
            }
        }
    }
    
    fn handle_unhandled_exception(
        &mut self,
        exception_record: &ExceptionRecord,
        context: &ContextRecord,
    ) -> ExceptionDisposition {
        use crate::{println, serial_println};
        
        self.exception_statistics.unhandled_exceptions += 1;
        
        println!("KERNEL PANIC: Unhandled Exception!");
        println!("Exception Code: {:?}", exception_record.exception_code);
        println!("Exception Address: {:?}", exception_record.exception_address);
        println!("Exception Flags: {:?}", exception_record.exception_flags);
        
        serial_println!("=== UNHANDLED EXCEPTION ===");
        serial_println!("Code: {:?}", exception_record.exception_code);
        serial_println!("Address: {:?}", exception_record.exception_address);
        serial_println!("Flags: {:?}", exception_record.exception_flags);
        serial_println!("Parameters: {}", exception_record.number_parameters);
        
        for i in 0..exception_record.number_parameters as usize {
            serial_println!("  Param[{}]: 0x{:016X}", i, exception_record.exception_information[i]);
        }
        
        serial_println!("=== CONTEXT RECORD ===");
        serial_println!("RIP: 0x{:016X}", context.rip);
        serial_println!("RSP: 0x{:016X}", context.rsp);
        serial_println!("RBP: 0x{:016X}", context.rbp);
        serial_println!("RAX: 0x{:016X}", context.rax);
        serial_println!("RBX: 0x{:016X}", context.rbx);
        serial_println!("RCX: 0x{:016X}", context.rcx);
        serial_println!("RDX: 0x{:016X}", context.rdx);
        
        // For critical exceptions, halt the system
        match exception_record.exception_code {
            ExceptionCode::AccessViolation |
            ExceptionCode::IllegalInstruction |
            ExceptionCode::StackOverflow |
            ExceptionCode::KernelModeExceptionNotHandled => {
                serial_println!("=== SYSTEM HALTED ===");
                loop {
                    x86_64::instructions::hlt();
                }
            }
            _ => ExceptionDisposition::ExceptionContinueSearch
        }
    }
    
    pub fn enable_kernel_debugger(&mut self, enable: bool) {
        self.kernel_debugger_enabled = enable;
    }
    
    pub fn set_debug_break_on_exception(&mut self, enable: bool) {
        self.debug_break_on_exception = enable;
    }
    
    pub fn get_statistics(&self) -> &ExceptionStatistics {
        &self.exception_statistics
    }
    
    pub fn install_default_handlers(&mut self) {
        use crate::serial_println;
        
        // Install default access violation handler
        self.register_exception_handler(ExceptionCode::AccessViolation, |record, _context| {
            serial_println!("Access Violation: Address {:?}, Params: [{:016X}, {:016X}]",
                           record.exception_address,
                           record.exception_information[0],
                           record.exception_information[1]);
            ExceptionDisposition::ExceptionContinueSearch
        });
        
        // Install default divide by zero handler
        self.register_exception_handler(ExceptionCode::IntegerDivideByZero, |record, _context| {
            serial_println!("Integer Divide by Zero at {:?}", record.exception_address);
            ExceptionDisposition::ExceptionContinueSearch
        });
        
        // Install default illegal instruction handler
        self.register_exception_handler(ExceptionCode::IllegalInstruction, |record, _context| {
            serial_println!("Illegal Instruction at {:?}", record.exception_address);
            ExceptionDisposition::ExceptionContinueSearch
        });
        
        // Install default stack overflow handler
        self.register_exception_handler(ExceptionCode::StackOverflow, |record, _context| {
            serial_println!("Stack Overflow at {:?}", record.exception_address);
            ExceptionDisposition::ExceptionContinueSearch
        });
        
        serial_println!("Exception: Default exception handlers installed");
    }
}

// Global exception manager
lazy_static! {
    pub static ref EXCEPTION_MANAGER: Mutex<ExceptionManager> = Mutex::new(ExceptionManager::new());
}

// Interrupt handlers for exceptions
pub fn handle_divide_error() {
    let mut exception_record = ExceptionRecord::new(
        ExceptionCode::IntegerDivideByZero,
        VirtAddr::new(0) // Would be filled with actual RIP
    );
    let mut context = ContextRecord::default();
    
    let mut manager = EXCEPTION_MANAGER.lock();
    manager.dispatch_exception(&mut exception_record, &mut context);
}

pub fn handle_debug_exception() {
    let mut exception_record = ExceptionRecord::new(
        ExceptionCode::SingleStep,
        VirtAddr::new(0)
    );
    let mut context = ContextRecord::default();
    
    let mut manager = EXCEPTION_MANAGER.lock();
    manager.dispatch_exception(&mut exception_record, &mut context);
}

pub fn handle_breakpoint_exception() {
    let mut exception_record = ExceptionRecord::new(
        ExceptionCode::BreakPoint,
        VirtAddr::new(0)
    );
    let mut context = ContextRecord::default();
    
    let mut manager = EXCEPTION_MANAGER.lock();
    manager.dispatch_exception(&mut exception_record, &mut context);
}

pub fn handle_overflow_exception() {
    let mut exception_record = ExceptionRecord::new(
        ExceptionCode::IntegerOverflow,
        VirtAddr::new(0)
    );
    let mut context = ContextRecord::default();
    
    let mut manager = EXCEPTION_MANAGER.lock();
    manager.dispatch_exception(&mut exception_record, &mut context);
}

pub fn handle_page_fault(error_code: u64, fault_address: VirtAddr) {
    let mut exception_record = ExceptionRecord::new(
        ExceptionCode::AccessViolation,
        fault_address
    ).with_parameters(&[
        error_code & 1, // 0 = read, 1 = write
        fault_address.as_u64(),
    ]);
    
    let mut context = ContextRecord::default();
    
    let mut manager = EXCEPTION_MANAGER.lock();
    manager.dispatch_exception(&mut exception_record, &mut context);
}

pub fn handle_general_protection_fault(error_code: u64) {
    let code = if error_code == 0 {
        ExceptionCode::IllegalInstruction
    } else {
        ExceptionCode::AccessViolation
    };
    
    let mut exception_record = ExceptionRecord::new(
        code,
        VirtAddr::new(0)
    ).with_parameters(&[error_code]);
    
    let mut context = ContextRecord::default();
    
    let mut manager = EXCEPTION_MANAGER.lock();
    manager.dispatch_exception(&mut exception_record, &mut context);
}

// Public API functions
pub fn initialize_exception_handling() -> NtStatus {
    use crate::serial_println;
    
    serial_println!("Exception: Initializing Windows-compatible exception handling");
    
    {
        let mut manager = EXCEPTION_MANAGER.lock();
        manager.install_default_handlers();
        manager.enable_kernel_debugger(true);
        manager.set_debug_break_on_exception(false);
    }
    
    serial_println!("Exception: Exception handling system initialized");
    NtStatus::Success
}

pub fn register_exception_handler(code: ExceptionCode, handler: ExceptionHandler) -> NtStatus {
    let mut manager = EXCEPTION_MANAGER.lock();
    manager.register_exception_handler(code, handler);
    NtStatus::Success
}

pub fn raise_exception(exception_record: &mut ExceptionRecord, context: &mut ContextRecord) -> ExceptionDisposition {
    let mut manager = EXCEPTION_MANAGER.lock();
    manager.dispatch_exception(exception_record, context)
}

pub fn get_exception_statistics() -> ExceptionStatistics {
    let manager = EXCEPTION_MANAGER.lock();
    manager.get_statistics().clone()
}

// NT API functions for exception handling
pub fn nt_raise_exception(
    exception_record: &mut ExceptionRecord,
    context_record: &mut ContextRecord,
    first_chance: bool,
) -> NtStatus {
    if first_chance {
        // First chance - try user mode handlers first
        // For kernel mode, we go straight to kernel handlers
    }
    
    let _disposition = raise_exception(exception_record, context_record);
    NtStatus::Success
}

pub fn nt_continue(context_record: &ContextRecord) -> NtStatus {
    // Restore context and continue execution
    // In a real implementation, this would restore CPU state
    let _ctx = context_record;
    NtStatus::Success
}

pub fn nt_get_context_thread(
    thread_handle: Handle,
    context: &mut ContextRecord,
) -> NtStatus {
    // Get thread context
    let _handle = thread_handle;
    *context = ContextRecord::default();
    NtStatus::Success
}

pub fn nt_set_context_thread(
    thread_handle: Handle,
    context: &ContextRecord,
) -> NtStatus {
    // Set thread context
    let _handle = thread_handle;
    let _ctx = context;
    NtStatus::Success
}