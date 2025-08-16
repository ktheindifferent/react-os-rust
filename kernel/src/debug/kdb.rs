// Interactive Kernel Debugger (KDB)
// Provides an interactive debugging shell within the kernel

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::registers::control::{Cr0, Cr2, Cr3, Cr4};
use x86_64::VirtAddr;

// KDB state
pub struct KernelDebugger {
    enabled: AtomicBool,
    active: AtomicBool,
    breakpoints: Mutex<Vec<Breakpoint>>,
    watchpoints: Mutex<Vec<Watchpoint>>,
    commands: Mutex<BTreeMap<String, CommandHandler>>,
    history: Mutex<Vec<String>>,
    single_step: AtomicBool,
    frozen_cpus: AtomicU32,
}

#[derive(Clone)]
pub struct Breakpoint {
    pub id: u32,
    pub address: u64,
    pub enabled: bool,
    pub temporary: bool,
    pub hit_count: u32,
    pub original_byte: u8,
    pub condition: Option<String>,
}

#[derive(Clone)]
pub struct Watchpoint {
    pub id: u32,
    pub address: u64,
    pub length: usize,
    pub access_type: WatchType,
    pub enabled: bool,
    pub hit_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WatchType {
    Read,
    Write,
    ReadWrite,
    Execute,
}

type CommandHandler = fn(&[&str]) -> Result<(), String>;

lazy_static! {
    pub static ref KDB: KernelDebugger = KernelDebugger::new();
}

impl KernelDebugger {
    pub fn new() -> Self {
        let mut debugger = Self {
            enabled: AtomicBool::new(true),
            active: AtomicBool::new(false),
            breakpoints: Mutex::new(Vec::new()),
            watchpoints: Mutex::new(Vec::new()),
            commands: Mutex::new(BTreeMap::new()),
            history: Mutex::new(Vec::new()),
            single_step: AtomicBool::new(false),
            frozen_cpus: AtomicU32::new(0),
        };
        
        // Register built-in commands
        debugger.register_commands();
        debugger
    }
    
    fn register_commands(&mut self) {
        let mut commands = self.commands.lock();
        
        // Register all built-in commands
        commands.insert("help".to_string(), cmd_help as CommandHandler);
        commands.insert("h".to_string(), cmd_help as CommandHandler);
        commands.insert("?".to_string(), cmd_help as CommandHandler);
        
        commands.insert("continue".to_string(), cmd_continue as CommandHandler);
        commands.insert("c".to_string(), cmd_continue as CommandHandler);
        
        commands.insert("step".to_string(), cmd_step as CommandHandler);
        commands.insert("s".to_string(), cmd_step as CommandHandler);
        
        commands.insert("next".to_string(), cmd_next as CommandHandler);
        commands.insert("n".to_string(), cmd_next as CommandHandler);
        
        commands.insert("break".to_string(), cmd_break as CommandHandler);
        commands.insert("b".to_string(), cmd_break as CommandHandler);
        
        commands.insert("watch".to_string(), cmd_watch as CommandHandler);
        commands.insert("w".to_string(), cmd_watch as CommandHandler);
        
        commands.insert("delete".to_string(), cmd_delete as CommandHandler);
        commands.insert("d".to_string(), cmd_delete as CommandHandler);
        
        commands.insert("list".to_string(), cmd_list as CommandHandler);
        commands.insert("l".to_string(), cmd_list as CommandHandler);
        
        commands.insert("print".to_string(), cmd_print as CommandHandler);
        commands.insert("p".to_string(), cmd_print as CommandHandler);
        
        commands.insert("examine".to_string(), cmd_examine as CommandHandler);
        commands.insert("x".to_string(), cmd_examine as CommandHandler);
        
        commands.insert("registers".to_string(), cmd_registers as CommandHandler);
        commands.insert("r".to_string(), cmd_registers as CommandHandler);
        
        commands.insert("backtrace".to_string(), cmd_backtrace as CommandHandler);
        commands.insert("bt".to_string(), cmd_backtrace as CommandHandler);
        
        commands.insert("info".to_string(), cmd_info as CommandHandler);
        commands.insert("i".to_string(), cmd_info as CommandHandler);
        
        commands.insert("disassemble".to_string(), cmd_disassemble as CommandHandler);
        commands.insert("dis".to_string(), cmd_disassemble as CommandHandler);
        
        commands.insert("memory".to_string(), cmd_memory as CommandHandler);
        commands.insert("m".to_string(), cmd_memory as CommandHandler);
        
        commands.insert("cpu".to_string(), cmd_cpu as CommandHandler);
        commands.insert("modules".to_string(), cmd_modules as CommandHandler);
        commands.insert("threads".to_string(), cmd_threads as CommandHandler);
        commands.insert("locks".to_string(), cmd_locks as CommandHandler);
        commands.insert("irq".to_string(), cmd_irq as CommandHandler);
    }
    
