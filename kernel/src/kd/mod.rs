// Kernel Debugger (KD) Protocol Implementation
pub mod protocol;
pub mod transport;
pub mod commands;

use spin::Mutex;
use lazy_static::lazy_static;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// Kernel debugger state
pub struct KernelDebugger {
    enabled: AtomicBool,
    connected: AtomicBool,
    transport: DebugTransport,
    breakpoints: Vec<Breakpoint>,
    watch_points: Vec<WatchPoint>,
    packet_id: AtomicU32,
    state: DebuggerState,
    connection_type: ConnectionType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DebuggerState {
    Disabled,
    WaitingForConnection,
    Connected,
    Breaking,
    Running,
    Stepping,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionType {
    Serial,
    Usb,
    Network,
    Firewire,
    Local,
}

#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub id: u32,
    pub address: u64,
    pub enabled: bool,
    pub hit_count: u32,
    pub bp_type: BreakpointType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BreakpointType {
    Software,
    Hardware,
    Conditional,
}

#[derive(Debug, Clone)]
pub struct WatchPoint {
    pub id: u32,
    pub address: u64,
    pub size: usize,
    pub access_type: WatchAccessType,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WatchAccessType {
    Read,
    Write,
    ReadWrite,
    Execute,
}

// Debug transport layer
pub enum DebugTransport {
    Serial(protocol::SerialTransport),
    Usb(protocol::UsbTransport),
    Network(protocol::NetworkTransport),
}

lazy_static! {
    pub static ref KERNEL_DEBUGGER: Mutex<KernelDebugger> = 
        Mutex::new(KernelDebugger::new());
}

impl KernelDebugger {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            connected: AtomicBool::new(false),
            transport: DebugTransport::Serial(protocol::SerialTransport::new()),
            breakpoints: Vec::new(),
            watch_points: Vec::new(),
            packet_id: AtomicU32::new(0),
            state: DebuggerState::Disabled,
            connection_type: ConnectionType::Serial,
        }
    }
    
    pub fn initialize(&mut self, connection_type: ConnectionType) -> bool {
        crate::serial_println!("KD: Initializing kernel debugger");
        
        self.connection_type = connection_type;
        
        // Initialize transport based on connection type
        match connection_type {
            ConnectionType::Serial => {
                self.transport = DebugTransport::Serial(protocol::SerialTransport::new());
            }
            ConnectionType::Network => {
                self.transport = DebugTransport::Network(protocol::NetworkTransport::new());
            }
            ConnectionType::Usb => {
                self.transport = DebugTransport::Usb(protocol::UsbTransport::new());
            }
            _ => {
                crate::serial_println!("KD: Unsupported connection type");
                return false;
            }
        }
        
        self.enabled.store(true, Ordering::Relaxed);
        self.state = DebuggerState::WaitingForConnection;
        
        crate::serial_println!("KD: Kernel debugger initialized, waiting for connection");
        true
    }
    
    pub fn wait_for_connection(&mut self) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return false;
        }
        
        crate::serial_println!("KD: Waiting for debugger connection...");
        
        // Send initial break-in packet
        self.send_breakin();
        
