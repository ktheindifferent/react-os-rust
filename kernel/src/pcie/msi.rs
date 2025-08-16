// MSI/MSI-X Interrupt Management
use super::*;
use super::capability::{MsiCapability, MsixCapability};
use crate::interrupts;

pub struct MsiManager {
    msi_base_vector: u8,
    allocated_vectors: Vec<bool>,
}

impl MsiManager {
    pub fn new() -> Self {
        Self {
            msi_base_vector: 32, // Start after legacy IRQs
            allocated_vectors: vec![false; 224], // 256 - 32 vectors
        }
    }
    
    pub fn allocate_vector(&mut self) -> Option<u8> {
        for (i, allocated) in self.allocated_vectors.iter_mut().enumerate() {
            if !*allocated {
                *allocated = true;
                return Some(self.msi_base_vector + i as u8);
            }
        }
        None
    }
    
    pub fn free_vector(&mut self, vector: u8) {
        if vector >= self.msi_base_vector {
            let index = (vector - self.msi_base_vector) as usize;
            if index < self.allocated_vectors.len() {
                self.allocated_vectors[index] = false;
            }
        }
    }
    
    pub fn configure_msi(&mut self, device: &PciDevice, controller: &PcieController) -> Result<u8, &'static str> {
        let msi_cap = device.get_capability(PCI_CAP_ID_MSI)
            .ok_or("Device does not support MSI")?;
        
        let vector = self.allocate_vector()
            .ok_or("No available MSI vectors")?;
        
        // Configure MSI
        let mut msi = MsiCapability::parse(&msi_cap.data);
        
        // Set message address (processor local APIC)
        msi.message_address = 0xFEE00000; // Local APIC address
        
        // Set message data (vector)
        msi.message_data = vector as u16;
        
        // Write configuration
        controller.write32(device.location, msi_cap.offset + 4, msi.message_address);
        if msi.message_control & 0x80 != 0 {
            // 64-bit capable
            controller.write32(device.location, msi_cap.offset + 8, msi.message_upper_address);
            controller.write16(device.location, msi_cap.offset + 12, msi.message_data);
        } else {
            controller.write16(device.location, msi_cap.offset + 8, msi.message_data);
        }
        
        // Enable MSI
        msi.enable(controller, device.location);
        
        serial_println!("MSI: Configured device {:04x}:{:04x} with vector {}", 
                      device.vendor_id, device.device_id, vector);
        
        Ok(vector)
    }
    
    pub fn configure_msix(&mut self, device: &PciDevice, controller: &PcieController, num_vectors: u16) -> Result<Vec<u8>, &'static str> {
        let msix_cap = device.get_capability(PCI_CAP_ID_MSIX)
            .ok_or("Device does not support MSI-X")?;
        
        let msix = MsixCapability::parse(&msix_cap.data);
        
        if num_vectors > msix.table_size {
            return Err("Requested vectors exceed MSI-X table size");
        }
        
        let mut vectors = Vec::new();
        
        // Allocate vectors
        for _ in 0..num_vectors {
            let vector = self.allocate_vector()
                .ok_or("No available MSI-X vectors")?;
            vectors.push(vector);
        }
        
        // Get BAR for MSI-X table
        let bars = device.decode_bars(controller);
        let table_bar = bars.get(msix.table_bir as usize)
            .ok_or("Invalid MSI-X table BAR")?;
        
        let table_base = match table_bar {
            BarType::Memory32 { address, .. } => *address as u64,
            BarType::Memory64 { address, .. } => *address,
            _ => return Err("MSI-X table must be in memory BAR"),
        };
        
        // Configure MSI-X table entries
        for (i, &vector) in vectors.iter().enumerate() {
            let entry_offset = i * 16; // Each entry is 16 bytes
            let entry_addr = table_base + msix.table_offset as u64 + entry_offset as u64;
            
            unsafe {
                // Message address (lower)
                let addr_ptr = (PHYS_MEM_OFFSET + entry_addr) as *mut u32;
                addr_ptr.write_volatile(0xFEE00000);
                
                // Message address (upper)
                let upper_ptr = (PHYS_MEM_OFFSET + entry_addr + 4) as *mut u32;
                upper_ptr.write_volatile(0);
                
                // Message data
                let data_ptr = (PHYS_MEM_OFFSET + entry_addr + 8) as *mut u32;
                data_ptr.write_volatile(vector as u32);
                
                // Vector control (unmask)
                let control_ptr = (PHYS_MEM_OFFSET + entry_addr + 12) as *mut u32;
                control_ptr.write_volatile(0);
            }
        }
        
        // Enable MSI-X
        let mut control = controller.read16(device.location, msix_cap.offset + 2);
        control |= 0x8000; // MSI-X Enable
        control &= !0x4000; // Function Mask
        controller.write16(device.location, msix_cap.offset + 2, control);
        
        serial_println!("MSI-X: Configured device {:04x}:{:04x} with {} vectors", 
                      device.vendor_id, device.device_id, vectors.len());
        
        Ok(vectors)
    }
}

// Interrupt handler registration
pub fn register_msi_handler(vector: u8, handler: fn()) {
    // This would register the handler with the interrupt system
    serial_println!("MSI: Registered handler for vector {}", vector);
}