// PCI Express Memory-Mapped Configuration
use super::tables::{Mcfg, McfgBaseAddress};
use super::SdtHeader;
use alloc::vec::Vec;
use core::mem;
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::PhysAddr;
use crate::memory::PHYS_MEM_OFFSET;
use crate::{println, serial_println};

// PCI Configuration Space
pub struct PciSegment {
    pub segment: u16,
    pub base_address: u64,
    pub start_bus: u8,
    pub end_bus: u8,
}

pub struct PciDevice {
    pub segment: u16,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub header_type: u8,
    pub driver: Option<alloc::string::String>,
}

impl PciSegment {
    pub fn config_addr(&self, bus: u8, device: u8, function: u8, offset: u16) -> u64 {
        if bus < self.start_bus || bus > self.end_bus {
            return 0;
        }
        
        if device > 31 || function > 7 || offset > 4095 {
            return 0;
        }
        
        self.base_address +
            ((bus as u64) << 20) +
            ((device as u64) << 15) +
            ((function as u64) << 12) +
            (offset as u64)
    }
    
    pub unsafe fn read_config_u8(&self, bus: u8, device: u8, function: u8, offset: u16) -> u8 {
        let addr = self.config_addr(bus, device, function, offset);
        if addr == 0 {
            return 0xFF;
        }
        
        let ptr = (PHYS_MEM_OFFSET + addr) as *const u8;
        ptr.read_volatile()
    }
    
    pub unsafe fn read_config_u16(&self, bus: u8, device: u8, function: u8, offset: u16) -> u16 {
        let addr = self.config_addr(bus, device, function, offset & !1);
        if addr == 0 {
            return 0xFFFF;
        }
        
        let ptr = (PHYS_MEM_OFFSET + addr) as *const u16;
        ptr.read_volatile()
    }
    
    pub unsafe fn read_config_u32(&self, bus: u8, device: u8, function: u8, offset: u16) -> u32 {
        let addr = self.config_addr(bus, device, function, offset & !3);
        if addr == 0 {
            return 0xFFFFFFFF;
        }
        
        let ptr = (PHYS_MEM_OFFSET + addr) as *const u32;
        ptr.read_volatile()
    }
    
    pub unsafe fn write_config_u8(&self, bus: u8, device: u8, function: u8, offset: u16, value: u8) {
        let addr = self.config_addr(bus, device, function, offset);
        if addr == 0 {
            return;
        }
        
        let ptr = (PHYS_MEM_OFFSET + addr) as *mut u8;
        ptr.write_volatile(value);
    }
    
    pub unsafe fn write_config_u16(&self, bus: u8, device: u8, function: u8, offset: u16, value: u16) {
        let addr = self.config_addr(bus, device, function, offset & !1);
        if addr == 0 {
            return;
        }
        
        let ptr = (PHYS_MEM_OFFSET + addr) as *mut u16;
        ptr.write_volatile(value);
    }
    
    pub unsafe fn write_config_u32(&self, bus: u8, device: u8, function: u8, offset: u16, value: u32) {
        let addr = self.config_addr(bus, device, function, offset & !3);
        if addr == 0 {
            return;
        }
        
        let ptr = (PHYS_MEM_OFFSET + addr) as *mut u32;
        ptr.write_volatile(value);
    }
    
    pub fn scan_bus(&self, bus: u8) -> Vec<PciDevice> {
        let mut devices = Vec::new();
        
        for device in 0..32 {
            unsafe {
                let vendor_id = self.read_config_u16(bus, device, 0, 0);
                
                if vendor_id == 0xFFFF {
                    continue; // No device
                }
                
                let device_id = self.read_config_u16(bus, device, 0, 2);
                let class_info = self.read_config_u32(bus, device, 0, 8);
                let header_type = self.read_config_u8(bus, device, 0, 0x0E);
                
                // Add function 0
                devices.push(PciDevice {
                    segment: self.segment,
                    bus,
                    device,
                    function: 0,
                    vendor_id,
                    device_id,
                    class_code: ((class_info >> 24) & 0xFF) as u8,
                    subclass: ((class_info >> 16) & 0xFF) as u8,
                    prog_if: ((class_info >> 8) & 0xFF) as u8,
                    header_type: header_type & 0x7F,
                    driver: None,
                });
                
                // Check if multi-function device
                if header_type & 0x80 != 0 {
                    for function in 1..8 {
                        let vendor_id = self.read_config_u16(bus, device, function, 0);
                        
                        if vendor_id == 0xFFFF {
                            continue;
                        }
                        
                        let device_id = self.read_config_u16(bus, device, function, 2);
                        let class_info = self.read_config_u32(bus, device, function, 8);
                        
                        devices.push(PciDevice {
                            segment: self.segment,
                            bus,
                            device,
                            function,
                            vendor_id,
                            device_id,
                            class_code: ((class_info >> 24) & 0xFF) as u8,
                            subclass: ((class_info >> 16) & 0xFF) as u8,
                            prog_if: ((class_info >> 8) & 0xFF) as u8,
                            header_type: header_type & 0x7F,
                            driver: None,
                        });
                    }
                }
            }
        }
        
        devices
    }
    
