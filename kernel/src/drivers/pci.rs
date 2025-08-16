// PCI Bus Driver Implementation
use super::*;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use crate::nt::NtStatus;

// PCI Configuration space registers
pub const PCI_VENDOR_ID: u8 = 0x00;
pub const PCI_DEVICE_ID: u8 = 0x02;
pub const PCI_COMMAND: u8 = 0x04;
pub const PCI_STATUS: u8 = 0x06;
pub const PCI_REVISION_ID: u8 = 0x08;
pub const PCI_CLASS_PROG: u8 = 0x09;
pub const PCI_CLASS_DEVICE: u8 = 0x0A;
pub const PCI_CLASS_CODE: u8 = 0x0B;
pub const PCI_CACHE_LINE_SIZE: u8 = 0x0C;
pub const PCI_LATENCY_TIMER: u8 = 0x0D;
pub const PCI_HEADER_TYPE: u8 = 0x0E;
pub const PCI_BIST: u8 = 0x0F;
pub const PCI_BASE_ADDRESS_0: u8 = 0x10;
pub const PCI_BASE_ADDRESS_1: u8 = 0x14;
pub const PCI_BASE_ADDRESS_2: u8 = 0x18;
pub const PCI_BASE_ADDRESS_3: u8 = 0x1C;
pub const PCI_BASE_ADDRESS_4: u8 = 0x20;
pub const PCI_BASE_ADDRESS_5: u8 = 0x24;
pub const PCI_CARDBUS_CIS: u8 = 0x28;
pub const PCI_SUBSYSTEM_VENDOR_ID: u8 = 0x2C;
pub const PCI_SUBSYSTEM_ID: u8 = 0x2E;
pub const PCI_ROM_ADDRESS: u8 = 0x30;
pub const PCI_CAPABILITIES_POINTER: u8 = 0x34;
pub const PCI_INTERRUPT_LINE: u8 = 0x3C;
pub const PCI_INTERRUPT_PIN: u8 = 0x3D;
pub const PCI_MIN_GNT: u8 = 0x3E;
pub const PCI_MAX_LAT: u8 = 0x3F;

// PCI Command register bits
pub const PCI_COMMAND_IO: u16 = 0x0001;
pub const PCI_COMMAND_MEMORY: u16 = 0x0002;
pub const PCI_COMMAND_MASTER: u16 = 0x0004;
pub const PCI_COMMAND_SPECIAL: u16 = 0x0008;
pub const PCI_COMMAND_INVALIDATE: u16 = 0x0010;
pub const PCI_COMMAND_VGA_PALETTE: u16 = 0x0020;
pub const PCI_COMMAND_PARITY: u16 = 0x0040;
pub const PCI_COMMAND_WAIT: u16 = 0x0080;
pub const PCI_COMMAND_SERR: u16 = 0x0100;
pub const PCI_COMMAND_FAST_BACK: u16 = 0x0200;
pub const PCI_COMMAND_INTX_DISABLE: u16 = 0x0400;

// PCI Status register bits
pub const PCI_STATUS_INTERRUPT: u16 = 0x0008;
pub const PCI_STATUS_CAP_LIST: u16 = 0x0010;
pub const PCI_STATUS_66MHZ: u16 = 0x0020;
pub const PCI_STATUS_UDF: u16 = 0x0040;
pub const PCI_STATUS_FAST_BACK: u16 = 0x0080;
pub const PCI_STATUS_PARITY: u16 = 0x0100;
pub const PCI_STATUS_DEVSEL_MASK: u16 = 0x0600;
pub const PCI_STATUS_DEVSEL_FAST: u16 = 0x0000;
pub const PCI_STATUS_DEVSEL_MEDIUM: u16 = 0x0200;
pub const PCI_STATUS_DEVSEL_SLOW: u16 = 0x0400;
pub const PCI_STATUS_SIG_TARGET_ABORT: u16 = 0x0800;
pub const PCI_STATUS_REC_TARGET_ABORT: u16 = 0x1000;
pub const PCI_STATUS_REC_MASTER_ABORT: u16 = 0x2000;
pub const PCI_STATUS_SIG_SYSTEM_ERROR: u16 = 0x4000;
pub const PCI_STATUS_DETECTED_PARITY: u16 = 0x8000;

