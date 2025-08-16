use uart_16550::SerialPort;
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::instructions::port::Port;

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

// Check if serial data is available
pub fn has_received_data() -> bool {
    unsafe {
        let mut lsr_port = Port::<u8>::new(0x3F8 + 5); // Line Status Register
        lsr_port.read() & 0x01 != 0
    }
}

// Read a byte from serial port
pub fn read_byte() -> Option<u8> {
    if has_received_data() {
        unsafe {
            let mut data_port = Port::<u8>::new(0x3F8);
            Some(data_port.read())
        }
    } else {
        None
    }
}

// Enable serial interrupt
pub fn enable_interrupt() {
    unsafe {
        // Enable receive data interrupt
        let mut ier_port = Port::<u8>::new(0x3F8 + 1); // Interrupt Enable Register
        ier_port.write(0x01); // Enable received data available interrupt
    }
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed");
    });
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}