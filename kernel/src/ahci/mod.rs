// AHCI (Advanced Host Controller Interface) Implementation
pub mod hba;
pub mod port;
pub mod fis;
pub mod command;

use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;
use core::mem;
use crate::{println, serial_println};
use crate::memory::PHYS_MEM_OFFSET;
use crate::drivers::disk::{DiskDriver, DiskError, DiskInfo};

// AHCI Constants
pub const AHCI_SIG: u32 = 0x00000101;  // SATA drive signature
pub const AHCI_SIG_ATAPI: u32 = 0xEB140101;  // SATAPI drive
pub const AHCI_SIG_SEMB: u32 = 0xC33C0101;   // Enclosure management bridge
pub const AHCI_SIG_PM: u32 = 0x96690101;     // Port multiplier

// AHCI Capability Bits
pub const HBA_CAP_S64A: u32 = 1 << 31;  // 64-bit addressing
pub const HBA_CAP_SNCQ: u32 = 1 << 30;  // Native Command Queuing
pub const HBA_CAP_SSNTF: u32 = 1 << 29; // SNotification register
pub const HBA_CAP_SMPS: u32 = 1 << 28;  // Mechanical presence switch
pub const HBA_CAP_SSS: u32 = 1 << 27;   // Staggered spin-up
pub const HBA_CAP_SALP: u32 = 1 << 26;  // Aggressive link power management
pub const HBA_CAP_SAL: u32 = 1 << 25;   // Activity LED
pub const HBA_CAP_SCLO: u32 = 1 << 24;  // Command list override

// HBA Memory Registers (Generic Host Control)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HbaMemory {
    pub cap: u32,         // 0x00, Host capability
    pub ghc: u32,         // 0x04, Global host control
    pub is: u32,          // 0x08, Interrupt status
    pub pi: u32,          // 0x0C, Port implemented
    pub vs: u32,          // 0x10, Version
    pub ccc_ctl: u32,     // 0x14, Command completion coalescing control
    pub ccc_pts: u32,     // 0x18, Command completion coalescing ports
    pub em_loc: u32,      // 0x1C, Enclosure management location
    pub em_ctl: u32,      // 0x20, Enclosure management control
    pub cap2: u32,        // 0x24, Host capabilities extended
    pub bohc: u32,        // 0x28, BIOS/OS handoff control
    
    // Reserved
    _reserved: [u8; 0xA0 - 0x2C],
    
    // Vendor specific registers
    vendor: [u8; 0x100 - 0xA0],
    
    // Port control registers (up to 32 ports)
    pub ports: [HbaPort; 32],
}

// HBA Port Registers
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HbaPort {
    pub clb: u32,         // 0x00, Command list base address, lower 32 bits
    pub clbu: u32,        // 0x04, Command list base address, upper 32 bits
    pub fb: u32,          // 0x08, FIS base address, lower 32 bits
    pub fbu: u32,         // 0x0C, FIS base address, upper 32 bits
    pub is: u32,          // 0x10, Interrupt status
    pub ie: u32,          // 0x14, Interrupt enable
    pub cmd: u32,         // 0x18, Command and status
    _reserved0: u32,      // 0x1C, Reserved
    pub tfd: u32,         // 0x20, Task file data
    pub sig: u32,         // 0x24, Signature
    pub ssts: u32,        // 0x28, SATA status (SCR0:SStatus)
    pub sctl: u32,        // 0x2C, SATA control (SCR2:SControl)
    pub serr: u32,        // 0x30, SATA error (SCR1:SError)
    pub sact: u32,        // 0x34, SATA active (SCR3:SActive)
    pub ci: u32,          // 0x38, Command issue
    pub sntf: u32,        // 0x3C, SATA notification (SCR4:SNotification)
    pub fbs: u32,         // 0x40, FIS-based switch control
    pub devslp: u32,      // 0x44, Device sleep
    _reserved1: [u8; 0x70 - 0x48],
    pub vendor: [u32; 4], // 0x70-0x7F, vendor specific
}

