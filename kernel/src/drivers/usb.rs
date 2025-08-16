// USB Subsystem Implementation
use super::*;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::boxed::Box;
use alloc::{vec, format};
use crate::nt::NtStatus;

// USB Standard Descriptor Types
pub const USB_DEVICE_DESCRIPTOR_TYPE: u8 = 1;
pub const USB_CONFIGURATION_DESCRIPTOR_TYPE: u8 = 2;
pub const USB_STRING_DESCRIPTOR_TYPE: u8 = 3;
pub const USB_INTERFACE_DESCRIPTOR_TYPE: u8 = 4;
pub const USB_ENDPOINT_DESCRIPTOR_TYPE: u8 = 5;

// USB Device Classes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsbDeviceClass {
    UseInterfaceDescriptor = 0x00,
    Audio = 0x01,
    Communication = 0x02,
    HumanInterface = 0x03,
    Monitor = 0x04,
    PhysicalInterface = 0x05,
    Power = 0x06,
    Printer = 0x07,
    Storage = 0x08,
    Hub = 0x09,
    Data = 0x0A,
    SmartCard = 0x0B,
    ContentSecurity = 0x0D,
    Video = 0x0E,
    PersonalHealthcare = 0x0F,
    AudioVideo = 0x10,
    Diagnostic = 0xDC,
    Wireless = 0xE0,
    Miscellaneous = 0xEF,
    ApplicationSpecific = 0xFE,
    VendorSpecific = 0xFF,
}

// USB Speeds
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsbSpeed {
    Unknown,
    Low,     // 1.5 Mbps
    Full,    // 12 Mbps
    High,    // 480 Mbps
    Super,   // 5 Gbps
    SuperPlus, // 10 Gbps
}

// USB Transfer Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsbTransferType {
    Control,
    Isochronous,
    Bulk,
    Interrupt,
}

// USB Endpoint Direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsbDirection {
    Out = 0,
    In = 1,
}

// USB Standard Requests
#[derive(Debug, Clone, Copy)]
pub enum UsbRequest {
    GetStatus = 0,
    ClearFeature = 1,
    SetFeature = 3,
    SetAddress = 5,
    GetDescriptor = 6,
    SetDescriptor = 7,
    GetConfiguration = 8,
    SetConfiguration = 9,
    GetInterface = 10,
    SetInterface = 11,
    SynchFrame = 12,
}

// USB Device States
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsbDeviceState {
    Attached,
    Powered,
    Default,
    Address,
    Configured,
    Suspended,
}

#[derive(Debug, Clone)]
pub struct UsbDeviceDescriptor {
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
    pub manufacturer: u8,
    pub product: u8,
    pub serial_number: u8,
    pub num_configurations: u8,
}

#[derive(Debug, Clone)]
pub struct UsbConfigurationDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub total_length: u16,
    pub num_interfaces: u8,
    pub configuration_value: u8,
    pub configuration: u8,
    pub attributes: u8,
    pub max_power: u8,
}

#[derive(Debug, Clone)]
pub struct UsbInterfaceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub interface_number: u8,
    pub alternate_setting: u8,
    pub num_endpoints: u8,
    pub interface_class: u8,
    pub interface_subclass: u8,
    pub interface_protocol: u8,
    pub interface: u8,
}

#[derive(Debug, Clone)]
pub struct UsbEndpointDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub endpoint_address: u8,
    pub attributes: u8,
    pub max_packet_size: u16,
    pub interval: u8,
}

#[derive(Debug, Clone)]
pub struct UsbStringDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub string: Vec<u16>, // UTF-16LE
}

#[derive(Debug, Clone)]
pub struct UsbSetupPacket {
    pub request_type: u8,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub length: u16,
}

#[derive(Debug, Clone)]
pub struct UsbTransferRequest {
    pub setup_packet: Option<UsbSetupPacket>,
    pub buffer: Option<Vec<u8>>,
    pub length: u32,
    pub transfer_type: UsbTransferType,
    pub endpoint: u8,
    pub direction: UsbDirection,
    pub timeout: u32,
    pub callback: Option<fn(NtStatus, u32)>,
}