// PCI Base Address Register bits
pub const PCI_BASE_ADDRESS_SPACE: u32 = 0x01;
pub const PCI_BASE_ADDRESS_SPACE_IO: u32 = 0x01;
pub const PCI_BASE_ADDRESS_SPACE_MEMORY: u32 = 0x00;
pub const PCI_BASE_ADDRESS_MEM_TYPE_MASK: u32 = 0x06;
pub const PCI_BASE_ADDRESS_MEM_TYPE_32: u32 = 0x00;
pub const PCI_BASE_ADDRESS_MEM_TYPE_1M: u32 = 0x02;
pub const PCI_BASE_ADDRESS_MEM_TYPE_64: u32 = 0x04;
pub const PCI_BASE_ADDRESS_MEM_PREFETCH: u32 = 0x08;
pub const PCI_BASE_ADDRESS_MEM_MASK: u32 = !0x0F;
pub const PCI_BASE_ADDRESS_IO_MASK: u32 = !0x03;

// PCI Class codes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PciClass {
    Unclassified = 0x00,
    MassStorage = 0x01,
    Network = 0x02,
    Display = 0x03,
    Multimedia = 0x04,
    Memory = 0x05,
    Bridge = 0x06,
    Communication = 0x07,
    SystemPeripheral = 0x08,
    InputDevice = 0x09,
    DockingStation = 0x0A,
    Processor = 0x0B,
    SerialBus = 0x0C,
    Wireless = 0x0D,
    IntelligentIo = 0x0E,
    Satellite = 0x0F,
    Encryption = 0x10,
    SignalProcessing = 0x11,
    ProcessingAccelerator = 0x12,
    NonEssentialInstrumentation = 0x13,
    CoProcessor = 0x40,
    Unassigned = 0xFF,
}

#[derive(Debug, Clone)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub command: u16,
    pub status: u16,
    pub revision_id: u8,
    pub prog_if: u8,
    pub subclass: u8,
    pub class_code: u8,
    pub cache_line_size: u8,
    pub latency_timer: u8,
    pub header_type: u8,
    pub bist: u8,
    pub base_addresses: [u32; 6],
    pub cardbus_cis_pointer: u32,
    pub subsystem_vendor_id: u16,
    pub subsystem_id: u16,
    pub rom_base_address: u32,
    pub capabilities_pointer: u8,
    pub interrupt_line: u8,
    pub interrupt_pin: u8,
    pub min_grant: u8,
    pub max_latency: u8,
    pub driver_handle: Option<Handle>,
    pub device_handle: Option<Handle>,
}

#[derive(Debug, Clone)]
pub struct PciBaseAddressRegister {
    pub address: u64,
    pub size: u64,
    pub is_io: bool,
    pub is_prefetchable: bool,
    pub is_64bit: bool,
}

pub struct PciBus {
    devices: Vec<PciDevice>,
    next_bus: u8,
}

impl PciDevice {
    pub fn new(bus: u8, device: u8, function: u8) -> Self {
        Self {
            bus,
            device,
            function,
            vendor_id: 0,
            device_id: 0,
            command: 0,
            status: 0,
            revision_id: 0,
            prog_if: 0,
            subclass: 0,
            class_code: 0,
            cache_line_size: 0,
            latency_timer: 0,
            header_type: 0,
            bist: 0,
            base_addresses: [0; 6],
            cardbus_cis_pointer: 0,
            subsystem_vendor_id: 0,
            subsystem_id: 0,
            rom_base_address: 0,
            capabilities_pointer: 0,
            interrupt_line: 0,
            interrupt_pin: 0,
            min_grant: 0,
            max_latency: 0,
            driver_handle: None,
            device_handle: None,
        }
    }

