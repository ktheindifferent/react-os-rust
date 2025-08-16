// NVMe Tests
#[cfg(test)]
mod tests {
    use crate::nvme::*;
    use crate::nvme::command::*;
    use crate::nvme::namespace::*;
    use alloc::vec::Vec;
    
    #[test]
    fn test_nvme_command_creation() {
        let cmd = NvmeCommand::new();
        assert_eq!(cmd.opcode, 0);
        assert_eq!(cmd.flags, 0);
        assert_eq!(cmd.command_id, 0);
        assert_eq!(cmd.nsid, 0);
    }
    
    #[test]
    fn test_nvme_command_builder() {
        let cmd = NvmeCommandBuilder::new()
            .opcode(NVME_IO_READ)
            .namespace(1)
            .prp1(0x1000)
            .prp2(0x2000)
            .cdw10(100)
            .cdw12(15)
            .build();
        
        assert_eq!(cmd.opcode, NVME_IO_READ);
        assert_eq!(cmd.nsid, 1);
        assert_eq!(cmd.prp1, 0x1000);
        assert_eq!(cmd.prp2, 0x2000);
        assert_eq!(cmd.cdw10, 100);
        assert_eq!(cmd.cdw12, 15);
    }
    
    #[test]
    fn test_nvme_read_command_builder() {
        let cmd = NvmeCommandBuilder::read(1, 1000, 8, 0x100000);
        
        assert_eq!(cmd.opcode, NVME_IO_READ);
        assert_eq!(cmd.nsid, 1);
        assert_eq!(cmd.prp1, 0x100000);
        assert_eq!(cmd.cdw10, 1000); // LBA low
        assert_eq!(cmd.cdw11, 0); // LBA high
        assert_eq!(cmd.cdw12, 7); // Count - 1
    }
    
    #[test]
    fn test_nvme_write_command_builder() {
        let cmd = NvmeCommandBuilder::write(2, 0x100000000, 16, 0x200000);
        
        assert_eq!(cmd.opcode, NVME_IO_WRITE);
        assert_eq!(cmd.nsid, 2);
        assert_eq!(cmd.prp1, 0x200000);
        assert_eq!(cmd.cdw10, 0); // LBA low (lower 32 bits of 0x100000000)
        assert_eq!(cmd.cdw11, 1); // LBA high (upper 32 bits)
        assert_eq!(cmd.cdw12, 15); // Count - 1
    }
    
    #[test]
    fn test_nvme_completion_error_check() {
        let mut completion = NvmeCompletion {
            result: 0,
            reserved: 0,
            sq_head: 0,
            sq_id: 0,
            command_id: 0,
            status: 0,
        };
        
        // No error
        assert!(!completion.is_error());
        
        // Set error status
        completion.status = 0x02; // Generic Command Status error
        assert!(completion.is_error());
        
        // Set phase bit
        completion.status = 0x01;
        assert!(!completion.is_error());
        assert!(completion.get_phase());
    }
    
    #[test]
    fn test_nvme_namespace() {
        let mut ns = NvmeNamespace::new(1);
        assert_eq!(ns.id, 1);
        assert_eq!(ns.block_size, 512);
        
        // Update from identify data
        let mut identify = NvmeIdentifyNamespace {
            nsze: 1000000,
            ncap: 1000000,
            nuse: 500000,
            nsfeat: 0x09, // Deallocate and write zeroes supported
            nlbaf: 1,
            flbas: 0,
            mc: 0,
            dpc: 0,
            dps: 0,
            nmic: 0,
            rescap: 0,
            fpi: 0,
            reserved1: 0,
            nawun: 0,
            nawupf: 0,
            nacwu: 0,
            nabsn: 0,
            nabo: 0,
            nabspf: 0,
            reserved2: 0,
            nvmcap: [0; 16],
            reserved3: [0; 40],
            nguid: [0; 16],
            eui64: [0; 8],
            lbaf: [NvmeLbaFormat { ms: 0, lbads: 9, rp: 0 }; 16], // 512 bytes (2^9)
            reserved4: [0; 192],
            vs: [0; 3712],
        };
        
        ns.update_from_identify(&identify);
        assert_eq!(ns.size, 1000000);
        assert_eq!(ns.block_size, 512);
        assert!(ns.supports_deallocate());
        assert!(ns.supports_write_zeroes());
    }
    