#[derive(Debug, Clone)]
pub struct UsbDevice {
    pub device_address: u8,
    pub speed: UsbSpeed,
    pub state: UsbDeviceState,
    pub descriptor: UsbDeviceDescriptor,
    pub configurations: Vec<UsbConfiguration>,
    pub current_configuration: u8,
    pub hub_address: u8,
    pub port_number: u8,
    pub driver_handle: Option<Handle>,
    pub device_handle: Option<Handle>,
    pub strings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UsbConfiguration {
    pub descriptor: UsbConfigurationDescriptor,
    pub interfaces: Vec<UsbInterface>,
}

#[derive(Debug, Clone)]
pub struct UsbInterface {
    pub descriptor: UsbInterfaceDescriptor,
    pub endpoints: Vec<UsbEndpoint>,
    pub driver_handle: Option<Handle>,
}

#[derive(Debug, Clone)]
pub struct UsbEndpoint {
    pub descriptor: UsbEndpointDescriptor,
    pub transfer_type: UsbTransferType,
    pub direction: UsbDirection,
    pub max_packet_size: u16,
    pub interval: u8,
}

#[derive(Debug, Clone)]
pub struct UsbHub {
    pub device: UsbDevice,
    pub num_ports: u8,
    pub port_status: Vec<UsbPortStatus>,
    pub power_on_to_power_good: u8,
    pub hub_current: u8,
}

#[derive(Debug, Clone)]
pub struct UsbPortStatus {
    pub connected: bool,
    pub enabled: bool,
    pub suspended: bool,
    pub over_current: bool,
    pub reset: bool,
    pub power: bool,
    pub low_speed: bool,
    pub high_speed: bool,
    pub test_mode: bool,
    pub indicator: u8,
}

// USB Host Controller Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsbHostControllerType {
    UHCI, // Universal Host Controller Interface (USB 1.1)
    OHCI, // Open Host Controller Interface (USB 1.1)
    EHCI, // Enhanced Host Controller Interface (USB 2.0)
    XHCI, // eXtensible Host Controller Interface (USB 3.0+)
}

pub trait UsbHostController {
    fn initialize(&mut self) -> NtStatus;
    fn shutdown(&mut self) -> NtStatus;
    fn reset(&mut self) -> NtStatus;
    fn suspend(&mut self) -> NtStatus;
    fn resume(&mut self) -> NtStatus;
    
    fn get_controller_type(&self) -> UsbHostControllerType;
    fn get_num_ports(&self) -> u8;
    fn get_port_status(&self, port: u8) -> UsbPortStatus;
    fn set_port_feature(&mut self, port: u8, feature: u16) -> NtStatus;
    fn clear_port_feature(&mut self, port: u8, feature: u16) -> NtStatus;
    
    fn submit_transfer(&mut self, device_address: u8, request: UsbTransferRequest) -> NtStatus;
    fn cancel_transfer(&mut self, transfer_id: u32) -> NtStatus;
    
    fn enumerate_devices(&mut self) -> NtStatus;
    fn attach_device(&mut self, port: u8) -> NtStatus;
    fn detach_device(&mut self, port: u8) -> NtStatus;
}

// EHCI Host Controller Implementation
pub struct EhciController {
    pub base_address: u64,
    pub operational_registers: u64,
    pub capability_registers: u64,
    pub num_ports: u8,
    pub devices: Vec<UsbDevice>,
    pub hubs: Vec<UsbHub>,
    pub next_address: u8,
    pub periodic_list: Vec<u32>,
    pub async_list: Vec<u32>,
}

impl EhciController {
    pub fn new(base_address: u64) -> Self {
        Self {
            base_address,
            operational_registers: base_address + 0x10, // Default offset
            capability_registers: base_address,
            num_ports: 0,
            devices: Vec::new(),
            hubs: Vec::new(),
            next_address: 1,
            periodic_list: Vec::new(),
            async_list: Vec::new(),
        }
    }