        // Wait for connection response
        if self.wait_for_handshake() {
            self.connected.store(true, Ordering::Relaxed);
            self.state = DebuggerState::Connected;
            crate::serial_println!("KD: Debugger connected");
            true
        } else {
            false
        }
    }
    
    fn send_breakin(&mut self) {
        // Send break-in sequence to notify debugger
        match &mut self.transport {
            DebugTransport::Serial(transport) => {
                transport.send_break_sequence();
            }
            _ => {}
        }
    }
    
    fn wait_for_handshake(&mut self) -> bool {
        // Wait for debugger handshake
        // In real implementation, this would implement the full KD protocol handshake
        true // Simplified for demo
    }
    
    pub fn handle_exception(&mut self, exception_code: u32, address: u64) {
        if !self.connected.load(Ordering::Relaxed) {
            return;
        }
        
        crate::serial_println!("KD: Exception {:08x} at address {:016x}", exception_code, address);
        
        // Check if we hit a breakpoint
        let mut breakpoint_id = None;
        for bp in &mut self.breakpoints {
            if bp.enabled && bp.address == address {
                bp.hit_count += 1;
                breakpoint_id = Some(bp.id);
                break;
            }
        }
        
        if let Some(id) = breakpoint_id {
            self.enter_debugger(DebugReason::Breakpoint(id));
        } else {
            // Send exception to debugger
            self.send_exception_packet(exception_code, address);
            self.enter_debugger(DebugReason::Exception(exception_code));
        }
    }
    
    pub fn enter_debugger(&mut self, reason: DebugReason) {
        if !self.connected.load(Ordering::Relaxed) {
            return;
        }
        
        self.state = DebuggerState::Breaking;
        
        // Send state change packet
        self.send_state_change(reason);
        
        // Enter debug loop
        self.debug_loop();
    }
    
    fn debug_loop(&mut self) {
        crate::serial_println!("KD: Entering debug loop");
        
        loop {
            // Wait for and process debugger commands
            if let Some(command) = self.receive_command() {
                match self.process_command(command) {
                    CommandResult::Continue => {
                        self.state = DebuggerState::Running;
                        break;
                    }
                    CommandResult::Step => {
                        self.state = DebuggerState::Stepping;
                        break;
                    }
                    CommandResult::Processed => {
                        // Command processed, wait for next
                    }
                }
            }
        }
        
        crate::serial_println!("KD: Exiting debug loop");
    }
    
    fn receive_command(&mut self) -> Option<DebugCommand> {
        // Receive and parse debugger command
        // Simplified implementation
        Some(DebugCommand::Continue)
    }
    
    fn process_command(&mut self, command: DebugCommand) -> CommandResult {
        match command {
            DebugCommand::Continue => {
                crate::serial_println!("KD: Continue execution");
                CommandResult::Continue
            }
            DebugCommand::Step => {
                crate::serial_println!("KD: Single step");
                CommandResult::Step
            }
            DebugCommand::SetBreakpoint(addr) => {
                self.set_breakpoint(addr);
                CommandResult::Processed
            }
            DebugCommand::ClearBreakpoint(id) => {
                self.clear_breakpoint(id);
                CommandResult::Processed
            }
            DebugCommand::ReadMemory(addr, size) => {
                self.read_memory(addr, size);
                CommandResult::Processed
            }
            DebugCommand::WriteMemory(addr, data) => {
                self.write_memory(addr, &data);
                CommandResult::Processed
            }
            DebugCommand::GetRegisters => {
                self.send_register_state();
                CommandResult::Processed
            }
            DebugCommand::GetCallStack => {
                self.send_call_stack();
                CommandResult::Processed
            }
            _ => CommandResult::Processed,
        }
    }
    
    pub fn set_breakpoint(&mut self, address: u64) -> u32 {
        let id = self.breakpoints.len() as u32;
        self.breakpoints.push(Breakpoint {
            id,
            address,
            enabled: true,
            hit_count: 0,
            bp_type: BreakpointType::Software,
        });
        
        crate::serial_println!("KD: Breakpoint {} set at {:016x}", id, address);
        id
    }
    
    pub fn clear_breakpoint(&mut self, id: u32) {
        self.breakpoints.retain(|bp| bp.id != id);
        crate::serial_println!("KD: Breakpoint {} cleared", id);
    }
    
    pub fn set_watchpoint(&mut self, address: u64, size: usize, access_type: WatchAccessType) -> u32 {
        let id = self.watch_points.len() as u32;
        self.watch_points.push(WatchPoint {
            id,
            address,
            size,
            access_type,
            enabled: true,
        });
        
        crate::serial_println!("KD: Watchpoint {} set at {:016x}", id, address);
        id
    }
    
    fn read_memory(&mut self, address: u64, size: usize) {
        // Read memory and send to debugger
        unsafe {
            let ptr = address as *const u8;
            let data = core::slice::from_raw_parts(ptr, size);
            self.send_memory_data(address, data);
        }
    }
    
    fn write_memory(&mut self, address: u64, data: &[u8]) {
        // Write memory from debugger
        unsafe {
            let ptr = address as *mut u8;
            core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        }
        crate::serial_println!("KD: Wrote {} bytes to {:016x}", data.len(), address);
    }
    
    fn send_exception_packet(&mut self, exception_code: u32, address: u64) {
        let packet_id = self.packet_id.fetch_add(1, Ordering::Relaxed);
        // Send exception packet to debugger
        crate::serial_println!("KD: Sending exception packet {}: code={:08x} addr={:016x}", 
                              packet_id, exception_code, address);
    }
    
    fn send_state_change(&mut self, reason: DebugReason) {
        let packet_id = self.packet_id.fetch_add(1, Ordering::Relaxed);
        crate::serial_println!("KD: Sending state change packet {}: {:?}", packet_id, reason);
    }
    
    fn send_memory_data(&mut self, address: u64, data: &[u8]) {
        let packet_id = self.packet_id.fetch_add(1, Ordering::Relaxed);
        crate::serial_println!("KD: Sending memory data packet {}: addr={:016x} len={}", 
                              packet_id, address, data.len());
    }
    
    fn send_register_state(&mut self) {
        let packet_id = self.packet_id.fetch_add(1, Ordering::Relaxed);
        crate::serial_println!("KD: Sending register state packet {}", packet_id);
        // Would send actual register values
    }
    
    fn send_call_stack(&mut self) {
        let packet_id = self.packet_id.fetch_add(1, Ordering::Relaxed);
        crate::serial_println!("KD: Sending call stack packet {}", packet_id);
        // Would send actual call stack
    }
    
    pub fn print(&mut self, message: &str) {
        if self.connected.load(Ordering::Relaxed) {
            // Send debug print to debugger
            let packet_id = self.packet_id.fetch_add(1, Ordering::Relaxed);
            crate::serial_println!("KD: Debug print {}: {}", packet_id, message);
        }
    }
}

