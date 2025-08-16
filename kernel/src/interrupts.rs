use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::{self, Mutex};
use crate::{println, serial_println};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

pub mod keyboard;

pub use keyboard::read_key;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
    // Add more interrupt vectors
    Cascade = PIC_1_OFFSET + 2,  // PIC cascade
    COM2 = PIC_1_OFFSET + 3,      // COM2
    COM1 = PIC_1_OFFSET + 4,      // COM1
    LPT2 = PIC_1_OFFSET + 5,      // LPT2
    Floppy = PIC_1_OFFSET + 6,    // Floppy
    LPT1 = PIC_1_OFFSET + 7,      // LPT1/Spurious
    RTC = PIC_2_OFFSET,           // RTC
    Free1 = PIC_2_OFFSET + 1,     // Free
    Free2 = PIC_2_OFFSET + 2,     // Free
    Free3 = PIC_2_OFFSET + 3,     // Free
    Mouse = PIC_2_OFFSET + 4,     // PS/2 Mouse
    FPU = PIC_2_OFFSET + 5,       // FPU
    PrimaryATA = PIC_2_OFFSET + 6,// Primary ATA
    SecondaryATA = PIC_2_OFFSET + 7,// Secondary ATA
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

// Global keyboard instance - initialized during boot, not in interrupt handler
pub static KEYBOARD: Mutex<Option<Keyboard<layouts::Us104Key, ScancodeSet1>>> = Mutex::new(None);

// Keyboard modifier state
pub struct KeyboardModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub caps_lock: bool,
}

pub static KEYBOARD_MODIFIERS: Mutex<KeyboardModifiers> = Mutex::new(KeyboardModifiers {
    shift: false,
    ctrl: false,
    alt: false,
    caps_lock: false,
});

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(crate::gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[InterruptIndex::Timer.as_usize()]
            .set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()]
            .set_handler_fn(keyboard_interrupt_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        
        // Add spurious interrupt handlers for both PICs
        idt[InterruptIndex::LPT1.as_usize()]
            .set_handler_fn(spurious_interrupt_handler_pic1);
        idt[(PIC_2_OFFSET + 15) as usize]
            .set_handler_fn(spurious_interrupt_handler_pic2);
        
        // Skip serial port handler - using polling instead
        idt[InterruptIndex::COM1.as_usize()]
            .set_handler_fn(default_interrupt_handler);
        idt[InterruptIndex::COM2.as_usize()]
            .set_handler_fn(default_interrupt_handler);
        
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

pub fn init_keyboard() {
    // Initialize keyboard during boot, not in interrupt handler
    let keyboard = Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);
    *KEYBOARD.lock() = Some(keyboard);
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: InterruptStackFrame)
{
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn default_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    // Send EOI to the PICs for the interrupt
    unsafe {
        PICS.lock().notify_end_of_interrupt(PIC_1_OFFSET);
    }
}

extern "x86-interrupt" fn serial_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    // Read and process serial input
    while let Some(byte) = crate::serial::read_byte() {
        // Convert byte to char and send to keyboard handler
        if let Some(handler) = *KEYBOARD_HANDLER.lock() {
            // Handle special characters
            match byte {
                0x0D => handler('\n'), // Carriage return -> newline
                0x08 | 0x7F => handler('\x08'), // Backspace/Delete
                b if b.is_ascii() => handler(byte as char),
                _ => {} // Ignore non-ASCII
            }
        }
    }
    
    // Send EOI to the PICs
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::COM1.as_u8());
    }
}

extern "x86-interrupt" fn spurious_interrupt_handler_pic1(
    _stack_frame: InterruptStackFrame)
{
    // Spurious interrupt from PIC1 - don't send EOI
    serial_println!("Spurious interrupt from PIC1");
}

