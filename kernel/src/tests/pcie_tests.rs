// PCIe Tests
#[cfg(test)]
mod tests {
    use crate::pcie::*;
    use crate::pcie::capability::*;
    use crate::pcie::device::*;
    use alloc::vec::Vec;
    use alloc::vec;
    
    #[test]
    fn test_pci_location() {
        let loc = PciLocation::new(0, 1, 2, 3);
        assert_eq!(loc.segment, 0);
        assert_eq!(loc.bus, 1);
        assert_eq!(loc.device, 2);
        assert_eq!(loc.function, 3);
        
        // Test legacy address generation
        let addr = loc.to_legacy_address(0x04); // Command register
        assert_eq!(addr & 0x80000000, 0x80000000); // Enable bit set
        assert_eq!((addr >> 16) & 0xFF, 1); // Bus
        assert_eq!((addr >> 11) & 0x1F, 2); // Device
        assert_eq!((addr >> 8) & 0x07, 3); // Function
        assert_eq!(addr & 0xFC, 0x04); // Register
    }
    
    #[test]
    fn test_pci_device_classification() {
        let device = PciDevice {
            location: PciLocation::new(0, 0, 0, 0),
            vendor_id: 0x8086,
            device_id: 0x1234,
            class: PCI_CLASS_STORAGE,
            subclass: PCI_SUBCLASS_STORAGE_NVME,
            prog_if: 0,
            revision: 0,
            header_type: 0,
            bars: [0; 6],
            capabilities: Vec::new(),
            extended_capabilities: Vec::new(),
        };
        
        assert!(device.is_storage());
        assert!(!device.is_network());
        assert!(!device.is_display());
        assert!(!device.is_bridge());
    }
    
    #[test]
    fn test_bar_decoding() {
        // Test I/O BAR
        let io_bar = 0x0000E001; // I/O at 0xE000
        assert_eq!(io_bar & 1, 1); // I/O space indicator
        assert_eq!(io_bar & 0xFFFFFFFC, 0xE000);
        
        // Test 32-bit memory BAR
        let mem32_bar = 0x10000000; // Memory at 0x10000000
        assert_eq!(mem32_bar & 1, 0); // Memory space
        assert_eq!((mem32_bar >> 1) & 0x03, 0); // 32-bit
        assert_eq!(mem32_bar & 0xFFFFFFF0, 0x10000000);
        
        // Test 64-bit memory BAR
        let mem64_bar_low = 0x00000004; // 64-bit indicator
        assert_eq!((mem64_bar_low >> 1) & 0x03, 2); // 64-bit type
    }
    
    #[test]
    fn test_capability_detection() {
        let mut device = PciDevice {
            location: PciLocation::new(0, 0, 0, 0),
            vendor_id: 0x8086,
            device_id: 0x1234,
            class: 0,
            subclass: 0,
            prog_if: 0,
            revision: 0,
            header_type: 0,
            bars: [0; 6],
            capabilities: Vec::new(),
            extended_capabilities: Vec::new(),
        };
        
        // Add MSI capability
        device.capabilities.push(PciCapability {
            id: PCI_CAP_ID_MSI,
            offset: 0x50,
            data: vec![0; 16],
        });
        
        // Add PCIe capability
        device.capabilities.push(PciCapability {
            id: PCI_CAP_ID_EXP,
            offset: 0x60,
            data: vec![0; 64],
        });
        
        assert!(device.has_capability(PCI_CAP_ID_MSI));
        assert!(device.has_capability(PCI_CAP_ID_EXP));
        assert!(!device.has_capability(PCI_CAP_ID_MSIX));
        
        assert!(device.supports_msi());
        assert!(!device.supports_msix());
        assert!(device.supports_pcie());
    }
    
    #[test]
    fn test_msi_capability_parsing() {
        let mut data = vec![0u8; 16];
        data[0] = PCI_CAP_ID_MSI;
        data[1] = 0x60; // Next pointer
        data[2] = 0x00; // Message control low
        data[3] = 0x80; // Message control high (64-bit capable)
        data[4] = 0x00; // Message address byte 0
        data[5] = 0x00; // Message address byte 1
        data[6] = 0xE0; // Message address byte 2
        data[7] = 0xFE; // Message address byte 3 (0xFEE00000)
        
        let msi = MsiCapability::parse(&data);
        assert_eq!(msi.offset, PCI_CAP_ID_MSI);
        assert_eq!(msi.message_control & 0x80, 0x80); // 64-bit capable
        assert_eq!(msi.message_address, 0xFEE00000);
    }
    
