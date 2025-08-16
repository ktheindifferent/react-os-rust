// PCI Express (PCIe) Implementation
pub mod config;
pub mod device;
pub mod bus;
pub mod capability;
pub mod msi;

use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::format;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::{println, serial_println};
use crate::memory::PHYS_MEM_OFFSET;

// PCI Configuration Space Registers
pub const PCI_VENDOR_ID: u8 = 0x00;
pub const PCI_DEVICE_ID: u8 = 0x02;
pub const PCI_COMMAND: u8 = 0x04;
pub const PCI_STATUS: u8 = 0x06;
pub const PCI_REVISION_ID: u8 = 0x08;
pub const PCI_PROG_IF: u8 = 0x09;
pub const PCI_SUBCLASS: u8 = 0x0A;
pub const PCI_CLASS: u8 = 0x0B;
pub const PCI_CACHE_LINE_SIZE: u8 = 0x0C;
pub const PCI_LATENCY_TIMER: u8 = 0x0D;
pub const PCI_HEADER_TYPE: u8 = 0x0E;
pub const PCI_BIST: u8 = 0x0F;
pub const PCI_BAR0: u8 = 0x10;
pub const PCI_BAR1: u8 = 0x14;
pub const PCI_BAR2: u8 = 0x18;
pub const PCI_BAR3: u8 = 0x1C;
pub const PCI_BAR4: u8 = 0x20;
pub const PCI_BAR5: u8 = 0x24;
pub const PCI_CARDBUS_CIS: u8 = 0x28;
pub const PCI_SUBSYSTEM_VENDOR_ID: u8 = 0x2C;
pub const PCI_SUBSYSTEM_ID: u8 = 0x2E;
pub const PCI_EXPANSION_ROM_BASE: u8 = 0x30;
pub const PCI_CAPABILITIES_PTR: u8 = 0x34;
pub const PCI_INTERRUPT_LINE: u8 = 0x3C;
pub const PCI_INTERRUPT_PIN: u8 = 0x3D;
pub const PCI_MIN_GRANT: u8 = 0x3E;
pub const PCI_MAX_LATENCY: u8 = 0x3F;

// PCI Command Register Bits
pub const PCI_COMMAND_IO: u16 = 1 << 0;
pub const PCI_COMMAND_MEMORY: u16 = 1 << 1;
pub const PCI_COMMAND_MASTER: u16 = 1 << 2;
pub const PCI_COMMAND_SPECIAL: u16 = 1 << 3;
pub const PCI_COMMAND_INVALIDATE: u16 = 1 << 4;
pub const PCI_COMMAND_VGA_PALETTE: u16 = 1 << 5;
pub const PCI_COMMAND_PARITY: u16 = 1 << 6;
pub const PCI_COMMAND_WAIT: u16 = 1 << 7;
pub const PCI_COMMAND_SERR: u16 = 1 << 8;
pub const PCI_COMMAND_FAST_BACK: u16 = 1 << 9;
pub const PCI_COMMAND_INTX_DISABLE: u16 = 1 << 10;

// PCI Status Register Bits
pub const PCI_STATUS_INTX: u16 = 1 << 3;
pub const PCI_STATUS_CAPABILITIES: u16 = 1 << 4;
pub const PCI_STATUS_66MHZ: u16 = 1 << 5;
pub const PCI_STATUS_FAST_BACK: u16 = 1 << 7;
pub const PCI_STATUS_MASTER_PARITY: u16 = 1 << 8;
pub const PCI_STATUS_DEVSEL_MASK: u16 = 3 << 9;
pub const PCI_STATUS_SIGNALED_TARGET_ABORT: u16 = 1 << 11;
pub const PCI_STATUS_RECEIVED_TARGET_ABORT: u16 = 1 << 12;
pub const PCI_STATUS_RECEIVED_MASTER_ABORT: u16 = 1 << 13;
pub const PCI_STATUS_SIGNALED_SYSTEM_ERROR: u16 = 1 << 14;
pub const PCI_STATUS_DETECTED_PARITY: u16 = 1 << 15;

