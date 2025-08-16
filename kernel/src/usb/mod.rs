// USB (Universal Serial Bus) Implementation
pub mod hid;
pub mod uhci;
pub mod ehci;
pub mod xhci;
pub mod hub;
pub mod device;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::{println, serial_println};

// USB Speed Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsbSpeed {
    Low,        // 1.5 Mbps (USB 1.0)
    Full,       // 12 Mbps (USB 1.1)
    High,       // 480 Mbps (USB 2.0)
    Super,      // 5 Gbps (USB 3.0)
    SuperPlus,  // 10 Gbps (USB 3.1)
}

// USB Transfer Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransferType {
    Control,
    Isochronous,
    Bulk,
    Interrupt,
}

// USB Descriptor Types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DescriptorType {
    Device = 0x01,
    Configuration = 0x02,
    String = 0x03,
    Interface = 0x04,
    Endpoint = 0x05,
    DeviceQualifier = 0x06,
    OtherSpeedConfig = 0x07,
    InterfacePower = 0x08,
    Hid = 0x21,
    HidReport = 0x22,
    HidPhysical = 0x23,
}

// USB Request Types
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum RequestType {
    GetStatus = 0x00,
    ClearFeature = 0x01,
    SetFeature = 0x03,
    SetAddress = 0x05,
    GetDescriptor = 0x06,
    SetDescriptor = 0x07,
    GetConfiguration = 0x08,
    SetConfiguration = 0x09,
    GetInterface = 0x0A,
    SetInterface = 0x0B,
    SynchFrame = 0x0C,
}

// USB Standard Device Request
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DeviceRequest {
    pub request_type: u8,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub length: u16,
}

// USB Device Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DeviceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub usb_version: u16,
    pub device_class: u8,
    pub device_subclass: u8,
    pub device_protocol: u8,
    pub max_packet_size: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_version: u16,
    pub manufacturer_index: u8,
    pub product_index: u8,
    pub serial_index: u8,
    pub num_configurations: u8,
}

// USB Configuration Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ConfigurationDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub total_length: u16,
    pub num_interfaces: u8,
    pub configuration_value: u8,
    pub configuration_index: u8,
    pub attributes: u8,
    pub max_power: u8,
}

// USB Interface Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct InterfaceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub interface_number: u8,
    pub alternate_setting: u8,
    pub num_endpoints: u8,
    pub interface_class: u8,
    pub interface_subclass: u8,
    pub interface_protocol: u8,
    pub interface_index: u8,
}

// USB Endpoint Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct EndpointDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub endpoint_address: u8,
    pub attributes: u8,
    pub max_packet_size: u16,
    pub interval: u8,
}

// USB Device Classes
pub const USB_CLASS_AUDIO: u8 = 0x01;
pub const USB_CLASS_CDC: u8 = 0x02;
pub const USB_CLASS_HID: u8 = 0x03;
pub const USB_CLASS_PHYSICAL: u8 = 0x05;
pub const USB_CLASS_IMAGE: u8 = 0x06;
pub const USB_CLASS_PRINTER: u8 = 0x07;
pub const USB_CLASS_MASS_STORAGE: u8 = 0x08;
pub const USB_CLASS_HUB: u8 = 0x09;
pub const USB_CLASS_CDC_DATA: u8 = 0x0A;
pub const USB_CLASS_SMART_CARD: u8 = 0x0B;
pub const USB_CLASS_VIDEO: u8 = 0x0E;
pub const USB_CLASS_HEALTHCARE: u8 = 0x0F;
pub const USB_CLASS_DIAGNOSTIC: u8 = 0xDC;
pub const USB_CLASS_WIRELESS: u8 = 0xE0;
pub const USB_CLASS_MISC: u8 = 0xEF;
pub const USB_CLASS_VENDOR: u8 = 0xFF;

