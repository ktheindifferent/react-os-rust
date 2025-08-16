// ACPI (Advanced Configuration and Power Interface) Implementation
pub mod tables;
pub mod power;
pub mod apic;
pub mod pci;

use crate::{println, serial_println};

use alloc::vec::Vec;
use alloc::string::String;
use core::mem;
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::PhysAddr;
use crate::memory::PHYS_MEM_OFFSET;

// ACPI Signatures
pub const RSDP_SIGNATURE: &[u8; 8] = b"RSD PTR ";
pub const RSDT_SIGNATURE: &[u8; 4] = b"RSDT";
pub const XSDT_SIGNATURE: &[u8; 4] = b"XSDT";
pub const FADT_SIGNATURE: &[u8; 4] = b"FACP";
pub const MADT_SIGNATURE: &[u8; 4] = b"APIC";
pub const HPET_SIGNATURE: &[u8; 4] = b"HPET";
pub const MCFG_SIGNATURE: &[u8; 4] = b"MCFG";
pub const DSDT_SIGNATURE: &[u8; 4] = b"DSDT";
pub const SSDT_SIGNATURE: &[u8; 4] = b"SSDT";

// RSDP (Root System Description Pointer) Structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Rsdp {
    pub signature: [u8; 8],
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub revision: u8,
    pub rsdt_address: u32,
}

// Extended RSDP for ACPI 2.0+
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct RsdpExtended {
    pub rsdp: Rsdp,
    pub length: u32,
    pub xsdt_address: u64,
    pub extended_checksum: u8,
    pub reserved: [u8; 3],
}

// System Description Table Header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct SdtHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

// ACPI Manager
pub struct AcpiManager {
    rsdp: Option<u64>,  // Store as address instead of raw pointer
    rsdt: Option<u64>,  // Store as address instead of raw pointer
    xsdt: Option<u64>,  // Store as address instead of raw pointer
    tables: Vec<AcpiTable>,
    power_state: PowerState,
}

#[derive(Debug, Clone)]
pub struct AcpiTable {
    pub signature: [u8; 4],
    pub address: u64,
    pub length: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerState {
    S0Working,
    S1PowerOnSuspend,
    S2CpuOff,
    S3SuspendToRam,
    S4SuspendToDisk,
    S5SoftOff,
}

impl AcpiManager {
    pub fn new() -> Self {
        Self {
            rsdp: None,
            rsdt: None,
            xsdt: None,
            tables: Vec::new(),
            power_state: PowerState::S0Working,
        }
    }
    
    pub fn init(&mut self) -> Result<(), &'static str> {
        // Find RSDP
        self.find_rsdp()?;
        
        // Parse system tables
        self.parse_tables()?;
        
        // Initialize subsystems
        self.init_power_management()?;
        self.init_apic()?;
        
        serial_println!("ACPI: Initialized successfully");
        Ok(())
    }
    
    fn find_rsdp(&mut self) -> Result<(), &'static str> {
        // Search for RSDP in BIOS areas
        // First search in EBDA (Extended BIOS Data Area)
        if let Some(rsdp) = self.search_rsdp(0x00080000, 0x000A0000) {
            self.rsdp = Some(rsdp as u64);
            return Ok(());
        }
        
        // Then search in BIOS ROM area
        if let Some(rsdp) = self.search_rsdp(0x000E0000, 0x00100000) {
            self.rsdp = Some(rsdp as u64);
            return Ok(());
        }
        