// PCI Header Types
pub const PCI_HEADER_TYPE_NORMAL: u8 = 0x00;
pub const PCI_HEADER_TYPE_BRIDGE: u8 = 0x01;
pub const PCI_HEADER_TYPE_CARDBUS: u8 = 0x02;
pub const PCI_HEADER_TYPE_MULTIFUNCTION: u8 = 0x80;

// PCI Express Capability IDs
pub const PCI_CAP_ID_PM: u8 = 0x01;        // Power Management
pub const PCI_CAP_ID_AGP: u8 = 0x02;       // AGP
pub const PCI_CAP_ID_VPD: u8 = 0x03;       // Vital Product Data
pub const PCI_CAP_ID_SLOTID: u8 = 0x04;    // Slot Identification
pub const PCI_CAP_ID_MSI: u8 = 0x05;       // Message Signaled Interrupts
pub const PCI_CAP_ID_CHSWP: u8 = 0x06;     // CompactPCI HotSwap
pub const PCI_CAP_ID_PCIX: u8 = 0x07;      // PCI-X
pub const PCI_CAP_ID_HT: u8 = 0x08;        // HyperTransport
pub const PCI_CAP_ID_VNDR: u8 = 0x09;      // Vendor-Specific
pub const PCI_CAP_ID_DBG: u8 = 0x0A;       // Debug port
pub const PCI_CAP_ID_CCRC: u8 = 0x0B;      // CompactPCI Central Resource Control
pub const PCI_CAP_ID_SHPC: u8 = 0x0C;      // PCI Standard Hot-Plug Controller
pub const PCI_CAP_ID_SSVID: u8 = 0x0D;     // Bridge subsystem vendor/device ID
pub const PCI_CAP_ID_AGP3: u8 = 0x0E;      // AGP 8x
pub const PCI_CAP_ID_SECDEV: u8 = 0x0F;    // Secure Device
pub const PCI_CAP_ID_EXP: u8 = 0x10;       // PCI Express
pub const PCI_CAP_ID_MSIX: u8 = 0x11;      // MSI-X
pub const PCI_CAP_ID_SATA: u8 = 0x12;      // SATA Data/Index Conf.
pub const PCI_CAP_ID_AF: u8 = 0x13;        // PCI Advanced Features

// PCIe Extended Capability IDs
pub const PCI_EXT_CAP_ID_ERR: u16 = 0x0001;    // Advanced Error Reporting
pub const PCI_EXT_CAP_ID_VC: u16 = 0x0002;     // Virtual Channel
pub const PCI_EXT_CAP_ID_DSN: u16 = 0x0003;    // Device Serial Number
pub const PCI_EXT_CAP_ID_PWR: u16 = 0x0004;    // Power Budgeting
pub const PCI_EXT_CAP_ID_RCLD: u16 = 0x0005;   // Root Complex Link Declaration
pub const PCI_EXT_CAP_ID_RCILC: u16 = 0x0006;  // Root Complex Internal Link Control
pub const PCI_EXT_CAP_ID_RCEC: u16 = 0x0007;   // Root Complex Event Collector
pub const PCI_EXT_CAP_ID_MFVC: u16 = 0x0008;   // Multi-Function Virtual Channel
pub const PCI_EXT_CAP_ID_RCRB: u16 = 0x000A;   // Root Complex Register Block
pub const PCI_EXT_CAP_ID_VNDR: u16 = 0x000B;   // Vendor-Specific Extended Capability
pub const PCI_EXT_CAP_ID_ACS: u16 = 0x000D;    // Access Control Services
pub const PCI_EXT_CAP_ID_ARI: u16 = 0x000E;    // Alternative Routing-ID
pub const PCI_EXT_CAP_ID_ATS: u16 = 0x000F;    // Address Translation Services
pub const PCI_EXT_CAP_ID_SRIOV: u16 = 0x0010;  // Single Root I/O Virtualization
pub const PCI_EXT_CAP_ID_PRI: u16 = 0x0013;    // Page Request Interface
pub const PCI_EXT_CAP_ID_TPH: u16 = 0x0017;    // TLP Processing Hints
pub const PCI_EXT_CAP_ID_LTR: u16 = 0x0018;    // Latency Tolerance Reporting