    pub fn enumerate_devices(&self) -> Vec<PciDevice> {
        let mut all_devices = Vec::new();
        
        for bus in self.start_bus..=self.end_bus {
            let devices = self.scan_bus(bus);
            all_devices.extend(devices);
        }
        
        all_devices
    }
}

impl PciDevice {
    pub fn bar(&self, segment: &PciSegment, bar_num: u8) -> Option<u64> {
        if bar_num > 5 {
            return None;
        }
        
        let offset = 0x10 + (bar_num as u16 * 4);
        
        unsafe {
            let bar = segment.read_config_u32(self.bus, self.device, self.function, offset);
            
            if bar == 0 {
                return None;
            }
            
            // Check if it's a 64-bit BAR
            if bar & 0x4 != 0 && bar_num < 5 {
                let bar_high = segment.read_config_u32(
                    self.bus, self.device, self.function, offset + 4
                );
                Some(((bar_high as u64) << 32) | (bar & !0xF) as u64)
            } else {
                Some((bar & !0xF) as u64)
            }
        }
    }
    
    pub fn enable_bus_mastering(&self, segment: &PciSegment) {
        unsafe {
            let command = segment.read_config_u16(self.bus, self.device, self.function, 4);
            segment.write_config_u16(self.bus, self.device, self.function, 4, command | 0x4);
        }
    }
    
    pub fn enable_memory_space(&self, segment: &PciSegment) {
        unsafe {
            let command = segment.read_config_u16(self.bus, self.device, self.function, 4);
            segment.write_config_u16(self.bus, self.device, self.function, 4, command | 0x2);
        }
    }
    
    pub fn enable_io_space(&self, segment: &PciSegment) {
        unsafe {
            let command = segment.read_config_u16(self.bus, self.device, self.function, 4);
            segment.write_config_u16(self.bus, self.device, self.function, 4, command | 0x1);
        }
    }
}

lazy_static! {
    static ref PCI_SEGMENTS: Mutex<Vec<PciSegment>> = Mutex::new(Vec::new());
}

pub fn parse_mcfg(table: *const SdtHeader) -> Result<(), &'static str> {
    let mcfg = table as *const Mcfg;
    
    unsafe {
        // Calculate number of allocation structures
        let num_entries = ((*table).length as usize - mem::size_of::<Mcfg>()) 
            / mem::size_of::<McfgBaseAddress>();
        
        let entries = (mcfg as *const u8).add(mem::size_of::<Mcfg>()) 
            as *const McfgBaseAddress;
        
        let mut segments = PCI_SEGMENTS.lock();
        
        for i in 0..num_entries {
            let entry = &*entries.add(i);
            
            // Copy fields to avoid unaligned access
            let segment = (*entry).segment_group_number;
            let base_addr = (*entry).base_address;
            let start_bus = (*entry).start_bus_number;
            let end_bus = (*entry).end_bus_number;
            
            segments.push(PciSegment {
                segment,
                base_address: base_addr,
                start_bus,
                end_bus,
            });
            
            serial_println!("ACPI: PCI Segment {} at 0x{:x} (buses {}-{})",
                     segment,
                     base_addr,
                     start_bus,
                     end_bus);
        }
    }
    
    Ok(())
}

pub fn enumerate_all_devices() -> Vec<PciDevice> {
    let mut all_devices = Vec::new();
    let segments = PCI_SEGMENTS.lock();
    
    for segment in segments.iter() {
        let devices = segment.enumerate_devices();
        all_devices.extend(devices);
    }
    
    all_devices
}

pub fn find_device(vendor_id: u16, device_id: u16) -> Option<PciDevice> {
    let segments = PCI_SEGMENTS.lock();
    
    for segment in segments.iter() {
        let devices = segment.enumerate_devices();
        for device in devices {
            if device.vendor_id == vendor_id && device.device_id == device_id {
                return Some(device);
            }
        }
    }
    
    None
}

pub fn find_devices_by_class(class_code: u8, subclass: u8) -> Vec<PciDevice> {
    let mut matching = Vec::new();
    let segments = PCI_SEGMENTS.lock();
    
    for segment in segments.iter() {
        let devices = segment.enumerate_devices();
        for device in devices {
            if device.class_code == class_code && device.subclass == subclass {
                matching.push(device);
            }
        }
    }
    
    matching
}

// Helper functions for monitoring/diagnostics module
pub fn enumerate_devices() -> Option<Vec<PciDevice>> {
    let segments = PCI_SEGMENTS.lock();
    
    if segments.is_empty() {
        return None;
    }
    
    let mut all_devices = Vec::new();
    for segment in segments.iter() {
        let devices = segment.enumerate_devices();
        all_devices.extend(devices);
    }
    
    Some(all_devices)
}