    pub fn get_class(&self) -> PciClass {
        match self.class_code {
            0x00 => PciClass::Unclassified,
            0x01 => PciClass::MassStorage,
            0x02 => PciClass::Network,
            0x03 => PciClass::Display,
            0x04 => PciClass::Multimedia,
            0x05 => PciClass::Memory,
            0x06 => PciClass::Bridge,
            0x07 => PciClass::Communication,
            0x08 => PciClass::SystemPeripheral,
            0x09 => PciClass::InputDevice,
            0x0A => PciClass::DockingStation,
            0x0B => PciClass::Processor,
            0x0C => PciClass::SerialBus,
            0x0D => PciClass::Wireless,
            0x0E => PciClass::IntelligentIo,
            0x0F => PciClass::Satellite,
            0x10 => PciClass::Encryption,
            0x11 => PciClass::SignalProcessing,
            0x12 => PciClass::ProcessingAccelerator,
            0x13 => PciClass::NonEssentialInstrumentation,
            0x40 => PciClass::CoProcessor,
            _ => PciClass::Unassigned,
        }
    }

    pub fn get_vendor_name(&self) -> &'static str {
        match self.vendor_id {
            0x1022 => "AMD",
            0x8086 => "Intel Corporation",
            0x10de => "NVIDIA Corporation",
            0x1002 => "ATI Technologies Inc",
            0x15ad => "VMware",
            0x1ab8 => "Parallels",
            0x80ee => "InnoTek Systemberatung GmbH (VirtualBox)",
            0x1234 => "QEMU",
            0x1414 => "Microsoft Corporation",
            0x144d => "Samsung Electronics Co Ltd",
            0x8888 => "Bochs/QEMU",
            _ => "Unknown Vendor",
        }
    }

    pub fn get_device_name(&self) -> String {
        match (self.vendor_id, self.device_id) {
            (0x8086, 0x1237) => String::from("Intel 440FX PCI-to-ISA bridge"),
            (0x8086, 0x7000) => String::from("Intel PIIX3 PCI-to-ISA bridge"),
            (0x8086, 0x7010) => String::from("Intel PIIX3 IDE controller"),
            (0x8086, 0x7113) => String::from("Intel PIIX4 ACPI controller"),
            (0x8086, 0x100e) => String::from("Intel 82540EM Gigabit Ethernet"),
            (0x1234, 0x1111) => String::from("QEMU VGA controller"),
            (0x15ad, 0x0405) => String::from("VMware SVGA II"),
            (0x80ee, 0xbeef) => String::from("VirtualBox Graphics Adapter"),
            (0x80ee, 0xcafe) => String::from("VirtualBox Guest Service"),
            _ => String::from("Unknown Device"),
        }
    }

    pub fn is_multifunction(&self) -> bool {
        (self.header_type & 0x80) != 0
    }

    pub fn get_base_address_register(&self, index: usize) -> Option<PciBaseAddressRegister> {
        if index >= 6 {
            return None;
        }

        let bar = self.base_addresses[index];
        if bar == 0 {
            return None;
        }

        let is_io = (bar & PCI_BASE_ADDRESS_SPACE) != 0;
        
        if is_io {
            // I/O space BAR
            Some(PciBaseAddressRegister {
                address: (bar & PCI_BASE_ADDRESS_IO_MASK) as u64,
                size: 0, // Size determination requires additional PCI configuration
                is_io: true,
                is_prefetchable: false,
                is_64bit: false,
            })
        } else {
            // Memory space BAR
            let is_prefetchable = (bar & PCI_BASE_ADDRESS_MEM_PREFETCH) != 0;
            let mem_type = bar & PCI_BASE_ADDRESS_MEM_TYPE_MASK;
            let is_64bit = mem_type == PCI_BASE_ADDRESS_MEM_TYPE_64;
            
            let mut address = (bar & PCI_BASE_ADDRESS_MEM_MASK) as u64;
            
            if is_64bit && index < 5 {
                // 64-bit BAR uses two consecutive registers
                address |= (self.base_addresses[index + 1] as u64) << 32;
            }
            
            Some(PciBaseAddressRegister {
                address,
                size: 0, // Size determination requires additional PCI configuration
                is_io: false,
                is_prefetchable,
                is_64bit,
            })
        }
    }

    pub fn has_capability(&self, capability_id: u8) -> bool {
        if (self.status & PCI_STATUS_CAP_LIST) == 0 {
            return false;
        }

        let mut cap_ptr = self.capabilities_pointer;
        while cap_ptr != 0 {
            let cap_id = pci_config_read_byte(self.bus, self.device, self.function, cap_ptr);
            if cap_id == capability_id {
                return true;
            }
            cap_ptr = pci_config_read_byte(self.bus, self.device, self.function, cap_ptr + 1);
        }
        false
    }

    pub fn enable_bus_master(&mut self) {
        self.command |= PCI_COMMAND_MASTER;
        pci_config_write_word(self.bus, self.device, self.function, PCI_COMMAND, self.command);
    }

    pub fn enable_memory_space(&mut self) {
        self.command |= PCI_COMMAND_MEMORY;
        pci_config_write_word(self.bus, self.device, self.function, PCI_COMMAND, self.command);
    }

    pub fn enable_io_space(&mut self) {
        self.command |= PCI_COMMAND_IO;
        pci_config_write_word(self.bus, self.device, self.function, PCI_COMMAND, self.command);
    }

    pub fn read_configuration(&mut self) {
        // Read standard configuration space
        self.vendor_id = pci_config_read_word(self.bus, self.device, self.function, PCI_VENDOR_ID);
        self.device_id = pci_config_read_word(self.bus, self.device, self.function, PCI_DEVICE_ID);
        self.command = pci_config_read_word(self.bus, self.device, self.function, PCI_COMMAND);
        self.status = pci_config_read_word(self.bus, self.device, self.function, PCI_STATUS);
        self.revision_id = pci_config_read_byte(self.bus, self.device, self.function, PCI_REVISION_ID);
        self.prog_if = pci_config_read_byte(self.bus, self.device, self.function, PCI_CLASS_PROG);
        self.subclass = pci_config_read_byte(self.bus, self.device, self.function, PCI_CLASS_DEVICE);
        self.class_code = pci_config_read_byte(self.bus, self.device, self.function, PCI_CLASS_CODE);
        self.cache_line_size = pci_config_read_byte(self.bus, self.device, self.function, PCI_CACHE_LINE_SIZE);
        self.latency_timer = pci_config_read_byte(self.bus, self.device, self.function, PCI_LATENCY_TIMER);
        self.header_type = pci_config_read_byte(self.bus, self.device, self.function, PCI_HEADER_TYPE);
        self.bist = pci_config_read_byte(self.bus, self.device, self.function, PCI_BIST);

        // Read Base Address Registers
        for i in 0..6 {
            self.base_addresses[i] = pci_config_read_dword(
                self.bus,
                self.device,
                self.function,
                PCI_BASE_ADDRESS_0 + (i as u8 * 4),
            );
        }

        self.cardbus_cis_pointer = pci_config_read_dword(self.bus, self.device, self.function, PCI_CARDBUS_CIS);
        self.subsystem_vendor_id = pci_config_read_word(self.bus, self.device, self.function, PCI_SUBSYSTEM_VENDOR_ID);
        self.subsystem_id = pci_config_read_word(self.bus, self.device, self.function, PCI_SUBSYSTEM_ID);
        self.rom_base_address = pci_config_read_dword(self.bus, self.device, self.function, PCI_ROM_ADDRESS);
        self.capabilities_pointer = pci_config_read_byte(self.bus, self.device, self.function, PCI_CAPABILITIES_POINTER);
        self.interrupt_line = pci_config_read_byte(self.bus, self.device, self.function, PCI_INTERRUPT_LINE);
        self.interrupt_pin = pci_config_read_byte(self.bus, self.device, self.function, PCI_INTERRUPT_PIN);
        self.min_grant = pci_config_read_byte(self.bus, self.device, self.function, PCI_MIN_GNT);
        self.max_latency = pci_config_read_byte(self.bus, self.device, self.function, PCI_MAX_LAT);
    }
}

