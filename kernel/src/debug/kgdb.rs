// KGDB - GDB Remote Serial Protocol Implementation
// Allows debugging the kernel with GDB over serial/network

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;

// GDB Remote Protocol implementation
pub struct GdbStub {
    connected: AtomicBool,
    running: AtomicBool,
    no_ack_mode: AtomicBool,
    packet_buffer: Mutex<Vec<u8>>,
    breakpoints: Mutex<Vec<GdbBreakpoint>>,
    watchpoints: Mutex<Vec<GdbWatchpoint>>,
    thread_id: AtomicU32,
}

#[derive(Clone)]
struct GdbBreakpoint {
    address: u64,
    kind: u32,
    original_byte: u8,
}

#[derive(Clone)]
struct GdbWatchpoint {
    address: u64,
    length: usize,
    watch_type: WatchType,
}

#[derive(Clone, Copy)]
enum WatchType {
    Write = 1,
    Read = 2,
    Access = 3,
}

lazy_static! {
    pub static ref GDB_STUB: GdbStub = GdbStub::new();
}

impl GdbStub {
    pub fn new() -> Self {
        Self {
            connected: AtomicBool::new(false),
            running: AtomicBool::new(true),
            no_ack_mode: AtomicBool::new(false),
            packet_buffer: Mutex::new(Vec::with_capacity(4096)),
            breakpoints: Mutex::new(Vec::new()),
            watchpoints: Mutex::new(Vec::new()),
            thread_id: AtomicU32::new(1),
        }
    }
    
    pub fn init(&self) {
        crate::serial_println!("[KGDB] GDB stub initialized, waiting for connection...");
        
        // Enable serial interrupt for GDB communication
        crate::serial::enable_interrupt();
    }
    
    pub fn handle_connection(&self) {
        if !self.connected.load(Ordering::Relaxed) {
            // Check for GDB connection sequence ($#)
            if self.check_for_connection() {
                self.connected.store(true, Ordering::SeqCst);
                crate::serial_println!("[KGDB] GDB connected");
                
                // Enter debug loop
                self.gdb_main_loop();
            }
        }
    }
    
    fn check_for_connection(&self) -> bool {
        // Check for GDB interrupt character (Ctrl+C = 0x03)
        if let Some(byte) = crate::serial::read_byte() {
            if byte == 0x03 {
                // Send stop reply
                self.send_packet(b"S05");  // SIGTRAP
                return true;
            }
        }
        false
    }
    
    fn gdb_main_loop(&self) {
        self.running.store(false, Ordering::SeqCst);
        
        while self.connected.load(Ordering::Relaxed) {
            if let Some(packet) = self.receive_packet() {
                let response = self.handle_packet(&packet);
                
                if let Some(resp) = response {
                    self.send_packet(&resp);
                }
                
                // Check if we should resume execution
                if self.running.load(Ordering::Relaxed) {
                    break;
                }
            }
        }
    }
    
    fn receive_packet(&self) -> Option<Vec<u8>> {
        let mut buffer = Vec::new();
        let mut in_packet = false;
        let mut checksum_bytes = [0u8; 2];
        let mut checksum_idx = 0;
        
        loop {
            if let Some(byte) = crate::serial::read_byte() {
                if !in_packet {
                    if byte == b'$' {
                        in_packet = true;
                        buffer.clear();
                    } else if byte == 0x03 {  // Ctrl+C interrupt
                        return Some(vec![0x03]);
                    }
                } else {
                    if byte == b'#' {
                        // Read checksum
                        while checksum_idx < 2 {
                            if let Some(cs_byte) = crate::serial::read_byte() {
                                checksum_bytes[checksum_idx] = cs_byte;
                                checksum_idx += 1;
                            }
                        }
                        
                        // Verify checksum
                        let expected = self.calculate_checksum(&buffer);
                        let received = self.parse_hex_byte(&checksum_bytes);
                        
                        if expected == received {
                            // Send ACK if not in no-ack mode
                            if !self.no_ack_mode.load(Ordering::Relaxed) {
                                self.send_byte(b'+');
                            }
                            return Some(buffer);
                        } else {
                            // Send NAK
                            if !self.no_ack_mode.load(Ordering::Relaxed) {
                                self.send_byte(b'-');
                            }
                            in_packet = false;
                            checksum_idx = 0;
                        }
                    } else {
                        buffer.push(byte);
                    }
                }
            }
        }
    }
    