extern "x86-interrupt" fn spurious_interrupt_handler_pic2(
    _stack_frame: InterruptStackFrame)
{
    // Spurious interrupt from PIC2 - only send EOI to PIC1
    unsafe {
        PICS.lock().notify_end_of_interrupt(PIC_1_OFFSET);
    }
    serial_println!("Spurious interrupt from PIC2");
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame, error_code: u64) -> !
{
    // Double fault is critical - try to save as much info as possible
    serial_println!("\n=== CRITICAL: DOUBLE FAULT EXCEPTION ===");
    serial_println!("Error Code: {:#x}", error_code);
    serial_println!("Stack Frame: {:#?}", stack_frame);
    
    // Try to get more CPU state info
    let cr2: u64;
    let cr3: u64;
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) cr2);
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
    }
    serial_println!("CR2 (Page Fault Address): {:#x}", cr2);
    serial_println!("CR3 (Page Table Base): {:#x}", cr3);
    
    panic!("EXCEPTION: DOUBLE FAULT - System cannot recover");
}

// Global timer tick counter
pub static TIMER_TICKS: Mutex<u64> = Mutex::new(0);

extern "x86-interrupt" fn timer_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    // Send EOI first to prevent interrupt stacking
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
    
    // Increment timer tick counter
    let ticks = {
        let mut counter = TIMER_TICKS.lock();
        *counter += 1;
        *counter
    };
    
    // Update system timer
    if let Some(mut timer) = crate::timer::TIMER.try_lock() {
        timer.tick();
    }
    
    // Call process scheduler every 10 ticks, but use try_lock to avoid deadlocks
    if ticks % 10 == 0 {  // Schedule every 10 ticks
        use crate::process::executor::EXECUTOR;
        // Only try to schedule if we can get the lock
        if let Some(mut executor) = EXECUTOR.try_lock() {
            executor.timer_tick();
        }
        // If we can't get the lock, skip this scheduling tick
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    use x86_64::instructions::port::Port;
    use pc_keyboard::{KeyCode, KeyState};
    
    // Read scancode immediately to clear the keyboard buffer
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    
    // Send EOI early to prevent interrupt stacking
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
    
    // Process keyboard input if keyboard is initialized
    if let Some(ref mut keyboard) = *KEYBOARD.lock() {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            // Update modifier states
            let mut modifiers = KEYBOARD_MODIFIERS.lock();
            match key_event.code {
                KeyCode::ShiftLeft | KeyCode::ShiftRight => {
                    modifiers.shift = key_event.state == KeyState::Down;
                },
                KeyCode::ControlLeft | KeyCode::ControlRight => {
                    modifiers.ctrl = key_event.state == KeyState::Down;
                },
                KeyCode::AltLeft | KeyCode::AltRight => {
                    modifiers.alt = key_event.state == KeyState::Down;
                },
                KeyCode::CapsLock => {
                    if key_event.state == KeyState::Down {
                        modifiers.caps_lock = !modifiers.caps_lock;
                    }
                },
                _ => {},
            }
            
            // Process the key event
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(mut character) => {
                        // Apply Caps Lock for letters
                        if modifiers.caps_lock && character.is_ascii_lowercase() {
                            character = character.to_ascii_uppercase();
                        } else if modifiers.caps_lock && character.is_ascii_uppercase() {
                            character = character.to_ascii_lowercase();
                        }
                        
                        // Handle Ctrl combinations
                        if modifiers.ctrl {
                            match character {
                                'c' | 'C' => {
                                    serial_println!("Ctrl+C pressed - Interrupt");
                                    // Could send interrupt signal to current process
                                },
                                'l' | 'L' => {
                                    // Clear screen
                                    crate::vga_buffer::clear_screen();
                                    if let Some(_handler) = *KEYBOARD_HANDLER.lock() {
                                        // Re-show prompt after clear
                                        crate::print!("ReactOS> ");
                                    }
                                },
                                'a' | 'A' => {
                                    serial_println!("Ctrl+A pressed - Select All");
                                },
                                'd' | 'D' => {
                                    serial_println!("Ctrl+D pressed - EOF");
                                },
                                _ => {
                                    // Pass Ctrl+key to handler if available
                                    if let Some(handler) = *KEYBOARD_HANDLER.lock() {
                                        handler(character);
                                    }
                                },
                            }
                        } else {
                            // Normal character - pass to handler
                            if let Some(handler) = *KEYBOARD_HANDLER.lock() {
                                handler(character);
                            } else {
                                // Default: just echo
                                crate::print!("{}", character);
                                if character == '\n' {
                                    crate::print!("ReactOS> ");
                                }
                            }
                        }
                    },
                    DecodedKey::RawKey(key) => {
                        // Handle special keys with modifiers
                        use pc_keyboard::KeyCode;
                        let modifiers = KEYBOARD_MODIFIERS.lock();
                        
                        match key {
                            KeyCode::F1 => {
                                if modifiers.alt {
                                    serial_println!("Alt+F1 pressed - System menu");
                                } else {
                                    serial_println!("F1 pressed - Help");
                                    if let Some(handler) = *KEYBOARD_HANDLER.lock() {
                                        // Send help command
                                        for c in "help\n".chars() {
                                            handler(c);
                                        }
                                    }
                                }
                            },
                            KeyCode::F12 => {
                                if modifiers.ctrl && modifiers.alt {
                                    serial_println!("Ctrl+Alt+F12 - System debug");
                                } else {
                                    serial_println!("F12 pressed");
                                }
                            },
                            KeyCode::Delete => {
                                if modifiers.ctrl && modifiers.alt {
                                    serial_println!("Ctrl+Alt+Del pressed - System interrupt");
                                    // Could trigger reboot or task manager
                                }
                            },
                            KeyCode::ArrowUp | KeyCode::ArrowDown => {
                                // Could implement command history navigation
                                serial_println!("Arrow key pressed - History navigation not yet implemented");
                            },
                            _ => {},
                        }
                    },
                }
            }
        }
    }
    // EOI already sent at the beginning of the handler
}

