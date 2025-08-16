use x86_64::instructions::port::Port;
use spin::Mutex;
use lazy_static::lazy_static;
use alloc::collections::VecDeque;

const KEYBOARD_DATA_PORT: u16 = 0x60;
const KEYBOARD_STATUS_PORT: u16 = 0x64;

lazy_static! {
    static ref KEY_BUFFER: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::with_capacity(256));
}

pub fn init_keyboard() {
    crate::serial_println!("Initializing keyboard");
}

pub fn handle_keyboard_interrupt() {
    let mut port = Port::<u8>::new(KEYBOARD_DATA_PORT);
    let scancode: u8 = unsafe { port.read() };
    
    if let Some(key) = scancode_to_ascii(scancode) {
        let mut buffer = KEY_BUFFER.lock();
        if buffer.len() < 256 {
            buffer.push_back(key);
        }
    }
}

pub fn read_key() -> Option<u8> {
    KEY_BUFFER.lock().pop_front()
}

fn scancode_to_ascii(scancode: u8) -> Option<u8> {
    match scancode {
        0x02 => Some(b'1'),
        0x03 => Some(b'2'),
        0x04 => Some(b'3'),
        0x05 => Some(b'4'),
        0x06 => Some(b'5'),
        0x07 => Some(b'6'),
        0x08 => Some(b'7'),
        0x09 => Some(b'8'),
        0x0A => Some(b'9'),
        0x0B => Some(b'0'),
        0x10 => Some(b'q'),
        0x11 => Some(b'w'),
        0x12 => Some(b'e'),
        0x13 => Some(b'r'),
        0x14 => Some(b't'),
        0x15 => Some(b'y'),
        0x16 => Some(b'u'),
        0x17 => Some(b'i'),
        0x18 => Some(b'o'),
        0x19 => Some(b'p'),
        0x1C => Some(b'\n'),
        0x1E => Some(b'a'),
        0x1F => Some(b's'),
        0x20 => Some(b'd'),
        0x21 => Some(b'f'),
        0x22 => Some(b'g'),
        0x23 => Some(b'h'),
        0x24 => Some(b'j'),
        0x25 => Some(b'k'),
        0x26 => Some(b'l'),
        0x2C => Some(b'z'),
        0x2D => Some(b'x'),
        0x2E => Some(b'c'),
        0x2F => Some(b'v'),
        0x30 => Some(b'b'),
        0x31 => Some(b'n'),
        0x32 => Some(b'm'),
        0x39 => Some(b' '),
        _ => None,
    }
}