impl PciBus {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            next_bus: 0,
        }
    }

    pub fn enumerate_devices(&mut self) {
        crate::println!("PCI: Starting bus enumeration");

        for bus in 0..=255 {
            for device in 0..32 {
                if let Some(pci_device) = self.probe_device(bus, device, 0) {
                    crate::println!(
                        "PCI: Found device {}:{}.{} - {} {} (Class: {:02X}.{:02X}.{:02X})",
                        bus,
                        device,
                        0,
                        pci_device.get_vendor_name(),
                        pci_device.get_device_name(),
                        pci_device.class_code,
                        pci_device.subclass,
                        pci_device.prog_if
                    );

                    // Check for multi-function device
                    if pci_device.is_multifunction() {
                        for function in 1..8 {
                            if let Some(func_device) = self.probe_device(bus, device, function) {
                                crate::println!(
                                    "PCI: Found function {}:{}.{} - {} {}",
                                    bus,
                                    device,
                                    function,
                                    func_device.get_vendor_name(),
                                    func_device.get_device_name()
                                );
                                self.devices.push(func_device);
                            }
                        }
                    }

                    // Handle PCI-to-PCI bridges
                    if pci_device.class_code == 0x06 && pci_device.subclass == 0x04 {
                        // This is a PCI-to-PCI bridge
                        let secondary_bus = pci_config_read_byte(bus, device, 0, 0x19);
                        crate::println!("PCI: Found PCI-to-PCI bridge, secondary bus: {}", secondary_bus);
                        // Recursively scan the secondary bus
                        // Note: In a real implementation, this would be more complex
                    }

                    self.devices.push(pci_device);
                }
            }
        }

        crate::println!("PCI: Enumeration complete, found {} devices", self.devices.len());
    }

    fn probe_device(&self, bus: u8, device: u8, function: u8) -> Option<PciDevice> {
        let vendor_id = pci_config_read_word(bus, device, function, PCI_VENDOR_ID);
        
        // Invalid vendor ID means no device present
        if vendor_id == 0xFFFF || vendor_id == 0x0000 {
            return None;
        }

        let mut pci_device = PciDevice::new(bus, device, function);
        pci_device.read_configuration();
        
        Some(pci_device)
    }

    pub fn find_devices_by_class(&self, class_code: u8, subclass: Option<u8>) -> Vec<&PciDevice> {
        self.devices
            .iter()
            .filter(|device| {
                device.class_code == class_code
                    && subclass.map_or(true, |sc| device.subclass == sc)
            })
            .collect()
    }

    pub fn find_device_by_vendor_device(&self, vendor_id: u16, device_id: u16) -> Option<&PciDevice> {
        self.devices
            .iter()
            .find(|device| device.vendor_id == vendor_id && device.device_id == device_id)
    }

    pub fn find_devices_by_vendor(&self, vendor_id: u16) -> Vec<&PciDevice> {
        self.devices
            .iter()
            .filter(|device| device.vendor_id == vendor_id)
            .collect()
    }

    pub fn get_device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn get_devices(&self) -> &[PciDevice] {
        &self.devices
    }

    pub fn install_device_driver(&mut self, device_index: usize, driver_handle: Handle) -> NtStatus {
        if let Some(device) = self.devices.get_mut(device_index) {
            device.driver_handle = Some(driver_handle);
            crate::println!(
                "PCI: Installed driver {:?} for device {}:{}:{} - {}",
                driver_handle,
                device.bus,
                device.device,
                device.function,
                device.get_device_name()
            );
            NtStatus::Success
        } else {
            NtStatus::NoSuchDevice
        }
    }

    pub fn create_device_object(&mut self, device_index: usize, device_handle: Handle) -> NtStatus {
        if let Some(device) = self.devices.get_mut(device_index) {
            device.device_handle = Some(device_handle);
            crate::println!(
                "PCI: Created device object {:?} for PCI device {}",
                device_handle,
                device.get_device_name()
            );
            NtStatus::Success
        } else {
            NtStatus::NoSuchDevice
        }
    }
}