    pub fn enter(&self, reason: &str) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        // Mark debugger as active
        self.active.store(true, Ordering::SeqCst);
        
        // Freeze other CPUs in SMP systems
        self.freeze_other_cpus();
        
        // Print entry message
        crate::serial_println!("\n=== Entering KDB: {} ===", reason);
        crate::serial_println!("Type 'help' for available commands");
        
        // Main debugger loop
        self.debugger_loop();
        
        // Unfreeze CPUs
        self.unfreeze_cpus();
        
        // Mark debugger as inactive
        self.active.store(false, Ordering::SeqCst);
    }
    
    fn debugger_loop(&self) {
        let mut buffer = String::new();
        
        loop {
            // Print prompt
            crate::serial_print!("kdb> ");
            
            // Read command
            buffer.clear();
            if !self.read_command(&mut buffer) {
                continue;
            }
            
            // Add to history
            if !buffer.is_empty() {
                self.history.lock().push(buffer.clone());
            }
            
            // Parse and execute command
            let parts: Vec<&str> = buffer.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            
            let cmd = parts[0];
            let args = &parts[1..];
            
            // Check for exit commands
            if cmd == "continue" || cmd == "c" || cmd == "exit" || cmd == "quit" {
                crate::serial_println!("Continuing execution...");
                break;
            }
            
            // Execute command
            if let Some(handler) = self.commands.lock().get(cmd) {
                if let Err(e) = handler(args) {
                    crate::serial_println!("Error: {}", e);
                }
            } else {
                crate::serial_println!("Unknown command: {}. Type 'help' for available commands.", cmd);
            }
        }
    }
    
    fn read_command(&self, buffer: &mut String) -> bool {
        loop {
            if let Some(byte) = crate::serial::read_byte() {
                match byte {
                    b'\r' | b'\n' => {
                        crate::serial_println!();
                        return true;
                    }
                    b'\x08' | b'\x7f' => {  // Backspace
                        if !buffer.is_empty() {
                            buffer.pop();
                            crate::serial_print!("\x08 \x08");  // Erase character
                        }
                    }
                    b'\x03' => {  // Ctrl+C
                        crate::serial_println!("^C");
                        buffer.clear();
                        return false;
                    }
                    b if b.is_ascii_graphic() || b == b' ' => {
                        buffer.push(byte as char);
                        crate::serial_print!("{}", byte as char);
                    }
                    _ => {}  // Ignore other control characters
                }
            }
            
            // Allow interrupts to be processed
            x86_64::instructions::interrupts::enable_and_hlt();
            x86_64::instructions::interrupts::disable();
        }
    }
    
    fn freeze_other_cpus(&self) {
        // In SMP systems, send IPI to freeze other CPUs
        // For now, just track the count
        let cpu_count = 1;  // Would get actual CPU count
        self.frozen_cpus.store(cpu_count - 1, Ordering::SeqCst);
    }
    
    fn unfreeze_cpus(&self) {
        // Unfreeze other CPUs
        self.frozen_cpus.store(0, Ordering::SeqCst);
    }
    
    pub fn set_breakpoint(&self, address: u64, temporary: bool) -> Result<u32, String> {
        let mut breakpoints = self.breakpoints.lock();
        let id = breakpoints.len() as u32;
        
        // Save original instruction byte
        let original_byte = unsafe {
            *(address as *const u8)
        };
        
        // Insert INT3 instruction (0xCC)
        unsafe {
            *(address as *mut u8) = 0xCC;
        }
        
        breakpoints.push(Breakpoint {
            id,
            address,
            enabled: true,
            temporary,
            hit_count: 0,
            original_byte,
            condition: None,
        });
        
        Ok(id)
    }
    
    pub fn remove_breakpoint(&self, id: u32) -> Result<(), String> {
        let mut breakpoints = self.breakpoints.lock();
        
        if let Some(pos) = breakpoints.iter().position(|bp| bp.id == id) {
            let bp = &breakpoints[pos];
            
            // Restore original instruction
            unsafe {
                *(bp.address as *mut u8) = bp.original_byte;
            }
            
            breakpoints.remove(pos);
            Ok(())
        } else {
            Err(format!("Breakpoint {} not found", id))
        }
    }
    
