// EHCI (Enhanced Host Controller Interface) - USB 2.0
use super::{UsbController, UsbDevice, UsbSpeed, DeviceRequest, ControllerType};
use alloc::vec::Vec;
use crate::{println, serial_println};
use crate::memory::PHYS_MEM_OFFSET;

// EHCI Capability Registers
#[repr(C, packed)]
struct EhciCapRegs {
    caplength: u8,
    reserved: u8,
    hciversion: u16,
    hcsparams: u32,
    hccparams: u32,
    hcsp_portroute: u64,
}

// EHCI Operational Registers Offsets
const EHCI_USBCMD: usize = 0x00;
const EHCI_USBSTS: usize = 0x04;
const EHCI_USBINTR: usize = 0x08;
const EHCI_FRINDEX: usize = 0x0C;
const EHCI_CTRLDSSEGMENT: usize = 0x10;
const EHCI_PERIODICLISTBASE: usize = 0x14;
const EHCI_ASYNCLISTADDR: usize = 0x18;
const EHCI_CONFIGFLAG: usize = 0x40;
const EHCI_PORTSC: usize = 0x44;

// Command Register Bits
const CMD_RUN: u32 = 1 << 0;
const CMD_RESET: u32 = 1 << 1;
const CMD_PSE: u32 = 1 << 4;  // Periodic Schedule Enable
const CMD_ASE: u32 = 1 << 5;  // Async Schedule Enable

// Status Register Bits
const STS_INT: u32 = 1 << 0;
const STS_ERR: u32 = 1 << 1;
const STS_PCD: u32 = 1 << 2;  // Port Change Detect
const STS_HALT: u32 = 1 << 12;
const STS_RECLAIM: u32 = 1 << 13;
const STS_PSS: u32 = 1 << 14; // Periodic Schedule Status
const STS_ASS: u32 = 1 << 15; // Async Schedule Status

pub struct EhciController {
    base_addr: u64,
    op_regs: u64,
    devices: Vec<UsbDevice>,
}

impl EhciController {
    pub fn new(base_addr: u64) -> Self {
        // Calculate operational registers offset
        let cap_regs = (PHYS_MEM_OFFSET + base_addr) as *const EhciCapRegs;
        let caplength = unsafe { (*cap_regs).caplength };
        let op_regs = base_addr + caplength as u64;
        
        Self {
            base_addr,
            op_regs,
            devices: Vec::new(),
        }
    }
    
    fn read_op_reg(&self, offset: usize) -> u32 {
        unsafe {
            let addr = (PHYS_MEM_OFFSET + self.op_regs + offset as u64) as *const u32;
            addr.read_volatile()
        }
    }
    
    fn write_op_reg(&self, offset: usize, value: u32) {
        unsafe {
            let addr = (PHYS_MEM_OFFSET + self.op_regs + offset as u64) as *mut u32;
            addr.write_volatile(value);
        }
    }
}

impl UsbController for EhciController {
    fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("EHCI: Initializing controller at 0x{:x}", self.base_addr);
        
        // Reset controller
        self.reset()?;
        
        // Set up periodic list (stub)
        // Set up async list (stub)
        
        // Enable interrupts
        self.write_op_reg(EHCI_USBINTR, 0x3F);
        
        // Start controller
        let cmd = self.read_op_reg(EHCI_USBCMD);
        self.write_op_reg(EHCI_USBCMD, cmd | CMD_RUN);
        
        // Wait for controller to start
        for _ in 0..100 {
            if (self.read_op_reg(EHCI_USBSTS) & STS_HALT) == 0 {
                serial_println!("EHCI: Controller started successfully");
                return Ok(());
            }
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
        
        Err("EHCI controller failed to start")
    }
    
    fn reset(&mut self) -> Result<(), &'static str> {
        // Stop controller
        let cmd = self.read_op_reg(EHCI_USBCMD);
        self.write_op_reg(EHCI_USBCMD, cmd & !CMD_RUN);
        
        // Wait for halt
        for _ in 0..100 {
            if self.read_op_reg(EHCI_USBSTS) & STS_HALT != 0 {
                break;
            }
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
        
        // Reset
        self.write_op_reg(EHCI_USBCMD, CMD_RESET);
        
        // Wait for reset to complete
        for _ in 0..100 {
            if self.read_op_reg(EHCI_USBCMD) & CMD_RESET == 0 {
                return Ok(());
            }
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
        
        Err("EHCI reset timeout")
    }
    
    fn enumerate_devices(&mut self) -> Vec<UsbDevice> {
        // Stub implementation
        serial_println!("EHCI: Device enumeration (stub)");
        Vec::new()
    }
    
    fn control_transfer(&mut self, device: &UsbDevice, request: &DeviceRequest, data: Option<&mut [u8]>) -> Result<usize, &'static str> {
        // Stub implementation
        serial_println!("EHCI: Control transfer (stub)");
        Ok(0)
    }
    
    fn bulk_transfer(&mut self, device: &UsbDevice, endpoint: u8, data: &mut [u8], is_write: bool) -> Result<usize, &'static str> {
        // Stub implementation
        Ok(data.len())
    }
    
    fn interrupt_transfer(&mut self, device: &UsbDevice, endpoint: u8, data: &mut [u8]) -> Result<usize, &'static str> {
        // Stub implementation
        Ok(0)
    }
    
    fn get_controller_type(&self) -> ControllerType {
        ControllerType::Ehci
    }
}

pub fn detect_ehci_controller() -> Option<EhciController> {
    // Would search PCI for EHCI controllers
    // Class 0x0C, Subclass 0x03, Prog IF 0x20
    None // Stub for now
}