// PCI Configuration Space Access Functions
// These would normally use port I/O or memory-mapped I/O

pub fn pci_config_read_byte(bus: u8, device: u8, function: u8, offset: u8) -> u8 {
    let address = make_config_address(bus, device, function, offset);
    
    // Write address to CONFIG_ADDRESS port (0xCF8)
    unsafe {
        use x86_64::instructions::port::Port;
        let mut port: Port<u32> = Port::new(0xCF8);
        port.write(address);
        
        // Read data from CONFIG_DATA port (0xCFC)
        let mut data_port: Port<u32> = Port::new(0xCFC);
        let data = data_port.read();
        
        // Extract the correct byte based on offset
        ((data >> ((offset & 3) * 8)) & 0xFF) as u8
    }
}

pub fn pci_config_read_word(bus: u8, device: u8, function: u8, offset: u8) -> u16 {
    let address = make_config_address(bus, device, function, offset);
    
    unsafe {
        use x86_64::instructions::port::Port;
        let mut port: Port<u32> = Port::new(0xCF8);
        port.write(address);
        
        let mut data_port: Port<u32> = Port::new(0xCFC);
        let data = data_port.read();
        
        // Extract the correct word based on offset
        ((data >> ((offset & 2) * 8)) & 0xFFFF) as u16
    }
}