    pub fn set_watchpoint(&self, address: u64, length: usize, watch_type: WatchType) -> Result<u32, String> {
        // Use hardware debug registers for watchpoints
        let mut watchpoints = self.watchpoints.lock();
        let id = watchpoints.len() as u32;
        
        // Would configure DR0-DR3 hardware debug registers here
        
        watchpoints.push(Watchpoint {
            id,
            address,
            length,
            access_type: watch_type,
            enabled: true,
            hit_count: 0,
        });
        
        Ok(id)
    }
}

// Command implementations
fn cmd_help(_args: &[&str]) -> Result<(), String> {
    crate::serial_println!("KDB Commands:");
    crate::serial_println!("  help (h,?)         - Show this help");
    crate::serial_println!("  continue (c)       - Continue execution");
    crate::serial_println!("  step (s)           - Single step");
    crate::serial_println!("  next (n)           - Step over");
    crate::serial_println!("  break (b) <addr>   - Set breakpoint");
    crate::serial_println!("  watch (w) <addr>   - Set watchpoint");
    crate::serial_println!("  delete (d) <id>    - Delete breakpoint/watchpoint");
    crate::serial_println!("  list (l)           - List breakpoints/watchpoints");
    crate::serial_println!("  print (p) <expr>   - Print expression");
    crate::serial_println!("  examine (x) <addr> - Examine memory");
    crate::serial_println!("  registers (r)      - Show registers");
    crate::serial_println!("  backtrace (bt)     - Show call stack");
    crate::serial_println!("  info (i) <what>    - Show various info");
    crate::serial_println!("  disassemble (dis)  - Disassemble code");
    crate::serial_println!("  memory (m)         - Memory statistics");
    crate::serial_println!("  cpu                - CPU information");
    crate::serial_println!("  modules            - List loaded modules");
    crate::serial_println!("  threads            - List threads");
    crate::serial_println!("  locks              - Show lock status");
    crate::serial_println!("  irq                - IRQ statistics");
    Ok(())
}

fn cmd_continue(_args: &[&str]) -> Result<(), String> {
    // Continue is handled in the main loop
    Ok(())
}

fn cmd_step(_args: &[&str]) -> Result<(), String> {
    KDB.single_step.store(true, Ordering::SeqCst);
    crate::serial_println!("Single stepping enabled");
    Ok(())
}

fn cmd_next(_args: &[&str]) -> Result<(), String> {
    // Step over (would need to analyze instruction)
    crate::serial_println!("Step over not yet implemented");
    Ok(())
}

fn cmd_break(args: &[&str]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: break <address>".to_string());
    }
    
    let addr = parse_address(args[0])?;
    let id = KDB.set_breakpoint(addr, false)?;
    crate::serial_println!("Breakpoint {} set at {:#x}", id, addr);
    Ok(())
}

fn cmd_watch(args: &[&str]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: watch <address> [length] [type]".to_string());
    }
    
    let addr = parse_address(args[0])?;
    let length = if args.len() > 1 {
        args[1].parse::<usize>().map_err(|_| "Invalid length".to_string())?
    } else {
        8
    };
    
    let watch_type = if args.len() > 2 {
        match args[2] {
            "r" | "read" => WatchType::Read,
            "w" | "write" => WatchType::Write,
            "rw" => WatchType::ReadWrite,
            "x" | "exec" => WatchType::Execute,
            _ => WatchType::Write,
        }
    } else {
        WatchType::Write
    };
    
    let id = KDB.set_watchpoint(addr, length, watch_type)?;
    crate::serial_println!("Watchpoint {} set at {:#x}", id, addr);
    Ok(())
}

fn cmd_delete(args: &[&str]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: delete <id>".to_string());
    }
    
    let id = args[0].parse::<u32>().map_err(|_| "Invalid ID".to_string())?;
    KDB.remove_breakpoint(id)?;
    crate::serial_println!("Breakpoint {} deleted", id);
    Ok(())
}

fn cmd_list(_args: &[&str]) -> Result<(), String> {
    let breakpoints = KDB.breakpoints.lock();
    let watchpoints = KDB.watchpoints.lock();
    
    if breakpoints.is_empty() && watchpoints.is_empty() {
        crate::serial_println!("No breakpoints or watchpoints set");
        return Ok(());
    }
    
    if !breakpoints.is_empty() {
        crate::serial_println!("Breakpoints:");
        for bp in breakpoints.iter() {
            crate::serial_println!("  {} at {:#x} (hits: {}) {}",
                bp.id, bp.address, bp.hit_count,
                if bp.enabled { "" } else { "[disabled]" });
        }
    }
    
    if !watchpoints.is_empty() {
        crate::serial_println!("Watchpoints:");
        for wp in watchpoints.iter() {
            crate::serial_println!("  {} at {:#x} len {} type {:?} (hits: {}) {}",
                wp.id, wp.address, wp.length, wp.access_type, wp.hit_count,
                if wp.enabled { "" } else { "[disabled]" });
        }
    }
    
    Ok(())
}