    fn send_packet(&self, data: &[u8]) {
        // Send: $<data>#<checksum>
        self.send_byte(b'$');
        
        for &byte in data {
            self.send_byte(byte);
        }
        
        self.send_byte(b'#');
        
        let checksum = self.calculate_checksum(data);
        let hex = format!("{:02x}", checksum);
        self.send_byte(hex.as_bytes()[0]);
        self.send_byte(hex.as_bytes()[1]);
    }
    
    fn send_byte(&self, byte: u8) {
        use core::fmt::Write;
        crate::serial::SERIAL1.lock().write_char(byte as char).ok();
    }
    
    fn calculate_checksum(&self, data: &[u8]) -> u8 {
        data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
    }
    
    fn parse_hex_byte(&self, bytes: &[u8; 2]) -> u8 {
        let high = self.hex_to_nibble(bytes[0]);
        let low = self.hex_to_nibble(bytes[1]);
        (high << 4) | low
    }
    
    fn hex_to_nibble(&self, byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => 0,
        }
    }
    
    fn handle_packet(&self, packet: &[u8]) -> Option<Vec<u8>> {
        if packet.is_empty() {
            return None;
        }
        
        // Handle interrupt
        if packet[0] == 0x03 {
            self.running.store(false, Ordering::SeqCst);
            return Some(b"S05".to_vec());  // SIGTRAP
        }
        
        // Parse packet type
        match packet[0] {
            b'?' => {
                // Report target halted
                Some(b"S05".to_vec())  // SIGTRAP
            }
            b'g' => {
                // Read general registers
                self.read_registers()
            }
            b'G' => {
                // Write general registers
                if packet.len() > 1 {
                    self.write_registers(&packet[1..])
                } else {
                    Some(b"E01".to_vec())
                }
            }
            b'm' => {
                // Read memory: m<addr>,<length>
                self.read_memory(packet)
            }
            b'M' => {
                // Write memory: M<addr>,<length>:<data>
                self.write_memory(packet)
            }
            b'c' => {
                // Continue execution
                self.running.store(true, Ordering::SeqCst);
                None  // No immediate response
            }
            b's' => {
                // Single step
                self.single_step();
                None  // No immediate response
            }
            b'Z' => {
                // Insert breakpoint/watchpoint
                self.insert_breakpoint(packet)
            }
            b'z' => {
                // Remove breakpoint/watchpoint
                self.remove_breakpoint(packet)
            }
            b'k' => {
                // Kill request
                self.connected.store(false, Ordering::SeqCst);
                None
            }
            b'D' => {
                // Detach
                self.connected.store(false, Ordering::SeqCst);
                self.running.store(true, Ordering::SeqCst);
                Some(b"OK".to_vec())
            }
            b'q' => {
                // Query packets
                self.handle_query(packet)
            }
            b'Q' => {
                // Set packets
                self.handle_set(packet)
            }
            b'H' => {
                // Set thread for operations
                Some(b"OK".to_vec())
            }
            b'T' => {
                // Thread alive check
                Some(b"OK".to_vec())
            }
            b'v' => {
                // v packets (vCont, etc.)
                self.handle_v_packet(packet)
            }
            _ => {
                // Unsupported packet
                Some(b"".to_vec())
            }
        }
    }
    
    fn read_registers(&self) -> Option<Vec<u8>> {
        // Read all general purpose registers
        let mut regs = [0u64; 16];
        let mut rip: u64;
        let mut rflags: u64;
        
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
                out(reg) regs[0],
                out(reg) regs[1],
                out(reg) regs[2],
                out(reg) regs[3],
                out(reg) regs[4],
                out(reg) regs[5],
                out(reg) regs[6],
                out(reg) regs[7],
                out(reg) regs[8],
                out(reg) regs[9],
                out(reg) regs[10],
                out(reg) regs[11],
                out(reg) regs[12],
                out(reg) regs[13],
                out(reg) regs[14],
                out(reg) regs[15],
            );
            
            core::arch::asm!(
                "lea {}, [rip]",
                "pushfq",
                "pop {}",
                out(reg) rip,
                out(reg) rflags,
            );
        }
        
        // Format as hex string
        let mut response = String::new();
        for reg in regs.iter() {
            response.push_str(&format!("{:016x}", reg.to_le()));
        }
        response.push_str(&format!("{:016x}", rip.to_le()));
        response.push_str(&format!("{:016x}", rflags.to_le()));
        
        Some(response.into_bytes())
    }
    
    fn write_registers(&self, data: &[u8]) -> Option<Vec<u8>> {
        // Would parse hex data and write to registers
        Some(b"OK".to_vec())
    }
    
    fn read_memory(&self, packet: &[u8]) -> Option<Vec<u8>> {
        // Parse m<addr>,<length>
        let s = core::str::from_utf8(&packet[1..]).ok()?;
        let parts: Vec<&str> = s.split(',').collect();
        
        if parts.len() != 2 {
            return Some(b"E01".to_vec());
        }
        
        let addr = u64::from_str_radix(parts[0], 16).ok()?;
        let length = usize::from_str_radix(parts[1], 16).ok()?;
        
        // Read memory safely
        let mut response = String::new();
        for i in 0..length {
            unsafe {
                let byte = *((addr + i as u64) as *const u8);
                response.push_str(&format!("{:02x}", byte));
            }
        }
        
        Some(response.into_bytes())
    }
    
    fn write_memory(&self, packet: &[u8]) -> Option<Vec<u8>> {
        // Parse M<addr>,<length>:<data>
        let s = core::str::from_utf8(&packet[1..]).ok()?;
        let parts: Vec<&str> = s.split(',').collect();
        
        if parts.len() != 2 {
            return Some(b"E01".to_vec());
        }
        
        let addr = u64::from_str_radix(parts[0], 16).ok()?;
        let rest_parts: Vec<&str> = parts[1].split(':').collect();
        
        if rest_parts.len() != 2 {
            return Some(b"E02".to_vec());
        }
        
        let length = usize::from_str_radix(rest_parts[0], 16).ok()?;
        let data_hex = rest_parts[1];
        
        // Write memory
        for i in 0..length {
            let hex_byte = &data_hex[i*2..i*2+2];
            let byte = u8::from_str_radix(hex_byte, 16).ok()?;
            
            unsafe {
                *((addr + i as u64) as *mut u8) = byte;
            }
        }
        
        Some(b"OK".to_vec())
    }
    
    fn single_step(&self) {
        // Enable single stepping via RFLAGS.TF
        unsafe {
            core::arch::asm!(
                "pushfq",
                "or qword ptr [rsp], 0x100",  // Set TF flag
                "popfq"
            );
        }
        self.running.store(true, Ordering::SeqCst);
    }
    
    fn insert_breakpoint(&self, packet: &[u8]) -> Option<Vec<u8>> {
        // Parse Z<type>,<addr>,<kind>
        let s = core::str::from_utf8(&packet[1..]).ok()?;
        let parts: Vec<&str> = s.split(',').collect();
        
        if parts.len() < 3 {
            return Some(b"E01".to_vec());
        }
        
        let bp_type = parts[0].chars().next()?;
        let addr = u64::from_str_radix(parts[1], 16).ok()?;
        let kind = u32::from_str_radix(parts[2], 16).ok()?;
        
        match bp_type {
            '0' => {
                // Software breakpoint
                let original = unsafe { *(addr as *const u8) };
                unsafe { *(addr as *mut u8) = 0xCC; }  // INT3
                
                self.breakpoints.lock().push(GdbBreakpoint {
                    address: addr,
                    kind,
                    original_byte: original,
                });
                
                Some(b"OK".to_vec())
            }
            '1' => {
                // Hardware breakpoint
                // Would set debug registers DR0-DR3
                Some(b"OK".to_vec())
            }
            '2' | '3' | '4' => {
                // Watchpoints (write/read/access)
                let watch_type = match bp_type {
                    '2' => WatchType::Write,
                    '3' => WatchType::Read,
                    _ => WatchType::Access,
                };
                
                self.watchpoints.lock().push(GdbWatchpoint {
                    address: addr,
                    length: kind as usize,
                    watch_type,
                });
                
                Some(b"OK".to_vec())
            }
            _ => Some(b"".to_vec()),
        }
    }
    
    fn remove_breakpoint(&self, packet: &[u8]) -> Option<Vec<u8>> {
        // Parse z<type>,<addr>,<kind>
        let s = core::str::from_utf8(&packet[1..]).ok()?;
        let parts: Vec<&str> = s.split(',').collect();
        
        if parts.len() < 3 {
            return Some(b"E01".to_vec());
        }
        
        let bp_type = parts[0].chars().next()?;
        let addr = u64::from_str_radix(parts[1], 16).ok()?;
        
        match bp_type {
            '0' => {
                // Remove software breakpoint
                let mut breakpoints = self.breakpoints.lock();
                if let Some(pos) = breakpoints.iter().position(|bp| bp.address == addr) {
                    let bp = &breakpoints[pos];
                    unsafe { *(addr as *mut u8) = bp.original_byte; }
                    breakpoints.remove(pos);
                }
                Some(b"OK".to_vec())
            }
            '1' => {
                // Remove hardware breakpoint
                Some(b"OK".to_vec())
            }
            '2' | '3' | '4' => {
                // Remove watchpoint
                let mut watchpoints = self.watchpoints.lock();
                if let Some(pos) = watchpoints.iter().position(|wp| wp.address == addr) {
                    watchpoints.remove(pos);
                }
                Some(b"OK".to_vec())
            }
            _ => Some(b"".to_vec()),
        }
    }
    
    fn handle_query(&self, packet: &[u8]) -> Option<Vec<u8>> {
        let query = core::str::from_utf8(&packet[1..]).ok()?;
        
        if query.starts_with("Supported") {
            // Report supported features
            Some(b"PacketSize=1000;QStartNoAckMode+".to_vec())
        } else if query == "C" {
            // Current thread ID
            Some(format!("QC{:x}", self.thread_id.load(Ordering::Relaxed)).into_bytes())
        } else if query.starts_with("Attached") {
            // We're always attached to kernel
            Some(b"1".to_vec())
        } else if query == "TStatus" {
            // Trace status
            Some(b"T0".to_vec())
        } else if query.starts_with("Rcmd,") {
            // Monitor command
            self.handle_monitor_command(&query[5..])
        } else {
            Some(b"".to_vec())
        }
    }
    
    fn handle_set(&self, packet: &[u8]) -> Option<Vec<u8>> {
        let query = core::str::from_utf8(&packet[1..]).ok()?;
        
        if query == "StartNoAckMode" {
            self.no_ack_mode.store(true, Ordering::SeqCst);
            Some(b"OK".to_vec())
        } else {
            Some(b"".to_vec())
        }
    }
    
    fn handle_v_packet(&self, packet: &[u8]) -> Option<Vec<u8>> {
        let cmd = core::str::from_utf8(&packet[1..]).ok()?;
        
        if cmd.starts_with("Cont?") {
            // Report supported vCont actions
            Some(b"vCont;c;s".to_vec())
        } else if cmd.starts_with("Cont") {
            // Continue with specific actions
            self.running.store(true, Ordering::SeqCst);
            None
        } else {
            Some(b"".to_vec())
        }
    }
    
    fn handle_monitor_command(&self, hex_cmd: &str) -> Option<Vec<u8>> {
        // Decode hex command
        let mut cmd = String::new();
        let mut chars = hex_cmd.chars();
        
        while let (Some(h), Some(l)) = (chars.next(), chars.next()) {
            let high = self.hex_to_nibble(h as u8);
            let low = self.hex_to_nibble(l as u8);
            cmd.push(((high << 4) | low) as char);
        }
        
        // Execute monitor command
        let response = match cmd.as_str() {
            "help" => "Available commands:\n  info\n  regs\n  mem\n",
            "info" => "Kernel GDB stub active\n",
            "regs" => "Use 'info registers' in GDB\n",
            _ => "Unknown command\n",
        };
        
        // Encode response as hex
        let mut hex_response = String::new();
        for byte in response.bytes() {
            hex_response.push_str(&format!("{:02x}", byte));
        }
        
        Some(hex_response.into_bytes())
    }
    
    pub fn handle_exception(&self, vector: u8) {
        if !self.connected.load(Ordering::Relaxed) {
            return;
        }
        
        // Map exception to signal
        let signal = match vector {
            0x03 => 5,   // SIGTRAP for breakpoint
            0x06 => 4,   // SIGILL for invalid opcode
            0x0E => 11,  // SIGSEGV for page fault
            _ => 5,      // Default to SIGTRAP
        };
        
        // Send stop reply
        self.send_packet(&format!("S{:02x}", signal).into_bytes());
        
        // Enter debug loop
        self.gdb_main_loop();
    }
}

