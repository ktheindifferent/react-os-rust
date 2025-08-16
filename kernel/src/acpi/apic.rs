// APIC (Advanced Programmable Interrupt Controller) Support
use super::tables::{Madt, MadtEntryHeader, MadtEntryType, MadtLocalApic, MadtIoApic, MadtInterruptSourceOverride};
use super::SdtHeader;
use alloc::vec::Vec;
use x86_64::PhysAddr;
use crate::memory::PHYS_MEM_OFFSET;
use core::mem;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::{println, serial_println};

// Local APIC Registers
const LAPIC_ID: u32 = 0x20;
const LAPIC_VERSION: u32 = 0x30;
const LAPIC_TPR: u32 = 0x80;      // Task Priority Register
const LAPIC_EOI: u32 = 0xB0;      // End of Interrupt
const LAPIC_SVR: u32 = 0xF0;      // Spurious Interrupt Vector
const LAPIC_ICR_LOW: u32 = 0x300; // Interrupt Command Register
const LAPIC_ICR_HIGH: u32 = 0x310;
const LAPIC_LVT_TIMER: u32 = 0x320;
const LAPIC_LVT_LINT0: u32 = 0x350;
const LAPIC_LVT_LINT1: u32 = 0x360;
const LAPIC_LVT_ERROR: u32 = 0x370;
const LAPIC_TIMER_INIT: u32 = 0x380;
const LAPIC_TIMER_CURRENT: u32 = 0x390;
const LAPIC_TIMER_DIV: u32 = 0x3E0;

// I/O APIC Registers
const IOAPIC_REG: u32 = 0x00;
const IOAPIC_DATA: u32 = 0x10;
const IOAPIC_ID: u8 = 0x00;
const IOAPIC_VERSION: u8 = 0x01;
const IOAPIC_REDTBL: u8 = 0x10;

// APIC Information
pub struct ApicInfo {
    pub local_apic_addr: u64,
    pub local_apics: Vec<LocalApicInfo>,
    pub io_apics: Vec<IoApicInfo>,
    pub interrupt_overrides: Vec<InterruptOverride>,
}

#[derive(Debug, Clone)]
pub struct LocalApicInfo {
    pub processor_id: u8,
    pub apic_id: u8,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct IoApicInfo {
    pub id: u8,
    pub address: u32,
    pub gsi_base: u32,
    pub max_entries: u8,
}

#[derive(Debug, Clone)]
pub struct InterruptOverride {
    pub source: u8,
    pub gsi: u32,
    pub flags: u16,
}

impl ApicInfo {
    pub fn new() -> Self {
        Self {
            local_apic_addr: 0xFEE00000, // Default address
            local_apics: Vec::new(),
            io_apics: Vec::new(),
            interrupt_overrides: Vec::new(),
        }
    }
}

// Local APIC Manager
pub struct LocalApic {
    base_addr: u64,
}

impl LocalApic {
    pub fn new(base_addr: u64) -> Self {
        Self { base_addr }
    }
    
    unsafe fn read(&self, reg: u32) -> u32 {
        let addr = (PHYS_MEM_OFFSET + self.base_addr + reg as u64) as *const u32;
        addr.read_volatile()
    }
    
    unsafe fn write(&self, reg: u32, value: u32) {
        let addr = (PHYS_MEM_OFFSET + self.base_addr + reg as u64) as *mut u32;
        addr.write_volatile(value);
    }
    
    pub fn init(&self) {
        unsafe {
            // Enable Local APIC
            let mut svr = self.read(LAPIC_SVR);
            svr |= 0x100; // Enable APIC
            svr |= 0xFF;  // Spurious interrupt vector
            self.write(LAPIC_SVR, svr);
            
            // Set task priority to accept all interrupts
            self.write(LAPIC_TPR, 0);
            
            // Disable logical interrupt lines
            self.write(LAPIC_LVT_LINT0, 0x10000);
            self.write(LAPIC_LVT_LINT1, 0x10000);
            
            // Disable performance counter overflow interrupts
            self.write(LAPIC_LVT_ERROR, 0x10000);
            
            // Map error interrupt to IRQ vector
            self.write(LAPIC_LVT_ERROR, 0x20 + 19);
            
            // Clear error status register
            self.write(0x280, 0);
            self.write(0x280, 0);
            
            // Send EOI
            self.eoi();
        }
    }
    