pub fn pci_config_read_dword(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address = make_config_address(bus, device, function, offset);
    
    unsafe {
        use x86_64::instructions::port::Port;
        let mut port: Port<u32> = Port::new(0xCF8);
        port.write(address);
        
        let mut data_port: Port<u32> = Port::new(0xCFC);
        data_port.read()
    }
}

pub fn pci_config_write_byte(bus: u8, device: u8, function: u8, offset: u8, value: u8) {
    let address = make_config_address(bus, device, function, offset);
    
    unsafe {
        use x86_64::instructions::port::Port;
        let mut port: Port<u32> = Port::new(0xCF8);
        port.write(address);
        
        let mut data_port: Port<u32> = Port::new(0xCFC);
        let mut data = data_port.read();
        
        // Modify the correct byte
        let shift = (offset & 3) * 8;
        data = (data & !(0xFF << shift)) | ((value as u32) << shift);
        data_port.write(data);
    }
}

pub fn pci_config_write_word(bus: u8, device: u8, function: u8, offset: u8, value: u16) {
    let address = make_config_address(bus, device, function, offset);
    
    unsafe {
        use x86_64::instructions::port::Port;
        let mut port: Port<u32> = Port::new(0xCF8);
        port.write(address);
        
        let mut data_port: Port<u32> = Port::new(0xCFC);
        let mut data = data_port.read();
        
        // Modify the correct word
        let shift = (offset & 2) * 8;
        data = (data & !(0xFFFF << shift)) | ((value as u32) << shift);
        data_port.write(data);
    }
}

pub fn pci_config_write_dword(bus: u8, device: u8, function: u8, offset: u8, value: u32) {
    let address = make_config_address(bus, device, function, offset);
    
    unsafe {
        use x86_64::instructions::port::Port;
        let mut port: Port<u32> = Port::new(0xCF8);
        port.write(address);
        
        let mut data_port: Port<u32> = Port::new(0xCFC);
        data_port.write(value);
    }
}