    fn read_capability_register(&self, offset: u8) -> u32 {
        // In a real implementation, this would read from memory-mapped registers
        0
    }

    fn read_operational_register(&self, offset: u8) -> u32 {
        // In a real implementation, this would read from memory-mapped registers
        0
    }

    fn write_operational_register(&self, offset: u8, value: u32) {
        // In a real implementation, this would write to memory-mapped registers
    }

    fn setup_periodic_schedule(&mut self) -> NtStatus {
        crate::println!("EHCI: Setting up periodic schedule");
        // Initialize periodic frame list
        self.periodic_list.resize(1024, 0);
        NtStatus::Success
    }

    fn setup_async_schedule(&mut self) -> NtStatus {
        crate::println!("EHCI: Setting up asynchronous schedule");
        // Initialize async queue heads
        NtStatus::Success
    }

    fn reset_controller(&mut self) -> NtStatus {
        crate::println!("EHCI: Resetting controller");
        // Reset the EHCI controller
        NtStatus::Success
    }

    fn start_controller(&mut self) -> NtStatus {
        crate::println!("EHCI: Starting controller");
        // Start the EHCI controller
        NtStatus::Success
    }

    fn detect_ports(&mut self) -> NtStatus {
        // Read number of ports from capability registers
        self.num_ports = 4; // Default for simulation
        crate::println!("EHCI: Detected {} ports", self.num_ports);
        NtStatus::Success
    }
}

impl UsbHostController for EhciController {
    fn initialize(&mut self) -> NtStatus {
        crate::println!("EHCI: Initializing Enhanced Host Controller Interface");

        // Reset controller
        if let status @ NtStatus::Success = self.reset_controller() {
            // Detect number of ports
            if let status @ NtStatus::Success = self.detect_ports() {
                // Setup schedules
                if let status @ NtStatus::Success = self.setup_periodic_schedule() {
                    if let status @ NtStatus::Success = self.setup_async_schedule() {
                        // Start controller
                        return self.start_controller();
                    }
                }
            }
        }

        NtStatus::Unsuccessful
    }

    fn shutdown(&mut self) -> NtStatus {
        crate::println!("EHCI: Shutting down controller");
        // Stop controller and disable interrupts
        NtStatus::Success
    }

    fn reset(&mut self) -> NtStatus {
        self.reset_controller()
    }

    fn suspend(&mut self) -> NtStatus {
        crate::println!("EHCI: Suspending controller");
        NtStatus::Success
    }

    fn resume(&mut self) -> NtStatus {
        crate::println!("EHCI: Resuming controller");
        NtStatus::Success
    }

    fn get_controller_type(&self) -> UsbHostControllerType {
        UsbHostControllerType::EHCI
    }

    fn get_num_ports(&self) -> u8 {
        self.num_ports
    }

    fn get_port_status(&self, port: u8) -> UsbPortStatus {
        // Read port status from registers
        UsbPortStatus {
            connected: true,  // Simulated
            enabled: true,
            suspended: false,
            over_current: false,
            reset: false,
            power: true,
            low_speed: false,
            high_speed: true,
            test_mode: false,
            indicator: 0,
        }
    }

    fn set_port_feature(&mut self, port: u8, feature: u16) -> NtStatus {
        crate::println!("EHCI: Setting port {} feature {}", port, feature);
        // Set port feature in registers
        NtStatus::Success
    }

    fn clear_port_feature(&mut self, port: u8, feature: u16) -> NtStatus {
        crate::println!("EHCI: Clearing port {} feature {}", port, feature);
        // Clear port feature in registers
        NtStatus::Success
    }

    fn submit_transfer(&mut self, device_address: u8, request: UsbTransferRequest) -> NtStatus {
        crate::println!("EHCI: Submitting transfer to device {}", device_address);
        
        match request.transfer_type {
            UsbTransferType::Control => {
                // Handle control transfer
                self.submit_control_transfer(device_address, request)
            }
            UsbTransferType::Bulk => {
                // Handle bulk transfer
                self.submit_bulk_transfer(device_address, request)
            }
            UsbTransferType::Interrupt => {
                // Handle interrupt transfer
                self.submit_interrupt_transfer(device_address, request)
            }
            UsbTransferType::Isochronous => {
                // Handle isochronous transfer
                self.submit_isochronous_transfer(device_address, request)
            }
        }
    }