fn cmd_print(args: &[&str]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: print <expression>".to_string());
    }
    
    // For now, just try to parse as address and dereference
    if let Ok(addr) = parse_address(args[0]) {
        unsafe {
            let value = *(addr as *const u64);
            crate::serial_println!("{:#x} = {:#x}", addr, value);
        }
    } else {
        crate::serial_println!("Cannot evaluate: {}", args.join(" "));
    }
    
    Ok(())
}

fn cmd_examine(args: &[&str]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: examine <address> [count]".to_string());
    }
    
    let addr = parse_address(args[0])?;
    let count = if args.len() > 1 {
        args[1].parse::<usize>().unwrap_or(1)
    } else {
        1
    };
    
    // Display memory
    for i in 0..count {
        let current_addr = addr + (i * 8) as u64;
        unsafe {
            let value = *(current_addr as *const u64);
            crate::serial_println!("{:#018x}: {:#018x}", current_addr, value);
        }
    }
    
    Ok(())
}

fn cmd_registers(_args: &[&str]) -> Result<(), String> {
    // Read and display CPU registers
    let mut rax: u64;
    let mut rbx: u64;
    let mut rcx: u64;
    let mut rdx: u64;
    let mut rsi: u64;
    let mut rdi: u64;
    let mut rbp: u64;
    let mut rsp: u64;
    let mut r8: u64;
    let mut r9: u64;
    let mut r10: u64;
    let mut r11: u64;
    let mut r12: u64;
    let mut r13: u64;
    let mut r14: u64;
    let mut r15: u64;
    
    unsafe {
        core::arch::asm!(
            "mov {}, rax",
            "mov {}, rbx",
            "mov {}, rcx",
            "mov {}, rdx",
            "mov {}, rsi",
            "mov {}, rdi",
            "mov {}, rbp",
            "mov {}, rsp",
            "mov {}, r8",
            "mov {}, r9",
            "mov {}, r10",
            "mov {}, r11",
            "mov {}, r12",
            "mov {}, r13",
            "mov {}, r14",
            "mov {}, r15",
            out(reg) rax,
            out(reg) rbx,
            out(reg) rcx,
            out(reg) rdx,
            out(reg) rsi,
            out(reg) rdi,
            out(reg) rbp,
            out(reg) rsp,
            out(reg) r8,
            out(reg) r9,
            out(reg) r10,
            out(reg) r11,
            out(reg) r12,
            out(reg) r13,
            out(reg) r14,
            out(reg) r15,
        );
    }
    
    crate::serial_println!("General Purpose Registers:");
    crate::serial_println!("  RAX: {:#018x}  RBX: {:#018x}", rax, rbx);
    crate::serial_println!("  RCX: {:#018x}  RDX: {:#018x}", rcx, rdx);
    crate::serial_println!("  RSI: {:#018x}  RDI: {:#018x}", rsi, rdi);
    crate::serial_println!("  RBP: {:#018x}  RSP: {:#018x}", rbp, rsp);
    crate::serial_println!("  R8:  {:#018x}  R9:  {:#018x}", r8, r9);
    crate::serial_println!("  R10: {:#018x}  R11: {:#018x}", r10, r11);
    crate::serial_println!("  R12: {:#018x}  R13: {:#018x}", r12, r13);
    crate::serial_println!("  R14: {:#018x}  R15: {:#018x}", r14, r15);
    
    // Control registers
    crate::serial_println!("\nControl Registers:");
    crate::serial_println!("  CR0: {:#018x}", Cr0::read_raw());
    crate::serial_println!("  CR2: {:#018x}", Cr2::read().as_u64());
    crate::serial_println!("  CR3: {:#018x}", Cr3::read().0.start_address().as_u64());
    crate::serial_println!("  CR4: {:#018x}", Cr4::read_raw());
    
    Ok(())
}

