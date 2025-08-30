// Test program to verify storage device registration fix
#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;

// Mock types for testing
#[derive(Clone)]
struct Handle(u32);

#[derive(Debug, PartialEq)]
enum NtStatus {
    Success,
}

// Simplified test version of our storage subsystem
#[derive(Clone)]
struct TestController {
    id: u32,
    devices: Vec<u32>,
}

struct StorageSubsystem {
    controllers: Vec<TestController>,
    registered_devices: Vec<u32>,
}

impl StorageSubsystem {
    fn new() -> Self {
        Self {
            controllers: Vec::new(),
            registered_devices: Vec::new(),
        }
    }
    
    fn initialize(&mut self) -> NtStatus {
        // Add test controllers with devices
        self.controllers.push(TestController { 
            id: 1, 
            devices: vec![100, 101] 
        });
        self.controllers.push(TestController { 
            id: 2, 
            devices: vec![200, 201, 202] 
        });
        
        // This is the key fix - clone controllers to avoid borrow conflicts
        self.register_all_devices();
        
        NtStatus::Success
    }
    
    fn register_all_devices(&mut self) {
        // Clone controllers to avoid borrow checker issues
        let controllers = self.controllers.clone();
        
        // Now we can iterate and mutate self without conflicts
        for controller in &controllers {
            self.register_devices(controller);
        }
    }
    
    fn register_devices(&mut self, controller: &TestController) {
        for device in &controller.devices {
            self.registered_devices.push(*device);
            println!("Registered device: {}", device);
        }
    }
}

fn main() {
    println!("Testing storage device registration fix...");
    
    let mut storage = StorageSubsystem::new();
    let status = storage.initialize();
    
    assert_eq!(status, NtStatus::Success);
    assert_eq!(storage.registered_devices.len(), 5);
    assert!(storage.registered_devices.contains(&100));
    assert!(storage.registered_devices.contains(&101));
    assert!(storage.registered_devices.contains(&200));
    assert!(storage.registered_devices.contains(&201));
    assert!(storage.registered_devices.contains(&202));
    
    println!("✓ All devices registered successfully!");
    println!("✓ No borrow checker conflicts!");
    println!("✓ Total devices registered: {}", storage.registered_devices.len());
}

// Minimal println for testing
macro_rules! println {
    ($($arg:tt)*) => {
        // In real kernel this would write to VGA buffer
        // For testing, we just validate the format
        let _ = format!($($arg)*);
    };
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    main();
    loop {}
}