    fn cancel_transfer(&mut self, transfer_id: u32) -> NtStatus {
        crate::println!("EHCI: Cancelling transfer {}", transfer_id);
        // Cancel transfer by removing from schedules
        NtStatus::Success
    }

    fn enumerate_devices(&mut self) -> NtStatus {
        crate::println!("EHCI: Enumerating USB devices");

        for port in 1..=self.num_ports {
            let status = self.get_port_status(port);
            if status.connected {
                crate::println!("EHCI: Device connected on port {}", port);
                let attach_result = self.attach_device(port);
                if attach_result != NtStatus::Success {
                    crate::println!("EHCI: Failed to attach device on port {}: {:?}", port, attach_result);
                }
            }
        }

        NtStatus::Success
    }

    fn attach_device(&mut self, port: u8) -> NtStatus {
        crate::println!("EHCI: Attaching device on port {}", port);

        // Reset port
        self.set_port_feature(port, 4); // PORT_RESET
        
        // Wait for reset completion (simulated)
        
        self.clear_port_feature(port, 4); // Clear PORT_RESET

        // Determine device speed
        let status = self.get_port_status(port);
        let speed = if status.high_speed {
            UsbSpeed::High
        } else if status.low_speed {
            UsbSpeed::Low
        } else {
            UsbSpeed::Full
        };

        // Get device descriptor
        let device_descriptor = match self.get_device_descriptor(0) { // Default address
            Ok(desc) => desc,
            Err(status) => return status,
        };

        // Assign new address
        let new_address = self.next_address;
        self.next_address += 1;

        let status = self.set_device_address(0, new_address);
        if status != NtStatus::Success {
            return status;
        }

        // Get full device descriptor with new address
        let full_descriptor = match self.get_device_descriptor(new_address) {
            Ok(desc) => desc,
            Err(status) => return status,
        };

        // Get configuration descriptor
        let config_descriptor = match self.get_configuration_descriptor(new_address, 0) {
            Ok(desc) => desc,
            Err(status) => return status,
        };

        // Create device object
        let mut device = UsbDevice {
            device_address: new_address,
            speed,
            state: UsbDeviceState::Address,
            descriptor: full_descriptor,
            configurations: vec![UsbConfiguration {
                descriptor: config_descriptor,
                interfaces: Vec::new(),
            }],
            current_configuration: 0,
            hub_address: 0,
            port_number: port,
            driver_handle: None,
            device_handle: None,
            strings: Vec::new(),
        };

        // Get string descriptors
        let status = self.get_string_descriptors(&mut device);
        if status != NtStatus::Success {
            return status;
        }

        // Set configuration
        let status = self.set_configuration(new_address, 1);
        if status != NtStatus::Success {
            return status;
        }
        device.state = UsbDeviceState::Configured;

        // Load appropriate driver
        self.load_device_driver(&device);

        self.devices.push(device);

        crate::println!("EHCI: Device attached successfully on port {} with address {}", port, new_address);
        NtStatus::Success
    }

    fn detach_device(&mut self, port: u8) -> NtStatus {
        crate::println!("EHCI: Detaching device on port {}", port);

        // Find and remove device
        self.devices.retain(|device| device.port_number != port);

        NtStatus::Success
    }
}

impl EhciController {
    fn submit_control_transfer(&mut self, device_address: u8, request: UsbTransferRequest) -> NtStatus {
        crate::println!("EHCI: Submitting control transfer to device {}", device_address);
        
        if let Some(setup_packet) = request.setup_packet {
            crate::println!("EHCI: Setup packet - Type: 0x{:02X}, Request: 0x{:02X}, Value: 0x{:04X}",
                setup_packet.request_type, setup_packet.request, setup_packet.value);
        }

        // In a real implementation, this would:
        // 1. Create Queue Transfer Descriptors (QTDs)
        // 2. Link them to Queue Heads (QHs)
        // 3. Add to async schedule
        // 4. Ring doorbell
        
        NtStatus::Success
    }

