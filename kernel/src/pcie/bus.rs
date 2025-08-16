// PCIe Bus Management
use super::*;

pub struct PciBus {
    pub segment: u16,
    pub number: u8,
    pub devices: Vec<PciLocation>,
}

impl PciBus {
    pub fn new(segment: u16, number: u8) -> Self {
        Self {
            segment,
            number,
            devices: Vec::new(),
        }
    }
    
    pub fn scan(&mut self, controller: &PcieController) -> Result<(), &'static str> {
        for device in 0..32 {
            for function in 0..8 {
                let location = PciLocation::new(self.segment, self.number, device, function);
                let vendor_id = controller.read16(location, PCI_VENDOR_ID);
                
                if vendor_id != 0xFFFF && vendor_id != 0x0000 {
                    self.devices.push(location);
                    
                    // Check if this is a multi-function device
                    if function == 0 {
                        let header_type = controller.read8(location, PCI_HEADER_TYPE);
                        if header_type & PCI_HEADER_TYPE_MULTIFUNCTION == 0 {
                            break; // Single function device
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}

pub struct PciBridge {
    pub location: PciLocation,
    pub primary_bus: u8,
    pub secondary_bus: u8,
    pub subordinate_bus: u8,
}

impl PciBridge {
    pub fn new(location: PciLocation, controller: &PcieController) -> Self {
        let primary_bus = controller.read8(location, 0x18);
        let secondary_bus = controller.read8(location, 0x19);
        let subordinate_bus = controller.read8(location, 0x1A);
        
        Self {
            location,
            primary_bus,
            secondary_bus,
            subordinate_bus,
        }
    }
    
    pub fn configure(&self, controller: &PcieController) {
        // Enable bridge
        let mut command = controller.read16(self.location, PCI_COMMAND);
        command |= PCI_COMMAND_MASTER | PCI_COMMAND_MEMORY | PCI_COMMAND_IO;
        controller.write16(self.location, PCI_COMMAND, command);
        
        // Enable forwarding
        let mut bridge_control = controller.read16(self.location, 0x3E);
        bridge_control |= 0x03; // Enable parity error response and SERR#
        controller.write16(self.location, 0x3E, bridge_control);
    }
}