fn make_config_address(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let bus = bus as u32;
    let device = (device & 0x1F) as u32;
    let function = (function & 0x07) as u32;
    let offset = (offset & 0xFC) as u32;
    
    // Enable bit | Bus | Device | Function | Register offset
    0x80000000 | (bus << 16) | (device << 11) | (function << 8) | offset
}

// PCI Driver initialization and management
static mut PCI_BUS: Option<PciBus> = None;

pub fn initialize_pci_subsystem() -> NtStatus {
    crate::println!("PCI: Initializing PCI subsystem");
    
    unsafe {
        PCI_BUS = Some(PciBus::new());
        
        if let Some(ref mut pci_bus) = PCI_BUS {
            pci_bus.enumerate_devices();
            
            // Load drivers for detected devices
            load_pci_device_drivers(pci_bus);
            
            crate::println!("PCI: Subsystem initialized successfully");
            NtStatus::Success
        } else {
            NtStatus::InsufficientResources
        }
    }
}

fn load_pci_device_drivers(pci_bus: &mut PciBus) {
    crate::println!("PCI: Loading device drivers");
    
    for (index, device) in pci_bus.devices.iter().enumerate() {
        let driver_name = match (device.vendor_id, device.device_id, device.get_class()) {
            // Intel network controllers
            (0x8086, 0x100e, PciClass::Network) => Some("\\Driver\\E1000"),
            (0x8086, 0x100f, PciClass::Network) => Some("\\Driver\\E1000"),
            (0x8086, 0x10d3, PciClass::Network) => Some("\\Driver\\E1000e"),
            
            // GPU/Display controllers
            (0x8086, _, PciClass::Display) => {
                // Intel GPU - initialize GPU driver
                if let Some(gpu_driver) = crate::gpu::probe_gpu_device(device) {
                    let mut gpu_manager = crate::gpu::GPU_MANAGER.write();
                    gpu_manager.register_driver(gpu_driver);
                }
                Some("\\Driver\\IntelGPU")
            }
            (0x1002, _, PciClass::Display) => {
                // AMD GPU - initialize GPU driver
                if let Some(gpu_driver) = crate::gpu::probe_gpu_device(device) {
                    let mut gpu_manager = crate::gpu::GPU_MANAGER.write();
                    gpu_manager.register_driver(gpu_driver);
                }
                Some("\\Driver\\AMDGPU")
            }
            (_, _, PciClass::Display) => Some("\\Driver\\VGA"),
            
            // IDE/ATA controllers
            (_, _, PciClass::MassStorage) if device.subclass == 0x01 => Some("\\Driver\\PCIIDE"),
            
            // AHCI SATA controllers
            (_, _, PciClass::MassStorage) if device.subclass == 0x06 => Some("\\Driver\\StorAHCI"),
            
            // USB controllers
            (_, _, PciClass::SerialBus) if device.subclass == 0x03 => {
                match device.prog_if {
                    0x00 => Some("\\Driver\\UHCI"), // UHCI
                    0x10 => Some("\\Driver\\OHCI"), // OHCI
                    0x20 => Some("\\Driver\\EHCI"), // EHCI
                    0x30 => Some("\\Driver\\XHCI"), // xHCI
                    _ => None,
                }
            }
            
            // Audio controllers
            (_, _, PciClass::Multimedia) if device.subclass == 0x01 => Some("\\Driver\\HDAudio"),
            
            // PCI-to-PCI bridges
            (_, _, PciClass::Bridge) if device.subclass == 0x04 => Some("\\Driver\\PCI"),
            
            _ => None,
        };
        
        if let Some(driver_name) = driver_name {
            crate::println!(
                "PCI: Loading driver {} for device {}:{}.{} - {}",
                driver_name,
                device.bus,
                device.device,
                device.function,
                device.get_device_name()
            );
            
            // In a real implementation, we would load the driver here
            // For now, just log the intent
        } else {
            crate::println!(
                "PCI: No driver available for device {}:{}.{} - {} (Class: {:02X}.{:02X})",
                device.bus,
                device.device,
                device.function,
                device.get_device_name(),
                device.class_code,
                device.subclass
            );
        }
    }
}

