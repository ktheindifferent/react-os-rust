// USB HID (Human Interface Device) Implementation
use super::{UsbDevice, UsbController, DeviceRequest, EndpointInfo, TransferType};
use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::{println, serial_println};

// HID Class Specific Requests
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum HidRequest {
    GetReport = 0x01,
    GetIdle = 0x02,
    GetProtocol = 0x03,
    SetReport = 0x09,
    SetIdle = 0x0A,
    SetProtocol = 0x0B,
}

// HID Report Types
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ReportType {
    Input = 0x01,
    Output = 0x02,
    Feature = 0x03,
}

// HID Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct HidDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub bcd_hid: u16,
    pub country_code: u8,
    pub num_descriptors: u8,
    pub report_descriptor_type: u8,
    pub report_descriptor_length: u16,
}

// HID Protocol
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HidProtocol {
    None = 0,
    Keyboard = 1,
    Mouse = 2,
}

// Mouse Button States
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseButtons {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub button4: bool,
    pub button5: bool,
}

// Mouse State
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub wheel: i8,
    pub buttons: MouseButtons,
}

// Keyboard Modifiers
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyboardModifiers {
    pub left_ctrl: bool,
    pub left_shift: bool,
    pub left_alt: bool,
    pub left_gui: bool,
    pub right_ctrl: bool,
    pub right_shift: bool,
    pub right_alt: bool,
    pub right_gui: bool,
}

// Keyboard State
#[derive(Debug, Clone, Default)]
pub struct KeyboardState {
    pub modifiers: KeyboardModifiers,
    pub pressed_keys: Vec<u8>,  // HID usage codes
}

// HID Device
pub struct HidDevice {
    pub device: UsbDevice,
    pub protocol: HidProtocol,
    pub report_descriptor: Vec<u8>,
    pub interrupt_endpoint: Option<EndpointInfo>,
    pub report_size: usize,
}

impl HidDevice {
    pub fn new(device: UsbDevice) -> Self {
        // Find interrupt IN endpoint
        let interrupt_endpoint = device.endpoints.iter()
            .find(|ep| {
                ep.transfer_type == TransferType::Interrupt && 
                (ep.address & 0x80) != 0  // IN endpoint
            })
            .cloned();
        
        Self {
            device,
            protocol: HidProtocol::None,
            report_descriptor: Vec::new(),
            interrupt_endpoint,
            report_size: 8,  // Default
        }
    }
    
    pub fn init(&mut self) -> Result<(), &'static str> {
        // Get HID descriptor
        self.get_hid_descriptor()?;
        
        // Get report descriptor
        self.get_report_descriptor()?;
        
        // Parse report descriptor to determine device type
        self.determine_protocol()?;
        
        // Set boot protocol for simplicity
        if self.protocol != HidProtocol::None {
            self.set_protocol(0)?;  // Boot protocol
        }
        
        // Set idle rate (0 = infinite)
        self.set_idle(0, 0)?;
        
        serial_println!("HID: Initialized {} device", 
                       match self.protocol {
                           HidProtocol::Keyboard => "keyboard",
                           HidProtocol::Mouse => "mouse",
                           _ => "unknown"
                       });
        
        Ok(())
    }
    
    fn get_hid_descriptor(&mut self) -> Result<(), &'static str> {
        // HID descriptor is usually part of the configuration descriptor
        // For now, we'll skip this and rely on report descriptor
        Ok(())
    }
    
    fn get_report_descriptor(&mut self) -> Result<(), &'static str> {
        // This would need controller access to perform USB transfer
        // For now, use default report sizes
        match self.device.protocol {
            1 => {
                // Keyboard boot protocol
                self.protocol = HidProtocol::Keyboard;
                self.report_size = 8;
            }
            2 => {
                // Mouse boot protocol
                self.protocol = HidProtocol::Mouse;
                self.report_size = 4;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn determine_protocol(&mut self) -> Result<(), &'static str> {
        // Parse report descriptor to determine if keyboard or mouse
        // For boot protocol devices, we can use the interface protocol
        if self.device.subclass == 1 {  // Boot interface subclass
            match self.device.protocol {
                1 => self.protocol = HidProtocol::Keyboard,
                2 => self.protocol = HidProtocol::Mouse,
                _ => {}
            }
        }
        
        Ok(())
    }
    
    fn set_protocol(&mut self, protocol: u8) -> Result<(), &'static str> {
        // Set boot protocol (0) or report protocol (1)
        // This would need controller access
        Ok(())
    }
    
    fn set_idle(&mut self, duration: u8, report_id: u8) -> Result<(), &'static str> {
        // Set idle rate for the device
        // This would need controller access
        Ok(())
    }
}