// PCI Device Classes
pub const PCI_CLASS_UNCLASSIFIED: u8 = 0x00;
pub const PCI_CLASS_STORAGE: u8 = 0x01;
pub const PCI_CLASS_NETWORK: u8 = 0x02;
pub const PCI_CLASS_DISPLAY: u8 = 0x03;
pub const PCI_CLASS_MULTIMEDIA: u8 = 0x04;
pub const PCI_CLASS_MEMORY: u8 = 0x05;
pub const PCI_CLASS_BRIDGE: u8 = 0x06;
pub const PCI_CLASS_COMMUNICATION: u8 = 0x07;
pub const PCI_CLASS_SYSTEM: u8 = 0x08;
pub const PCI_CLASS_INPUT: u8 = 0x09;
pub const PCI_CLASS_DOCKING: u8 = 0x0A;
pub const PCI_CLASS_PROCESSOR: u8 = 0x0B;
pub const PCI_CLASS_SERIAL_BUS: u8 = 0x0C;
pub const PCI_CLASS_WIRELESS: u8 = 0x0D;
pub const PCI_CLASS_INTELLIGENT: u8 = 0x0E;
pub const PCI_CLASS_SATELLITE: u8 = 0x0F;
pub const PCI_CLASS_ENCRYPTION: u8 = 0x10;
pub const PCI_CLASS_SIGNAL_PROCESSING: u8 = 0x11;

// PCI Storage Subclasses
pub const PCI_SUBCLASS_STORAGE_SCSI: u8 = 0x00;
pub const PCI_SUBCLASS_STORAGE_IDE: u8 = 0x01;
pub const PCI_SUBCLASS_STORAGE_FLOPPY: u8 = 0x02;
pub const PCI_SUBCLASS_STORAGE_IPI: u8 = 0x03;
pub const PCI_SUBCLASS_STORAGE_RAID: u8 = 0x04;
pub const PCI_SUBCLASS_STORAGE_ATA: u8 = 0x05;
pub const PCI_SUBCLASS_STORAGE_SATA: u8 = 0x06;
pub const PCI_SUBCLASS_STORAGE_SAS: u8 = 0x07;
pub const PCI_SUBCLASS_STORAGE_NVME: u8 = 0x08;

// PCIe Configuration Access Methods
#[derive(Debug, Clone, Copy)]
pub enum PciAccessMethod {
    Legacy,     // I/O ports 0xCF8/0xCFC
    MemoryMapped, // MMCONFIG/ECAM
}

// PCI Device Location
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PciLocation {
    pub segment: u16,  // PCIe segment group
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}

impl PciLocation {
    pub fn new(segment: u16, bus: u8, device: u8, function: u8) -> Self {
        Self { segment, bus, device, function }
    }
    
    pub fn to_legacy_address(&self, register: u8) -> u32 {
        let enable_bit = 1u32 << 31;
        let bus = (self.bus as u32) << 16;
        let device = (self.device as u32) << 11;
        let function = (self.function as u32) << 8;
        let reg = (register as u32) & 0xFC;
        
        enable_bit | bus | device | function | reg
    }
}

// PCI Device
#[derive(Debug, Clone)]
pub struct PciDevice {
    pub location: PciLocation,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub revision: u8,
    pub header_type: u8,
    pub bars: [u32; 6],
    pub capabilities: Vec<PciCapability>,
    pub extended_capabilities: Vec<PciExtendedCapability>,
}

