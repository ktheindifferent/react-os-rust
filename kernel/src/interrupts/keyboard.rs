use x86_64::instructions::port::Port;
use spin::Mutex;
use lazy_static::lazy_static;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

const KEYBOARD_DATA_PORT: u16 = 0x60;
const KEYBOARD_STATUS_PORT: u16 = 0x64;

lazy_static! {
    static ref KEY_BUFFER: Mutex<VecDeque<KeyEvent>> = Mutex::new(VecDeque::with_capacity(256));
    static ref KEYBOARD_STATE: Mutex<KeyboardState> = Mutex::new(KeyboardState::new());
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyCode {
    Char(u8),
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    Insert,
    Tab,
    Escape,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

struct KeyboardState {
    shift_pressed: bool,
    ctrl_pressed: bool,
    alt_pressed: bool,
    caps_lock: bool,
}

impl KeyboardState {
    fn new() -> Self {
        Self {
            shift_pressed: false,
            ctrl_pressed: false,
            alt_pressed: false,
            caps_lock: false,
        }
    }
}

pub fn init_keyboard() {
    crate::serial_println!("Initializing keyboard");
}

pub fn handle_keyboard_interrupt() {
    let mut port = Port::<u8>::new(KEYBOARD_DATA_PORT);
    let scancode: u8 = unsafe { port.read() };
    
    let mut state = KEYBOARD_STATE.lock();
    let mut buffer = KEY_BUFFER.lock();
    
    // Handle key release events (scancode with bit 7 set)
    if scancode & 0x80 != 0 {
        let release_code = scancode & 0x7F;
        match release_code {
            0x2A | 0x36 => state.shift_pressed = false, // Left/Right Shift
            0x1D => state.ctrl_pressed = false,         // Ctrl
            0x38 => state.alt_pressed = false,          // Alt
            _ => {}
        }
        return;
    }
    
    // Handle special keys and modifiers
    match scancode {
        0x2A | 0x36 => {
            state.shift_pressed = true;
            return;
        }
        0x1D => {
            state.ctrl_pressed = true;
            return;
        }
        0x38 => {
            state.alt_pressed = true;
            return;
        }
        0x3A => {
            state.caps_lock = !state.caps_lock;
            return;
        }
        _ => {}
    }
    
    // Process the key event
    if let Some(key_code) = scancode_to_keycode(scancode, &state) {
        if buffer.len() < 256 {
            buffer.push_back(KeyEvent {
                code: key_code,
                shift: state.shift_pressed,
                ctrl: state.ctrl_pressed,
                alt: state.alt_pressed,
            });
        }
    }
}

pub fn read_key() -> Option<KeyEvent> {
    KEY_BUFFER.lock().pop_front()
}

pub fn read_char() -> Option<u8> {
    let event = read_key()?;
    match event.code {
        KeyCode::Char(c) => Some(c),
        _ => None,
    }
}

fn scancode_to_keycode(scancode: u8, state: &KeyboardState) -> Option<KeyCode> {
    match scancode {
        // Special keys
        0x0F => Some(KeyCode::Tab),
        0x01 => Some(KeyCode::Escape),
        0x0E => Some(KeyCode::Char(0x08)), // Backspace
        0x1C => Some(KeyCode::Char(b'\n')),
        0x39 => Some(KeyCode::Char(b' ')),
        0x53 => Some(KeyCode::Delete),
        0x52 => Some(KeyCode::Insert),
        0x47 => Some(KeyCode::Home),
        0x4F => Some(KeyCode::End),
        0x49 => Some(KeyCode::PageUp),
        0x51 => Some(KeyCode::PageDown),
        
        // Arrow keys
        0x48 => Some(KeyCode::ArrowUp),
        0x50 => Some(KeyCode::ArrowDown),
        0x4B => Some(KeyCode::ArrowLeft),
        0x4D => Some(KeyCode::ArrowRight),
        
        // Function keys
        0x3B => Some(KeyCode::F1),
        0x3C => Some(KeyCode::F2),
        0x3D => Some(KeyCode::F3),
        0x3E => Some(KeyCode::F4),
        0x3F => Some(KeyCode::F5),
        0x40 => Some(KeyCode::F6),
        0x41 => Some(KeyCode::F7),
        0x42 => Some(KeyCode::F8),
        0x43 => Some(KeyCode::F9),
        0x44 => Some(KeyCode::F10),
        0x57 => Some(KeyCode::F11),
        0x58 => Some(KeyCode::F12),
        
        // Number keys
        0x02 => Some(KeyCode::Char(if state.shift_pressed { b'!' } else { b'1' })),
        0x03 => Some(KeyCode::Char(if state.shift_pressed { b'@' } else { b'2' })),
        0x04 => Some(KeyCode::Char(if state.shift_pressed { b'#' } else { b'3' })),
        0x05 => Some(KeyCode::Char(if state.shift_pressed { b'$' } else { b'4' })),
        0x06 => Some(KeyCode::Char(if state.shift_pressed { b'%' } else { b'5' })),
        0x07 => Some(KeyCode::Char(if state.shift_pressed { b'^' } else { b'6' })),
        0x08 => Some(KeyCode::Char(if state.shift_pressed { b'&' } else { b'7' })),
        0x09 => Some(KeyCode::Char(if state.shift_pressed { b'*' } else { b'8' })),
        0x0A => Some(KeyCode::Char(if state.shift_pressed { b'(' } else { b'9' })),
        0x0B => Some(KeyCode::Char(if state.shift_pressed { b')' } else { b'0' })),
        0x0C => Some(KeyCode::Char(if state.shift_pressed { b'_' } else { b'-' })),
        0x0D => Some(KeyCode::Char(if state.shift_pressed { b'+' } else { b'=' })),
        
        // Letter keys
        0x10 => Some(KeyCode::Char(get_letter(b'q', state))),
        0x11 => Some(KeyCode::Char(get_letter(b'w', state))),
        0x12 => Some(KeyCode::Char(get_letter(b'e', state))),
        0x13 => Some(KeyCode::Char(get_letter(b'r', state))),
        0x14 => Some(KeyCode::Char(get_letter(b't', state))),
        0x15 => Some(KeyCode::Char(get_letter(b'y', state))),
        0x16 => Some(KeyCode::Char(get_letter(b'u', state))),
        0x17 => Some(KeyCode::Char(get_letter(b'i', state))),
        0x18 => Some(KeyCode::Char(get_letter(b'o', state))),
        0x19 => Some(KeyCode::Char(get_letter(b'p', state))),
        0x1A => Some(KeyCode::Char(if state.shift_pressed { b'{' } else { b'[' })),
        0x1B => Some(KeyCode::Char(if state.shift_pressed { b'}' } else { b']' })),
        0x1E => Some(KeyCode::Char(get_letter(b'a', state))),
        0x1F => Some(KeyCode::Char(get_letter(b's', state))),
        0x20 => Some(KeyCode::Char(get_letter(b'd', state))),
        0x21 => Some(KeyCode::Char(get_letter(b'f', state))),
        0x22 => Some(KeyCode::Char(get_letter(b'g', state))),
        0x23 => Some(KeyCode::Char(get_letter(b'h', state))),
        0x24 => Some(KeyCode::Char(get_letter(b'j', state))),
        0x25 => Some(KeyCode::Char(get_letter(b'k', state))),
        0x26 => Some(KeyCode::Char(get_letter(b'l', state))),
        0x27 => Some(KeyCode::Char(if state.shift_pressed { b':' } else { b';' })),
        0x28 => Some(KeyCode::Char(if state.shift_pressed { b'"' } else { b'\'' })),
        0x29 => Some(KeyCode::Char(if state.shift_pressed { b'~' } else { b'`' })),
        0x2B => Some(KeyCode::Char(if state.shift_pressed { b'|' } else { b'\\' })),
        0x2C => Some(KeyCode::Char(get_letter(b'z', state))),
        0x2D => Some(KeyCode::Char(get_letter(b'x', state))),
        0x2E => Some(KeyCode::Char(get_letter(b'c', state))),
        0x2F => Some(KeyCode::Char(get_letter(b'v', state))),
        0x30 => Some(KeyCode::Char(get_letter(b'b', state))),
        0x31 => Some(KeyCode::Char(get_letter(b'n', state))),
        0x32 => Some(KeyCode::Char(get_letter(b'm', state))),
        0x33 => Some(KeyCode::Char(if state.shift_pressed { b'<' } else { b',' })),
        0x34 => Some(KeyCode::Char(if state.shift_pressed { b'>' } else { b'.' })),
        0x35 => Some(KeyCode::Char(if state.shift_pressed { b'?' } else { b'/' })),
        
        _ => None,
    }
}

fn get_letter(c: u8, state: &KeyboardState) -> u8 {
    let should_uppercase = state.shift_pressed ^ state.caps_lock;
    if should_uppercase {
        c.to_ascii_uppercase()
    } else {
        c
    }
}