    fn submit_bulk_transfer(&mut self, device_address: u8, request: UsbTransferRequest) -> NtStatus {
        crate::println!("EHCI: Submitting bulk transfer to device {}", device_address);
        // Similar to control transfers but for bulk endpoints
        NtStatus::Success
    }

    fn submit_interrupt_transfer(&mut self, device_address: u8, request: UsbTransferRequest) -> NtStatus {
        crate::println!("EHCI: Submitting interrupt transfer to device {}", device_address);
        // Add to periodic schedule
        NtStatus::Success
    }

    fn submit_isochronous_transfer(&mut self, device_address: u8, request: UsbTransferRequest) -> NtStatus {
        crate::println!("EHCI: Submitting isochronous transfer to device {}", device_address);
        // Add to periodic schedule with timing constraints
        NtStatus::Success
    }

    fn get_device_descriptor(&mut self, device_address: u8) -> Result<UsbDeviceDescriptor, NtStatus> {
        crate::println!("EHCI: Getting device descriptor from device {}", device_address);

        let setup_packet = UsbSetupPacket {
            request_type: 0x80, // Device to host, standard, device
            request: UsbRequest::GetDescriptor as u8,
            value: (USB_DEVICE_DESCRIPTOR_TYPE as u16) << 8,
            index: 0,
            length: 18, // Size of device descriptor
        };

        let request = UsbTransferRequest {
            setup_packet: Some(setup_packet),
            buffer: Some(vec![0; 18]),
            length: 18,
            transfer_type: UsbTransferType::Control,
            endpoint: 0,
            direction: UsbDirection::In,
            timeout: 5000,
            callback: None,
        };

        let status = self.submit_transfer(device_address, request);
        if status != NtStatus::Success {
            return Err(status);
        }

        // Parse the response (simulated)
        Ok(UsbDeviceDescriptor {
            length: 18,
            descriptor_type: USB_DEVICE_DESCRIPTOR_TYPE,
            usb_version: 0x0200, // USB 2.0
            device_class: 0,
            device_subclass: 0,
            device_protocol: 0,
            max_packet_size: 64,
            vendor_id: 0x1234,
            product_id: 0x5678,
            device_version: 0x0100,
            manufacturer: 1,
            product: 2,
            serial_number: 3,
            num_configurations: 1,
        })
    }

    fn get_configuration_descriptor(&mut self, device_address: u8, config_index: u8) -> Result<UsbConfigurationDescriptor, NtStatus> {
        crate::println!("EHCI: Getting configuration descriptor {} from device {}", config_index, device_address);

        let setup_packet = UsbSetupPacket {
            request_type: 0x80,
            request: UsbRequest::GetDescriptor as u8,
            value: ((USB_CONFIGURATION_DESCRIPTOR_TYPE as u16) << 8) | (config_index as u16),
            index: 0,
            length: 9, // Size of configuration descriptor
        };

        let request = UsbTransferRequest {
            setup_packet: Some(setup_packet),
            buffer: Some(vec![0; 9]),
            length: 9,
            transfer_type: UsbTransferType::Control,
            endpoint: 0,
            direction: UsbDirection::In,
            timeout: 5000,
            callback: None,
        };

        let status = self.submit_transfer(device_address, request);
        if status != NtStatus::Success {
            return Err(status);
        }

        // Parse the response (simulated)
        Ok(UsbConfigurationDescriptor {
            length: 9,
            descriptor_type: USB_CONFIGURATION_DESCRIPTOR_TYPE,
            total_length: 32, // Total length including interfaces and endpoints
            num_interfaces: 1,
            configuration_value: 1,
            configuration: 0,
            attributes: 0x80, // Bus powered
            max_power: 50,    // 100 mA
        })
    }

