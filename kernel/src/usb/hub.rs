// USB Hub Support
use super::{UsbDevice, DeviceRequest};
use alloc::vec::Vec;
use alloc::vec;
use crate::{println, serial_println};

// Hub Class Requests
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum HubRequest {
    GetStatus = 0x00,
    ClearFeature = 0x01,
    SetFeature = 0x03,
    GetDescriptor = 0x06,
    SetDescriptor = 0x07,
    ClearTtBuffer = 0x08,
    ResetTt = 0x09,
    GetTtState = 0x0A,
    StopTt = 0x0B,
}

// Hub Features
#[repr(u16)]
#[derive(Debug, Clone, Copy)]
pub enum HubFeature {
    CHubLocalPower = 0,
    CHubOverCurrent = 1,
    PortConnection = 2,
    PortEnable = 3,
    PortSuspend = 4,
    PortOverCurrent = 5,
    PortReset = 6,
    PortPower = 8,
    PortLowSpeed = 9,
    CPortConnection = 16,
    CPortEnable = 17,
    CPortSuspend = 18,
    CPortOverCurrent = 19,
    CPortReset = 20,
}

// Hub Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct HubDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub num_ports: u8,
    pub hub_characteristics: u16,
    pub power_on_to_good: u8,
    pub hub_control_current: u8,
    // Variable length fields follow
}

// Hub Status
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct HubStatus {
    pub status: u16,
    pub change: u16,
}

// Port Status
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct PortStatus {
    pub status: u16,
    pub change: u16,
}

// Port Status Bits
const PORT_CONNECTION: u16 = 1 << 0;
const PORT_ENABLE: u16 = 1 << 1;
const PORT_SUSPEND: u16 = 1 << 2;
const PORT_OVER_CURRENT: u16 = 1 << 3;
const PORT_RESET: u16 = 1 << 4;
const PORT_POWER: u16 = 1 << 8;
const PORT_LOW_SPEED: u16 = 1 << 9;
const PORT_HIGH_SPEED: u16 = 1 << 10;

// Port Change Bits
const C_PORT_CONNECTION: u16 = 1 << 0;
const C_PORT_ENABLE: u16 = 1 << 1;
const C_PORT_SUSPEND: u16 = 1 << 2;
const C_PORT_OVER_CURRENT: u16 = 1 << 3;
const C_PORT_RESET: u16 = 1 << 4;

pub struct UsbHub {
    device: UsbDevice,
    num_ports: u8,
    port_status: Vec<PortStatus>,
    children: Vec<Option<UsbDevice>>,
}

impl UsbHub {
    pub fn new(device: UsbDevice) -> Self {
        Self {
            device,
            num_ports: 0,
            port_status: Vec::new(),
            children: Vec::new(),
        }
    }
    
    pub fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("USB Hub: Initializing hub at address {}", self.device.address);
        
        // Get hub descriptor to find number of ports
        self.get_hub_descriptor()?;
        
        // Initialize port status array
        self.port_status = vec![PortStatus { status: 0, change: 0 }; self.num_ports as usize];
        self.children = vec![None; self.num_ports as usize];
        
        // Power on all ports
        for port in 1..=self.num_ports {
            self.set_port_feature(port, HubFeature::PortPower)?;
        }
        
        // Wait for power good
        for _ in 0..100000 {
            core::hint::spin_loop();
        }
        
        // Check for connected devices
        for port in 1..=self.num_ports {
            self.check_port_status(port)?;
        }
        
        Ok(())
    }
    
    fn get_hub_descriptor(&mut self) -> Result<(), &'static str> {
        // This would perform actual USB transfer
        // For now, use default values
        self.num_ports = 4; // Assume 4 port hub
        Ok(())
    }
    
    fn set_port_feature(&mut self, port: u8, feature: HubFeature) -> Result<(), &'static str> {
        serial_println!("USB Hub: Setting feature {:?} on port {}", feature, port);
        // This would perform actual USB transfer
        Ok(())
    }
    
    fn clear_port_feature(&mut self, port: u8, feature: HubFeature) -> Result<(), &'static str> {
        serial_println!("USB Hub: Clearing feature {:?} on port {}", feature, port);
        // This would perform actual USB transfer
        Ok(())
    }
    
    fn check_port_status(&mut self, port: u8) -> Result<(), &'static str> {
        // This would perform actual USB transfer to get port status
        // For now, simulate no devices connected
        self.port_status[(port - 1) as usize] = PortStatus {
            status: PORT_POWER,
            change: 0,
        };
        Ok(())
    }
    
    pub fn handle_port_change(&mut self, port: u8) -> Result<(), &'static str> {
        let status = self.port_status[(port - 1) as usize];
        
        if status.change & C_PORT_CONNECTION != 0 {
            // Connection change
            if status.status & PORT_CONNECTION != 0 {
                // Device connected
                serial_println!("USB Hub: Device connected on port {}", port);
                self.enumerate_port(port)?;
            } else {
                // Device disconnected
                serial_println!("USB Hub: Device disconnected from port {}", port);
                self.children[(port - 1) as usize] = None;
            }
            
            // Clear change bit
            self.clear_port_feature(port, HubFeature::CPortConnection)?;
        }
        
        Ok(())
    }
    
    fn enumerate_port(&mut self, port: u8) -> Result<(), &'static str> {
        // Reset port
        self.set_port_feature(port, HubFeature::PortReset)?;
        
        // Wait for reset
        for _ in 0..100000 {
            core::hint::spin_loop();
        }
        
        // Clear reset change
        self.clear_port_feature(port, HubFeature::CPortReset)?;
        
        // Check speed
        let status = self.port_status[(port - 1) as usize];
        let speed = if status.status & PORT_HIGH_SPEED != 0 {
            super::UsbSpeed::High
        } else if status.status & PORT_LOW_SPEED != 0 {
            super::UsbSpeed::Low
        } else {
            super::UsbSpeed::Full
        };
        
        // Create device
        let mut device = UsbDevice::new(0, speed);
        device.parent_hub = Some(self.device.address);
        device.port = port;
        
        // Store child device
        self.children[(port - 1) as usize] = Some(device);
        
        Ok(())
    }
}