// Mouse Report Parser (Boot Protocol)
pub fn parse_mouse_report(data: &[u8]) -> MouseState {
    if data.len() < 3 {
        return MouseState::default();
    }
    
    let mut state = MouseState::default();
    
    // Byte 0: Button states
    state.buttons.left = (data[0] & 0x01) != 0;
    state.buttons.right = (data[0] & 0x02) != 0;
    state.buttons.middle = (data[0] & 0x04) != 0;
    
    // Byte 1: X movement (signed)
    state.x = data[1] as i8 as i32;
    
    // Byte 2: Y movement (signed)
    state.y = data[2] as i8 as i32;
    
    // Byte 3: Wheel (if present)
    if data.len() >= 4 {
        state.wheel = data[3] as i8;
    }
    
    state
}

// Keyboard Report Parser (Boot Protocol)
pub fn parse_keyboard_report(data: &[u8]) -> KeyboardState {
    if data.len() < 8 {
        return KeyboardState::default();
    }
    
    let mut state = KeyboardState::default();
    
    // Byte 0: Modifier keys
    state.modifiers.left_ctrl = (data[0] & 0x01) != 0;
    state.modifiers.left_shift = (data[0] & 0x02) != 0;
    state.modifiers.left_alt = (data[0] & 0x04) != 0;
    state.modifiers.left_gui = (data[0] & 0x08) != 0;
    state.modifiers.right_ctrl = (data[0] & 0x10) != 0;
    state.modifiers.right_shift = (data[0] & 0x20) != 0;
    state.modifiers.right_alt = (data[0] & 0x40) != 0;
    state.modifiers.right_gui = (data[0] & 0x80) != 0;
    
    // Byte 1: Reserved (usually 0)
    
    // Bytes 2-7: Key codes (up to 6 simultaneous keys)
    for i in 2..8 {
        if data[i] != 0 {
            state.pressed_keys.push(data[i]);
        }
    }
    
    state
}

// HID to ASCII conversion for common keys
pub fn hid_to_ascii(hid_code: u8, shift: bool) -> Option<char> {
    match hid_code {
        0x04..=0x1D => {
            // A-Z
            let base = if shift { b'A' } else { b'a' };
            Some((base + (hid_code - 0x04)) as char)
        }
        0x1E..=0x26 => {
            // 1-9
            if shift {
                match hid_code {
                    0x1E => Some('!'),
                    0x1F => Some('@'),
                    0x20 => Some('#'),
                    0x21 => Some('$'),
                    0x22 => Some('%'),
                    0x23 => Some('^'),
                    0x24 => Some('&'),
                    0x25 => Some('*'),
                    0x26 => Some('('),
                    _ => None,
                }
            } else {
                Some((b'1' + (hid_code - 0x1E)) as char)
            }
        }
        0x27 => Some(if shift { ')' } else { '0' }),
        0x28 => Some('\n'),  // Enter
        0x29 => None,        // Escape
        0x2A => Some('\x08'), // Backspace
        0x2B => Some('\t'),  // Tab
        0x2C => Some(' '),   // Space
        0x2D => Some(if shift { '_' } else { '-' }),
        0x2E => Some(if shift { '+' } else { '=' }),
        0x2F => Some(if shift { '{' } else { '[' }),
        0x30 => Some(if shift { '}' } else { ']' }),
        0x31 => Some(if shift { '|' } else { '\\' }),
        0x33 => Some(if shift { ':' } else { ';' }),
        0x34 => Some(if shift { '"' } else { '\'' }),
        0x35 => Some(if shift { '~' } else { '`' }),
        0x36 => Some(if shift { '<' } else { ',' }),
        0x37 => Some(if shift { '>' } else { '.' }),
        0x38 => Some(if shift { '?' } else { '/' }),
        _ => None,
    }
}

// Mouse Driver
pub struct MouseDriver {
    current_state: MouseState,
    absolute_x: i32,
    absolute_y: i32,
    screen_width: u32,
    screen_height: u32,
    sensitivity: f32,
    event_handler: Option<fn(MouseState)>,
}

impl MouseDriver {
    pub fn new(screen_width: u32, screen_height: u32) -> Self {
        Self {
            current_state: MouseState::default(),
            absolute_x: (screen_width / 2) as i32,
            absolute_y: (screen_height / 2) as i32,
            screen_width,
            screen_height,
            sensitivity: 1.0,
            event_handler: None,
        }
    }
    
    pub fn process_report(&mut self, data: &[u8]) {
        let state = parse_mouse_report(data);
        
        // Update absolute position
        self.absolute_x += (state.x as f32 * self.sensitivity) as i32;
        self.absolute_y -= (state.y as f32 * self.sensitivity) as i32;  // Y is inverted
        
        // Clamp to screen bounds
        self.absolute_x = self.absolute_x.max(0).min(self.screen_width as i32 - 1);
        self.absolute_y = self.absolute_y.max(0).min(self.screen_height as i32 - 1);
        
        // Update current state
        self.current_state = state;
        self.current_state.x = self.absolute_x;
        self.current_state.y = self.absolute_y;
        
        // Call event handler if set
        if let Some(handler) = self.event_handler {
            handler(self.current_state);
        }
    }
    