// Port Command Bits
pub const HBA_PxCMD_ST: u32 = 1 << 0;   // Start
pub const HBA_PxCMD_SUD: u32 = 1 << 1;  // Spin-Up Device
pub const HBA_PxCMD_POD: u32 = 1 << 2;  // Power On Device
pub const HBA_PxCMD_CLO: u32 = 1 << 3;  // Command List Override
pub const HBA_PxCMD_FRE: u32 = 1 << 4;  // FIS Receive Enable
pub const HBA_PxCMD_CCS_MASK: u32 = 0x1F << 8;  // Current Command Slot
pub const HBA_PxCMD_MPSS: u32 = 1 << 13; // Mechanical Presence Switch State
pub const HBA_PxCMD_FR: u32 = 1 << 14;   // FIS Receive Running
pub const HBA_PxCMD_CR: u32 = 1 << 15;   // Command List Running
pub const HBA_PxCMD_CPS: u32 = 1 << 16;  // Cold Presence State
pub const HBA_PxCMD_PMA: u32 = 1 << 17;  // Port Multiplier Attached
pub const HBA_PxCMD_HPCP: u32 = 1 << 18; // Hot Plug Capable Port
pub const HBA_PxCMD_MPSP: u32 = 1 << 19; // Mechanical Presence Switch Port
pub const HBA_PxCMD_CPD: u32 = 1 << 20;  // Cold Presence Detection
pub const HBA_PxCMD_ESP: u32 = 1 << 21;  // External SATA Port
pub const HBA_PxCMD_FBSCP: u32 = 1 << 22; // FIS-based Switching Capable Port
pub const HBA_PxCMD_APSTE: u32 = 1 << 23; // Automatic Partial to Slumber Transitions Enabled
pub const HBA_PxCMD_ATAPI: u32 = 1 << 24; // Device is ATAPI
pub const HBA_PxCMD_DLAE: u32 = 1 << 25; // Drive LED on ATAPI Enable
pub const HBA_PxCMD_ALPE: u32 = 1 << 26; // Aggressive Link Power Management Enable
pub const HBA_PxCMD_ASP: u32 = 1 << 27;  // Aggressive Slumber / Partial
pub const HBA_PxCMD_ICC_MASK: u32 = 0xF << 28; // Interface Communication Control

// Port Interrupt Status Bits
pub const HBA_PxIS_DHRS: u32 = 1 << 0;  // Device to Host Register FIS
pub const HBA_PxIS_PSS: u32 = 1 << 1;   // PIO Setup FIS
pub const HBA_PxIS_DSS: u32 = 1 << 2;   // DMA Setup FIS
pub const HBA_PxIS_SDBS: u32 = 1 << 3;  // Set Device Bits FIS
pub const HBA_PxIS_UFS: u32 = 1 << 4;   // Unknown FIS
pub const HBA_PxIS_DPS: u32 = 1 << 5;   // Descriptor Processed
pub const HBA_PxIS_PCS: u32 = 1 << 6;   // Port Connect Change Status
pub const HBA_PxIS_DMPS: u32 = 1 << 7;  // Device Mechanical Presence Status
pub const HBA_PxIS_PRCS: u32 = 1 << 22; // PhyRdy Change Status
pub const HBA_PxIS_IPMS: u32 = 1 << 23; // Incorrect Port Multiplier Status
pub const HBA_PxIS_OFS: u32 = 1 << 24;  // Overflow Status
pub const HBA_PxIS_INFS: u32 = 1 << 26; // Interface Non-fatal Error Status
pub const HBA_PxIS_IFS: u32 = 1 << 27;  // Interface Fatal Error Status
pub const HBA_PxIS_HBDS: u32 = 1 << 28; // Host Bus Data Error Status
pub const HBA_PxIS_HBFS: u32 = 1 << 29; // Host Bus Fatal Error Status
pub const HBA_PxIS_TFES: u32 = 1 << 30; // Task File Error Status
pub const HBA_PxIS_CPDS: u32 = 1 << 31; // Cold Port Detect Status

