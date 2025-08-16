// PCIe Configuration Space Management
use super::*;

// PCIe Configuration Space Structure
pub struct PcieConfigSpace {
    location: PciLocation,
    controller: &'static PcieController,
}

impl PcieConfigSpace {
    pub fn new(location: PciLocation, controller: &'static PcieController) -> Self {
        Self { location, controller }
    }
    
    pub fn read8(&self, offset: u8) -> u8 {
        self.controller.read8(self.location, offset)
    }
    
    pub fn read16(&self, offset: u8) -> u16 {
        self.controller.read16(self.location, offset)
    }
    
    pub fn read32(&self, offset: u8) -> u32 {
        self.controller.read32(self.location, offset)
    }
    
    pub fn write8(&self, offset: u8, value: u8) {
        self.controller.write8(self.location, offset, value);
    }
    
    pub fn write16(&self, offset: u8, value: u16) {
        self.controller.write16(self.location, offset, value);
    }
    
    pub fn write32(&self, offset: u8, value: u32) {
        self.controller.write32(self.location, offset, value);
    }
    
    pub fn enable_bus_master(&self) {
        let mut command = self.read16(PCI_COMMAND);
        command |= PCI_COMMAND_MASTER;
        self.write16(PCI_COMMAND, command);
    }
    
    pub fn disable_bus_master(&self) {
        let mut command = self.read16(PCI_COMMAND);
        command &= !PCI_COMMAND_MASTER;
        self.write16(PCI_COMMAND, command);
    }
    
    pub fn enable_memory_space(&self) {
        let mut command = self.read16(PCI_COMMAND);
        command |= PCI_COMMAND_MEMORY;
        self.write16(PCI_COMMAND, command);
    }
    
    pub fn enable_io_space(&self) {
        let mut command = self.read16(PCI_COMMAND);
        command |= PCI_COMMAND_IO;
        self.write16(PCI_COMMAND, command);
    }
    
    pub fn set_interrupt_disable(&self, disable: bool) {
        let mut command = self.read16(PCI_COMMAND);
        if disable {
            command |= PCI_COMMAND_INTX_DISABLE;
        } else {
            command &= !PCI_COMMAND_INTX_DISABLE;
        }
        self.write16(PCI_COMMAND, command);
    }
    
    pub fn get_interrupt_line(&self) -> u8 {
        self.read8(PCI_INTERRUPT_LINE)
    }
    
    pub fn get_interrupt_pin(&self) -> u8 {
        self.read8(PCI_INTERRUPT_PIN)
    }
}