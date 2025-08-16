// PCIe Capability Handling
use super::*;

// MSI Capability
pub struct MsiCapability {
    pub offset: u8,
    pub message_control: u16,
    pub message_address: u32,
    pub message_upper_address: u32,
    pub message_data: u16,
}

impl MsiCapability {
    pub fn parse(data: &[u8]) -> Self {
        let message_control = u16::from_le_bytes([data[2], data[3]]);
        let message_address = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        
        let (message_upper_address, message_data) = if message_control & 0x80 != 0 {
            // 64-bit capable
            let upper = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
            let data = u16::from_le_bytes([data[12], data[13]]);
            (upper, data)
        } else {
            // 32-bit only
            let data = u16::from_le_bytes([data[8], data[9]]);
            (0, data)
        };
        
        Self {
            offset: data[0],
            message_control,
            message_address,
            message_upper_address,
            message_data,
        }
    }
    
    pub fn enable(&mut self, controller: &PcieController, location: PciLocation) {
        self.message_control |= 1; // Enable MSI
        controller.write16(location, self.offset + 2, self.message_control);
    }
    
    pub fn disable(&mut self, controller: &PcieController, location: PciLocation) {
        self.message_control &= !1; // Disable MSI
        controller.write16(location, self.offset + 2, self.message_control);
    }
    
    pub fn set_vector(&mut self, controller: &PcieController, location: PciLocation, vector: u8) {
        self.message_data = (self.message_data & 0xFF00) | vector as u16;
        let data_offset = if self.message_control & 0x80 != 0 { 12 } else { 8 };
        controller.write16(location, self.offset + data_offset, self.message_data);
    }
}

// MSI-X Capability
pub struct MsixCapability {
    pub offset: u8,
    pub table_size: u16,
    pub table_offset: u32,
    pub table_bir: u8,
    pub pba_offset: u32,
    pub pba_bir: u8,
}

impl MsixCapability {
    pub fn parse(data: &[u8]) -> Self {
        let message_control = u16::from_le_bytes([data[2], data[3]]);
        let table_size = (message_control & 0x7FF) + 1;
        
        let table_offset_bir = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let table_bir = (table_offset_bir & 0x7) as u8;
        let table_offset = table_offset_bir & !0x7;
        
        let pba_offset_bir = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let pba_bir = (pba_offset_bir & 0x7) as u8;
        let pba_offset = pba_offset_bir & !0x7;
        
        Self {
            offset: data[0],
            table_size,
            table_offset,
            table_bir,
            pba_offset,
            pba_bir,
        }
    }
}

// Power Management Capability
pub struct PowerManagementCapability {
    pub offset: u8,
    pub capabilities: u16,
    pub control_status: u16,
}

impl PowerManagementCapability {
    pub fn parse(data: &[u8]) -> Self {
        let capabilities = u16::from_le_bytes([data[2], data[3]]);
        let control_status = u16::from_le_bytes([data[4], data[5]]);
        
        Self {
            offset: data[0],
            capabilities,
            control_status,
        }
    }
    
    pub fn set_power_state(&mut self, controller: &PcieController, location: PciLocation, state: u8) {
        self.control_status = (self.control_status & !0x03) | (state as u16 & 0x03);
        controller.write16(location, self.offset + 4, self.control_status);
    }
}

// PCIe Express Capability
pub struct PcieCapability {
    pub offset: u8,
    pub version: u8,
    pub device_type: u8,
    pub max_payload_size: u16,
    pub max_read_request: u16,
    pub link_speed: u8,
    pub link_width: u8,
}

impl PcieCapability {
    pub fn parse(data: &[u8]) -> Self {
        let pcie_cap = u16::from_le_bytes([data[2], data[3]]);
        let version = (pcie_cap & 0x0F) as u8;
        let device_type = ((pcie_cap >> 4) & 0x0F) as u8;
        
        let device_cap = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let max_payload_size = 128 << (device_cap & 0x07);
        
        let device_control = u16::from_le_bytes([data[8], data[9]]);
        let max_read_request = 128 << ((device_control >> 12) & 0x07);
        
        let link_status = u16::from_le_bytes([data[18], data[19]]);
        let link_speed = (link_status & 0x0F) as u8;
        let link_width = ((link_status >> 4) & 0x3F) as u8;
        
        Self {
            offset: data[0],
            version,
            device_type,
            max_payload_size,
            max_read_request,
            link_speed,
            link_width,
        }
    }
    
    pub fn get_device_type_string(&self) -> &'static str {
        match self.device_type {
            0x0 => "PCIe Endpoint",
            0x1 => "Legacy PCIe Endpoint",
            0x4 => "Root Port",
            0x5 => "Upstream Port",
            0x6 => "Downstream Port",
            0x7 => "PCIe-to-PCI/PCI-X Bridge",
            0x8 => "PCI/PCI-X-to-PCIe Bridge",
            0x9 => "Root Complex Integrated Endpoint",
            0xA => "Root Complex Event Collector",
            _ => "Unknown",
        }
    }
    
    pub fn get_link_speed_string(&self) -> &'static str {
        match self.link_speed {
            1 => "2.5 GT/s",
            2 => "5.0 GT/s",
            3 => "8.0 GT/s",
            4 => "16.0 GT/s",
            5 => "32.0 GT/s",
            _ => "Unknown",
        }
    }
}