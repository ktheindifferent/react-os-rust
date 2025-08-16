// NVMe Command Builder
use super::*;

pub struct NvmeCommandBuilder {
    cmd: NvmeCommand,
}

impl NvmeCommandBuilder {
    pub fn new() -> Self {
        Self {
            cmd: NvmeCommand::new(),
        }
    }
    
    pub fn opcode(mut self, opcode: u8) -> Self {
        self.cmd.opcode = opcode;
        self
    }
    
    pub fn namespace(mut self, nsid: u32) -> Self {
        self.cmd.nsid = nsid;
        self
    }
    
    pub fn prp1(mut self, addr: u64) -> Self {
        self.cmd.prp1 = addr;
        self
    }
    
    pub fn prp2(mut self, addr: u64) -> Self {
        self.cmd.prp2 = addr;
        self
    }
    
    pub fn metadata(mut self, addr: u64) -> Self {
        self.cmd.metadata = addr;
        self
    }
    
    pub fn cdw10(mut self, value: u32) -> Self {
        self.cmd.cdw10 = value;
        self
    }
    
    pub fn cdw11(mut self, value: u32) -> Self {
        self.cmd.cdw11 = value;
        self
    }
    
    pub fn cdw12(mut self, value: u32) -> Self {
        self.cmd.cdw12 = value;
        self
    }
    
    pub fn cdw13(mut self, value: u32) -> Self {
        self.cmd.cdw13 = value;
        self
    }
    
    pub fn cdw14(mut self, value: u32) -> Self {
        self.cmd.cdw14 = value;
        self
    }
    
    pub fn cdw15(mut self, value: u32) -> Self {
        self.cmd.cdw15 = value;
        self
    }
    
    pub fn build(self) -> NvmeCommand {
        self.cmd
    }
}

// Specific command builders
impl NvmeCommandBuilder {
    pub fn read(nsid: u32, lba: u64, count: u16, buffer: u64) -> NvmeCommand {
        Self::new()
            .opcode(NVME_IO_READ)
            .namespace(nsid)
            .prp1(buffer)
            .cdw10(lba as u32)
            .cdw11((lba >> 32) as u32)
            .cdw12((count - 1) as u32)
            .build()
    }
    
    pub fn write(nsid: u32, lba: u64, count: u16, buffer: u64) -> NvmeCommand {
        Self::new()
            .opcode(NVME_IO_WRITE)
            .namespace(nsid)
            .prp1(buffer)
            .cdw10(lba as u32)
            .cdw11((lba >> 32) as u32)
            .cdw12((count - 1) as u32)
            .build()
    }
    
    pub fn flush(nsid: u32) -> NvmeCommand {
        Self::new()
            .opcode(NVME_IO_FLUSH)
            .namespace(nsid)
            .build()
    }
    
    pub fn identify_controller() -> NvmeCommand {
        Self::new()
            .opcode(NVME_ADMIN_IDENTIFY)
            .cdw10(1) // CNS = 1 for controller
            .build()
    }
    
    pub fn identify_namespace(nsid: u32) -> NvmeCommand {
        Self::new()
            .opcode(NVME_ADMIN_IDENTIFY)
            .namespace(nsid)
            .cdw10(0) // CNS = 0 for namespace
            .build()
    }
    
    pub fn create_io_cq(queue_id: u16, size: u16, buffer: u64) -> NvmeCommand {
        Self::new()
            .opcode(NVME_ADMIN_CREATE_CQ)
            .prp1(buffer)
            .cdw10(((size - 1) as u32) << 16 | queue_id as u32)
            .cdw11(1) // Physically contiguous, interrupts enabled
            .build()
    }
    
    pub fn create_io_sq(queue_id: u16, size: u16, cq_id: u16, buffer: u64) -> NvmeCommand {
        Self::new()
            .opcode(NVME_ADMIN_CREATE_SQ)
            .prp1(buffer)
            .cdw10(((size - 1) as u32) << 16 | queue_id as u32)
            .cdw11((cq_id as u32) << 16 | 1) // CQ ID and physically contiguous
            .build()
    }
    
    pub fn delete_io_cq(queue_id: u16) -> NvmeCommand {
        Self::new()
            .opcode(NVME_ADMIN_DELETE_CQ)
            .cdw10(queue_id as u32)
            .build()
    }
    
    pub fn delete_io_sq(queue_id: u16) -> NvmeCommand {
        Self::new()
            .opcode(NVME_ADMIN_DELETE_SQ)
            .cdw10(queue_id as u32)
            .build()
    }
    
    pub fn set_features(feature_id: u8, value: u32) -> NvmeCommand {
        Self::new()
            .opcode(NVME_ADMIN_SET_FEATURES)
            .cdw10(feature_id as u32)
            .cdw11(value)
            .build()
    }
    
    pub fn get_features(feature_id: u8) -> NvmeCommand {
        Self::new()
            .opcode(NVME_ADMIN_GET_FEATURES)
            .cdw10(feature_id as u32)
            .build()
    }
    
    pub fn get_log_page(log_id: u8, num_dwords: u16, buffer: u64) -> NvmeCommand {
        Self::new()
            .opcode(NVME_ADMIN_GET_LOG_PAGE)
            .namespace(0xFFFFFFFF) // All namespaces
            .prp1(buffer)
            .cdw10(log_id as u32 | ((num_dwords as u32 - 1) << 16))
            .build()
    }
}

// Feature IDs
pub mod features {
    pub const ARBITRATION: u8 = 0x01;
    pub const POWER_MANAGEMENT: u8 = 0x02;
    pub const LBA_RANGE_TYPE: u8 = 0x03;
    pub const TEMPERATURE_THRESHOLD: u8 = 0x04;
    pub const ERROR_RECOVERY: u8 = 0x05;
    pub const VOLATILE_WRITE_CACHE: u8 = 0x06;
    pub const NUMBER_OF_QUEUES: u8 = 0x07;
    pub const INTERRUPT_COALESCING: u8 = 0x08;
    pub const INTERRUPT_VECTOR_CONFIG: u8 = 0x09;
    pub const WRITE_ATOMICITY: u8 = 0x0A;
    pub const ASYNC_EVENT_CONFIG: u8 = 0x0B;
}

// Log Page IDs
pub mod log_pages {
    pub const ERROR_INFORMATION: u8 = 0x01;
    pub const SMART_HEALTH: u8 = 0x02;
    pub const FIRMWARE_SLOT: u8 = 0x03;
    pub const CHANGED_NAMESPACE: u8 = 0x04;
    pub const COMMANDS_SUPPORTED: u8 = 0x05;
}