// Device Detection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceType {
    None,
    Sata,
    Satapi,
    Semb,
    Pm,
}

// AHCI Controller
pub struct AhciController {
    pub base_addr: u64,
    pub hba: u64,  // Store as address instead of raw pointer
    pub ports: Vec<AhciPort>,
}

// AHCI Port
pub struct AhciPort {
    pub number: u8,
    pub hba_port: u64,  // Store as address instead of raw pointer
    pub device_type: DeviceType,
    pub clb: u64,  // Command List Base
    pub fb: u64,   // FIS Base
    pub ctba: Vec<u64>, // Command Table Base Addresses
    pub sector_count: u64,
    pub sector_size: u32,
}

impl AhciController {
    pub unsafe fn new(base_addr: u64) -> Result<Self, &'static str> {
        let hba_ptr = (PHYS_MEM_OFFSET + base_addr) as *mut HbaMemory;
        let hba = base_addr;
        
        // Verify AHCI version
        let version = (*hba_ptr).vs;
        let major = (version >> 16) & 0xFFFF;
        let minor = version & 0xFFFF;
        serial_println!("AHCI: Version {}.{}", major, minor);
        
        // Check capabilities
        let cap = (*hba_ptr).cap;
        let num_ports = ((cap & 0x1F) + 1) as usize;
        let num_cmd_slots = ((cap >> 8) & 0x1F) + 1;
        
        serial_println!("AHCI: {} ports, {} command slots", num_ports, num_cmd_slots);
        
        if cap & HBA_CAP_S64A != 0 {
            serial_println!("AHCI: 64-bit addressing supported");
        }
        
        if cap & HBA_CAP_SNCQ != 0 {
            serial_println!("AHCI: Native Command Queuing supported");
        }
        
        Ok(Self {
            base_addr,
            hba,
            ports: Vec::new(),
        })
    }
    
    pub fn init(&mut self) -> Result<(), &'static str> {
        unsafe {
            let hba = (PHYS_MEM_OFFSET + self.hba) as *mut HbaMemory;
            
            // Take ownership of controller from BIOS
            self.bios_handoff()?;
            
            // Enable AHCI mode
            (*hba).ghc |= 1 << 31; // AHCI Enable
            
            // Reset controller
            (*hba).ghc |= 1 << 0; // HBA Reset
            
            // Wait for reset to complete
            while (*hba).ghc & 1 != 0 {
                core::hint::spin_loop();
            }
            
            // Re-enable AHCI mode
            (*hba).ghc |= 1 << 31;
            
            // Enable interrupts
            (*hba).ghc |= 1 << 1;
            
            // Probe ports
            self.probe_ports()?;
        }
        
        Ok(())
    }
    
    unsafe fn bios_handoff(&mut self) -> Result<(), &'static str> {
        let hba = (PHYS_MEM_OFFSET + self.hba) as *mut HbaMemory;
        let cap2 = (*hba).cap2;
        
        // Check if BIOS/OS handoff is supported
        if cap2 & (1 << 0) == 0 {
            return Ok(()); // Not supported, assume we have control
        }
        
        // Request ownership
        (*hba).bohc |= 1 << 1; // OS Ownership Change
        
        // Wait for BIOS to release control
        for _ in 0..1000 {
            if (*hba).bohc & (1 << 0) == 0 {
                // BIOS Busy cleared
                break;
            }
            for _ in 0..1000 {
                core::hint::spin_loop();
            }
        }
        
        if (*hba).bohc & (1 << 4) != 0 {
            // OS Ownership Status set
            serial_println!("AHCI: BIOS handoff complete");
        }
        
        Ok(())
    }
    
    unsafe fn probe_ports(&mut self) -> Result<(), &'static str> {
        let hba = (PHYS_MEM_OFFSET + self.hba) as *mut HbaMemory;
        let pi = (*hba).pi;
        
        for i in 0..32 {
            if pi & (1 << i) != 0 {
                let port = &mut (*hba).ports[i];
                let device_type = self.check_device_type(port);
                
                if device_type != DeviceType::None {
                    serial_println!("AHCI: Port {} - {:?} device detected", i, device_type);
                    
                    // Initialize port
                    let mut ahci_port = AhciPort::new(i as u8, port as *mut _ as u64, device_type);
                    ahci_port.init()?;
                    
                    // Identify device
                    if device_type == DeviceType::Sata {
                        ahci_port.identify()?;
                    }
                    
                    self.ports.push(ahci_port);
                }
            }
        }
        
        Ok(())
    }
    
    unsafe fn check_device_type(&self, port: *mut HbaPort) -> DeviceType {
        let ssts = (*port).ssts;
        
        // Check device detection
        let det = ssts & 0x0F;
        let ipm = (ssts >> 8) & 0x0F;
        
        if det != 3 {
            return DeviceType::None; // No device detected or communication not established
        }
        
        if ipm != 1 {
            return DeviceType::None; // Not in active state
        }
        
        // Check signature
        match (*port).sig {
            AHCI_SIG => DeviceType::Sata,
            AHCI_SIG_ATAPI => DeviceType::Satapi,
            AHCI_SIG_SEMB => DeviceType::Semb,
            AHCI_SIG_PM => DeviceType::Pm,
            _ => DeviceType::None,
        }
    }
}