// PCI Capability
#[derive(Debug, Clone)]
pub struct PciCapability {
    pub id: u8,
    pub offset: u8,
    pub data: Vec<u8>,
}

// PCIe Extended Capability
#[derive(Debug, Clone)]
pub struct PciExtendedCapability {
    pub id: u16,
    pub offset: u16,
    pub version: u8,
    pub data: Vec<u8>,
}

// Base Address Register (BAR)
#[derive(Debug, Clone, Copy)]
pub enum BarType {
    Memory32 { address: u32, size: u32, prefetchable: bool },
    Memory64 { address: u64, size: u64, prefetchable: bool },
    Io { address: u32, size: u32 },
}

// PCIe Controller
pub struct PcieController {
    access_method: PciAccessMethod,
    mmconfig_base: u64,
    devices: Vec<PciDevice>,
}

impl PcieController {
    pub fn new() -> Self {
        Self {
            access_method: PciAccessMethod::Legacy,
            mmconfig_base: 0,
            devices: Vec::new(),
        }
    }
    
    pub fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("PCIe: Initializing controller");
        
        // Try to detect MMCONFIG/ECAM from ACPI MCFG table
        if let Some(base) = self.detect_mmconfig() {
            self.mmconfig_base = base;
            self.access_method = PciAccessMethod::MemoryMapped;
            serial_println!("PCIe: Using MMCONFIG at 0x{:x}", base);
        } else {
            serial_println!("PCIe: Using legacy I/O port access");
        }
        
        // Enumerate all devices
        self.enumerate_devices()?;
        
        serial_println!("PCIe: Found {} devices", self.devices.len());
        