    #[test]
    fn test_nvme_namespace_manager() {
        let mut manager = NvmeNamespaceManager::new();
        
        let ns1 = NvmeNamespace {
            id: 1,
            size: 1000000,
            block_size: 512,
            capacity: 512000000,
            features: 0,
        };
        
        let ns2 = NvmeNamespace {
            id: 2,
            size: 2000000,
            block_size: 4096,
            capacity: 8192000000,
            features: 0,
        };
        
        manager.add_namespace(ns1);
        manager.add_namespace(ns2);
        
        assert_eq!(manager.active_count(), 2);
        assert_eq!(manager.total_capacity(), 512000000 + 8192000000);
        
        let ns = manager.get_namespace(1);
        assert!(ns.is_some());
        assert_eq!(ns.unwrap().id, 1);
        
        let namespaces = manager.list_namespaces();
        assert_eq!(namespaces, vec![1, 2]);
    }
    
    #[test]
    fn test_nvme_io_request() {
        let read_req = NvmeIoRequest::read(1, 1000, 8);
        assert_eq!(read_req.namespace_id, 1);
        assert_eq!(read_req.opcode, NVME_IO_READ);
        assert_eq!(read_req.lba, 1000);
        assert_eq!(read_req.count, 8);
        
        let write_data = vec![0xFF; 4096];
        let write_req = NvmeIoRequest::write(2, 500, write_data.clone());
        assert_eq!(write_req.namespace_id, 2);
        assert_eq!(write_req.opcode, NVME_IO_WRITE);
        assert_eq!(write_req.lba, 500);
        assert_eq!(write_req.count, 8); // 4096 / 512
        assert_eq!(write_req.buffer, write_data);
        
        let flush_req = NvmeIoRequest::flush(1);
        assert_eq!(flush_req.opcode, NVME_IO_FLUSH);
        
        let trim_req = NvmeIoRequest::trim(1, 1000, 100);
        assert_eq!(trim_req.opcode, NVME_IO_DSM);
        assert_eq!(trim_req.lba, 1000);
        assert_eq!(trim_req.count, 100);
    }
    
    #[test]
    fn test_nvme_namespace_stats() {
        let mut stats = NvmeNamespaceStats::new();
        
        stats.record_read(100);
        stats.record_read(200);
        assert_eq!(stats.read_commands, 2);
        assert_eq!(stats.read_blocks, 300);
        
        stats.record_write(50);
        assert_eq!(stats.write_commands, 1);
        assert_eq!(stats.write_blocks, 50);
        
        stats.record_error(true);
        stats.record_error(false);
        assert_eq!(stats.read_errors, 1);
        assert_eq!(stats.write_errors, 1);
        
        stats.reset();
        assert_eq!(stats.read_commands, 0);
        assert_eq!(stats.write_commands, 0);
    }
    
    #[test]
    fn test_pci_location_to_legacy_address() {
        let location = PciLocation::new(0, 1, 2, 3);
        let addr = location.to_legacy_address(0x10);
        
        // Enable bit | bus 1 << 16 | device 2 << 11 | function 3 << 8 | register 0x10
        let expected = 0x80000000 | (1 << 16) | (2 << 11) | (3 << 8) | 0x10;
        assert_eq!(addr, expected);
    }
    
    #[test]
    fn test_feature_ids() {
        assert_eq!(features::POWER_MANAGEMENT, 0x02);
        assert_eq!(features::NUMBER_OF_QUEUES, 0x07);
        assert_eq!(features::VOLATILE_WRITE_CACHE, 0x06);
    }
    
    #[test]
    fn test_log_page_ids() {
        assert_eq!(log_pages::ERROR_INFORMATION, 0x01);
        assert_eq!(log_pages::SMART_HEALTH, 0x02);
        assert_eq!(log_pages::FIRMWARE_SLOT, 0x03);
    }
}