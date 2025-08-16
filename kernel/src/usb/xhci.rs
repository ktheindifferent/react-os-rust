// XHCI (eXtensible Host Controller Interface) - USB 3.0+
use super::{UsbController, UsbDevice, UsbSpeed, DeviceRequest, ControllerType};
use alloc::vec::Vec;
use crate::{println, serial_println};
use crate::memory::PHYS_MEM_OFFSET;

// XHCI Capability Registers
#[repr(C, packed)]
struct XhciCapRegs {
    caplength: u8,
    reserved: u8,
    hciversion: u16,
    hcsparams1: u32,
    hcsparams2: u32,
    hcsparams3: u32,
    hccparams1: u32,
    dboff: u32,      // Doorbell offset
    rtsoff: u32,     // Runtime registers offset
    hccparams2: u32,
}

// XHCI Operational Register Offsets
const XHCI_USBCMD: usize = 0x00;
const XHCI_USBSTS: usize = 0x04;
const XHCI_PAGESIZE: usize = 0x08;
const XHCI_DNCTRL: usize = 0x14;
const XHCI_CRCR: usize = 0x18;
const XHCI_DCBAAP: usize = 0x30;
const XHCI_CONFIG: usize = 0x38;

// Command Register Bits
const CMD_RUN: u32 = 1 << 0;
const CMD_RESET: u32 = 1 << 1;
const CMD_INTE: u32 = 1 << 2;
const CMD_HSEE: u32 = 1 << 3;

// Status Register Bits
const STS_HCH: u32 = 1 << 0;  // Host Controller Halted
const STS_HSE: u32 = 1 << 2;  // Host System Error
const STS_EINT: u32 = 1 << 3; // Event Interrupt
const STS_PCD: u32 = 1 << 4;  // Port Change Detect
const STS_CNR: u32 = 1 << 11; // Controller Not Ready

pub struct XhciController {
    base_addr: u64,
    op_regs: u64,
    runtime_regs: u64,
    doorbell_regs: u64,
    devices: Vec<UsbDevice>,
}

impl XhciController {
    pub fn new(base_addr: u64) -> Self {
        // Calculate register offsets
        let cap_regs = (PHYS_MEM_OFFSET + base_addr) as *const XhciCapRegs;
        let (caplength, dboff, rtsoff) = unsafe {
            ((*cap_regs).caplength, (*cap_regs).dboff, (*cap_regs).rtsoff)
        };
        
        Self {
            base_addr,
            op_regs: base_addr + caplength as u64,
            runtime_regs: base_addr + rtsoff as u64,
            doorbell_regs: base_addr + dboff as u64,
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
    
    fn read_op_reg64(&self, offset: usize) -> u64 {
        unsafe {
            let addr = (PHYS_MEM_OFFSET + self.op_regs + offset as u64) as *const u64;
            addr.read_volatile()
        }
    }
    
    fn write_op_reg64(&self, offset: usize, value: u64) {
        unsafe {
            let addr = (PHYS_MEM_OFFSET + self.op_regs + offset as u64) as *mut u64;
            addr.write_volatile(value);
        }
    }
}

impl UsbController for XhciController {
    fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("XHCI: Initializing controller at 0x{:x}", self.base_addr);
        
        // Wait for controller ready
        for _ in 0..100 {
            if self.read_op_reg(XHCI_USBSTS) & STS_CNR == 0 {
                break;
            }
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
        
        // Reset controller
        self.reset()?;
        
        // Set up data structures (stub)
        // - Device Context Base Address Array
        // - Command Ring
        // - Event Ring
        // - Scratchpad buffers
        
        // Enable interrupts
        let cmd = self.read_op_reg(XHCI_USBCMD);
        self.write_op_reg(XHCI_USBCMD, cmd | CMD_INTE);
        
        // Start controller
        self.write_op_reg(XHCI_USBCMD, cmd | CMD_RUN);
        
        // Wait for controller to start
        for _ in 0..100 {
            if (self.read_op_reg(XHCI_USBSTS) & STS_HCH) == 0 {
                serial_println!("XHCI: Controller started successfully");
                return Ok(());
            }
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
        
        Err("XHCI controller failed to start")
    }
    
    fn reset(&mut self) -> Result<(), &'static str> {
        // Stop controller
        let cmd = self.read_op_reg(XHCI_USBCMD);
        self.write_op_reg(XHCI_USBCMD, cmd & !CMD_RUN);
        
        // Wait for halt
        for _ in 0..100 {
            if self.read_op_reg(XHCI_USBSTS) & STS_HCH != 0 {
                break;
            }
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
        
        // Reset
        self.write_op_reg(XHCI_USBCMD, CMD_RESET);
        
        // Wait for reset to complete
        for _ in 0..100 {
            if self.read_op_reg(XHCI_USBCMD) & CMD_RESET == 0 {
                // Wait for CNR to clear
                for _ in 0..100 {
                    if self.read_op_reg(XHCI_USBSTS) & STS_CNR == 0 {
                        return Ok(());
                    }
                    for _ in 0..10000 {
                        core::hint::spin_loop();
                    }
                }
            }
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
        
        Err("XHCI reset timeout")
    }
    
    fn enumerate_devices(&mut self) -> Vec<UsbDevice> {
        // Stub implementation
        serial_println!("XHCI: Device enumeration (stub)");
        Vec::new()
    }
    
    fn control_transfer(&mut self, device: &UsbDevice, request: &DeviceRequest, data: Option<&mut [u8]>) -> Result<usize, &'static str> {
        // Stub implementation
        serial_println!("XHCI: Control transfer (stub)");
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
        ControllerType::Xhci
    }
}

pub fn detect_xhci_controller() -> Option<XhciController> {
    // Would search PCI for XHCI controllers
    // Class 0x0C, Subclass 0x03, Prog IF 0x30
    None // Stub for now
}