        Ok(())
    }
    
    fn detect_mmconfig(&self) -> Option<u64> {
        // This would read the ACPI MCFG table
        // For now, return None to use legacy access
        None
    }
    
    pub fn enumerate_devices(&mut self) -> Result<(), &'static str> {
        // Scan all buses
        for bus in 0..=255u8 {
            self.enumerate_bus(0, bus)?;
        }
        
        Ok(())
    }
    
    fn enumerate_bus(&mut self, segment: u16, bus: u8) -> Result<(), &'static str> {
        for device in 0..32u8 {
            if self.device_exists(segment, bus, device, 0) {
                let header_type = self.read8(PciLocation::new(segment, bus, device, 0), PCI_HEADER_TYPE);
                
                if header_type & PCI_HEADER_TYPE_MULTIFUNCTION != 0 {
                    // Multi-function device
                    for function in 0..8u8 {
                        if self.device_exists(segment, bus, device, function) {
                            self.enumerate_device(segment, bus, device, function)?;
                        }
                    }
                } else {
                    // Single function device
                    self.enumerate_device(segment, bus, device, 0)?;
                }
            }
        }
        
        Ok(())
    }
    
    fn enumerate_device(&mut self, segment: u16, bus: u8, device: u8, function: u8) -> Result<(), &'static str> {
        let location = PciLocation::new(segment, bus, device, function);
        
        let vendor_id = self.read16(location, PCI_VENDOR_ID);
        let device_id = self.read16(location, PCI_DEVICE_ID);
        let class = self.read8(location, PCI_CLASS);
        let subclass = self.read8(location, PCI_SUBCLASS);
        let prog_if = self.read8(location, PCI_PROG_IF);
        let revision = self.read8(location, PCI_REVISION_ID);
        let header_type = self.read8(location, PCI_HEADER_TYPE) & 0x7F;
        
        // Read BARs
        let mut bars = [0u32; 6];
        for i in 0..6 {
            bars[i] = self.read32(location, PCI_BAR0 + (i as u8 * 4));
        }
        
        // Enumerate capabilities
        let capabilities = self.enumerate_capabilities(location)?;
        let extended_capabilities = self.enumerate_extended_capabilities(location)?;
        
        let pci_device = PciDevice {
            location,
            vendor_id,
            device_id,
            class,
            subclass,
            prog_if,
            revision,
            header_type,
            bars,
            capabilities,
            extended_capabilities,
        };
        
        serial_println!("PCIe: {:02x}:{:02x}.{} [{:04x}:{:04x}] Class {:02x}:{:02x} {}",
                      bus, device, function, vendor_id, device_id, class, subclass,
                      self.get_device_description(class, subclass));
        
        self.devices.push(pci_device);
        
        // If this is a bridge, enumerate the secondary bus
        if class == PCI_CLASS_BRIDGE && (subclass == 0x04 || subclass == 0x09) {
            let secondary_bus = self.read8(location, 0x19);
            if secondary_bus != 0 {
                self.enumerate_bus(segment, secondary_bus)?;
            }
        }
        
        Ok(())
    }
    
    fn enumerate_capabilities(&self, location: PciLocation) -> Result<Vec<PciCapability>, &'static str> {
        let mut capabilities = Vec::new();
        
        let status = self.read16(location, PCI_STATUS);
        if status & PCI_STATUS_CAPABILITIES == 0 {
            return Ok(capabilities);
        }
        
        let mut cap_ptr = self.read8(location, PCI_CAPABILITIES_PTR) & 0xFC;
        
        while cap_ptr != 0 && cap_ptr < 0xFC {
            let cap_id = self.read8(location, cap_ptr);
            let next_ptr = self.read8(location, cap_ptr + 1) & 0xFC;
            
            // Read capability data
            let mut data = Vec::new();
            for i in 0..16 {
                data.push(self.read8(location, cap_ptr + i));
            }
            
            capabilities.push(PciCapability {
                id: cap_id,
                offset: cap_ptr,
                data,
            });
            
            cap_ptr = next_ptr;
        }
        
        Ok(capabilities)
    }
    
    fn enumerate_extended_capabilities(&self, location: PciLocation) -> Result<Vec<PciExtendedCapability>, &'static str> {
        let mut capabilities = Vec::new();
        
        // Extended capabilities start at offset 0x100
        let mut cap_offset = 0x100u16;
        
        loop {
            let cap_header = self.read32(location, cap_offset as u8);
            if cap_header == 0 || cap_header == 0xFFFFFFFF {
                break;
            }
            
            let cap_id = (cap_header & 0xFFFF) as u16;
            let cap_version = ((cap_header >> 16) & 0x0F) as u8;
            let next_offset = ((cap_header >> 20) & 0xFFF) as u16;
            
            // Read capability data
            let mut data = Vec::new();
            for i in 0..64 {
                data.push(self.read8(location, (cap_offset + i) as u8));
            }
            
            capabilities.push(PciExtendedCapability {
                id: cap_id,
                offset: cap_offset,
                version: cap_version,
                data,
            });
            
            if next_offset == 0 {
                break;
            }
            cap_offset = next_offset;
        }
        
        Ok(capabilities)
    }
    
    fn device_exists(&self, segment: u16, bus: u8, device: u8, function: u8) -> bool {
        let location = PciLocation::new(segment, bus, device, function);
        let vendor_id = self.read16(location, PCI_VENDOR_ID);
        vendor_id != 0xFFFF && vendor_id != 0x0000
    }
    
    pub fn read8(&self, location: PciLocation, register: u8) -> u8 {
        match self.access_method {
            PciAccessMethod::Legacy => self.legacy_read8(location, register),
            PciAccessMethod::MemoryMapped => self.mmconfig_read8(location, register),
        }
    }
    
    pub fn read16(&self, location: PciLocation, register: u8) -> u16 {
        match self.access_method {
            PciAccessMethod::Legacy => self.legacy_read16(location, register),
            PciAccessMethod::MemoryMapped => self.mmconfig_read16(location, register),
        }
    }
    
    pub fn read32(&self, location: PciLocation, register: u8) -> u32 {
        match self.access_method {
            PciAccessMethod::Legacy => self.legacy_read32(location, register),
            PciAccessMethod::MemoryMapped => self.mmconfig_read32(location, register),
        }
    }
    
    pub fn write8(&self, location: PciLocation, register: u8, value: u8) {
        match self.access_method {
            PciAccessMethod::Legacy => self.legacy_write8(location, register, value),
            PciAccessMethod::MemoryMapped => self.mmconfig_write8(location, register, value),
        }
    }
    
    pub fn write16(&self, location: PciLocation, register: u8, value: u16) {
        match self.access_method {
            PciAccessMethod::Legacy => self.legacy_write16(location, register, value),
            PciAccessMethod::MemoryMapped => self.mmconfig_write16(location, register, value),
        }
    }
    
    pub fn write32(&self, location: PciLocation, register: u8, value: u32) {
        match self.access_method {
            PciAccessMethod::Legacy => self.legacy_write32(location, register, value),
            PciAccessMethod::MemoryMapped => self.mmconfig_write32(location, register, value),
        }
    }
    
    // Legacy I/O port access
    fn legacy_read8(&self, location: PciLocation, register: u8) -> u8 {
        let value = self.legacy_read32(location, register & 0xFC);
        ((value >> ((register & 3) * 8)) & 0xFF) as u8
    }
    
    fn legacy_read16(&self, location: PciLocation, register: u8) -> u16 {
        let value = self.legacy_read32(location, register & 0xFC);
        ((value >> ((register & 2) * 8)) & 0xFFFF) as u16
    }
    
    fn legacy_read32(&self, location: PciLocation, register: u8) -> u32 {
        use x86_64::instructions::port::Port;
        
        unsafe {
            let mut address_port = Port::<u32>::new(0xCF8);
            let mut data_port = Port::<u32>::new(0xCFC);
            
            let address = location.to_legacy_address(register);
            address_port.write(address);
            data_port.read()
        }
    }
    
    fn legacy_write8(&self, location: PciLocation, register: u8, value: u8) {
        let old_value = self.legacy_read32(location, register & 0xFC);
        let shift = (register & 3) * 8;
        let mask = !(0xFFu32 << shift);
        let new_value = (old_value & mask) | ((value as u32) << shift);
        self.legacy_write32(location, register & 0xFC, new_value);
    }
    
    fn legacy_write16(&self, location: PciLocation, register: u8, value: u16) {
        let old_value = self.legacy_read32(location, register & 0xFC);
        let shift = (register & 2) * 8;
        let mask = !(0xFFFFu32 << shift);
        let new_value = (old_value & mask) | ((value as u32) << shift);
        self.legacy_write32(location, register & 0xFC, new_value);
    }
    
    fn legacy_write32(&self, location: PciLocation, register: u8, value: u32) {
        use x86_64::instructions::port::Port;
        
        unsafe {
            let mut address_port = Port::<u32>::new(0xCF8);
            let mut data_port = Port::<u32>::new(0xCFC);
            
            let address = location.to_legacy_address(register);
            address_port.write(address);
            data_port.write(value);
        }
    }
    
    // Memory-mapped configuration access (MMCONFIG/ECAM)
    fn mmconfig_read8(&self, location: PciLocation, register: u8) -> u8 {
        unsafe {
            let addr = self.mmconfig_address(location, register);
            let ptr = (PHYS_MEM_OFFSET + addr) as *const u8;
            ptr.read_volatile()
        }
    }
    
    fn mmconfig_read16(&self, location: PciLocation, register: u8) -> u16 {
        unsafe {
            let addr = self.mmconfig_address(location, register);
            let ptr = (PHYS_MEM_OFFSET + addr) as *const u16;
            ptr.read_volatile()
        }
    }
    
    fn mmconfig_read32(&self, location: PciLocation, register: u8) -> u32 {
        unsafe {
            let addr = self.mmconfig_address(location, register);
            let ptr = (PHYS_MEM_OFFSET + addr) as *const u32;
            ptr.read_volatile()
        }
    }
    
    fn mmconfig_write8(&self, location: PciLocation, register: u8, value: u8) {
        unsafe {
            let addr = self.mmconfig_address(location, register);
            let ptr = (PHYS_MEM_OFFSET + addr) as *mut u8;
            ptr.write_volatile(value);
        }
    }
    
    fn mmconfig_write16(&self, location: PciLocation, register: u8, value: u16) {
        unsafe {
            let addr = self.mmconfig_address(location, register);
            let ptr = (PHYS_MEM_OFFSET + addr) as *mut u16;
            ptr.write_volatile(value);
        }
    }
    
    fn mmconfig_write32(&self, location: PciLocation, register: u8, value: u32) {
        unsafe {
            let addr = self.mmconfig_address(location, register);
            let ptr = (PHYS_MEM_OFFSET + addr) as *mut u32;
            ptr.write_volatile(value);
        }
    }
    
    fn mmconfig_address(&self, location: PciLocation, register: u8) -> u64 {
        self.mmconfig_base +
            ((location.segment as u64) << 28) +
            ((location.bus as u64) << 20) +
            ((location.device as u64) << 15) +
            ((location.function as u64) << 12) +
            (register as u64)
    }
    
    fn get_device_description(&self, class: u8, subclass: u8) -> &'static str {
        match class {
            PCI_CLASS_STORAGE => match subclass {
                PCI_SUBCLASS_STORAGE_SCSI => "SCSI Controller",
                PCI_SUBCLASS_STORAGE_IDE => "IDE Controller",
                PCI_SUBCLASS_STORAGE_SATA => "SATA Controller",
                PCI_SUBCLASS_STORAGE_NVME => "NVMe Controller",
                _ => "Storage Controller",
            },
            PCI_CLASS_NETWORK => "Network Controller",
            PCI_CLASS_DISPLAY => "Display Controller",
            PCI_CLASS_MULTIMEDIA => "Multimedia Controller",
            PCI_CLASS_BRIDGE => "Bridge Device",
            PCI_CLASS_COMMUNICATION => "Communication Controller",
            PCI_CLASS_SYSTEM => "System Device",
            PCI_CLASS_INPUT => "Input Device",
            PCI_CLASS_SERIAL_BUS => "Serial Bus Controller",
            _ => "Unknown Device",
        }
    }
    
    pub fn find_devices_by_class(&self, class: u8, subclass: Option<u8>) -> Vec<&PciDevice> {
        self.devices.iter()
            .filter(|d| d.class == class && subclass.map_or(true, |sc| d.subclass == sc))
            .collect()
    }
    
    pub fn find_device_by_id(&self, vendor_id: u16, device_id: u16) -> Option<&PciDevice> {
        self.devices.iter()
            .find(|d| d.vendor_id == vendor_id && d.device_id == device_id)
    }
    
    pub fn enable_device(&self, device: &PciDevice) {
        let mut command = self.read16(device.location, PCI_COMMAND);
        command |= PCI_COMMAND_IO | PCI_COMMAND_MEMORY | PCI_COMMAND_MASTER;
        self.write16(device.location, PCI_COMMAND, command);
    }
    
    pub fn disable_device(&self, device: &PciDevice) {
        let mut command = self.read16(device.location, PCI_COMMAND);
        command &= !(PCI_COMMAND_IO | PCI_COMMAND_MEMORY | PCI_COMMAND_MASTER);
        self.write16(device.location, PCI_COMMAND, command);
    }
}

