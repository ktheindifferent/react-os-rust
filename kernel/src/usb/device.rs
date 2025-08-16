// USB Device Management
use super::{UsbDevice, UsbSpeed};
use alloc::vec::Vec;
use alloc::string::String;
use crate::{println, serial_println};

// USB Device Info for diagnostics
pub struct UsbDeviceInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub driver: Option<String>,
}

// USB Device States
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceState {
    Attached,
    Powered,
    Default,
    Address,
    Configured,
    Suspended,
}

// USB Device Manager
pub struct DeviceManager {
    devices: Vec<UsbDevice>,
    next_address: u8,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            next_address: 1,
        }
    }
    
    pub fn register_device(&mut self, mut device: UsbDevice) -> u8 {
        // Assign address
        device.address = self.next_address;
        self.next_address += 1;
        
        let address = device.address;
        self.devices.push(device);
        
        serial_println!("USB Device: Registered device at address {}", address);
        address
    }
    
    pub fn unregister_device(&mut self, address: u8) {
        self.devices.retain(|d| d.address != address);
        serial_println!("USB Device: Unregistered device at address {}", address);
    }
    
    pub fn get_device(&self, address: u8) -> Option<&UsbDevice> {
        self.devices.iter().find(|d| d.address == address)
    }
    
    pub fn get_device_mut(&mut self, address: u8) -> Option<&mut UsbDevice> {
        self.devices.iter_mut().find(|d| d.address == address)
    }
    
    pub fn list_devices(&self) -> Vec<DeviceInfo> {
        self.devices.iter().map(|d| DeviceInfo {
            address: d.address,
            vendor_id: d.device_desc.vendor_id,
            product_id: d.device_desc.product_id,
            class: d.class,
            subclass: d.subclass,
            protocol: d.protocol,
            manufacturer: d.manufacturer.clone(),
            product: d.product.clone(),
            serial: d.serial.clone(),
            speed: d.speed,
        }).collect()
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub address: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub class: u8,
    pub subclass: u8,
    pub protocol: u8,
    pub manufacturer: String,
    pub product: String,
    pub serial: String,
    pub speed: UsbSpeed,
}

impl DeviceInfo {
    pub fn class_name(&self) -> &str {
        match self.class {
            0x01 => "Audio",
            0x02 => "Communications",
            0x03 => "HID",
            0x05 => "Physical",
            0x06 => "Image",
            0x07 => "Printer",
            0x08 => "Mass Storage",
            0x09 => "Hub",
            0x0A => "CDC Data",
            0x0B => "Smart Card",
            0x0D => "Content Security",
            0x0E => "Video",
            0x0F => "Healthcare",
            0xDC => "Diagnostic",
            0xE0 => "Wireless",
            0xEF => "Miscellaneous",
            0xFE => "Application Specific",
            0xFF => "Vendor Specific",
            _ => "Unknown",
        }
    }
    
    pub fn speed_name(&self) -> &str {
        match self.speed {
            UsbSpeed::Low => "Low (1.5 Mbps)",
            UsbSpeed::Full => "Full (12 Mbps)",
            UsbSpeed::High => "High (480 Mbps)",
            UsbSpeed::Super => "Super (5 Gbps)",
            UsbSpeed::SuperPlus => "SuperSpeed+ (10 Gbps)",
        }
    }
}

// USB String handling
pub fn decode_usb_string(data: &[u8]) -> String {
    if data.len() < 2 {
        return String::new();
    }
    
    let length = data[0] as usize;
    let descriptor_type = data[1];
    
    if descriptor_type != 0x03 || length > data.len() {
        return String::new();
    }
    
    // Parse UTF-16LE string
    let mut result = String::new();
    for i in (2..length).step_by(2) {
        if i + 1 < data.len() {
            let ch = u16::from_le_bytes([data[i], data[i + 1]]);
            if let Some(c) = char::from_u32(ch as u32) {
                result.push(c);
            }
        }
    }
    
    result
}