pub fn get_pci_device_count() -> usize {
    unsafe {
        PCI_BUS.as_ref().map_or(0, |bus| bus.get_device_count())
    }
}

pub fn get_pci_device_info(index: usize) -> Option<String> {
    unsafe {
        if let Some(ref pci_bus) = PCI_BUS {
            if let Some(device) = pci_bus.devices.get(index) {
                return Some(format!(
                    "{}:{}.{} {} {} (VID:{:04X} DID:{:04X} Class:{:02X}.{:02X}.{:02X})",
                    device.bus,
                    device.device,
                    device.function,
                    device.get_vendor_name(),
                    device.get_device_name(),
                    device.vendor_id,
                    device.device_id,
                    device.class_code,
                    device.subclass,
                    device.prog_if
                ));
            }
        }
    }
    None
}

pub fn find_pci_device_by_class(class_code: u8, subclass: Option<u8>) -> Vec<String> {
    let mut result = Vec::new();
    
    unsafe {
        if let Some(ref pci_bus) = PCI_BUS {
            let devices = pci_bus.find_devices_by_class(class_code, subclass);
            for device in devices {
                result.push(format!(
                    "{}:{}.{} {} {}",
                    device.bus,
                    device.device,
                    device.function,
                    device.get_vendor_name(),
                    device.get_device_name()
                ));
            }
        }
    }
    
    result
}

// PCI device resource allocation
pub fn allocate_pci_resources(device_index: usize) -> NtStatus {
    unsafe {
        if let Some(ref mut pci_bus) = PCI_BUS {
            if let Some(device) = pci_bus.devices.get_mut(device_index) {
                crate::println!("PCI: Allocating resources for device {}", device.get_device_name());
                
                // Analyze Base Address Registers
                for i in 0..6 {
                    if let Some(bar) = device.get_base_address_register(i) {
                        if bar.is_io {
                            crate::println!("  BAR{}: I/O space at 0x{:08X}", i, bar.address);
                        } else {
                            crate::println!(
                                "  BAR{}: Memory space at 0x{:016X} ({}bit{})",
                                i,
                                bar.address,
                                if bar.is_64bit { "64" } else { "32" },
                                if bar.is_prefetchable { ", prefetchable" } else { "" }
                            );
                        }
                    }
                }
                
                // Enable appropriate command bits
                if device.base_addresses.iter().any(|&bar| bar != 0 && (bar & 1) == 0) {
                    device.enable_memory_space();
                }
                
                if device.base_addresses.iter().any(|&bar| bar != 0 && (bar & 1) != 0) {
                    device.enable_io_space();
                }
                
                device.enable_bus_master();
                
                crate::println!("PCI: Resource allocation complete for device {}", device.get_device_name());
                return NtStatus::Success;
            }
        }
    }
    
    NtStatus::NoSuchDevice
}

// PCI interrupt routing
pub fn setup_pci_interrupts(device_index: usize) -> NtStatus {
    unsafe {
        if let Some(ref pci_bus) = PCI_BUS {
            if let Some(device) = pci_bus.devices.get(device_index) {
                if device.interrupt_pin != 0 {
                    crate::println!(
                        "PCI: Setting up interrupt for device {} - INT{} -> IRQ {}",
                        device.get_device_name(),
                        match device.interrupt_pin {
                            1 => "A",
                            2 => "B", 
                            3 => "C",
                            4 => "D",
                            _ => "?",
                        },
                        device.interrupt_line
                    );
                    
                    // In a real implementation, we would:
                    // 1. Program the interrupt controller
                    // 2. Install interrupt handler
                    // 3. Enable interrupt in device
                    
                    return NtStatus::Success;
                }
            }
        }
    }
    
    NtStatus::NoSuchDevice
}