impl PciDevice {
    pub fn decode_bars(&self, controller: &PcieController) -> Vec<BarType> {
        let mut bars = Vec::new();
        let mut i = 0;
        
        while i < 6 {
            let bar_value = self.bars[i];
            
            if bar_value == 0 {
                i += 1;
                continue;
            }
            
            if bar_value & 1 == 1 {
                // I/O BAR
                let address = bar_value & 0xFFFFFFFC;
                
                // Get size by writing all 1s and reading back
                controller.write32(self.location, PCI_BAR0 + (i as u8 * 4), 0xFFFFFFFF);
                let size_mask = controller.read32(self.location, PCI_BAR0 + (i as u8 * 4));
                controller.write32(self.location, PCI_BAR0 + (i as u8 * 4), bar_value);
                
                let size = (!size_mask | 3) + 1;
                
                bars.push(BarType::Io { address, size });
                i += 1;
            } else {
                // Memory BAR
                let prefetchable = (bar_value & 0x08) != 0;
                let bar_type = (bar_value >> 1) & 0x03;
                
                match bar_type {
                    0 => {
                        // 32-bit BAR
                        let address = bar_value & 0xFFFFFFF0;
                        
                        // Get size
                        controller.write32(self.location, PCI_BAR0 + (i as u8 * 4), 0xFFFFFFFF);
                        let size_mask = controller.read32(self.location, PCI_BAR0 + (i as u8 * 4));
                        controller.write32(self.location, PCI_BAR0 + (i as u8 * 4), bar_value);
                        
                        let size = (!size_mask | 0x0F) + 1;
                        
                        bars.push(BarType::Memory32 { address, size, prefetchable });
                        i += 1;
                    }
                    2 => {
                        // 64-bit BAR
                        if i >= 5 {
                            break; // Not enough space for 64-bit BAR
                        }
                        
                        let address_low = bar_value & 0xFFFFFFF0;
                        let address_high = self.bars[i + 1];
                        let address = ((address_high as u64) << 32) | (address_low as u64);
                        
                        // Get size
                        controller.write32(self.location, PCI_BAR0 + (i as u8 * 4), 0xFFFFFFFF);
                        controller.write32(self.location, PCI_BAR0 + ((i + 1) as u8 * 4), 0xFFFFFFFF);
                        let size_low = controller.read32(self.location, PCI_BAR0 + (i as u8 * 4));
                        let size_high = controller.read32(self.location, PCI_BAR0 + ((i + 1) as u8 * 4));
                        controller.write32(self.location, PCI_BAR0 + (i as u8 * 4), bar_value);
                        controller.write32(self.location, PCI_BAR0 + ((i + 1) as u8 * 4), self.bars[i + 1]);
                        
                        let size_mask = ((size_high as u64) << 32) | (size_low as u64);
                        let size = (!size_mask | 0x0F) + 1;
                        
                        bars.push(BarType::Memory64 { address, size, prefetchable });
                        i += 2;
                    }
                    _ => {
                        // Reserved
                        i += 1;
                    }
                }
            }
        }
        
        bars
    }
    
