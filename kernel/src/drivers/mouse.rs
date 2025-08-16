use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};
use spin::Mutex;
use lazy_static::lazy_static;
use alloc::collections::VecDeque;

const MOUSE_DATA_PORT: u16 = 0x60;
const MOUSE_STATUS_PORT: u16 = 0x64;
const MOUSE_COMMAND_PORT: u16 = 0x64;

const MOUSE_ENABLE_AUX: u8 = 0xA8;
const MOUSE_GET_COMPAQ_STATUS: u8 = 0x20;
const MOUSE_SET_COMPAQ_STATUS: u8 = 0x60;
const MOUSE_USE_DEFAULTS: u8 = 0xF6;
const MOUSE_ENABLE_PACKET_STREAMING: u8 = 0xF4;
const MOUSE_SET_SAMPLE_RATE: u8 = 0xF3;
const MOUSE_GET_DEVICE_ID: u8 = 0xF2;
const MOUSE_SET_RESOLUTION: u8 = 0xE8;
const MOUSE_WRITE_BYTE: u8 = 0xD4;

#[derive(Debug, Clone, Copy)]
pub struct MouseState {
    pub x: i16,
    pub y: i16,
    pub left_button: bool,
    pub right_button: bool,
    pub middle_button: bool,
    pub x_overflow: bool,
    pub y_overflow: bool,
}

impl MouseState {
    pub const fn new() -> Self {
        MouseState {
            x: 0,
            y: 0,
            left_button: false,
            right_button: false,
            middle_button: false,
            x_overflow: false,
            y_overflow: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MousePacket {
    pub dx: i16,
    pub dy: i16,
    pub left_button: bool,
    pub right_button: bool,
    pub middle_button: bool,
    pub z_delta: i8,
}

pub struct MouseDriver {
    data_port: PortReadOnly<u8>,
    command_port: PortWriteOnly<u8>,
    status_port: PortReadOnly<u8>,
    packet_buffer: [u8; 4],
    packet_index: usize,
    has_wheel: bool,
    current_state: MouseState,
    event_queue: VecDeque<MousePacket>,
    screen_width: u16,
    screen_height: u16,
}

impl MouseDriver {
    pub fn new() -> Self {
        MouseDriver {
            data_port: PortReadOnly::new(MOUSE_DATA_PORT),
            command_port: PortWriteOnly::new(MOUSE_COMMAND_PORT),
            status_port: PortReadOnly::new(MOUSE_STATUS_PORT),
            packet_buffer: [0; 4],
            packet_index: 0,
            has_wheel: false,
            current_state: MouseState::new(),
            event_queue: VecDeque::with_capacity(256),
            screen_width: 1024,
            screen_height: 768,
        }
    }
    
    pub fn init(&mut self) -> Result<(), &'static str> {
        unsafe {
            self.wait_write()?;
            self.command_port.write(MOUSE_ENABLE_AUX);
            
            self.wait_write()?;
            self.command_port.write(MOUSE_GET_COMPAQ_STATUS);
            self.wait_read()?;
            let status = self.data_port.read();
            
            let new_status = status | 0x02;
            self.wait_write()?;
            self.command_port.write(MOUSE_SET_COMPAQ_STATUS);
            self.wait_write()?;
            Port::<u8>::new(MOUSE_DATA_PORT).write(new_status);
            
            self.write_mouse_command(MOUSE_USE_DEFAULTS)?;
            self.read_mouse_response()?;
            
            self.has_wheel = self.detect_wheel();
            
            self.write_mouse_command(MOUSE_SET_RESOLUTION)?;
            self.write_mouse_command(3)?;
            
            self.write_mouse_command(MOUSE_SET_SAMPLE_RATE)?;
            self.write_mouse_command(100)?;
            
            self.write_mouse_command(MOUSE_ENABLE_PACKET_STREAMING)?;
            self.read_mouse_response()?;
            
            x86_64::instructions::interrupts::without_interrupts(|| {
                use crate::interrupts::{InterruptIndex, PICS};
                
                let mut pics = PICS.lock();
                let mouse_mask = !(1 << 4);
                let current_mask = unsafe { Port::<u8>::new(0x21).read() };
                unsafe { Port::<u8>::new(0x21).write(current_mask & mouse_mask) };
            });
        }
        
        crate::serial_println!("PS/2 mouse initialized (wheel support: {})", self.has_wheel);
        Ok(())
    }
    
    fn detect_wheel(&mut self) -> bool {
        unsafe {
            if self.write_mouse_command(MOUSE_SET_SAMPLE_RATE).is_err() { return false; }
            if self.write_mouse_command(200).is_err() { return false; }
            
            if self.write_mouse_command(MOUSE_SET_SAMPLE_RATE).is_err() { return false; }
            if self.write_mouse_command(100).is_err() { return false; }
            
            if self.write_mouse_command(MOUSE_SET_SAMPLE_RATE).is_err() { return false; }
            if self.write_mouse_command(80).is_err() { return false; }
            
            if self.write_mouse_command(MOUSE_GET_DEVICE_ID).is_err() { return false; }
            
            if let Ok(id) = self.read_mouse_response() {
                id == 3 || id == 4
            } else {
                false
            }
        }
    }
    
    unsafe fn wait_write(&mut self) -> Result<(), &'static str> {
        let mut timeout = 100000;
        while timeout > 0 {
            if (self.status_port.read() & 0x02) == 0 {
                return Ok(());
            }
            timeout -= 1;
        }
        Err("Mouse write timeout")
    }
    
    unsafe fn wait_read(&mut self) -> Result<(), &'static str> {
        let mut timeout = 100000;
        while timeout > 0 {
            if (self.status_port.read() & 0x01) != 0 {
                return Ok(());
            }
            timeout -= 1;
        }
        Err("Mouse read timeout")
    }
    