impl AhciPort {
    pub fn new(number: u8, hba_port: u64, device_type: DeviceType) -> Self {
        Self {
            number,
            hba_port,
            device_type,
            clb: 0,
            fb: 0,
            ctba: Vec::new(),
            sector_count: 0,
            sector_size: 512, // Default
        }
    }
    
    pub unsafe fn init(&mut self) -> Result<(), &'static str> {
        let hba_port = (PHYS_MEM_OFFSET + self.hba_port) as *mut HbaPort;
        
        // Stop command engine
        self.stop_cmd()?;
        
        // Allocate command list (1K, 32 entries * 32 bytes)
        let clb = allocate_aligned(1024, 1024);
        (*hba_port).clb = clb as u32;
        (*hba_port).clbu = (clb >> 32) as u32;
        self.clb = clb;
        
        // Clear command list
        core::ptr::write_bytes((PHYS_MEM_OFFSET + clb) as *mut u8, 0, 1024);
        
        // Allocate FIS receive area (256 bytes)
        let fb = allocate_aligned(256, 256);
        (*hba_port).fb = fb as u32;
        (*hba_port).fbu = (fb >> 32) as u32;
        self.fb = fb;
        
        // Clear FIS receive area
        core::ptr::write_bytes((PHYS_MEM_OFFSET + fb) as *mut u8, 0, 256);
        
        // Allocate command tables (8K each, 32 slots)
        for i in 0..32 {
            let ctba = allocate_aligned(8192, 128);
            self.ctba.push(ctba);
            
            // Set command table base address in command list
            let cmd_header = (PHYS_MEM_OFFSET + clb + (i * 32) as u64) as *mut HbaCmdHeader;
            (*cmd_header).ctba = ctba as u32;
            (*cmd_header).ctbau = (ctba >> 32) as u32;
            (*cmd_header).prdtl = 8; // 8 PRDT entries
        }
        
        // Start command engine
        self.start_cmd()?;
        
        // Clear error status
        (*hba_port).serr = 0xFFFFFFFF;
        
        // Enable interrupts
        (*hba_port).ie = 0xFFFFFFFF;
        