    pub fn has_capability(&self, cap_id: u8) -> bool {
        self.capabilities.iter().any(|c| c.id == cap_id)
    }
    
    pub fn get_capability(&self, cap_id: u8) -> Option<&PciCapability> {
        self.capabilities.iter().find(|c| c.id == cap_id)
    }
    
    pub fn has_extended_capability(&self, cap_id: u16) -> bool {
        self.extended_capabilities.iter().any(|c| c.id == cap_id)
    }
    
    pub fn get_extended_capability(&self, cap_id: u16) -> Option<&PciExtendedCapability> {
        self.extended_capabilities.iter().find(|c| c.id == cap_id)
    }
}

lazy_static! {
    pub static ref PCIE_CONTROLLER: Mutex<PcieController> = Mutex::new(PcieController::new());
}

pub fn init() -> Result<(), &'static str> {
    PCIE_CONTROLLER.lock().init()
}

pub fn enumerate_devices() {
    let controller = PCIE_CONTROLLER.lock();
    
    serial_println!("PCIe: Device listing:");
    for device in &controller.devices {
        serial_println!("  {:02x}:{:02x}.{} [{:04x}:{:04x}] Class {:02x}:{:02x}",
                      device.location.bus,
                      device.location.device,
                      device.location.function,
                      device.vendor_id,
                      device.device_id,
                      device.class,
                      device.subclass);
    }
}