    pub fn set_event_handler(&mut self, handler: fn(MouseState)) {
        self.event_handler = Some(handler);
    }
    
    pub fn get_position(&self) -> (i32, i32) {
        (self.absolute_x, self.absolute_y)
    }
    
    pub fn get_buttons(&self) -> MouseButtons {
        self.current_state.buttons
    }
    
    pub fn set_sensitivity(&mut self, sensitivity: f32) {
        self.sensitivity = sensitivity.max(0.1).min(5.0);
    }
}

// Keyboard Driver  
pub struct KeyboardDriver {
    current_state: KeyboardState,
    previous_keys: Vec<u8>,
    event_handler: Option<fn(u8, bool)>,  // key_code, is_pressed
}

impl KeyboardDriver {
    pub fn new() -> Self {
        Self {
            current_state: KeyboardState::default(),
            previous_keys: Vec::new(),
            event_handler: None,
        }
    }
    
    pub fn process_report(&mut self, data: &[u8]) {
        let state = parse_keyboard_report(data);
        
        // Detect key press events
        for &key in &state.pressed_keys {
            if !self.previous_keys.contains(&key) {
                // Key pressed
                if let Some(handler) = self.event_handler {
                    handler(key, true);
                }
                
                // Convert to ASCII and print
                if let Some(ch) = hid_to_ascii(key, state.modifiers.left_shift || state.modifiers.right_shift) {
                    // This would normally go to the input buffer
                    // For now, just log it
                    serial_println!("HID Keyboard: '{}'", ch);
                }
            }
        }
        
        // Detect key release events
        for &key in &self.previous_keys {
            if !state.pressed_keys.contains(&key) {
                // Key released
                if let Some(handler) = self.event_handler {
                    handler(key, false);
                }
            }
        }
        
        // Update state
        self.current_state = state.clone();
        self.previous_keys = state.pressed_keys;
    }
    
    pub fn set_event_handler(&mut self, handler: fn(u8, bool)) {
        self.event_handler = Some(handler);
    }
    
    pub fn get_modifiers(&self) -> KeyboardModifiers {
        self.current_state.modifiers
    }
}

// Global HID Manager
pub struct HidManager {
    devices: Vec<HidDevice>,
    mouse_driver: Option<MouseDriver>,
    keyboard_driver: Option<KeyboardDriver>,
}

impl HidManager {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            mouse_driver: None,
            keyboard_driver: None,
        }
    }
    
    pub fn add_device(&mut self, mut device: HidDevice) -> Result<(), &'static str> {
        device.init()?;
        
        match device.protocol {
            HidProtocol::Mouse => {
                if self.mouse_driver.is_none() {
                    self.mouse_driver = Some(MouseDriver::new(1024, 768));  // Default resolution
                    serial_println!("HID: Mouse driver initialized");
                }
            }
            HidProtocol::Keyboard => {
                if self.keyboard_driver.is_none() {
                    self.keyboard_driver = Some(KeyboardDriver::new());
                    serial_println!("HID: Keyboard driver initialized");
                }
            }
            _ => {}
        }
        
        self.devices.push(device);
        Ok(())
    }
    
    pub fn process_interrupt(&mut self, device_address: u8, data: &[u8]) {
        // Find the device
        if let Some(device) = self.devices.iter().find(|d| d.device.address == device_address) {
            match device.protocol {
                HidProtocol::Mouse => {
                    if let Some(ref mut driver) = self.mouse_driver {
                        driver.process_report(data);
                    }
                }
                HidProtocol::Keyboard => {
                    if let Some(ref mut driver) = self.keyboard_driver {
                        driver.process_report(data);
                    }
                }
                _ => {}
            }
        }
    }
    
    pub fn get_mouse_driver(&mut self) -> Option<&mut MouseDriver> {
        self.mouse_driver.as_mut()
    }
    
    pub fn get_keyboard_driver(&mut self) -> Option<&mut KeyboardDriver> {
        self.keyboard_driver.as_mut()
    }
}

lazy_static! {
    pub static ref HID_MANAGER: Mutex<HidManager> = Mutex::new(HidManager::new());
}

pub fn init_hid_device(device: &UsbDevice) -> Result<(), &'static str> {
    let hid_device = HidDevice::new(device.clone());
    HID_MANAGER.lock().add_device(hid_device)?;
    Ok(())
}

pub fn process_hid_interrupt(device_address: u8, data: &[u8]) {
    HID_MANAGER.lock().process_interrupt(device_address, data);
}

// Mouse event handler for integration with window system
pub fn set_mouse_handler(handler: fn(MouseState)) {
    if let Some(ref mut driver) = HID_MANAGER.lock().mouse_driver {
        driver.set_event_handler(handler);
    }
}

// Keyboard event handler for integration with input system
pub fn set_keyboard_handler(handler: fn(u8, bool)) {
    if let Some(ref mut driver) = HID_MANAGER.lock().keyboard_driver {
        driver.set_event_handler(handler);
    }
}