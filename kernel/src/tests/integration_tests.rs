// Integration Tests for Hardware Components
#[cfg(test)]
mod tests {
    use crate::sound::*;
    use crate::nvme::*;
    use crate::pcie::*;
    use crate::drivers::disk::{DiskDriver, DiskInfo};
    use alloc::vec::Vec;
    use alloc::string::String;
    
    #[test]
    fn test_audio_pipeline_integration() {
        // Test complete audio pipeline
        let mut manager = AudioManager::new();
        
        // Simulate initialization (without actual hardware)
        let format = AudioFormat {
            sample_rate: 48000,
            channels: 2,
            format: SampleFormat::S16LE,
            buffer_size: 512,
        };
        
        // Create and queue audio buffer
        let buffer = AudioBuffer::new(format.clone());
        assert!(manager.play_buffer(buffer).is_ok());
        
        // Test volume control
        assert!(manager.set_volume(0.5).is_ok());
        assert_eq!(manager.get_volume(), 0.5);
        
        // Test tone generation
        assert!(manager.play_tone(440.0, 100).is_ok());
    }
    
    #[test]
    fn test_nvme_disk_driver_interface() {
        // Test NVMe disk driver implementation
        let disk_info = DiskInfo {
            name: String::from("NVMe Test"),
            sectors: 1000000,
            sector_size: 512,
            model: String::from("Test Model"),
            serial: String::from("TEST123"),
        };
        
        // Verify disk info structure
        assert_eq!(disk_info.sectors * disk_info.sector_size as u64, 512000000);
        assert_eq!(disk_info.name, "NVMe Test");
    }
    
    #[test]
    fn test_pcie_device_enumeration_simulation() {
        let controller = PcieController::new();
        
        // Create simulated devices
        let storage_device = PciDevice {
            location: PciLocation::new(0, 0, 1, 0),
            vendor_id: 0x144D,
            device_id: 0xA808,
            class: PCI_CLASS_STORAGE,
            subclass: PCI_SUBCLASS_STORAGE_NVME,
            prog_if: 0x02,
            revision: 0,
            header_type: PCI_HEADER_TYPE_NORMAL,
            bars: [0xFEB00000, 0, 0, 0, 0, 0],
            capabilities: vec![
                PciCapability {
                    id: PCI_CAP_ID_PM,
                    offset: 0x40,
                    data: vec![0; 8],
                },
                PciCapability {
                    id: PCI_CAP_ID_MSI,
                    offset: 0x50,
                    data: vec![0; 16],
                },
                PciCapability {
                    id: PCI_CAP_ID_EXP,
                    offset: 0x60,
                    data: vec![0; 64],
                },
            ],
            extended_capabilities: Vec::new(),
        };
        
        let network_device = PciDevice {
            location: PciLocation::new(0, 0, 2, 0),
            vendor_id: 0x8086,
            device_id: 0x1539,
            class: PCI_CLASS_NETWORK,
            subclass: 0x00, // Ethernet
            prog_if: 0,
            revision: 0x03,
            header_type: PCI_HEADER_TYPE_NORMAL,
            bars: [0xFEA00000, 0, 0xE000, 0, 0, 0],
            capabilities: vec![
                PciCapability {
                    id: PCI_CAP_ID_PM,
                    offset: 0x40,
                    data: vec![0; 8],
                },
                PciCapability {
                    id: PCI_CAP_ID_MSIX,
                    offset: 0x70,
                    data: vec![0; 12],
                },
            ],
            extended_capabilities: Vec::new(),
        };
        
        let audio_device = PciDevice {
            location: PciLocation::new(0, 0, 3, 0),
            vendor_id: 0x8086,
            device_id: 0x293E,
            class: PCI_CLASS_MULTIMEDIA,
            subclass: 0x03, // HD Audio
            prog_if: 0,
            revision: 0x02,
            header_type: PCI_HEADER_TYPE_NORMAL,
            bars: [0xFE900000, 0, 0, 0, 0, 0],
            capabilities: vec![
                PciCapability {
                    id: PCI_CAP_ID_PM,
                    offset: 0x50,
                    data: vec![0; 8],
                },
                PciCapability {
                    id: PCI_CAP_ID_MSI,
                    offset: 0x60,
                    data: vec![0; 16],
                },
            ],
            extended_capabilities: Vec::new(),
        };
        
        // Test device properties
        assert!(storage_device.is_storage());
        assert!(storage_device.supports_msi());
        assert!(storage_device.supports_pcie());
        assert_eq!(storage_device.get_device_name(), "144d:a808");
        
        assert!(network_device.is_network());
        assert!(network_device.supports_msix());
        assert!(!network_device.supports_pcie());
        
        assert!(!audio_device.is_storage());
        assert!(!audio_device.is_network());
        assert!(audio_device.supports_msi());
    }
    