// USB Controller Interface
pub trait UsbController: Send + Sync {
    fn init(&mut self) -> Result<(), &'static str>;
    fn reset(&mut self) -> Result<(), &'static str>;
    fn enumerate_devices(&mut self) -> Vec<UsbDevice>;
    fn control_transfer(&mut self, device: &UsbDevice, request: &DeviceRequest, data: Option<&mut [u8]>) -> Result<usize, &'static str>;
    fn bulk_transfer(&mut self, device: &UsbDevice, endpoint: u8, data: &mut [u8], is_write: bool) -> Result<usize, &'static str>;
    fn interrupt_transfer(&mut self, device: &UsbDevice, endpoint: u8, data: &mut [u8]) -> Result<usize, &'static str>;
    fn get_controller_type(&self) -> ControllerType;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControllerType {
    Uhci,  // USB 1.0/1.1
    Ohci,  // USB 1.0/1.1
    Ehci,  // USB 2.0
    Xhci,  // USB 3.0+
}

// USB Device
#[derive(Debug, Clone)]
pub struct UsbDevice {
    pub address: u8,
    pub speed: UsbSpeed,
    pub device_desc: DeviceDescriptor,
    pub config_desc: Option<ConfigurationDescriptor>,
    pub manufacturer: String,
    pub product: String,
    pub serial: String,
    pub class: u8,
    pub subclass: u8,
    pub protocol: u8,
    pub endpoints: Vec<EndpointInfo>,
    pub parent_hub: Option<u8>,
    pub port: u8,
}

#[derive(Debug, Clone)]
pub struct EndpointInfo {
    pub address: u8,
    pub transfer_type: TransferType,
    pub max_packet_size: u16,
    pub interval: u8,
}

impl UsbDevice {
    pub fn new(address: u8, speed: UsbSpeed) -> Self {
        Self {
            address,
            speed,
            device_desc: unsafe { core::mem::zeroed() },
            config_desc: None,
            manufacturer: String::new(),
            product: String::new(),
            serial: String::new(),
            class: 0,
            subclass: 0,
            protocol: 0,
            endpoints: Vec::new(),
            parent_hub: None,
            port: 0,
        }
    }
    
    pub fn is_hid(&self) -> bool {
        self.class == USB_CLASS_HID
    }
    
    pub fn is_mass_storage(&self) -> bool {
        self.class == USB_CLASS_MASS_STORAGE
    }
    
    pub fn is_hub(&self) -> bool {
        self.class == USB_CLASS_HUB
    }
}

// USB Manager
pub struct UsbManager {
    controllers: Vec<Box<dyn UsbController>>,
    devices: Vec<UsbDevice>,
    next_address: u8,
}

impl UsbManager {
    pub fn new() -> Self {
        Self {
            controllers: Vec::new(),
            devices: Vec::new(),
            next_address: 1,
        }
    }
    
    pub fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("USB: Initializing USB subsystem");
        
        // Detect and initialize USB controllers
        self.detect_controllers()?;
        
        // Initialize each controller
        for controller in &mut self.controllers {
            controller.init()?;
        }
        
        // Enumerate devices on each controller
        self.enumerate_all_devices()?;
        
        serial_println!("USB: Found {} devices", self.devices.len());
        
        // Initialize HID devices
        for device in &self.devices {
            if device.is_hid() {
                hid::init_hid_device(device)?;
            }
        }
        
        Ok(())
    }
    
    fn detect_controllers(&mut self) -> Result<(), &'static str> {
        // Check for UHCI controllers (USB 1.0/1.1)
        if let Some(uhci) = uhci::detect_uhci_controller() {
            self.controllers.push(Box::new(uhci));
            serial_println!("USB: Found UHCI controller");
        }
        
        // Check for EHCI controllers (USB 2.0)
        if let Some(ehci) = ehci::detect_ehci_controller() {
            self.controllers.push(Box::new(ehci));
            serial_println!("USB: Found EHCI controller");
        }
        
        // Check for XHCI controllers (USB 3.0+)
        if let Some(xhci) = xhci::detect_xhci_controller() {
            self.controllers.push(Box::new(xhci));
            serial_println!("USB: Found XHCI controller");
        }
        