    unsafe fn write_mouse_command(&mut self, command: u8) -> Result<(), &'static str> {
        self.wait_write()?;
        self.command_port.write(MOUSE_WRITE_BYTE);
        self.wait_write()?;
        Port::<u8>::new(MOUSE_DATA_PORT).write(command);
        Ok(())
    }
    
    unsafe fn read_mouse_response(&mut self) -> Result<u8, &'static str> {
        self.wait_read()?;
        Ok(self.data_port.read())
    }
    
    pub fn handle_interrupt(&mut self) {
        unsafe {
            if (self.status_port.read() & 0x21) != 0x21 {
                return;
            }
            
            let data = self.data_port.read();
            
            if self.packet_index == 0 && (data & 0x08) == 0 {
                return;
            }
            
            self.packet_buffer[self.packet_index] = data;
            self.packet_index += 1;
            
            let expected_size = if self.has_wheel { 4 } else { 3 };
            
            if self.packet_index >= expected_size {
                self.process_packet();
                self.packet_index = 0;
            }
        }
    }
    
    fn process_packet(&mut self) {
        let flags = self.packet_buffer[0];
        let x_raw = self.packet_buffer[1];
        let y_raw = self.packet_buffer[2];
        
        let x_sign = (flags & 0x10) != 0;
        let y_sign = (flags & 0x20) != 0;
        
        let mut dx = x_raw as i16;
        if x_sign {
            dx |= 0xFF00u16 as i16;
        }
        
        let mut dy = y_raw as i16;
        if y_sign {
            dy |= 0xFF00u16 as i16;
        }
        
        dy = -dy;
        
        self.current_state.x = (self.current_state.x + dx)
            .max(0)
            .min(self.screen_width as i16 - 1);
        
        self.current_state.y = (self.current_state.y + dy)
            .max(0)
            .min(self.screen_height as i16 - 1);
        
        self.current_state.left_button = (flags & 0x01) != 0;
        self.current_state.right_button = (flags & 0x02) != 0;
        self.current_state.middle_button = (flags & 0x04) != 0;
        self.current_state.x_overflow = (flags & 0x40) != 0;
        self.current_state.y_overflow = (flags & 0x80) != 0;
        
        let z_delta = if self.has_wheel {
            self.packet_buffer[3] as i8
        } else {
            0
        };
        
        let packet = MousePacket {
            dx,
            dy,
            left_button: self.current_state.left_button,
            right_button: self.current_state.right_button,
            middle_button: self.current_state.middle_button,
            z_delta,
        };
        
        if self.event_queue.len() < 256 {
            self.event_queue.push_back(packet);
        }
        
        self.update_cursor_position();
    }
    
    fn update_cursor_position(&self) {
        use crate::graphics::desktop::DESKTOP_MANAGER;
        
        if let Some(ref mut desktop) = *DESKTOP_MANAGER.lock() {
            desktop.set_cursor_position(self.current_state.x as i32, self.current_state.y as i32);
        }
    }
    
    pub fn get_state(&self) -> MouseState {
        self.current_state
    }
    
    pub fn poll_event(&mut self) -> Option<MousePacket> {
        self.event_queue.pop_front()
    }
    
    pub fn set_screen_size(&mut self, width: u16, height: u16) {
        self.screen_width = width;
        self.screen_height = height;
        
        self.current_state.x = self.current_state.x.min(width as i16 - 1);
        self.current_state.y = self.current_state.y.min(height as i16 - 1);
    }
}

lazy_static! {
    pub static ref MOUSE_DRIVER: Mutex<MouseDriver> = Mutex::new(MouseDriver::new());
}

pub fn init() -> Result<(), &'static str> {
    let mut driver = MOUSE_DRIVER.lock();
    driver.init()
}

pub fn handle_mouse_interrupt() {
    let mut driver = MOUSE_DRIVER.lock();
    driver.handle_interrupt();
}