        Ok(())
    }
    
    pub unsafe fn stop_cmd(&mut self) -> Result<(), &'static str> {
        let hba_port = (PHYS_MEM_OFFSET + self.hba_port) as *mut HbaPort;
        
        // Clear ST (Start)
        (*hba_port).cmd &= !HBA_PxCMD_ST;
        
        // Clear FRE (FIS Receive Enable)
        (*hba_port).cmd &= !HBA_PxCMD_FRE;
        
        // Wait for FR (FIS Receive Running) and CR (Command List Running) to clear
        for _ in 0..1000 {
            if (*hba_port).cmd & (HBA_PxCMD_FR | HBA_PxCMD_CR) == 0 {
                return Ok(());
            }
            for _ in 0..1000 {
                core::hint::spin_loop();
            }
        }
        
        Err("Failed to stop command engine")
    }
    
    pub unsafe fn start_cmd(&mut self) -> Result<(), &'static str> {
        let hba_port = (PHYS_MEM_OFFSET + self.hba_port) as *mut HbaPort;
        
        // Wait for CR to clear
        while (*hba_port).cmd & HBA_PxCMD_CR != 0 {
            core::hint::spin_loop();
        }
        
        // Set FRE (FIS Receive Enable)
        (*hba_port).cmd |= HBA_PxCMD_FRE;
        
        // Set ST (Start)
        (*hba_port).cmd |= HBA_PxCMD_ST;
        
        Ok(())
    }
    
    pub fn identify(&mut self) -> Result<(), &'static str> {
        let mut id_data = [0u16; 256];
        
        unsafe {
            // Send IDENTIFY DEVICE command
            let cmd = if self.device_type == DeviceType::Satapi {
                0xA1 // IDENTIFY PACKET DEVICE
            } else {
                0xEC // IDENTIFY DEVICE
            };
            
            self.send_command(cmd, 0, 0, &mut id_data)?;
            
            // Parse identification data
            // Word 60-61: Total number of user addressable sectors (LBA28)
            let sectors_28 = ((id_data[61] as u64) << 16) | (id_data[60] as u64);
            
            // Word 100-103: Total number of user addressable sectors (LBA48)
            let sectors_48 = ((id_data[103] as u64) << 48) |
                           ((id_data[102] as u64) << 32) |
                           ((id_data[101] as u64) << 16) |
                           (id_data[100] as u64);
            
            self.sector_count = if sectors_48 > 0 { sectors_48 } else { sectors_28 };
            
            // Word 106: Physical/logical sector size
            if id_data[106] & (1 << 12) != 0 {
                // Logical sector size is greater than 512 bytes
                let log_per_phys = 1 << (id_data[106] & 0x0F);
                self.sector_size = 512 * log_per_phys;
            }
            
            // Extract model string (words 27-46)
            let mut model = String::new();
            for i in 27..=46 {
                let word = id_data[i];
                model.push((word >> 8) as u8 as char);
                model.push((word & 0xFF) as u8 as char);
            }
            
            serial_println!("AHCI Port {}: {} sectors ({} MB), {} bytes/sector",
                          self.number,
                          self.sector_count,
                          (self.sector_count * self.sector_size as u64) / (1024 * 1024),
                          self.sector_size);
            serial_println!("AHCI Port {}: Model: {}", self.number, model.trim());
        }
        
        Ok(())
    }
    
    unsafe fn send_command<T>(&mut self, cmd: u8, lba: u64, count: u16, buffer: &mut [T]) -> Result<(), &'static str> {
        // Find free command slot
        let slot = self.find_free_slot()?;
        
        // Setup command header
        let cmd_header = (PHYS_MEM_OFFSET + self.clb + (slot * 32) as u64) as *mut HbaCmdHeader;
        (*cmd_header).cfl = 5; // Command FIS size: 5 DWORDs
        (*cmd_header).w = 0;   // Read from device
        (*cmd_header).prdtl = 1; // 1 PRDT entry
        
        // Setup command table
        let cmd_table = (PHYS_MEM_OFFSET + self.ctba[slot as usize]) as *mut HbaCmdTable;
        core::ptr::write_bytes(cmd_table, 0, 8192);
        
        // Setup PRDT (Physical Region Descriptor Table)
        let prdt = &mut (*cmd_table).prdt_entry[0];
        prdt.dba = buffer.as_ptr() as u32;
        prdt.dbau = (buffer.as_ptr() as u64 >> 32) as u32;
        prdt.dbc = (mem::size_of_val(buffer) - 1) as u32; // Byte count - 1
        prdt.i = 0; // No interrupt on completion
        
        // Setup command FIS
        let fis = &mut (*cmd_table).cfis;
        let h2d = fis::FisRegH2D {
            fis_type: fis::FIS_TYPE_REG_H2D,
            pmport_c: 0x80, // Command
            command: cmd,
            featurel: 0,
            lba0: (lba & 0xFF) as u8,
            lba1: ((lba >> 8) & 0xFF) as u8,
            lba2: ((lba >> 16) & 0xFF) as u8,
            device: 0x40, // LBA mode
            lba3: ((lba >> 24) & 0xFF) as u8,
            lba4: ((lba >> 32) & 0xFF) as u8,
            lba5: ((lba >> 40) & 0xFF) as u8,
            featureh: 0,
            countl: (count & 0xFF) as u8,
            counth: ((count >> 8) & 0xFF) as u8,
            icc: 0,
            control: 0,
            rsv1: [0; 4],
        };
        
        core::ptr::copy_nonoverlapping(
            &h2d as *const _ as *const u8,
            fis.as_mut_ptr(),
            mem::size_of::<fis::FisRegH2D>()
        );
        
        let hba_port = (PHYS_MEM_OFFSET + self.hba_port) as *mut HbaPort;
        
        // Issue command
        (*hba_port).ci = 1 << slot;
        
        // Wait for completion
        loop {
            if (*hba_port).ci & (1 << slot) == 0 {
                break;
            }
            if (*hba_port).is & HBA_PxIS_TFES != 0 {
                return Err("Task file error");
            }
        }
        
        Ok(())
    }
    
    unsafe fn find_free_slot(&self) -> Result<u32, &'static str> {
        let hba_port = (PHYS_MEM_OFFSET + self.hba_port) as *mut HbaPort;
        let slots = (*hba_port).sact | (*hba_port).ci;
        for i in 0..32 {
            if slots & (1 << i) == 0 {
                return Ok(i);
            }
        }
        Err("No free command slots")
    }
}