    pub fn id(&self) -> u8 {
        unsafe {
            ((self.read(LAPIC_ID) >> 24) & 0xFF) as u8
        }
    }
    
    pub fn version(&self) -> u32 {
        unsafe {
            self.read(LAPIC_VERSION)
        }
    }
    
    pub fn eoi(&self) {
        unsafe {
            self.write(LAPIC_EOI, 0);
        }
    }
    
    pub fn send_ipi(&self, dest: u8, vector: u8, delivery_mode: u8) {
        unsafe {
            // Write destination
            self.write(LAPIC_ICR_HIGH, (dest as u32) << 24);
            
            // Write command
            let mut icr_low = vector as u32;
            icr_low |= (delivery_mode as u32) << 8;
            icr_low |= 1 << 14; // Level triggered
            self.write(LAPIC_ICR_LOW, icr_low);
            
            // Wait for delivery
            while (self.read(LAPIC_ICR_LOW) & (1 << 12)) != 0 {
                core::hint::spin_loop();
            }
        }
    }
    
    pub fn start_timer(&self, count: u32, vector: u8, periodic: bool) {
        unsafe {
            // Set divider to 16
            self.write(LAPIC_TIMER_DIV, 0x3);
            
            // Configure timer
            let mut lvt = vector as u32;
            if periodic {
                lvt |= 0x20000; // Periodic mode
            }
            self.write(LAPIC_LVT_TIMER, lvt);
            
            // Set initial count
            self.write(LAPIC_TIMER_INIT, count);
        }
    }
    
    pub fn stop_timer(&self) {
        unsafe {
            // Mask timer interrupt
            self.write(LAPIC_LVT_TIMER, 0x10000);
            // Zero initial count stops timer
            self.write(LAPIC_TIMER_INIT, 0);
        }
    }
}

// I/O APIC Manager
pub struct IoApic {
    base_addr: u32,
    gsi_base: u32,
    max_entries: u8,
}

impl IoApic {
    pub fn new(base_addr: u32, gsi_base: u32) -> Self {
        let mut ioapic = Self {
            base_addr,
            gsi_base,
            max_entries: 0,
        };
        
        // Read max entries
        unsafe {
            let version = ioapic.read(IOAPIC_VERSION);
            ioapic.max_entries = ((version >> 16) & 0xFF) as u8 + 1;
        }
        
        ioapic
    }
    
    unsafe fn read(&self, reg: u8) -> u32 {
        let reg_addr = (PHYS_MEM_OFFSET + self.base_addr as u64) as *mut u32;
        let data_addr = (PHYS_MEM_OFFSET + self.base_addr as u64 + IOAPIC_DATA as u64) as *mut u32;
        
        reg_addr.write_volatile(reg as u32);
        data_addr.read_volatile()
    }
    
    unsafe fn write(&self, reg: u8, value: u32) {
        let reg_addr = (PHYS_MEM_OFFSET + self.base_addr as u64) as *mut u32;
        let data_addr = (PHYS_MEM_OFFSET + self.base_addr as u64 + IOAPIC_DATA as u64) as *mut u32;
        
        reg_addr.write_volatile(reg as u32);
        data_addr.write_volatile(value);
    }
    
    pub fn init(&self) {
        // Mask all interrupts
        for i in 0..self.max_entries {
            self.mask_irq(i);
        }
    }
    
    pub fn map_irq(&self, irq: u8, vector: u8, dest: u8) {
        let gsi = self.gsi_base + irq as u32;
        let entry = self.irq_to_entry(irq);
        
        if entry >= self.max_entries {
            return;
        }
        
        unsafe {
            let low_reg = IOAPIC_REDTBL + entry * 2;
            let high_reg = IOAPIC_REDTBL + entry * 2 + 1;
            
            // Set destination
            self.write(high_reg, (dest as u32) << 24);
            
            // Set vector and unmask
            let mut low = vector as u32;
            // Delivery mode: Fixed (000)
            // Destination mode: Physical (0)
            // Pin polarity: Active high (0)
            // Trigger mode: Edge (0)
            // Mask: Unmasked (0)
            self.write(low_reg, low);
        }
    }
    