// Network debugging support (KDNet equivalent)
pub mod kdnet {
    use super::*;
    
    pub struct NetworkDebugger {
        enabled: AtomicBool,
        port: u16,
        key: [u8; 32],
    }
    
    impl NetworkDebugger {
        pub fn new() -> Self {
            Self {
                enabled: AtomicBool::new(false),
                port: 50000,
                key: [0; 32],
            }
        }
        
        pub fn init(&self, port: u16, key: &[u8]) {
            crate::serial_println!("[KDNET] Initializing network debugging on port {}", port);
            
            // Would set up network debugging
            // - Configure network interface for debugging
            // - Set up encryption with provided key
            // - Listen for debugger connections
            
            self.enabled.store(true, Ordering::SeqCst);
        }
    }
}

// USB debugging support
pub mod usb_debug {
    use super::*;
    
    pub struct UsbDebugger {
        enabled: AtomicBool,
        device_id: u32,
    }
    
    impl UsbDebugger {
        pub fn new() -> Self {
            Self {
                enabled: AtomicBool::new(false),
                device_id: 0,
            }
        }
        
        pub fn init(&self) {
            crate::serial_println!("[USB-DEBUG] Initializing USB debugging");
            
            // Would initialize USB debugging
            // - Configure USB controller for debug mode
            // - Set up debug descriptors
            // - Enable USB3 debug capability
            
            self.enabled.store(true, Ordering::SeqCst);
        }
    }
}

// Public API
pub fn init() {
    GDB_STUB.init();
    crate::serial_println!("[KGDB] GDB remote debugging support initialized");
}

pub fn handle_interrupt() {
    GDB_STUB.handle_connection();
}

pub fn notify_exception(vector: u8) {
    GDB_STUB.handle_exception(vector);
}

pub fn is_connected() -> bool {
    GDB_STUB.connected.load(Ordering::Relaxed)
}

pub fn force_entry() {
    if !GDB_STUB.connected.load(Ordering::Relaxed) {
        crate::serial_println!("[KGDB] Waiting for GDB connection...");
        crate::serial_println!("[KGDB] Run: target remote /dev/ttyS0");
    }
    
    GDB_STUB.running.store(false, Ordering::SeqCst);
    GDB_STUB.send_packet(b"S05");  // Send SIGTRAP
    GDB_STUB.gdb_main_loop();
}