        if self.controllers.is_empty() {
            return Err("No USB controllers found");
        }
        
        Ok(())
    }
    
    fn enumerate_all_devices(&mut self) -> Result<(), &'static str> {
        let mut all_devices = Vec::new();
        
        for i in 0..self.controllers.len() {
            let devices = self.controllers[i].enumerate_devices();
            
            for mut device in devices {
                // Assign address
                device.address = self.next_address;
                self.next_address += 1;
                
                // Get device descriptor
                self.get_device_descriptor(i, &mut device)?;
                
                // Get configuration descriptor
                self.get_configuration_descriptor(i, &mut device)?;
                
                // Get string descriptors
                self.get_string_descriptors(i, &mut device)?;
                
                // Set configuration
                self.set_configuration(i, &mut device)?;
                
                all_devices.push(device);
            }
        }
        
        self.devices = all_devices;
        Ok(())
    }
    
    fn get_device_descriptor(&mut self, controller_idx: usize, device: &mut UsbDevice) -> Result<(), &'static str> {
        let request = DeviceRequest {
            request_type: 0x80,  // Device to host, standard, device
            request: RequestType::GetDescriptor as u8,
            value: (DescriptorType::Device as u16) << 8,
            index: 0,
            length: core::mem::size_of::<DeviceDescriptor>() as u16,
        };
        
        let mut buffer = [0u8; 18];
        self.controllers[controller_idx].control_transfer(device, &request, Some(&mut buffer))?;
        
        unsafe {
            device.device_desc = *(buffer.as_ptr() as *const DeviceDescriptor);
        }
        
        device.class = device.device_desc.device_class;
        device.subclass = device.device_desc.device_subclass;
        device.protocol = device.device_desc.device_protocol;
        
        Ok(())
    }
    
    fn get_configuration_descriptor(&mut self, controller_idx: usize, device: &mut UsbDevice) -> Result<(), &'static str> {
        // First get just the configuration descriptor header
        let request = DeviceRequest {
            request_type: 0x80,
            request: RequestType::GetDescriptor as u8,
            value: (DescriptorType::Configuration as u16) << 8,
            index: 0,
            length: core::mem::size_of::<ConfigurationDescriptor>() as u16,
        };
        
        let mut buffer = [0u8; 256];
        let len = self.controllers[controller_idx].control_transfer(device, &request, Some(&mut buffer))?;
        
        if len >= core::mem::size_of::<ConfigurationDescriptor>() {
            unsafe {
                device.config_desc = Some(*(buffer.as_ptr() as *const ConfigurationDescriptor));
            }
            
            // Parse interfaces and endpoints
            self.parse_configuration(&buffer[..len], device)?;
        }
        
        Ok(())
    }
    
    fn parse_configuration(&mut self, data: &[u8], device: &mut UsbDevice) -> Result<(), &'static str> {
        let mut offset = core::mem::size_of::<ConfigurationDescriptor>();
        
        while offset < data.len() {
            let length = data[offset] as usize;
            let desc_type = data[offset + 1];
            
            if length == 0 || offset + length > data.len() {
                break;
            }
            
            match desc_type {
                0x04 => {
                    // Interface descriptor
                    if length >= core::mem::size_of::<InterfaceDescriptor>() {
                        let interface = unsafe {
                            *(data[offset..].as_ptr() as *const InterfaceDescriptor)
                        };
                        
                        // Update device class if not set
                        if device.class == 0 {
                            device.class = interface.interface_class;
                            device.subclass = interface.interface_subclass;
                            device.protocol = interface.interface_protocol;
                        }
                    }
                }
                0x05 => {
                    // Endpoint descriptor
                    if length >= core::mem::size_of::<EndpointDescriptor>() {
                        let endpoint = unsafe {
                            *(data[offset..].as_ptr() as *const EndpointDescriptor)
                        };
                        
                        let transfer_type = match endpoint.attributes & 0x03 {
                            0 => TransferType::Control,
                            1 => TransferType::Isochronous,
                            2 => TransferType::Bulk,
                            3 => TransferType::Interrupt,
                            _ => TransferType::Control,
                        };
                        
                        device.endpoints.push(EndpointInfo {
                            address: endpoint.endpoint_address,
                            transfer_type,
                            max_packet_size: endpoint.max_packet_size,
                            interval: endpoint.interval,
                        });
                    }
                }
                _ => {}
            }
            
            offset += length;
        }
        
        Ok(())
    }
    
    fn get_string_descriptors(&mut self, controller_idx: usize, device: &mut UsbDevice) -> Result<(), &'static str> {
        // Get manufacturer string
        if device.device_desc.manufacturer_index != 0 {
            device.manufacturer = self.get_string_descriptor(controller_idx, device, device.device_desc.manufacturer_index)?;
        }
        
        // Get product string
        if device.device_desc.product_index != 0 {
            device.product = self.get_string_descriptor(controller_idx, device, device.device_desc.product_index)?;
        }
        
        // Get serial string
        if device.device_desc.serial_index != 0 {
            device.serial = self.get_string_descriptor(controller_idx, device, device.device_desc.serial_index)?;
        }
        
        Ok(())
    }
    
    fn get_string_descriptor(&mut self, controller_idx: usize, device: &UsbDevice, index: u8) -> Result<String, &'static str> {
        let request = DeviceRequest {
            request_type: 0x80,
            request: RequestType::GetDescriptor as u8,
            value: ((DescriptorType::String as u16) << 8) | index as u16,
            index: 0x0409,  // English (US)
            length: 256,
        };
        
        let mut buffer = [0u8; 256];
        let len = self.controllers[controller_idx].control_transfer(device, &request, Some(&mut buffer))?;
        
        if len < 2 {
            return Ok(String::new());
        }
        
        // Parse UTF-16 string
        let string_len = (buffer[0] as usize - 2) / 2;
        let mut result = String::new();
        
        for i in 0..string_len {
            let offset = 2 + i * 2;
            if offset + 1 < len {
                let ch = u16::from_le_bytes([buffer[offset], buffer[offset + 1]]);
                if let Some(c) = char::from_u32(ch as u32) {
                    result.push(c);
                }
            }
        }
        
        Ok(result)
    }
    
    fn set_configuration(&mut self, controller_idx: usize, device: &mut UsbDevice) -> Result<(), &'static str> {
        if let Some(config) = device.config_desc {
            let request = DeviceRequest {
                request_type: 0x00,  // Host to device, standard, device
                request: RequestType::SetConfiguration as u8,
                value: config.configuration_value as u16,
                index: 0,
                length: 0,
            };
            
            self.controllers[controller_idx].control_transfer(device, &request, None)?;
        }
        
        Ok(())
    }
    
    pub fn get_devices(&self) -> &[UsbDevice] {
        &self.devices
    }
    
    pub fn get_hid_devices(&self) -> Vec<&UsbDevice> {
        self.devices.iter().filter(|d| d.is_hid()).collect()
    }
}

lazy_static! {
    pub static ref USB_MANAGER: Mutex<UsbManager> = Mutex::new(UsbManager::new());
}

pub fn init() {
    USB_MANAGER.lock().init().unwrap_or_else(|e| {
        serial_println!("USB: Failed to initialize: {}", e);
    });
}

// Helper functions for monitoring/diagnostics module
pub fn enumerate_devices() -> Option<Vec<device::UsbDeviceInfo>> {
    let manager = USB_MANAGER.lock();
    let devices = manager.get_devices();
    
    if devices.is_empty() {
        return None;
    }
    
    let mut device_infos = Vec::new();
    for device in devices {
        device_infos.push(device::UsbDeviceInfo {
            vendor_id: device.device_descriptor.vendor_id,
            product_id: device.device_descriptor.product_id,
            driver: None,
        });
    }
    
    Some(device_infos)
}