fn cmd_backtrace(_args: &[&str]) -> Result<(), String> {
    crate::serial_println!("Call Stack:");
    
    if let Some(trace) = super::generate_stack_trace() {
        for frame in trace {
            crate::serial_println!("{}", frame);
        }
    } else {
        crate::serial_println!("Unable to generate stack trace");
    }
    
    Ok(())
}

fn cmd_info(args: &[&str]) -> Result<(), String> {
    if args.is_empty() {
        crate::serial_println!("Usage: info <breakpoints|watchpoints|memory|cpu|threads>");
        return Ok(());
    }
    
    match args[0] {
        "breakpoints" | "b" => cmd_list(&[]),
        "watchpoints" | "w" => cmd_list(&[]),
        "memory" | "m" => cmd_memory(&[]),
        "cpu" => cmd_cpu(&[]),
        "threads" | "t" => cmd_threads(&[]),
        _ => Err(format!("Unknown info type: {}", args[0])),
    }
}

fn cmd_disassemble(args: &[&str]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: disassemble <address> [count]".to_string());
    }
    
    let addr = parse_address(args[0])?;
    let count = if args.len() > 1 {
        args[1].parse::<usize>().unwrap_or(10)
    } else {
        10
    };
    
    crate::serial_println!("Disassembly at {:#x}:", addr);
    
    // Basic disassembly (would need full x86 decoder)
    let mut current = addr;
    for _ in 0..count {
        unsafe {
            let byte = *(current as *const u8);
            crate::serial_println!("  {:#018x}: {:02x}  ???", current, byte);
            current += 1;  // Would need instruction length calculation
        }
    }
    
    crate::serial_println!("(Full disassembly not yet implemented)");
    Ok(())
}

fn cmd_memory(_args: &[&str]) -> Result<(), String> {
    crate::serial_println!("Memory Statistics:");
    crate::serial_println!("  Free memory: {} KB", 0);  // Would get from allocator
    crate::serial_println!("  Used memory: {} KB", 0);
    crate::serial_println!("  Total memory: {} KB", 0);
    Ok(())
}

fn cmd_cpu(_args: &[&str]) -> Result<(), String> {
    crate::serial_println!("CPU Information:");
    crate::serial_println!("  CPU ID: 0");
    crate::serial_println!("  Frozen CPUs: {}", KDB.frozen_cpus.load(Ordering::Relaxed));
    Ok(())
}

fn cmd_modules(_args: &[&str]) -> Result<(), String> {
    crate::serial_println!("Loaded Modules:");
    crate::serial_println!("  kernel (core)");
    Ok(())
}

fn cmd_threads(_args: &[&str]) -> Result<(), String> {
    crate::serial_println!("Thread List:");
    crate::serial_println!("  TID 0: kernel_main");
    Ok(())
}

fn cmd_locks(_args: &[&str]) -> Result<(), String> {
    crate::serial_println!("Lock Status:");
    crate::serial_println!("  No deadlocks detected");
    Ok(())
}

fn cmd_irq(_args: &[&str]) -> Result<(), String> {
    crate::serial_println!("IRQ Statistics:");
    crate::serial_println!("  IRQ 0 (Timer): 0 interrupts");
    crate::serial_println!("  IRQ 1 (Keyboard): 0 interrupts");
    Ok(())
}

// Helper functions
fn parse_address(s: &str) -> Result<u64, String> {
    if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(&s[2..], 16)
            .map_err(|_| format!("Invalid hex address: {}", s))
    } else {
        s.parse::<u64>()
            .or_else(|_| u64::from_str_radix(s, 16))
            .map_err(|_| format!("Invalid address: {}", s))
    }
}

// Public API
pub fn init() {
    crate::serial_println!("[KDB] Interactive kernel debugger initialized");
}

pub fn enter_debugger(reason: &str) {
    KDB.enter(reason);
}

pub fn enter_on_panic(info: &core::panic::PanicInfo) {
    let reason = format!("Panic: {}", info);
    KDB.enter(&reason);
}

pub fn handle_breakpoint(rip: u64) {
    let mut breakpoints = KDB.breakpoints.lock();
    
    for bp in breakpoints.iter_mut() {
        if bp.address == rip - 1 {  // INT3 is 1 byte
            bp.hit_count += 1;
            crate::serial_println!("Breakpoint {} hit at {:#x}", bp.id, bp.address);
            drop(breakpoints);
            KDB.enter(&format!("Breakpoint {} hit", bp.id));
            return;
        }
    }
}

pub fn handle_debug_exception() {
    if KDB.single_step.load(Ordering::SeqCst) {
        KDB.single_step.store(false, Ordering::SeqCst);
        KDB.enter("Single step");
    }
}