    fn set_device_address(&mut self, current_address: u8, new_address: u8) -> NtStatus {
        crate::println!("EHCI: Setting device address from {} to {}", current_address, new_address);

        let setup_packet = UsbSetupPacket {
            request_type: 0x00, // Host to device, standard, device
            request: UsbRequest::SetAddress as u8,
            value: new_address as u16,
            index: 0,
            length: 0,
        };

        let request = UsbTransferRequest {
            setup_packet: Some(setup_packet),
            buffer: None,
            length: 0,
            transfer_type: UsbTransferType::Control,
            endpoint: 0,
            direction: UsbDirection::Out,
            timeout: 5000,
            callback: None,
        };

        self.submit_transfer(current_address, request)
    }

    fn set_configuration(&mut self, device_address: u8, config_value: u8) -> NtStatus {
        crate::println!("EHCI: Setting configuration {} for device {}", config_value, device_address);

        let setup_packet = UsbSetupPacket {
            request_type: 0x00,
            request: UsbRequest::SetConfiguration as u8,
            value: config_value as u16,
            index: 0,
            length: 0,
        };

        let request = UsbTransferRequest {
            setup_packet: Some(setup_packet),
            buffer: None,
            length: 0,
            transfer_type: UsbTransferType::Control,
            endpoint: 0,
            direction: UsbDirection::Out,
            timeout: 5000,
            callback: None,
        };

        self.submit_transfer(device_address, request)
    }

    fn get_string_descriptors(&mut self, device: &mut UsbDevice) -> NtStatus {
        // Get language IDs first
        let setup_packet = UsbSetupPacket {
            request_type: 0x80,
            request: UsbRequest::GetDescriptor as u8,
            value: ((USB_STRING_DESCRIPTOR_TYPE as u16) << 8) | 0,
            index: 0,
            length: 4,
        };

        let request = UsbTransferRequest {
            setup_packet: Some(setup_packet),
            buffer: Some(vec![0; 4]),
            length: 4,
            transfer_type: UsbTransferType::Control,
            endpoint: 0,
            direction: UsbDirection::In,
            timeout: 5000,
            callback: None,
        };

        if self.submit_transfer(device.device_address, request) == NtStatus::Success {
            // Get manufacturer string
            if device.descriptor.manufacturer != 0 {
                device.strings.push(String::from("Example Manufacturer"));
            }

            // Get product string
            if device.descriptor.product != 0 {
                device.strings.push(String::from("Example USB Device"));
            }

            // Get serial number string
            if device.descriptor.serial_number != 0 {
                device.strings.push(String::from("123456789"));
            }
        }

        NtStatus::Success
    }

    fn load_device_driver(&self, device: &UsbDevice) {
        let driver_name = match device.descriptor.device_class {
            0x03 => "\\Driver\\HIDClass", // Human Interface Device
            0x08 => "\\Driver\\USBSTOR",  // Mass Storage
            0x09 => "\\Driver\\USBHUB",   // Hub
            0x0A => "\\Driver\\USBCCGP",  // CDC Data
            0x0E => "\\Driver\\USBVIDEO", // Video
            _ => {
                // Check interface classes
                if !device.configurations.is_empty() && !device.configurations[0].interfaces.is_empty() {
                    match device.configurations[0].interfaces[0].descriptor.interface_class {
                        0x03 => "\\Driver\\HIDClass",
                        0x08 => "\\Driver\\USBSTOR",
                        0x09 => "\\Driver\\USBHUB",
                        0x0A => "\\Driver\\USBCCGP",
                        0x0E => "\\Driver\\USBVIDEO",
                        _ => "\\Driver\\USBCCGP", // Generic composite driver
                    }
                } else {
                    "\\Driver\\USBCCGP"
                }
            }
        };

        crate::println!("USB: Loading driver {} for device VID:{:04X} PID:{:04X}",
            driver_name, device.descriptor.vendor_id, device.descriptor.product_id);
    }
}

// USB Subsystem Manager
pub struct UsbSubsystem {
    controllers: Vec<Box<dyn UsbHostController>>,
    devices: Vec<UsbDevice>,
    device_drivers: BTreeMap<String, Handle>,
}