        Err("RSDP not found")
    }
    
    fn search_rsdp(&self, start: u64, end: u64) -> Option<*const Rsdp> {
        let mut addr = start;
        
        while addr < end {
            let ptr = (PHYS_MEM_OFFSET + addr) as *const Rsdp;
            
            unsafe {
                // Check signature
                if (*ptr).signature == *RSDP_SIGNATURE {
                    // Verify checksum
                    if self.verify_checksum(ptr as *const u8, mem::size_of::<Rsdp>()) {
                        return Some(ptr);
                    }
                }
            }
            
            // RSDP is 16-byte aligned
            addr += 16;
        }
        
        None
    }
    
    fn verify_checksum(&self, ptr: *const u8, len: usize) -> bool {
        let mut sum = 0u8;
        
        for i in 0..len {
            unsafe {
                sum = sum.wrapping_add(*ptr.add(i));
            }
        }
        
        sum == 0
    }
    
    fn parse_tables(&mut self) -> Result<(), &'static str> {
        let rsdp_addr = self.rsdp.ok_or("RSDP not initialized")?;
        let rsdp = (PHYS_MEM_OFFSET + rsdp_addr) as *const Rsdp;
        
        unsafe {
            let revision = (*rsdp).revision;
            
            if revision >= 2 {
                // ACPI 2.0+ - use XSDT
                let extended = rsdp as *const RsdpExtended;
                let xsdt_addr = (*extended).xsdt_address;
                
                if xsdt_addr != 0 {
                    self.xsdt = Some(xsdt_addr);
                    self.parse_xsdt()?;
                }
            } else {
                // ACPI 1.0 - use RSDT
                let rsdt_addr = (*rsdp).rsdt_address as u64;
                
                if rsdt_addr != 0 {
                    self.rsdt = Some(rsdt_addr);
                    self.parse_rsdt()?;
                }
            }
        }
        
        Ok(())
    }
    
    fn parse_rsdt(&mut self) -> Result<(), &'static str> {
        let rsdt_addr = self.rsdt.ok_or("RSDT not initialized")?;
        let rsdt = (PHYS_MEM_OFFSET + rsdt_addr) as *const SdtHeader;
        
        unsafe {
            // Verify RSDT signature
            if (*rsdt).signature != *RSDT_SIGNATURE {
                return Err("Invalid RSDT signature");
            }
            
            // Calculate number of entries
            let entry_count = ((*rsdt).length as usize - mem::size_of::<SdtHeader>()) / 4;
            let entries = (rsdt as *const u8).add(mem::size_of::<SdtHeader>()) as *const u32;
            
            // Parse each table
            for i in 0..entry_count {
                let table_addr = *entries.add(i) as u64;
                let table_ptr = (PHYS_MEM_OFFSET + table_addr) as *const SdtHeader;
                
                self.tables.push(AcpiTable {
                    signature: (*table_ptr).signature,
                    address: table_addr,
                    length: (*table_ptr).length,
                });
                
                // Process specific tables
                self.process_table(table_ptr)?;
            }
        }
        
        Ok(())
    }
    
    fn parse_xsdt(&mut self) -> Result<(), &'static str> {
        let xsdt_addr = self.xsdt.ok_or("XSDT not initialized")?;
        let xsdt = (PHYS_MEM_OFFSET + xsdt_addr) as *const SdtHeader;
        
        unsafe {
            // Verify XSDT signature
            if (*xsdt).signature != *XSDT_SIGNATURE {
                return Err("Invalid XSDT signature");
            }
            
            // Calculate number of entries
            let entry_count = ((*xsdt).length as usize - mem::size_of::<SdtHeader>()) / 8;
            let entries = (xsdt as *const u8).add(mem::size_of::<SdtHeader>()) as *const u64;
            
            // Parse each table
            for i in 0..entry_count {
                let table_addr = *entries.add(i);
                let table_ptr = (PHYS_MEM_OFFSET + table_addr) as *const SdtHeader;
                
                self.tables.push(AcpiTable {
                    signature: (*table_ptr).signature,
                    address: table_addr,
                    length: (*table_ptr).length,
                });
                
                // Process specific tables
                self.process_table(table_ptr)?;
            }
        }
        
        Ok(())
    }
    
    fn process_table(&mut self, table: *const SdtHeader) -> Result<(), &'static str> {
        unsafe {
            match &(*table).signature {
                sig if sig == FADT_SIGNATURE => {
                    self.process_fadt(table)?;
                }
                sig if sig == MADT_SIGNATURE => {
                    self.process_madt(table)?;
                }
                sig if sig == HPET_SIGNATURE => {
                    self.process_hpet(table)?;
                }
                sig if sig == MCFG_SIGNATURE => {
                    self.process_mcfg(table)?;
                }
                _ => {
                    // Unknown or unhandled table
                }
            }
        }
        
        Ok(())
    }
    
    fn process_fadt(&mut self, table: *const SdtHeader) -> Result<(), &'static str> {
        // Process Fixed ACPI Description Table
        let fadt = table as *const tables::Fadt;
        
        unsafe {
            // Store power management addresses
            power::init_fadt(fadt)?;
            
            // Load DSDT
            let dsdt_addr = (*fadt).dsdt;
            if dsdt_addr != 0 {
                let dsdt = (PHYS_MEM_OFFSET + dsdt_addr as u64) as *const SdtHeader;
                if (*dsdt).signature == *DSDT_SIGNATURE {
                    // Parse DSDT for device configuration
                    serial_println!("ACPI: Found DSDT at 0x{:x}", dsdt_addr);
                }
            }
        }
        
        Ok(())
    }
    
    fn process_madt(&mut self, table: *const SdtHeader) -> Result<(), &'static str> {
        // Process Multiple APIC Description Table
        apic::parse_madt(table)?;
        Ok(())
    }
    
    fn process_hpet(&mut self, table: *const SdtHeader) -> Result<(), &'static str> {
        // Process High Precision Event Timer
        serial_println!("ACPI: Found HPET table");
        Ok(())
    }
    
    fn process_mcfg(&mut self, table: *const SdtHeader) -> Result<(), &'static str> {
        // Process PCI Express configuration
        pci::parse_mcfg(table)?;
        Ok(())
    }
    
    fn init_power_management(&mut self) -> Result<(), &'static str> {
        power::init()?;
        Ok(())
    }
    
    fn init_apic(&mut self) -> Result<(), &'static str> {
        apic::init()?;
        Ok(())
    }
    
    pub fn set_power_state(&mut self, state: PowerState) -> Result<(), &'static str> {
        match state {
            PowerState::S0Working => {
                // Already in working state
                Ok(())
            }
            PowerState::S3SuspendToRam => {
                power::suspend_to_ram()?;
                self.power_state = state;
                Ok(())
            }
            PowerState::S5SoftOff => {
                power::shutdown()?;
                self.power_state = state;
                Ok(())
            }
            _ => Err("Power state not supported")
        }
    }
    
    pub fn get_power_state(&self) -> PowerState {
        self.power_state
    }
    
    pub fn find_table(&self, signature: &[u8; 4]) -> Option<&AcpiTable> {
        self.tables.iter().find(|t| &t.signature == signature)
    }
}

lazy_static! {
    pub static ref ACPI: Mutex<AcpiManager> = Mutex::new(AcpiManager::new());
}

pub fn init() {
    ACPI.lock().init().expect("Failed to initialize ACPI");
}