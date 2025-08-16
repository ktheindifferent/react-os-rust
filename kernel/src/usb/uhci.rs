// UHCI (Universal Host Controller Interface) - USB 1.0/1.1
use super::{UsbController, UsbDevice, UsbSpeed, DeviceRequest, ControllerType};
use alloc::vec::Vec;
use alloc::vec;
use x86_64::instructions::port::Port;
use crate::{println, serial_println};

// UHCI I/O Port Registers
const UHCI_CMD: u16 = 0x00;        // Command
const UHCI_STS: u16 = 0x02;        // Status
const UHCI_INTR: u16 = 0x04;       // Interrupt Enable
const UHCI_FRNUM: u16 = 0x06;      // Frame Number
const UHCI_FRBASEADD: u16 = 0x08;  // Frame List Base Address
const UHCI_SOFMOD: u16 = 0x0C;     // Start of Frame Modify
const UHCI_PORTSC1: u16 = 0x10;    // Port 1 Status/Control
const UHCI_PORTSC2: u16 = 0x12;    // Port 2 Status/Control

// UHCI Command Register Bits
const UHCI_CMD_RUN: u16 = 0x0001;
const UHCI_CMD_HCRESET: u16 = 0x0002;
const UHCI_CMD_GRESET: u16 = 0x0004;
const UHCI_CMD_SUSPEND: u16 = 0x0008;
const UHCI_CMD_RESUME: u16 = 0x0010;
const UHCI_CMD_SWDBG: u16 = 0x0020;
const UHCI_CMD_CF: u16 = 0x0040;
const UHCI_CMD_MAXP: u16 = 0x0080;

// UHCI Status Register Bits
const UHCI_STS_USBINT: u16 = 0x0001;
const UHCI_STS_ERROR: u16 = 0x0002;
const UHCI_STS_RD: u16 = 0x0004;
const UHCI_STS_HSE: u16 = 0x0008;
const UHCI_STS_HCPE: u16 = 0x0010;
const UHCI_STS_HCH: u16 = 0x0020;

// Port Status/Control Bits
const UHCI_PORTSC_CCS: u16 = 0x0001;   // Current Connect Status
const UHCI_PORTSC_CSC: u16 = 0x0002;   // Connect Status Change
const UHCI_PORTSC_PE: u16 = 0x0004;    // Port Enabled
const UHCI_PORTSC_PEC: u16 = 0x0008;   // Port Enable Change
const UHCI_PORTSC_LS: u16 = 0x0100;    // Low Speed Device
const UHCI_PORTSC_RD: u16 = 0x0040;    // Resume Detect
const UHCI_PORTSC_RESET: u16 = 0x0200; // Port Reset
const UHCI_PORTSC_SUSPEND: u16 = 0x1000; // Suspend

pub struct UhciController {
    base_port: u16,
    frame_list: Vec<u32>,
    devices: Vec<UsbDevice>,
}

impl UhciController {
    pub fn new(base_port: u16) -> Self {
        Self {
            base_port,
            frame_list: Vec::new(),
            devices: Vec::new(),
        }
    }
    
    fn read_cmd(&self) -> u16 {
        unsafe {
            let mut port = Port::<u16>::new(self.base_port + UHCI_CMD);
            port.read()
        }
    }
    
    fn write_cmd(&self, value: u16) {
        unsafe {
            let mut port = Port::<u16>::new(self.base_port + UHCI_CMD);
            port.write(value);
        }
    }
    
    fn read_status(&self) -> u16 {
        unsafe {
            let mut port = Port::<u16>::new(self.base_port + UHCI_STS);
            port.read()
        }
    }
    
    fn write_status(&self, value: u16) {
        unsafe {
            let mut port = Port::<u16>::new(self.base_port + UHCI_STS);
            port.write(value);
        }
    }
    
    fn read_port_status(&self, port_num: u8) -> u16 {
        let offset = if port_num == 1 { UHCI_PORTSC1 } else { UHCI_PORTSC2 };
        unsafe {
            let mut port = Port::<u16>::new(self.base_port + offset);
            port.read()
        }
    }
    
    fn write_port_status(&self, port_num: u8, value: u16) {
        let offset = if port_num == 1 { UHCI_PORTSC1 } else { UHCI_PORTSC2 };
        unsafe {
            let mut port = Port::<u16>::new(self.base_port + offset);
            port.write(value);
        }
    }
}

impl UsbController for UhciController {
    fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("UHCI: Initializing controller at port 0x{:x}", self.base_port);
        
        // Reset controller
        self.reset()?;
        
        // Allocate and initialize frame list (1024 frames)
        self.frame_list = vec![0x00000001; 1024]; // All entries marked as invalid
        
        // Set frame list base address
        let frame_list_addr = self.frame_list.as_ptr() as u32;
        unsafe {
            let mut port = Port::<u32>::new(self.base_port + UHCI_FRBASEADD);
            port.write(frame_list_addr);
        }
        