impl UsbSubsystem {
    pub fn new() -> Self {
        Self {
            controllers: Vec::new(),
            devices: Vec::new(),
            device_drivers: BTreeMap::new(),
        }
    }

    pub fn initialize(&mut self) -> NtStatus {
        crate::println!("USB: Initializing USB subsystem");

        // Detect and initialize USB host controllers
        let status = self.detect_controllers();
        if status != NtStatus::Success {
            return status;
        }

        // Initialize each controller
        for controller in &mut self.controllers {
            let status = controller.initialize();
            if status != NtStatus::Success {
                return status;
            }
        }

        // Enumerate devices on all controllers
        for controller in &mut self.controllers {
            let status = controller.enumerate_devices();
            if status != NtStatus::Success {
                return status;
            }
        }

        crate::println!("USB: Subsystem initialized successfully");
        NtStatus::Success
    }

    fn detect_controllers(&mut self) -> NtStatus {
        crate::println!("USB: Detecting USB host controllers");

        // This would normally scan PCI bus for USB controllers
        // For now, create a simulated EHCI controller
        let ehci_controller = EhciController::new(0xFE000000);
        self.controllers.push(Box::new(ehci_controller));

        crate::println!("USB: Found {} USB host controllers", self.controllers.len());
        NtStatus::Success
    }

    pub fn get_device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn get_device_info(&self, index: usize) -> Option<String> {
        if let Some(device) = self.devices.get(index) {
            Some(format!(
                "USB Device: VID:{:04X} PID:{:04X} Class:{:02X} Address:{} Port:{}",
                device.descriptor.vendor_id,
                device.descriptor.product_id,
                device.descriptor.device_class,
                device.device_address,
                device.port_number
            ))
        } else {
            None
        }
    }

    pub fn hot_plug_event(&mut self, controller_index: usize, port: u8, connected: bool) -> NtStatus {
        if let Some(controller) = self.controllers.get_mut(controller_index) {
            if connected {
                crate::println!("USB: Device connected on port {} of controller {}", port, controller_index);
                controller.attach_device(port)
            } else {
                crate::println!("USB: Device disconnected from port {} of controller {}", port, controller_index);
                controller.detach_device(port)
            }
        } else {
            NtStatus::NoSuchDevice
        }
    }

    pub fn load_driver(&mut self, driver_name: &str, driver_handle: Handle) {
        self.device_drivers.insert(String::from(driver_name), driver_handle);
        crate::println!("USB: Loaded driver {} with handle {:?}", driver_name, driver_handle);
    }

    pub fn suspend_controller(&mut self, controller_index: usize) -> NtStatus {
        if let Some(controller) = self.controllers.get_mut(controller_index) {
            controller.suspend()
        } else {
            NtStatus::NoSuchDevice
        }
    }

    pub fn resume_controller(&mut self, controller_index: usize) -> NtStatus {
        if let Some(controller) = self.controllers.get_mut(controller_index) {
            controller.resume()
        } else {
            NtStatus::NoSuchDevice
        }
    }
}

// Global USB subsystem instance
static mut USB_SUBSYSTEM: Option<UsbSubsystem> = None;

pub fn initialize_usb_subsystem() -> NtStatus {
    unsafe {
        USB_SUBSYSTEM = Some(UsbSubsystem::new());
        
        if let Some(ref mut usb) = USB_SUBSYSTEM {
            usb.initialize()
        } else {
            NtStatus::InsufficientResources
        }
    }
}

pub fn get_usb_device_count() -> usize {
    unsafe {
        USB_SUBSYSTEM.as_ref().map_or(0, |usb| usb.get_device_count())
    }
}

pub fn get_usb_device_info(index: usize) -> Option<String> {
    unsafe {
        USB_SUBSYSTEM.as_ref().and_then(|usb| usb.get_device_info(index))
    }
}

