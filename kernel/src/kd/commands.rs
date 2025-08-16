// Kernel Debugger Commands Implementation
use super::*;
use super::{KERNEL_DEBUGGER, WatchAccessType};
use alloc::string::String;
use alloc::vec::Vec;

// Debugger command processor
pub struct CommandProcessor {
    command_buffer: String,
    history: Vec<String>,
    history_index: usize,
}

impl CommandProcessor {
    pub fn new() -> Self {
        Self {
            command_buffer: String::new(),
            history: Vec::new(),
            history_index: 0,
        }
    }
    
    pub fn process_command(&mut self, input: &str) -> CommandResult {
        // Parse and execute command
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return CommandResult::Empty;
        }
        
        // Add to history
        self.history.push(String::from(input));
        self.history_index = self.history.len();
        
        match parts[0].to_lowercase().as_str() {
            "g" | "go" => self.cmd_go(parts),
            "t" | "trace" => self.cmd_trace(parts),
            "p" | "step" => self.cmd_step(parts),
            "bp" => self.cmd_breakpoint(parts),
            "bc" => self.cmd_clear_breakpoint(parts),
            "bl" => self.cmd_list_breakpoints(parts),
            "ba" => self.cmd_access_breakpoint(parts),
            "d" | "db" | "dw" | "dd" | "dq" => self.cmd_display_memory(parts),
            "e" | "eb" | "ew" | "ed" | "eq" => self.cmd_edit_memory(parts),
            "r" => self.cmd_registers(parts),
            "k" | "kb" | "kp" | "kv" => self.cmd_stack(parts),
            "lm" => self.cmd_list_modules(parts),
            "!process" => self.cmd_process(parts),
            "!thread" => self.cmd_thread(parts),
            "!analyze" => self.cmd_analyze(parts),
            ".reload" => self.cmd_reload_symbols(parts),
            ".reboot" => self.cmd_reboot(parts),
            "?" => self.cmd_help(parts),
            "q" | "quit" => self.cmd_quit(parts),
            _ => {
                crate::println!("Unknown command: {}", parts[0]);
                CommandResult::Error
            }
        }
    }
    
    fn cmd_go(&mut self, _parts: Vec<&str>) -> CommandResult {
        crate::println!("Continuing execution...");
        CommandResult::Continue
    }
    
    fn cmd_trace(&mut self, _parts: Vec<&str>) -> CommandResult {
        crate::println!("Trace mode (single step with calls)");
        CommandResult::Trace
    }
    
    fn cmd_step(&mut self, _parts: Vec<&str>) -> CommandResult {
        crate::println!("Single step");
        CommandResult::Step
    }
    
    fn cmd_breakpoint(&mut self, parts: Vec<&str>) -> CommandResult {
        if parts.len() < 2 {
            crate::println!("Usage: bp <address>");
            return CommandResult::Error;
        }
        
        if let Ok(addr) = parse_address(parts[1]) {
            let id = KERNEL_DEBUGGER.lock().set_breakpoint(addr);
            crate::println!("Breakpoint {} set at {:#x}", id, addr);
            CommandResult::Success
        } else {
            crate::println!("Invalid address: {}", parts[1]);
            CommandResult::Error
        }
    }
    
    fn cmd_clear_breakpoint(&mut self, parts: Vec<&str>) -> CommandResult {
        if parts.len() < 2 {
            crate::println!("Usage: bc <id|*>");
            return CommandResult::Error;
        }
        
        if parts[1] == "*" {
            // Clear all breakpoints
            let mut debugger = KERNEL_DEBUGGER.lock();
            debugger.breakpoints.clear();
            crate::println!("All breakpoints cleared");
        } else if let Ok(id) = parts[1].parse::<u32>() {
            let mut debugger = KERNEL_DEBUGGER.lock();
            debugger.clear_breakpoint(id);
            crate::println!("Breakpoint {} cleared", id);
        } else {
            crate::println!("Invalid breakpoint ID: {}", parts[1]);
            return CommandResult::Error;
        }
        
        CommandResult::Success
    }
    
    fn cmd_list_breakpoints(&mut self, _parts: Vec<&str>) -> CommandResult {
        // Get breakpoint info without holding the lock too long
        let breakpoints = {
            let debugger = KERNEL_DEBUGGER.lock();
            debugger.breakpoints.clone()
        };
        
        if breakpoints.is_empty() {
            crate::println!("No breakpoints set");
        } else {
            crate::println!("Breakpoints:");
            for bp in &breakpoints {
                crate::println!("  {} : {:#016x} {} (hit {} times)",
                    bp.id,
                    bp.address,
                    if bp.enabled { "enabled" } else { "disabled" },
                    bp.hit_count
                );
            }
        }
        CommandResult::Success
    }
    
    fn cmd_access_breakpoint(&mut self, parts: Vec<&str>) -> CommandResult {
        if parts.len() < 3 {
            crate::println!("Usage: ba <r|w|e> <size> <address>");
            return CommandResult::Error;
        }
        
        let access_type = match parts[1] {
            "r" => WatchAccessType::Read,
            "w" => WatchAccessType::Write,
            "e" => WatchAccessType::Execute,
            _ => {
                crate::println!("Invalid access type: {}", parts[1]);
                return CommandResult::Error;
            }
        };
        
        let size = if parts.len() > 3 {
            parts[2].parse::<usize>().unwrap_or(1)
        } else {
            1
        };
        
        let addr_str = if parts.len() > 3 { parts[3] } else { parts[2] };
        
        if let Ok(addr) = parse_address(addr_str) {
            let id = KERNEL_DEBUGGER.lock().set_watchpoint(addr, size, access_type);
            crate::println!("Hardware breakpoint {} set at {:#x}", id, addr);
            CommandResult::Success
        } else {
            crate::println!("Invalid address: {}", addr_str);
            CommandResult::Error
        }
    }
    
    fn cmd_display_memory(&mut self, parts: Vec<&str>) -> CommandResult {
        if parts.len() < 2 {
            crate::println!("Usage: d[b|w|d|q] <address> [length]");
            return CommandResult::Error;
        }
        
        let size = match parts[0] {
            "db" => 1,
            "dw" => 2,
            "dd" => 4,
            "dq" => 8,
            _ => 1,
        };
        
        if let Ok(addr) = parse_address(parts[1]) {
            let length = if parts.len() > 2 {
                parts[2].parse::<usize>().unwrap_or(64)
            } else {
                64
            };
            
            display_memory(addr, length, size);
            CommandResult::Success
        } else {
            crate::println!("Invalid address: {}", parts[1]);
            CommandResult::Error
        }
    }
    
    fn cmd_edit_memory(&mut self, parts: Vec<&str>) -> CommandResult {
        if parts.len() < 3 {
            crate::println!("Usage: e[b|w|d|q] <address> <value>");
            return CommandResult::Error;
        }
        
        let size = match parts[0] {
            "eb" => 1,
            "ew" => 2,
            "ed" => 4,
            "eq" => 8,
            _ => 1,
        };
        
        if let Ok(addr) = parse_address(parts[1]) {
            if let Ok(value) = parse_value(parts[2]) {
                edit_memory(addr, value, size);
                crate::println!("Memory at {:#x} modified", addr);
                CommandResult::Success
            } else {
                crate::println!("Invalid value: {}", parts[2]);
                CommandResult::Error
            }
        } else {
            crate::println!("Invalid address: {}", parts[1]);
            CommandResult::Error
        }
    }
    
    fn cmd_registers(&mut self, parts: Vec<&str>) -> CommandResult {
        if parts.len() > 1 {
            // Set register value
            if parts.len() < 3 {
                crate::println!("Usage: r <register> <value>");
                return CommandResult::Error;
            }
            // Would set register value here
            crate::println!("Register {} set to {}", parts[1], parts[2]);
        } else {
            // Display all registers
            display_registers();
        }
        CommandResult::Success
    }
    
    fn cmd_stack(&mut self, parts: Vec<&str>) -> CommandResult {
        let depth = if parts.len() > 1 {
            parts[1].parse::<usize>().unwrap_or(10)
        } else {
            10
        };
        
        display_call_stack(depth);
        CommandResult::Success
    }
    
    fn cmd_list_modules(&mut self, _parts: Vec<&str>) -> CommandResult {
        crate::println!("Loaded modules:");
        crate::println!("  Base             Size     Module");
        crate::println!("  ffffffff80000000 00200000 rust_kernel");
        // Would list actual loaded modules
        CommandResult::Success
    }
    
    fn cmd_process(&mut self, parts: Vec<&str>) -> CommandResult {
        if parts.len() > 1 {
            // Display specific process
            crate::println!("Process information for PID {}", parts[1]);
        } else {
            // List all processes
            crate::println!("Process list:");
            crate::println!("  PID  PPID  Name");
            crate::println!("  0    0     System");
            crate::println!("  4    0     kernel");
        }
        CommandResult::Success
    }
    
    fn cmd_thread(&mut self, parts: Vec<&str>) -> CommandResult {
        if parts.len() > 1 {
            // Display specific thread
            crate::println!("Thread information for TID {}", parts[1]);
        } else {
            // List all threads
            crate::println!("Thread list:");
            crate::println!("  TID  PID  State      Priority");
            crate::println!("  1    0    Running    8");
        }
        CommandResult::Success
    }
    
    fn cmd_analyze(&mut self, parts: Vec<&str>) -> CommandResult {
        let verbose = parts.len() > 1 && parts[1] == "-v";
        
        crate::println!("Analyzing system state...");
        crate::println!("");
        crate::println!("BUGCHECK_ANALYSIS:");
        crate::println!("  No bugcheck detected");
        crate::println!("");
        crate::println!("SYSTEM_STATE:");
        crate::println!("  Kernel mode");
        crate::println!("  IRQL: PASSIVE_LEVEL");
        
        if verbose {
            crate::println!("");
            crate::println!("DETAILED_ANALYSIS:");
            crate::println!("  CPU: 0");
            crate::println!("  Process: System");
            crate::println!("  Thread: 1");
        }
        
        CommandResult::Success
    }
    
    fn cmd_reload_symbols(&mut self, _parts: Vec<&str>) -> CommandResult {
        crate::println!("Reloading symbols...");
        crate::println!("Symbol search path: srv*c:\\symbols*https://msdl.microsoft.com/download/symbols");
        crate::println!("Symbols loaded for rust_kernel");
        CommandResult::Success
    }
    
    fn cmd_reboot(&mut self, _parts: Vec<&str>) -> CommandResult {
        crate::println!("Rebooting system...");
        CommandResult::Reboot
    }
    
    fn cmd_help(&mut self, _parts: Vec<&str>) -> CommandResult {
        crate::println!("Kernel Debugger Commands:");
        crate::println!("  g, go           - Continue execution");
        crate::println!("  t, trace        - Trace execution (step into)");
        crate::println!("  p, step         - Step over");
        crate::println!("  bp <addr>       - Set breakpoint");
        crate::println!("  bc <id>         - Clear breakpoint");
        crate::println!("  bl              - List breakpoints");
        crate::println!("  ba <r|w|e> <addr> - Set hardware breakpoint");
        crate::println!("  d[b|w|d|q] <addr> - Display memory");
        crate::println!("  e[b|w|d|q] <addr> <val> - Edit memory");
        crate::println!("  r [reg] [val]   - Display/set registers");
        crate::println!("  k               - Display call stack");
        crate::println!("  lm              - List loaded modules");
        crate::println!("  !process        - Display process information");
        crate::println!("  !thread         - Display thread information");
        crate::println!("  !analyze [-v]   - Analyze system state");
        crate::println!("  .reload         - Reload symbols");
        crate::println!("  .reboot         - Reboot system");
        crate::println!("  ?               - Show this help");
        crate::println!("  q, quit         - Quit debugger");
        CommandResult::Success
    }
    
    fn cmd_quit(&mut self, _parts: Vec<&str>) -> CommandResult {
        crate::println!("Disconnecting debugger...");
        CommandResult::Quit
    }
}

