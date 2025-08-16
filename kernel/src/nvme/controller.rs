// NVMe Controller Management
use super::*;
use crate::serial_println;

impl NvmeController {
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        unsafe {
            // Initiate shutdown
            let mut cc = Self::read32(self.base_addr, NVME_CC);
            cc &= !(3 << 14); // Clear shutdown notification bits
            cc |= NVME_CC_SHN_NORMAL;
            Self::write32(self.base_addr, NVME_CC, cc);
            
            // Wait for shutdown to complete
            for _ in 0..100 {
                let csts = Self::read32(self.base_addr, NVME_CSTS);
                if (csts & NVME_CSTS_SHST_MASK) == NVME_CSTS_SHST_COMPLETE {
                    serial_println!("NVMe: Shutdown complete");
                    return Ok(());
                }
                
                // Wait 10ms
                for _ in 0..10000 {
                    core::hint::spin_loop();
                }
            }
            
            Err("NVMe shutdown timeout")
        }
    }
    
    pub fn reset(&mut self) -> Result<(), &'static str> {
        unsafe {
            // Save configuration
            let aqa = Self::read32(self.base_addr, NVME_AQA);
            let asq = Self::read64(self.base_addr, NVME_ASQ);
            let acq = Self::read64(self.base_addr, NVME_ACQ);
            
            // Perform controller reset
            Self::write32(self.base_addr, NVME_NSSR, 0x4E564D65); // "NVMe"
            
            // Wait for reset to complete
            for _ in 0..100 {
                let csts = Self::read32(self.base_addr, NVME_CSTS);
                if csts & NVME_CSTS_RDY == 0 {
                    break;
                }
                
                // Wait 10ms
                for _ in 0..10000 {
                    core::hint::spin_loop();
                }
            }
            
            // Restore configuration
            Self::write32(self.base_addr, NVME_AQA, aqa);
            Self::write64(self.base_addr, NVME_ASQ, asq);
            Self::write64(self.base_addr, NVME_ACQ, acq);
            
            // Re-enable controller
            self.init()
        }
    }
    
    pub fn get_temperature(&mut self) -> Result<u16, &'static str> {
        // Get temperature from SMART log
        let mut log_data = [0u8; 512];
        
        let mut cmd = NvmeCommand::new();
        cmd.opcode = NVME_ADMIN_GET_LOG_PAGE;
        cmd.nsid = 0xFFFFFFFF; // All namespaces
        cmd.prp1 = log_data.as_ptr() as u64;
        cmd.cdw10 = 0x02 | (127 << 16); // Log ID 2 (SMART), 128 dwords
        
        self.admin_command(&cmd)?;
        
        // Temperature is at offset 1-2 (composite temperature)
        let temp_kelvin = u16::from_le_bytes([log_data[1], log_data[2]]);
        
        if temp_kelvin == 0 {
            return Err("Temperature not available");
        }
        
        // Convert from Kelvin to Celsius
        let temp_celsius = temp_kelvin.saturating_sub(273);
        
        serial_println!("NVMe: Temperature: {}°C", temp_celsius);
        
        Ok(temp_celsius)
    }
    
    pub fn set_features(&mut self, feature_id: u8, value: u32) -> Result<(), &'static str> {
        let mut cmd = NvmeCommand::new();
        cmd.opcode = NVME_ADMIN_SET_FEATURES;
        cmd.cdw10 = feature_id as u32;
        cmd.cdw11 = value;
        
        self.admin_command(&cmd)
    }
    
    pub fn get_features(&mut self, feature_id: u8) -> Result<u32, &'static str> {
        let mut cmd = NvmeCommand::new();
        cmd.opcode = NVME_ADMIN_GET_FEATURES;
        cmd.cdw10 = feature_id as u32;
        
        self.admin_command(&cmd)?;
        
        // Result would be in completion entry
        // For now, return 0
        Ok(0)
    }
    
    pub fn format_namespace(&mut self, namespace_id: u32, lba_format: u8) -> Result<(), &'static str> {
        let mut cmd = NvmeCommand::new();
        cmd.opcode = NVME_ADMIN_FORMAT_NVM;
        cmd.nsid = namespace_id;
        cmd.cdw10 = lba_format as u32; // LBA format index
        
        serial_println!("NVMe: Formatting namespace {} with LBA format {}", namespace_id, lba_format);
        
        self.admin_command(&cmd)?;
        
        // Re-identify namespace after format
        self.identify_namespaces()
    }
    
    pub fn flush(&mut self, namespace_id: u32) -> Result<(), &'static str> {
        let mut cmd = NvmeCommand::new();
        cmd.opcode = NVME_IO_FLUSH;
        cmd.nsid = namespace_id;
        
        if self.io_queues.is_empty() {
            return Err("No I/O queues available");
        }
        
        self.io_queues[0].submit_command(&cmd, self.base_addr)?;
        self.io_queues[0].wait_completion(self.base_addr)
    }
    
    pub fn trim(&mut self, namespace_id: u32, ranges: &[(u64, u32)]) -> Result<(), &'static str> {
        // Dataset Management command for TRIM
        let mut dsm_ranges = Vec::new();
        
        for &(lba, count) in ranges {
            dsm_ranges.push(DsmRange {
                cattr: 0,
                nlb: count,
                slba: lba,
            });
        }
        
        let mut cmd = NvmeCommand::new();
        cmd.opcode = NVME_IO_DSM;
        cmd.nsid = namespace_id;
        cmd.prp1 = dsm_ranges.as_ptr() as u64;
        cmd.cdw10 = ranges.len() as u32 - 1; // Number of ranges - 1
        cmd.cdw11 = 0x04; // Deallocate attribute
        
        if self.io_queues.is_empty() {
            return Err("No I/O queues available");
        }
        
        self.io_queues[0].submit_command(&cmd, self.base_addr)?;
        self.io_queues[0].wait_completion(self.base_addr)
    }
}

#[repr(C, packed)]
struct DsmRange {
    cattr: u32,
    nlb: u32,
    slba: u64,
}

// Power management
impl NvmeController {
    pub fn set_power_state(&mut self, power_state: u8) -> Result<(), &'static str> {
        if power_state > 31 {
            return Err("Invalid power state");
        }
        
        self.set_features(0x02, power_state as u32) // Feature ID 2: Power Management
    }
    
    pub fn enable_async_events(&mut self) -> Result<(), &'static str> {
        let mut cmd = NvmeCommand::new();
        cmd.opcode = NVME_ADMIN_ASYNC_EVENT;
        
        self.admin_command(&cmd)
    }
}