pub fn usb_hot_plug_event(controller_index: usize, port: u8, connected: bool) -> NtStatus {
    unsafe {
        if let Some(ref mut usb) = USB_SUBSYSTEM {
            usb.hot_plug_event(controller_index, port, connected)
        } else {
            NtStatus::DeviceNotReady
        }
    }
}

pub fn load_usb_driver(driver_name: &str, driver_handle: Handle) {
    unsafe {
        if let Some(ref mut usb) = USB_SUBSYSTEM {
            usb.load_driver(driver_name, driver_handle);
        }
    }
}

// USB Helper Functions
impl UsbEndpointDescriptor {
    pub fn get_transfer_type(&self) -> UsbTransferType {
        match self.attributes & 0x03 {
            0 => UsbTransferType::Control,
            1 => UsbTransferType::Isochronous,
            2 => UsbTransferType::Bulk,
            3 => UsbTransferType::Interrupt,
            _ => UsbTransferType::Control,
        }
    }

    pub fn get_direction(&self) -> UsbDirection {
        if (self.endpoint_address & 0x80) != 0 {
            UsbDirection::In
        } else {
            UsbDirection::Out
        }
    }

    pub fn get_endpoint_number(&self) -> u8 {
        self.endpoint_address & 0x0F
    }
}

impl UsbDeviceClass {
    pub fn from_u8(class: u8) -> Self {
        match class {
            0x00 => UsbDeviceClass::UseInterfaceDescriptor,
            0x01 => UsbDeviceClass::Audio,
            0x02 => UsbDeviceClass::Communication,
            0x03 => UsbDeviceClass::HumanInterface,
            0x04 => UsbDeviceClass::Monitor,
            0x05 => UsbDeviceClass::PhysicalInterface,
            0x06 => UsbDeviceClass::Power,
            0x07 => UsbDeviceClass::Printer,
            0x08 => UsbDeviceClass::Storage,
            0x09 => UsbDeviceClass::Hub,
            0x0A => UsbDeviceClass::Data,
            0x0B => UsbDeviceClass::SmartCard,
            0x0D => UsbDeviceClass::ContentSecurity,
            0x0E => UsbDeviceClass::Video,
            0x0F => UsbDeviceClass::PersonalHealthcare,
            0x10 => UsbDeviceClass::AudioVideo,
            0xDC => UsbDeviceClass::Diagnostic,
            0xE0 => UsbDeviceClass::Wireless,
            0xEF => UsbDeviceClass::Miscellaneous,
            0xFE => UsbDeviceClass::ApplicationSpecific,
            0xFF => UsbDeviceClass::VendorSpecific,
            _ => UsbDeviceClass::UseInterfaceDescriptor,
        }
    }

    pub fn get_name(&self) -> &'static str {
        match self {
            UsbDeviceClass::UseInterfaceDescriptor => "Use Interface Descriptor",
            UsbDeviceClass::Audio => "Audio",
            UsbDeviceClass::Communication => "Communication",
            UsbDeviceClass::HumanInterface => "Human Interface Device",
            UsbDeviceClass::Monitor => "Monitor",
            UsbDeviceClass::PhysicalInterface => "Physical Interface",
            UsbDeviceClass::Power => "Power",
            UsbDeviceClass::Printer => "Printer",
            UsbDeviceClass::Storage => "Mass Storage",
            UsbDeviceClass::Hub => "Hub",
            UsbDeviceClass::Data => "Data",
            UsbDeviceClass::SmartCard => "Smart Card",
            UsbDeviceClass::ContentSecurity => "Content Security",
            UsbDeviceClass::Video => "Video",
            UsbDeviceClass::PersonalHealthcare => "Personal Healthcare",
            UsbDeviceClass::AudioVideo => "Audio/Video",
            UsbDeviceClass::Diagnostic => "Diagnostic",
            UsbDeviceClass::Wireless => "Wireless",
            UsbDeviceClass::Miscellaneous => "Miscellaneous",
            UsbDeviceClass::ApplicationSpecific => "Application Specific",
            UsbDeviceClass::VendorSpecific => "Vendor Specific",
        }
    }
}