// Command List Structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HbaCmdHeader {
    pub cfl: u8,      // Command FIS length in DWORDS, 2 ~ 16
    pub a: u8,        // ATAPI
    pub w: u8,        // Write, 1: H2D, 0: D2H
    pub p: u8,        // Prefetchable
    pub r: u8,        // Reset
    pub b: u8,        // BIST
    pub c: u8,        // Clear busy upon R_OK
    pub rsv0: u8,     // Reserved
    pub pmp: u8,      // Port multiplier port
    pub prdtl: u16,   // Physical region descriptor table length in entries
    pub prdbc: u32,   // Physical region descriptor byte count transferred
    pub ctba: u32,    // Command table descriptor base address
    pub ctbau: u32,   // Command table descriptor base address upper 32 bits
    pub rsv1: [u32; 4],
}

// Physical Region Descriptor Table Entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HbaPrdtEntry {
    pub dba: u32,     // Data base address
    pub dbau: u32,    // Data base address upper 32 bits
    pub rsv0: u32,    // Reserved
    pub dbc: u32,     // Byte count, bit 31 indicates interrupt on completion
    pub i: u32,       // Interrupt on completion
}

// Command Table
#[repr(C)]
pub struct HbaCmdTable {
    pub cfis: [u8; 64],    // Command FIS
    pub acmd: [u8; 16],    // ATAPI command, 12 or 16 bytes
    pub rsv: [u8; 48],     // Reserved
    pub prdt_entry: [HbaPrdtEntry; 8], // Physical region descriptor table entries
}

