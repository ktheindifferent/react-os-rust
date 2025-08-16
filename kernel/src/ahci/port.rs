// AHCI Port Management

use super::{HbaPort, DeviceType};
use crate::serial_println;

// Port Signature values
pub const SIG_ATA: u32 = 0x00000101;
pub const SIG_ATAPI: u32 = 0xEB140101;
pub const SIG_SEMB: u32 = 0xC33C0101;
pub const SIG_PM: u32 = 0x96690101;

// Port Command and Status Field Definitions
pub const PORT_CMD_ICC_MASK: u32 = 0xF << 28;
pub const PORT_CMD_ICC_ACTIVE: u32 = 1 << 28;
pub const PORT_CMD_ICC_PARTIAL: u32 = 2 << 28;
pub const PORT_CMD_ICC_SLUMBER: u32 = 6 << 28;

// SATA Device Detection
pub const HBA_PORT_DET_PRESENT: u8 = 3;
pub const HBA_PORT_IPM_ACTIVE: u8 = 1;

pub fn get_device_type(port: &HbaPort) -> DeviceType {
    let ssts = port.ssts;
    
    let det = (ssts & 0x0F) as u8;
    let ipm = ((ssts >> 8) & 0x0F) as u8;
    
    if det != HBA_PORT_DET_PRESENT {
        return DeviceType::None;
    }
    
    if ipm != HBA_PORT_IPM_ACTIVE {
        return DeviceType::None;
    }
    
    match port.sig {
        SIG_ATAPI => DeviceType::Satapi,
        SIG_SEMB => DeviceType::Semb,
        SIG_PM => DeviceType::Pm,
        SIG_ATA => DeviceType::Sata,
        _ => DeviceType::None,
    }
}

pub fn port_rebase(port: &mut HbaPort, port_no: u32, clb: u64, fb: u64) -> Result<(), &'static str> {
    serial_println!("AHCI: Rebasing port {}", port_no);
    
    // Stop command engine
    stop_cmd(port)?;
    
    // Set command list base address
    port.clb = clb as u32;
    port.clbu = (clb >> 32) as u32;
    
    // Set FIS receive area base address
    port.fb = fb as u32;
    port.fbu = (fb >> 32) as u32;
    
    // Clear the memory areas
    unsafe {
        use crate::memory::PHYS_MEM_OFFSET;
        // Clear command list (1K)
        core::ptr::write_bytes((PHYS_MEM_OFFSET + clb) as *mut u8, 0, 1024);
        // Clear FIS receive area (256 bytes)
        core::ptr::write_bytes((PHYS_MEM_OFFSET + fb) as *mut u8, 0, 256);
    }
    
    // Start command engine
    start_cmd(port)?;
    
    Ok(())
}

pub fn stop_cmd(port: &mut HbaPort) -> Result<(), &'static str> {
    // Clear ST bit
    port.cmd &= !(1 << 0);
    
    // Clear FRE bit
    port.cmd &= !(1 << 4);
    
    // Wait until FR and CR bits are cleared
    for _ in 0..1000 {
        if (port.cmd & ((1 << 14) | (1 << 15))) == 0 {
            return Ok(());
        }
        for _ in 0..1000 {
            core::hint::spin_loop();
        }
    }
    
    Err("Failed to stop command engine")
}

pub fn start_cmd(port: &mut HbaPort) -> Result<(), &'static str> {
    // Wait until CR is cleared
    while port.cmd & (1 << 15) != 0 {
        core::hint::spin_loop();
    }
    
    // Set FRE bit
    port.cmd |= 1 << 4;
    
    // Set ST bit
    port.cmd |= 1 << 0;
    
    Ok(())
}