// Keyboard input handler callback
type KeyboardHandler = fn(char);
pub static KEYBOARD_HANDLER: Mutex<Option<KeyboardHandler>> = Mutex::new(None);

pub fn set_keyboard_handler(handler: fn(char)) {
    *KEYBOARD_HANDLER.lock() = Some(handler);
}

use x86_64::structures::idt::PageFaultErrorCode;
use crate::hlt_loop;

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    let addr = Cr2::read();
    
    // Check if this is a stack overflow
    let rsp = stack_frame.stack_pointer.as_u64();
    let fault_addr = addr.as_u64();
    
    // Typical stack overflow: fault address is close to stack pointer
    if fault_addr.saturating_sub(rsp) < 0x1000 || rsp.saturating_sub(fault_addr) < 0x1000 {
        serial_println!("\n=== STACK OVERFLOW DETECTED ===");
        serial_println!("Stack Pointer: {:#x}", rsp);
        serial_println!("Fault Address: {:#x}", fault_addr);
        serial_println!("Instruction Pointer: {:#x}", stack_frame.instruction_pointer.as_u64());
        println!("\n=== STACK OVERFLOW DETECTED ===");
        println!("Stack exhausted at address: {:#x}", fault_addr);
        panic!("Stack overflow - increase stack size or reduce recursion");
    }
    
    // Try to handle the page fault with demand paging
    if let Err(e) = crate::memory::demand_paging::handle_page_fault(addr, error_code.bits()) {
        // Page fault couldn't be handled
        serial_println!("\n=== PAGE FAULT ===");
        serial_println!("Address: {:?}", addr);
        serial_println!("Error Code: {:?}", error_code);
        serial_println!("  Present: {}", error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION));
        serial_println!("  Write: {}", error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE));
        serial_println!("  User mode: {}", error_code.contains(PageFaultErrorCode::USER_MODE));
        serial_println!("  Instruction fetch: {}", error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH));
        serial_println!("Handler Error: {}", e);
        
        println!("EXCEPTION: PAGE FAULT");
        println!("Accessed Address: {:?}", addr);
        println!("Error Code: {:?}", error_code);
        println!("Handler Error: {}", e);
        println!("{:#?}", stack_frame);
        hlt_loop();
    }
    // Page fault handled successfully, return to continue execution
}