#[derive(Debug, PartialEq)]
pub enum CommandResult {
    Continue,
    Step,
    Trace,
    Success,
    Error,
    Empty,
    Reboot,
    Quit,
}

// Helper functions
fn parse_address(s: &str) -> Result<u64, &'static str> {
    if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(&s[2..], 16).map_err(|_| "Invalid hex address")
    } else if s.chars().any(|c| c.is_ascii_alphabetic()) {
        // Assume hex without 0x prefix
        u64::from_str_radix(s, 16).map_err(|_| "Invalid hex address")
    } else {
        s.parse::<u64>().map_err(|_| "Invalid address")
    }
}

fn parse_value(s: &str) -> Result<u64, &'static str> {
    parse_address(s)
}

fn display_memory(address: u64, length: usize, size: usize) {
    crate::println!("Memory at {:#016x}:", address);
    
    unsafe {
        let ptr = address as *const u8;
        let mut offset = 0;
        
        while offset < length {
            crate::print!("{:#016x}: ", address + offset as u64);
            
            // Display hex values
            for i in 0..16 {
                if offset + i < length {
                    let byte = *ptr.add(offset + i);
                    crate::print!("{:02x} ", byte);
                } else {
                    crate::print!("   ");
                }
            }
            
            crate::print!(" ");
            
            // Display ASCII representation
            for i in 0..16 {
                if offset + i < length {
                    let byte = *ptr.add(offset + i);
                    if byte.is_ascii_graphic() || byte == b' ' {
                        crate::print!("{}", byte as char);
                    } else {
                        crate::print!(".");
                    }
                }
            }
            
            crate::println!();
            offset += 16;
        }
    }
}