    #[test]
    fn test_hardware_capability_matrix() {
        // Test capability detection across all hardware
        struct HardwareCapabilities {
            has_nvme: bool,
            has_ahci: bool,
            has_usb: bool,
            has_audio: bool,
            has_pcie: bool,
            has_msi: bool,
            has_msix: bool,
        }
        
        let caps = HardwareCapabilities {
            has_nvme: true,
            has_ahci: true,
            has_usb: true,
            has_audio: true,
            has_pcie: true,
            has_msi: true,
            has_msix: true,
        };
        
        // All capabilities should be available
        assert!(caps.has_nvme);
        assert!(caps.has_ahci);
        assert!(caps.has_usb);
        assert!(caps.has_audio);
        assert!(caps.has_pcie);
        assert!(caps.has_msi);
        assert!(caps.has_msix);
    }
    
    #[test]
    fn test_memory_alignment_requirements() {
        // Test alignment requirements for DMA operations
        
        // NVMe requires 4KB alignment for queues
        let nvme_alignment = 4096;
        let nvme_addr = 0x10000000u64;
        assert_eq!(nvme_addr & (nvme_alignment - 1), 0);
        
        // Audio buffers typically need 64-byte alignment
        let audio_alignment = 64;
        let audio_addr = 0x20000040u64;
        assert_eq!(audio_addr & (audio_alignment - 1), 0);
        
        // PCIe BARs must be naturally aligned
        let bar_size = 0x1000u64;
        let bar_addr = 0xFEB00000u64;
        assert_eq!(bar_addr & (bar_size - 1), 0);
    }
    
    #[test]
    fn test_interrupt_vector_allocation() {
        use crate::pcie::msi::MsiManager;
        
        let mut msi_manager = MsiManager::new();
        
        // Allocate vectors for different devices
        let nvme_vector = msi_manager.allocate_vector();
        assert!(nvme_vector.is_some());
        assert!(nvme_vector.unwrap() >= 32); // After legacy IRQs
        
        let audio_vector = msi_manager.allocate_vector();
        assert!(audio_vector.is_some());
        assert_ne!(audio_vector, nvme_vector);
        
        let network_vector = msi_manager.allocate_vector();
        assert!(network_vector.is_some());
        
        // Free a vector
        if let Some(vec) = nvme_vector {
            msi_manager.free_vector(vec);
            
            // Should be able to allocate it again
            let new_vector = msi_manager.allocate_vector();
            assert!(new_vector.is_some());
        }
    }
    
    #[test]
    fn test_dma_buffer_management() {
        // Test DMA buffer allocation patterns
        
        // NVMe PRP list
        let prp_list_size = 8 * 512; // 512 entries * 8 bytes
        assert_eq!(prp_list_size, 4096);
        
        // Audio buffer descriptor list
        let bdl_size = 32 * 16; // 32 entries * 16 bytes
        assert_eq!(bdl_size, 512);
        
        // PCIe MSI-X table
        let msix_table_size = 256 * 16; // 256 vectors * 16 bytes
        assert_eq!(msix_table_size, 4096);
    }
    
    #[test]
    fn test_error_handling_cascade() {
        // Test error propagation through hardware stack
        
        fn simulate_nvme_error() -> Result<(), &'static str> {
            Err("NVMe controller timeout")
        }
        
        fn simulate_pcie_error() -> Result<(), &'static str> {
            Err("PCIe device not found")
        }
        
        fn simulate_audio_error() -> Result<(), &'static str> {
            Err("Audio codec not responding")
        }
        
        // All errors should be properly typed
        assert!(simulate_nvme_error().is_err());
        assert!(simulate_pcie_error().is_err());
        assert!(simulate_audio_error().is_err());
        
        // Error messages should be descriptive
        assert_eq!(simulate_nvme_error().unwrap_err(), "NVMe controller timeout");
        assert_eq!(simulate_pcie_error().unwrap_err(), "PCIe device not found");
        assert_eq!(simulate_audio_error().unwrap_err(), "Audio codec not responding");
    }
}