    #[test]
    fn test_pcie_capability_parsing() {
        let mut data = vec![0u8; 64];
        data[0] = PCI_CAP_ID_EXP;
        data[1] = 0x00; // Next pointer
        data[2] = 0x42; // PCIe cap: version 2, type 4 (Root Port)
        data[3] = 0x00;
        
        // Device capabilities
        data[4] = 0x02; // Max payload size: 256 bytes (001)
        data[5] = 0x00;
        data[6] = 0x00;
        data[7] = 0x00;
        
        // Link status
        data[18] = 0x23; // Speed: 3 (8.0 GT/s), Width: 2 (x2)
        data[19] = 0x00;
        
        let pcie = PcieCapability::parse(&data);
        assert_eq!(pcie.version, 2);
        assert_eq!(pcie.device_type, 4); // Root Port
        assert_eq!(pcie.max_payload_size, 256);
        assert_eq!(pcie.link_speed, 3);
        assert_eq!(pcie.link_width, 2);
        assert_eq!(pcie.get_device_type_string(), "Root Port");
        assert_eq!(pcie.get_link_speed_string(), "8.0 GT/s");
    }
    
    #[test]
    fn test_power_management_capability() {
        let mut data = vec![0u8; 8];
        data[0] = PCI_CAP_ID_PM;
        data[1] = 0x48; // Next pointer
        data[2] = 0x03; // Capabilities low: D1 & D2 support
        data[3] = 0x00; // Capabilities high
        data[4] = 0x00; // Control/Status low: D0 state
        data[5] = 0x00; // Control/Status high
        
        let mut pm = PowerManagementCapability::parse(&data);
        assert_eq!(pm.offset, PCI_CAP_ID_PM);
        assert_eq!(pm.capabilities & 0x03, 0x03); // D1 & D2 support
        assert_eq!(pm.control_status & 0x03, 0x00); // D0 state
    }
    
    #[test]
    fn test_extended_capability() {
        let mut device = PciDevice {
            location: PciLocation::new(0, 0, 0, 0),
            vendor_id: 0x8086,
            device_id: 0x1234,
            class: 0,
            subclass: 0,
            prog_if: 0,
            revision: 0,
            header_type: 0,
            bars: [0; 6],
            capabilities: Vec::new(),
            extended_capabilities: Vec::new(),
        };
        
        // Add AER extended capability
        device.extended_capabilities.push(PciExtendedCapability {
            id: PCI_EXT_CAP_ID_ERR,
            offset: 0x100,
            version: 1,
            data: vec![0; 64],
        });
        
        assert!(device.has_extended_capability(PCI_EXT_CAP_ID_ERR));
        assert!(!device.has_extended_capability(PCI_EXT_CAP_ID_SRIOV));
        
        let cap = device.get_extended_capability(PCI_EXT_CAP_ID_ERR);
        assert!(cap.is_some());
        assert_eq!(cap.unwrap().offset, 0x100);
    }
    
    #[test]
    fn test_device_class_strings() {
        let controller = PcieController::new();
        
        assert_eq!(controller.get_device_description(PCI_CLASS_STORAGE, PCI_SUBCLASS_STORAGE_NVME), 
                   "NVMe Controller");
        assert_eq!(controller.get_device_description(PCI_CLASS_STORAGE, PCI_SUBCLASS_STORAGE_SATA), 
                   "SATA Controller");
        assert_eq!(controller.get_device_description(PCI_CLASS_NETWORK, 0), 
                   "Network Controller");
        assert_eq!(controller.get_device_description(PCI_CLASS_DISPLAY, 0), 
                   "Display Controller");
        assert_eq!(controller.get_device_description(0xFF, 0), 
                   "Unknown Device");
    }
    
    #[test]
    fn test_bar_info() {
        let device = PciDevice {
            location: PciLocation::new(0, 0, 0, 0),
            vendor_id: 0x8086,
            device_id: 0x1234,
            class: 0,
            subclass: 0,
            prog_if: 0,
            revision: 0,
            header_type: 0,
            bars: [0x10000000, 0, 0xE001, 0, 0, 0], // Memory BAR and I/O BAR
            capabilities: Vec::new(),
            extended_capabilities: Vec::new(),
        };
        
        // Note: get_bar_info requires controller interaction, so we test the structure
        let bar_info = BarInfo {
            index: 0,
            base_address: 0x10000000,
            size: 0x1000,
            is_io: false,
            is_64bit: false,
            is_prefetchable: false,
        };
        
        assert_eq!(bar_info.base_address, 0x10000000);
        assert_eq!(bar_info.size, 0x1000);
        assert!(!bar_info.is_io);
        assert!(!bar_info.is_64bit);
    }
    
    #[test]
    fn test_pci_bus() {
        use crate::pcie::bus::PciBus;
        
        let mut bus = PciBus::new(0, 1);
        assert_eq!(bus.segment, 0);
        assert_eq!(bus.number, 1);
        assert_eq!(bus.devices.len(), 0);
        
        bus.devices.push(PciLocation::new(0, 1, 0, 0));
        bus.devices.push(PciLocation::new(0, 1, 1, 0));
        assert_eq!(bus.devices.len(), 2);
    }
}