// KD Protocol Implementation (WinDbg compatible)
use alloc::vec::Vec;
use alloc::string::String;
use core::mem;

// KD Packet structure
#[repr(C, packed)]
pub struct KdPacket {
    pub leader: u32,
    pub packet_type: u16,
    pub byte_count: u16,
    pub packet_id: u32,
    pub checksum: u32,
}

// Packet leaders
pub const PACKET_LEADER_NONE: u32 = 0x00000000;
pub const PACKET_LEADER_KD: u32 = 0x30303030;  // "0000"
pub const PACKET_LEADER_BREAKIN: u32 = 0x62626262;  // "bbbb"
pub const CONTROL_PACKET_LEADER: u32 = 0x69696969;  // "iiii"

// Packet types
pub const PACKET_TYPE_UNUSED: u16 = 0;
pub const PACKET_TYPE_STATE_CHANGE32: u16 = 1;
pub const PACKET_TYPE_STATE_MANIPULATE: u16 = 2;
pub const PACKET_TYPE_DEBUG_IO: u16 = 3;
pub const PACKET_TYPE_ACKNOWLEDGE: u16 = 4;
pub const PACKET_TYPE_RESEND: u16 = 5;
pub const PACKET_TYPE_RESET: u16 = 6;
pub const PACKET_TYPE_STATE_CHANGE64: u16 = 7;
pub const PACKET_TYPE_POLL_BREAKIN: u16 = 8;
pub const PACKET_TYPE_TRACE_IO: u16 = 9;
pub const PACKET_TYPE_CONTROL_REQUEST: u16 = 10;
pub const PACKET_TYPE_FILE_IO: u16 = 11;

// API numbers for state manipulation
pub const API_GET_VERSION: u32 = 0x00000000;
pub const API_READ_MEMORY: u32 = 0x00000001;
pub const API_WRITE_MEMORY: u32 = 0x00000002;
pub const API_GET_CONTEXT: u32 = 0x00000003;
pub const API_SET_CONTEXT: u32 = 0x00000004;
pub const API_WRITE_BREAKPOINT: u32 = 0x00000005;
pub const API_RESTORE_BREAKPOINT: u32 = 0x00000006;
pub const API_CONTINUE: u32 = 0x00000007;
pub const API_READ_CONTROL_SPACE: u32 = 0x00000008;
pub const API_WRITE_CONTROL_SPACE: u32 = 0x00000009;
pub const API_READ_IO_SPACE: u32 = 0x0000000A;
pub const API_WRITE_IO_SPACE: u32 = 0x0000000B;
pub const API_REBOOT: u32 = 0x0000000C;
pub const API_CONTINUE2: u32 = 0x0000000D;

// Exception codes
pub const EXCEPTION_BREAKPOINT: u32 = 0x80000003;
pub const EXCEPTION_SINGLE_STEP: u32 = 0x80000004;
pub const EXCEPTION_ACCESS_VIOLATION: u32 = 0xC0000005;
pub const EXCEPTION_DATATYPE_MISALIGNMENT: u32 = 0x80000002;
pub const EXCEPTION_INT_DIVIDE_BY_ZERO: u32 = 0xC0000094;
pub const EXCEPTION_INT_OVERFLOW: u32 = 0xC0000095;
pub const EXCEPTION_ILLEGAL_INSTRUCTION: u32 = 0xC000001D;
pub const EXCEPTION_STACK_OVERFLOW: u32 = 0xC00000FD;

// State change structure
#[repr(C, packed)]
pub struct StateChange64 {
    pub new_state: u32,
    pub processor_level: u16,
    pub processor: u16,
    pub number_processors: u32,
    pub thread: u64,
    pub program_counter: u64,
    pub exception: ExceptionRecord64,
}

#[repr(C, packed)]
pub struct ExceptionRecord64 {
    pub exception_code: u32,
    pub exception_flags: u32,
    pub exception_record: u64,
    pub exception_address: u64,
    pub number_parameters: u32,
    pub exception_information: [u64; 15],
}

// Manipulate state structure
#[repr(C, packed)]
pub struct ManipulateState64 {
    pub api_number: u32,
    pub processor_level: u16,
    pub processor: u16,
    pub return_status: u32,
}

// Debug I/O structure
#[repr(C, packed)]
pub struct DebugIo {
    pub api_number: u32,
    pub processor_level: u16,
    pub processor: u16,
}

// Transport implementations
pub struct SerialTransport {
    port: u16,
    baud_rate: u32,
}

impl SerialTransport {
    pub fn new() -> Self {
        Self {
            port: 0x3F8,  // COM1
            baud_rate: 115200,
        }
    }
    
    pub fn send_break_sequence(&mut self) {
        // Send break-in sequence over serial
        self.send_bytes(&[0x62, 0x62, 0x62, 0x62]);  // "bbbb"
    }
    
    pub fn send_packet(&mut self, packet: &KdPacket, data: &[u8]) {
        // Send packet header
        let header_bytes = unsafe {
            let ptr = packet as *const KdPacket as *const u8;
            core::slice::from_raw_parts(ptr, mem::size_of::<KdPacket>())
        };
        self.send_bytes(header_bytes);
        
        // Send packet data
        self.send_bytes(data);
    }
    