#[derive(Debug)]
pub enum DebugReason {
    Breakpoint(u32),
    Exception(u32),
    SingleStep,
    DebugPrint,
    Manual,
}

#[derive(Debug)]
pub enum DebugCommand {
    Continue,
    Step,
    SetBreakpoint(u64),
    ClearBreakpoint(u32),
    ReadMemory(u64, usize),
    WriteMemory(u64, Vec<u8>),
    GetRegisters,
    SetRegister(String, u64),
    GetCallStack,
    GetModules,
    GetThreads,
    SwitchThread(u32),
}

#[derive(Debug)]
pub enum CommandResult {
    Continue,
    Step,
    Processed,
}

// Public API
pub fn kd_initialize(connection: ConnectionType) -> bool {
    KERNEL_DEBUGGER.lock().initialize(connection)
}

pub fn kd_wait_for_connection() -> bool {
    KERNEL_DEBUGGER.lock().wait_for_connection()
}

pub fn kd_enter_debugger() {
    KERNEL_DEBUGGER.lock().enter_debugger(DebugReason::Manual)
}

pub fn kd_print(message: &str) {
    KERNEL_DEBUGGER.lock().print(message)
}

pub fn kd_break() {
    if KERNEL_DEBUGGER.lock().connected.load(Ordering::Relaxed) {
        KERNEL_DEBUGGER.lock().enter_debugger(DebugReason::Manual);
    }
}

pub fn kd_handle_exception(exception_code: u32, address: u64) {
    KERNEL_DEBUGGER.lock().handle_exception(exception_code, address)
}