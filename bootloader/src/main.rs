#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod secure_boot;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Early security initialization
    secure_boot::early_init();
    
    // Verify kernel image before loading
    if !secure_boot::verify_kernel() {
        // Halt if verification fails
        panic!("Kernel verification failed");
    }
    
    // Enable early security features
    secure_boot::enable_early_protections();
    
    // Continue with normal boot process
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Output error to serial port if available
    if let Some(location) = info.location() {
        // Would output to serial: file, line, column
    }
    
    // Halt the CPU
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}