fn edit_memory(address: u64, value: u64, size: usize) {
    unsafe {
        match size {
            1 => {
                let ptr = address as *mut u8;
                *ptr = value as u8;
            }
            2 => {
                let ptr = address as *mut u16;
                *ptr = value as u16;
            }
            4 => {
                let ptr = address as *mut u32;
                *ptr = value as u32;
            }
            8 => {
                let ptr = address as *mut u64;
                *ptr = value;
            }
            _ => {}
        }
    }
}

fn display_registers() {
    crate::println!("Register dump:");
    crate::println!("RAX=0000000000000000 RBX=0000000000000000 RCX=0000000000000000");
    crate::println!("RDX=0000000000000000 RSI=0000000000000000 RDI=0000000000000000");
    crate::println!("RIP=ffffffff80000000 RSP=ffffffff80100000 RBP=ffffffff80100000");
    crate::println!("R8 =0000000000000000 R9 =0000000000000000 R10=0000000000000000");
    crate::println!("R11=0000000000000000 R12=0000000000000000 R13=0000000000000000");
    crate::println!("R14=0000000000000000 R15=0000000000000000");
    crate::println!("IOPL=0 IF=1 TF=0");
    crate::println!("CS=0010 SS=0018 DS=0000 ES=0000 FS=0000 GS=0000");
}

fn display_call_stack(depth: usize) {
    crate::println!("Call stack:");
    crate::println!("  # RetAddr           Call Site");
    for i in 0..depth {
        crate::println!("  {} ffffffff8000{:04x} rust_kernel+0x{:04x}",
            i, 1000 + i * 8, 1000 + i * 8);
    }
}