    pub fn receive_packet(&mut self) -> Option<(KdPacket, Vec<u8>)> {
        // Receive packet header
        let mut header = KdPacket {
            leader: 0,
            packet_type: 0,
            byte_count: 0,
            packet_id: 0,
            checksum: 0,
        };
        
        // Wait for packet leader
        loop {
            let leader = self.receive_u32()?;
            if leader == PACKET_LEADER_KD || leader == CONTROL_PACKET_LEADER {
                header.leader = leader;
                break;
            }
        }
        
        // Read rest of header
        header.packet_type = self.receive_u16()?;
        header.byte_count = self.receive_u16()?;
        header.packet_id = self.receive_u32()?;
        header.checksum = self.receive_u32()?;
        
        // Read packet data
        let mut data = Vec::with_capacity(header.byte_count as usize);
        for _ in 0..header.byte_count {
            data.push(self.receive_byte()?);
        }
        
        // Verify checksum
        if self.calculate_checksum(&data) != header.checksum {
            return None;
        }
        
        Some((header, data))
    }
    
    fn send_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.send_byte(byte);
        }
    }
    
    fn send_byte(&mut self, byte: u8) {
        unsafe {
            // Wait for transmit buffer to be empty
            while (x86_64::instructions::port::Port::<u8>::new(self.port + 5).read() & 0x20) == 0 {}
            // Send byte
            x86_64::instructions::port::Port::<u8>::new(self.port).write(byte);
        }
    }
    
    fn receive_byte(&mut self) -> Option<u8> {
        unsafe {
            // Check if data is available
            if (x86_64::instructions::port::Port::<u8>::new(self.port + 5).read() & 0x01) != 0 {
                Some(x86_64::instructions::port::Port::<u8>::new(self.port).read())
            } else {
                None
            }
        }
    }
    
    fn receive_u16(&mut self) -> Option<u16> {
        let low = self.receive_byte()? as u16;
        let high = self.receive_byte()? as u16;
        Some(low | (high << 8))
    }
    
    fn receive_u32(&mut self) -> Option<u32> {
        let b1 = self.receive_byte()? as u32;
        let b2 = self.receive_byte()? as u32;
        let b3 = self.receive_byte()? as u32;
        let b4 = self.receive_byte()? as u32;
        Some(b1 | (b2 << 8) | (b3 << 16) | (b4 << 24))
    }
    
    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        data.iter().fold(0u32, |acc, &b| acc.wrapping_add(b as u32))
    }
}

pub struct NetworkTransport {
    port: u16,
    connected: bool,
}

impl NetworkTransport {
    pub fn new() -> Self {
        Self {
            port: 50000,  // Default KD network port
            connected: false,
        }
    }
    
    pub fn connect(&mut self, _host: &str) -> bool {
        // Simulate network connection
        self.connected = true;
        true
    }
    
    pub fn send_packet(&mut self, _packet: &KdPacket, _data: &[u8]) {
        if self.connected {
            // Send over network
        }
    }
    
    pub fn receive_packet(&mut self) -> Option<(KdPacket, Vec<u8>)> {
        if self.connected {
            // Receive from network
        }
        None
    }
}

pub struct UsbTransport {
    endpoint: u8,
    connected: bool,
}

impl UsbTransport {
    pub fn new() -> Self {
        Self {
            endpoint: 0,
            connected: false,
        }
    }
    
    pub fn connect(&mut self) -> bool {
        // Initialize USB debug interface
        self.connected = true;
        true
    }
    
    pub fn send_packet(&mut self, _packet: &KdPacket, _data: &[u8]) {
        if self.connected {
            // Send over USB
        }
    }
    
    pub fn receive_packet(&mut self) -> Option<(KdPacket, Vec<u8>)> {
        if self.connected {
            // Receive from USB
        }
        None
    }
}

// Helper functions for packet creation
pub fn create_state_change_packet(
    exception_code: u32,
    exception_address: u64,
    thread: u64,
) -> (KdPacket, Vec<u8>) {
    let state_change = StateChange64 {
        new_state: exception_code,
        processor_level: 0,
        processor: 0,
        number_processors: 1,
        thread,
        program_counter: exception_address,
        exception: ExceptionRecord64 {
            exception_code,
            exception_flags: 0,
            exception_record: 0,
            exception_address,
            number_parameters: 0,
            exception_information: [0; 15],
        },
    };
    
    let data = unsafe {
        let ptr = &state_change as *const StateChange64 as *const u8;
        core::slice::from_raw_parts(ptr, mem::size_of::<StateChange64>()).to_vec()
    };
    
    let packet = KdPacket {
        leader: PACKET_LEADER_KD,
        packet_type: PACKET_TYPE_STATE_CHANGE64,
        byte_count: data.len() as u16,
        packet_id: 0,
        checksum: calculate_checksum(&data),
    };
    
    (packet, data)
}

pub fn create_manipulate_packet(api_number: u32) -> (KdPacket, Vec<u8>) {
    let manipulate = ManipulateState64 {
        api_number,
        processor_level: 0,
        processor: 0,
        return_status: 0,
    };
    
    let data = unsafe {
        let ptr = &manipulate as *const ManipulateState64 as *const u8;
        core::slice::from_raw_parts(ptr, mem::size_of::<ManipulateState64>()).to_vec()
    };
    
    let packet = KdPacket {
        leader: PACKET_LEADER_KD,
        packet_type: PACKET_TYPE_STATE_MANIPULATE,
        byte_count: data.len() as u16,
        packet_id: 0,
        checksum: calculate_checksum(&data),
    };
    
    (packet, data)
}

pub fn calculate_checksum(data: &[u8]) -> u32 {
    data.iter().fold(0u32, |acc, &b| acc.wrapping_add(b as u32))
}