// Helper function to allocate aligned memory
unsafe fn allocate_aligned(size: usize, align: usize) -> u64 {
    // This is a simplified allocation - in production, use proper memory allocation
    static mut NEXT_ADDR: u64 = 0x10000000; // Start at 256MB
    let addr = (NEXT_ADDR + (align as u64 - 1)) & !(align as u64 - 1);
    NEXT_ADDR = addr + size as u64;
    addr
}

// AHCI Disk Driver Implementation
pub struct AhciDisk {
    port: usize,
    info: DiskInfo,
}

impl AhciDisk {
    pub fn new(port_idx: usize) -> Result<Self, &'static str> {
        let ahci = AHCI_CONTROLLER.lock();
        
        if port_idx >= ahci.ports.len() {
            return Err("Invalid port index");
        }
        
        let port = &ahci.ports[port_idx];
        
        Ok(Self {
            port: port_idx,
            info: DiskInfo {
                name: String::from("AHCI SATA"),
                sectors: port.sector_count,
                sector_size: port.sector_size as usize,
                model: String::from("AHCI SATA Drive"),
                serial: String::from("N/A"),
            },
        })
    }
}

impl DiskDriver for AhciDisk {
    fn read_sectors(&mut self, start_sector: u64, count: u32, buffer: &mut [u8]) -> Result<(), DiskError> {
        let mut ahci = AHCI_CONTROLLER.lock();
        
        if self.port >= ahci.ports.len() {
            return Err(DiskError::InvalidSector);
        }
        
        let port = &mut ahci.ports[self.port];
        
        // Validate request
        if start_sector + count as u64 > port.sector_count {
            return Err(DiskError::InvalidSector);
        }
        
        if buffer.len() < (count as usize * port.sector_size as usize) {
            return Err(DiskError::BufferTooSmall);
        }
        
        unsafe {
            // Send READ DMA EXT command (0x25)
            port.send_command(0x25, start_sector, count as u16, buffer)
                .map_err(|_| DiskError::IoError)?;
        }
        
        Ok(())
    }
    
    fn write_sectors(&mut self, start_sector: u64, count: u32, data: &[u8]) -> Result<(), DiskError> {
        let mut ahci = AHCI_CONTROLLER.lock();
        
        if self.port >= ahci.ports.len() {
            return Err(DiskError::InvalidSector);
        }
        
        let port = &mut ahci.ports[self.port];
        
        // Validate request
        if start_sector + count as u64 > port.sector_count {
            return Err(DiskError::InvalidSector);
        }
        
        if data.len() < (count as usize * port.sector_size as usize) {
            return Err(DiskError::BufferTooSmall);
        }
        
        unsafe {
            // Create a mutable copy of the data for the command
            let mut data_copy = alloc::vec::Vec::from(data);
            
            // Prepare command header for write
            let slot = port.find_free_slot().map_err(|_| DiskError::IoError)?;
            let cmd_header = (PHYS_MEM_OFFSET + port.clb + (slot * 32) as u64) as *mut HbaCmdHeader;
            (*cmd_header).w = 1; // Write to device
            
            // Send WRITE DMA EXT command (0x35)
            port.send_command(0x35, start_sector, count as u16, &mut data_copy)
                .map_err(|_| DiskError::IoError)?;
        }
        
        Ok(())
    }
    
    fn get_info(&self) -> DiskInfo {
        self.info.clone()
    }
}

lazy_static! {
    pub static ref AHCI_CONTROLLER: Mutex<AhciController> = Mutex::new(unsafe {
        // This address would come from PCI enumeration
        // For now, use a common AHCI base address
        AhciController::new(0xFEB00000).unwrap()
    });
}

pub fn init() -> Result<(), &'static str> {
    serial_println!("AHCI: Initializing controller");
    AHCI_CONTROLLER.lock().init()?;
    
    let ahci = AHCI_CONTROLLER.lock();
    serial_println!("AHCI: Found {} SATA devices", ahci.ports.len());
    
    Ok(())
}