        // Clear status
        self.write_status(0xFFFF);
        
        // Enable interrupts
        unsafe {
            let mut port = Port::<u16>::new(self.base_port + UHCI_INTR);
            port.write(0x000F); // Enable all interrupts
        }
        
        // Start the controller
        self.write_cmd(UHCI_CMD_RUN | UHCI_CMD_CF | UHCI_CMD_MAXP);
        
        // Wait for controller to start
        for _ in 0..100 {
            if (self.read_status() & UHCI_STS_HCH) == 0 {
                serial_println!("UHCI: Controller started successfully");
                return Ok(());
            }
            // Small delay
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
        
        Err("UHCI controller failed to start")
    }
    
    fn reset(&mut self) -> Result<(), &'static str> {
        // Global reset
        self.write_cmd(UHCI_CMD_GRESET);
        
        // Wait at least 10ms
        for _ in 0..100000 {
            core::hint::spin_loop();
        }
        
        self.write_cmd(0);
        
        // Host controller reset
        self.write_cmd(UHCI_CMD_HCRESET);
        
        // Wait for reset to complete
        for _ in 0..100 {
            if (self.read_cmd() & UHCI_CMD_HCRESET) == 0 {
                return Ok(());
            }
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
        
        Err("UHCI reset timeout")
    }
    
    fn enumerate_devices(&mut self) -> Vec<UsbDevice> {
        let mut devices = Vec::new();
        
        // Check both ports
        for port_num in 1..=2 {
            let status = self.read_port_status(port_num);
            
            if status & UHCI_PORTSC_CCS != 0 {
                serial_println!("UHCI: Device connected on port {}", port_num);
                
                // Reset port
                self.write_port_status(port_num, UHCI_PORTSC_RESET);
                
                // Wait for reset (at least 10ms)
                for _ in 0..100000 {
                    core::hint::spin_loop();
                }
                
                // Clear reset
                self.write_port_status(port_num, 0);
                
                // Enable port
                self.write_port_status(port_num, UHCI_PORTSC_PE);
                
                // Check if low speed
                let status = self.read_port_status(port_num);
                let speed = if status & UHCI_PORTSC_LS != 0 {
                    UsbSpeed::Low
                } else {
                    UsbSpeed::Full
                };
                
                // Create device
                let mut device = UsbDevice::new(0, speed); // Address 0 initially
                device.port = port_num;
                devices.push(device);
            }
        }
        
        devices
    }
    
    fn control_transfer(&mut self, device: &UsbDevice, request: &DeviceRequest, data: Option<&mut [u8]>) -> Result<usize, &'static str> {
        // This would implement the actual USB control transfer
        // For now, return a stub implementation
        serial_println!("UHCI: Control transfer to device {} (stub)", device.address);
        
        if let Some(data) = data {
            // Fill with dummy data for testing
            for byte in data.iter_mut() {
                *byte = 0;
            }
            Ok(data.len())
        } else {
            Ok(0)
        }
    }
    
    fn bulk_transfer(&mut self, device: &UsbDevice, endpoint: u8, data: &mut [u8], is_write: bool) -> Result<usize, &'static str> {
        // Stub implementation
        serial_println!("UHCI: Bulk transfer to device {} endpoint {} (stub)", 
                       device.address, endpoint);
        Ok(data.len())
    }
    
    fn interrupt_transfer(&mut self, device: &UsbDevice, endpoint: u8, data: &mut [u8]) -> Result<usize, &'static str> {
        // Stub implementation
        serial_println!("UHCI: Interrupt transfer from device {} endpoint {} (stub)", 
                       device.address, endpoint);
        
        // Simulate mouse data for testing
        if device.is_hid() && data.len() >= 4 {
            data[0] = 0;     // No buttons
            data[1] = 1;     // X movement
            data[2] = 0;     // Y movement  
            data[3] = 0;     // Wheel
            Ok(4)
        } else {
            Ok(0)
        }
    }
    
    fn get_controller_type(&self) -> ControllerType {
        ControllerType::Uhci
    }
}

pub fn detect_uhci_controller() -> Option<UhciController> {
    // Check PCI for UHCI controllers
    // Class 0x0C (Serial Bus), Subclass 0x03 (USB), Prog IF 0x00 (UHCI)
    
    // For now, check common I/O ports
    let common_ports = [0x3000, 0x3020, 0x3040, 0x3060];
    
    for &port in &common_ports {
        // Try to read command register
        unsafe {
            let mut cmd_port = Port::<u16>::new(port);
            let cmd = cmd_port.read();
            
            // Check if it looks like a valid UHCI controller
            if cmd != 0xFFFF && cmd != 0x0000 {
                serial_println!("UHCI: Found potential controller at port 0x{:x}", port);
                return Some(UhciController::new(port));
            }
        }
    }
    
    None
}