    pub fn mask_irq(&self, irq: u8) {
        let entry = self.irq_to_entry(irq);
        
        if entry >= self.max_entries {
            return;
        }
        
        unsafe {
            let low_reg = IOAPIC_REDTBL + entry * 2;
            let value = self.read(low_reg);
            self.write(low_reg, value | 0x10000); // Set mask bit
        }
    }
    
    pub fn unmask_irq(&self, irq: u8) {
        let entry = self.irq_to_entry(irq);
        
        if entry >= self.max_entries {
            return;
        }
        
        unsafe {
            let low_reg = IOAPIC_REDTBL + entry * 2;
            let value = self.read(low_reg);
            self.write(low_reg, value & !0x10000); // Clear mask bit
        }
    }
    
    fn irq_to_entry(&self, irq: u8) -> u8 {
        // Map IRQ to I/O APIC entry
        // This would use interrupt overrides from MADT
        irq
    }
}

lazy_static! {
    static ref APIC_INFO: Mutex<ApicInfo> = Mutex::new(ApicInfo::new());
    static ref LOCAL_APIC: Mutex<Option<LocalApic>> = Mutex::new(None);
    static ref IO_APICS: Mutex<Vec<IoApic>> = Mutex::new(Vec::new());
}

pub fn parse_madt(table: *const SdtHeader) -> Result<(), &'static str> {
    let madt = table as *const Madt;
    
    unsafe {
        // Store Local APIC address
        APIC_INFO.lock().local_apic_addr = (*madt).local_apic_addr as u64;
        
        // Parse MADT entries
        let entries_start = (madt as *const u8).add(mem::size_of::<Madt>());
        let entries_end = (table as *const u8).add((*table).length as usize);
        let mut current = entries_start;
        
        while current < entries_end {
            let header = current as *const MadtEntryHeader;
            let entry_type = (*header).entry_type;
            let entry_len = (*header).length;
            
            match entry_type {
                0 => {
                    // Local APIC
                    let lapic = current as *const MadtLocalApic;
                    APIC_INFO.lock().local_apics.push(LocalApicInfo {
                        processor_id: (*lapic).processor_id,
                        apic_id: (*lapic).apic_id,
                        enabled: (*lapic).flags & 1 != 0,
                    });
                }
                1 => {
                    // I/O APIC
                    let ioapic = current as *const MadtIoApic;
                    APIC_INFO.lock().io_apics.push(IoApicInfo {
                        id: (*ioapic).io_apic_id,
                        address: (*ioapic).io_apic_address,
                        gsi_base: (*ioapic).global_system_interrupt_base,
                        max_entries: 24, // Will be read from I/O APIC
                    });
                }
                2 => {
                    // Interrupt Source Override
                    let iso = current as *const MadtInterruptSourceOverride;
                    APIC_INFO.lock().interrupt_overrides.push(InterruptOverride {
                        source: (*iso).source,
                        gsi: (*iso).global_system_interrupt,
                        flags: (*iso).flags,
                    });
                }
                _ => {
                    // Ignore other entry types
                }
            }
            
            current = current.add(entry_len as usize);
        }
    }
    
    serial_println!("ACPI: Found {} Local APICs, {} I/O APICs", 
             APIC_INFO.lock().local_apics.len(),
             APIC_INFO.lock().io_apics.len());
    
    Ok(())
}

pub fn init() -> Result<(), &'static str> {
    let apic_info = APIC_INFO.lock();
    
    // Initialize Local APIC
    let lapic = LocalApic::new(apic_info.local_apic_addr);
    lapic.init();
    serial_println!("APIC: Local APIC initialized (ID: {})", lapic.id());
    
    // Initialize I/O APICs
    let mut io_apics = IO_APICS.lock();
    for ioapic_info in &apic_info.io_apics {
        let ioapic = IoApic::new(ioapic_info.address, ioapic_info.gsi_base);
        ioapic.init();
        io_apics.push(ioapic);
    }
    
    drop(apic_info);
    *LOCAL_APIC.lock() = Some(lapic);
    
    Ok(())
}

pub fn eoi() {
    if let Some(ref lapic) = *LOCAL_APIC.lock() {
        lapic.eoi();
    }
}