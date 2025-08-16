// NVMe (Non-Volatile Memory Express) Driver Implementation
pub mod controller;
pub mod queue;
pub mod command;
pub mod namespace;

use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;
use core::mem;
use crate::{println, serial_println};
use crate::memory::PHYS_MEM_OFFSET;
use crate::drivers::disk::{DiskDriver, DiskError, DiskInfo};

// NVMe Constants
pub const NVME_CAP: usize = 0x0000;        // Controller Capabilities
pub const NVME_VS: usize = 0x0008;         // Version
pub const NVME_INTMS: usize = 0x000C;      // Interrupt Mask Set
pub const NVME_INTMC: usize = 0x0010;      // Interrupt Mask Clear
pub const NVME_CC: usize = 0x0014;         // Controller Configuration
pub const NVME_CSTS: usize = 0x001C;       // Controller Status
pub const NVME_NSSR: usize = 0x0020;       // NVM Subsystem Reset
pub const NVME_AQA: usize = 0x0024;        // Admin Queue Attributes
pub const NVME_ASQ: usize = 0x0028;        // Admin Submission Queue Base Address
pub const NVME_ACQ: usize = 0x0030;        // Admin Completion Queue Base Address
pub const NVME_CMBLOC: usize = 0x0038;     // Controller Memory Buffer Location
pub const NVME_CMBSZ: usize = 0x003C;      // Controller Memory Buffer Size

// Controller Configuration Bits
pub const NVME_CC_EN: u32 = 1 << 0;        // Enable
pub const NVME_CC_CSS_NVM: u32 = 0 << 4;   // NVM Command Set
pub const NVME_CC_MPS_SHIFT: u32 = 7;      // Memory Page Size
pub const NVME_CC_AMS_RR: u32 = 0 << 11;   // Round Robin Arbitration
pub const NVME_CC_SHN_NORMAL: u32 = 1 << 14; // Normal Shutdown
pub const NVME_CC_IOSQES: u32 = 6 << 16;   // I/O Submission Queue Entry Size (64 bytes)
pub const NVME_CC_IOCQES: u32 = 4 << 20;   // I/O Completion Queue Entry Size (16 bytes)

// Controller Status Bits
pub const NVME_CSTS_RDY: u32 = 1 << 0;     // Ready
pub const NVME_CSTS_CFS: u32 = 1 << 1;     // Controller Fatal Status
pub const NVME_CSTS_SHST_MASK: u32 = 3 << 2; // Shutdown Status
pub const NVME_CSTS_SHST_NORMAL: u32 = 0 << 2;
pub const NVME_CSTS_SHST_OCCURRING: u32 = 1 << 2;
pub const NVME_CSTS_SHST_COMPLETE: u32 = 2 << 2;

// Admin Commands
pub const NVME_ADMIN_DELETE_SQ: u8 = 0x00;
pub const NVME_ADMIN_CREATE_SQ: u8 = 0x01;
pub const NVME_ADMIN_GET_LOG_PAGE: u8 = 0x02;
pub const NVME_ADMIN_DELETE_CQ: u8 = 0x04;
pub const NVME_ADMIN_CREATE_CQ: u8 = 0x05;
pub const NVME_ADMIN_IDENTIFY: u8 = 0x06;
pub const NVME_ADMIN_ABORT: u8 = 0x08;
pub const NVME_ADMIN_SET_FEATURES: u8 = 0x09;
pub const NVME_ADMIN_GET_FEATURES: u8 = 0x0A;
pub const NVME_ADMIN_ASYNC_EVENT: u8 = 0x0C;
pub const NVME_ADMIN_NS_MGMT: u8 = 0x0D;
pub const NVME_ADMIN_FW_COMMIT: u8 = 0x10;
pub const NVME_ADMIN_FW_DOWNLOAD: u8 = 0x11;
pub const NVME_ADMIN_NS_ATTACH: u8 = 0x15;
pub const NVME_ADMIN_FORMAT_NVM: u8 = 0x80;
pub const NVME_ADMIN_SECURITY_SEND: u8 = 0x81;
pub const NVME_ADMIN_SECURITY_RECV: u8 = 0x82;

// I/O Commands
pub const NVME_IO_FLUSH: u8 = 0x00;
pub const NVME_IO_WRITE: u8 = 0x01;
pub const NVME_IO_READ: u8 = 0x02;
pub const NVME_IO_WRITE_UNCOR: u8 = 0x04;
pub const NVME_IO_COMPARE: u8 = 0x05;
pub const NVME_IO_WRITE_ZEROES: u8 = 0x08;
pub const NVME_IO_DSM: u8 = 0x09;
pub const NVME_IO_RESERVATION_REGISTER: u8 = 0x0D;
pub const NVME_IO_RESERVATION_REPORT: u8 = 0x0E;
pub const NVME_IO_RESERVATION_ACQUIRE: u8 = 0x11;
pub const NVME_IO_RESERVATION_RELEASE: u8 = 0x15;

// Submission Queue Entry (64 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NvmeCommand {
    pub opcode: u8,
    pub flags: u8,
    pub command_id: u16,
    pub nsid: u32,
    pub reserved: [u32; 2],
    pub metadata: u64,
    pub prp1: u64,      // Physical Region Page 1
    pub prp2: u64,      // Physical Region Page 2
    pub cdw10: u32,
    pub cdw11: u32,
    pub cdw12: u32,
    pub cdw13: u32,
    pub cdw14: u32,
    pub cdw15: u32,
}

impl NvmeCommand {
    pub fn new() -> Self {
        Self {
            opcode: 0,
            flags: 0,
            command_id: 0,
            nsid: 0,
            reserved: [0; 2],
            metadata: 0,
            prp1: 0,
            prp2: 0,
            cdw10: 0,
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }
}

// Completion Queue Entry (16 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NvmeCompletion {
    pub result: u32,
    pub reserved: u32,
    pub sq_head: u16,
    pub sq_id: u16,
    pub command_id: u16,
    pub status: u16,
}

impl NvmeCompletion {
    pub fn is_error(&self) -> bool {
        (self.status >> 1) & 0x7FF != 0
    }
    
    pub fn get_phase(&self) -> bool {
        (self.status & 1) != 0
    }
}

// Identify Controller Data Structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NvmeIdentifyController {
    pub vid: u16,           // Vendor ID
    pub ssvid: u16,         // Subsystem Vendor ID
    pub sn: [u8; 20],       // Serial Number
    pub mn: [u8; 40],       // Model Number
    pub fr: [u8; 8],        // Firmware Revision
    pub rab: u8,            // Recommended Arbitration Burst
    pub ieee: [u8; 3],      // IEEE OUI Identifier
    pub cmic: u8,           // Controller Multi-Path I/O and Namespace Sharing
    pub mdts: u8,           // Maximum Data Transfer Size
    pub cntlid: u16,        // Controller ID
    pub ver: u32,           // Version
    pub rtd3r: u32,         // RTD3 Resume Latency
    pub rtd3e: u32,         // RTD3 Entry Latency
    pub oaes: u32,          // Optional Asynchronous Events Supported
    pub ctratt: u32,        // Controller Attributes
    pub reserved1: [u8; 156],
    pub oacs: u16,          // Optional Admin Command Support
    pub acl: u8,            // Abort Command Limit
    pub aerl: u8,           // Asynchronous Event Request Limit
    pub frmw: u8,           // Firmware Updates
    pub lpa: u8,            // Log Page Attributes
    pub elpe: u8,           // Error Log Page Entries
    pub npss: u8,           // Number of Power States Support
    pub avscc: u8,          // Admin Vendor Specific Command Configuration
    pub apsta: u8,          // Autonomous Power State Transition Attributes
    pub wctemp: u16,        // Warning Composite Temperature Threshold
    pub cctemp: u16,        // Critical Composite Temperature Threshold
    pub mtfa: u16,          // Maximum Time for Firmware Activation
    pub hmpre: u32,         // Host Memory Buffer Preferred Size
    pub hmmin: u32,         // Host Memory Buffer Minimum Size
    pub tnvmcap: [u8; 16],  // Total NVM Capacity
    pub unvmcap: [u8; 16],  // Unallocated NVM Capacity
    pub rpmbs: u32,         // Replay Protected Memory Block Support
    pub edstt: u16,         // Extended Device Self-test Time
    pub dsto: u8,           // Device Self-test Options
    pub fwug: u8,           // Firmware Update Granularity
    pub kas: u16,           // Keep Alive Support
    pub hctma: u16,         // Host Controlled Thermal Management Attributes
    pub mntmt: u16,         // Minimum Thermal Management Temperature
    pub mxtmt: u16,         // Maximum Thermal Management Temperature
    pub sanicap: u32,       // Sanitize Capabilities
    pub reserved2: [u8; 180],
    pub sqes: u8,           // Submission Queue Entry Size
    pub cqes: u8,           // Completion Queue Entry Size
    pub maxcmd: u16,        // Maximum Outstanding Commands
    pub nn: u32,            // Number of Namespaces
    pub oncs: u16,          // Optional NVM Command Support
    pub fuses: u16,         // Fused Operation Support
    pub fna: u8,            // Format NVM Attributes
    pub vwc: u8,            // Volatile Write Cache
    pub awun: u16,          // Atomic Write Unit Normal
    pub awupf: u16,         // Atomic Write Unit Power Fail
    pub nvscc: u8,          // NVM Vendor Specific Command Configuration
    pub reserved3: u8,
    pub acwu: u16,          // Atomic Compare & Write Unit
    pub reserved4: [u8; 2],
    pub sgls: u32,          // SGL Support
    pub reserved5: [u8; 228],
    pub subnqn: [u8; 256],  // NVM Subsystem NVMe Qualified Name
    pub reserved6: [u8; 768],
    pub reserved7: [u8; 256],
}

// Identify Namespace Data Structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NvmeIdentifyNamespace {
    pub nsze: u64,          // Namespace Size
    pub ncap: u64,          // Namespace Capacity
    pub nuse: u64,          // Namespace Utilization
    pub nsfeat: u8,         // Namespace Features
    pub nlbaf: u8,          // Number of LBA Formats
    pub flbas: u8,          // Formatted LBA Size
    pub mc: u8,             // Metadata Capabilities
    pub dpc: u8,            // End-to-end Data Protection Capabilities
    pub dps: u8,            // End-to-end Data Protection Type Settings
    pub nmic: u8,           // Namespace Multi-path I/O and Namespace Sharing
    pub rescap: u8,         // Reservation Capabilities
    pub fpi: u8,            // Format Progress Indicator
    pub reserved1: u8,
    pub nawun: u16,         // Namespace Atomic Write Unit Normal
    pub nawupf: u16,        // Namespace Atomic Write Unit Power Fail
    pub nacwu: u16,         // Namespace Atomic Compare & Write Unit
    pub nabsn: u16,         // Namespace Atomic Boundary Size Normal
    pub nabo: u16,          // Namespace Atomic Boundary Offset
    pub nabspf: u16,        // Namespace Atomic Boundary Size Power Fail
    pub reserved2: u16,
    pub nvmcap: [u8; 16],   // NVM Capacity
    pub reserved3: [u8; 40],
    pub nguid: [u8; 16],    // Namespace Globally Unique Identifier
    pub eui64: [u8; 8],     // IEEE Extended Unique Identifier
    pub lbaf: [NvmeLbaFormat; 16], // LBA Format Support
    pub reserved4: [u8; 192],
    pub vs: [u8; 3712],     // Vendor Specific
}

// LBA Format
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NvmeLbaFormat {
    pub ms: u16,            // Metadata Size
    pub lbads: u8,          // LBA Data Size (as a power of 2)
    pub rp: u8,             // Relative Performance
}

// NVMe Controller
pub struct NvmeController {
    base_addr: u64,
    admin_queue: NvmeQueue,
    io_queues: Vec<NvmeQueue>,
    identify_controller: Option<NvmeIdentifyController>,
    namespaces: Vec<NvmeNamespace>,
    max_transfer_size: usize,
    version: u32,
}

// NVMe Queue
pub struct NvmeQueue {
    id: u16,
    size: u16,
    submission_queue: u64,  // Physical address
    completion_queue: u64,  // Physical address
    sq_tail: u16,
    cq_head: u16,
    cq_phase: bool,
    doorbell_addr: u64,
}

// NVMe Namespace
pub struct NvmeNamespace {
    id: u32,
    size: u64,          // Size in blocks
    block_size: u32,    // Block size in bytes
    capacity: u64,      // Capacity in bytes
    features: u8,
}

impl NvmeController {
    pub unsafe fn new(base_addr: u64) -> Result<Self, &'static str> {
        let version = Self::read32(base_addr, NVME_VS);
        let major = (version >> 16) & 0xFFFF;
        let minor = (version >> 8) & 0xFF;
        let tertiary = version & 0xFF;
        
        serial_println!("NVMe: Version {}.{}.{}", major, minor, tertiary);
        
        // Check capabilities
        let cap_low = Self::read32(base_addr, NVME_CAP);
        let cap_high = Self::read32(base_addr, NVME_CAP + 4);
        let cap = (cap_high as u64) << 32 | cap_low as u64;
        
        let mqes = (cap & 0xFFFF) + 1;  // Maximum Queue Entries Supported
        let css = (cap >> 37) & 0xFF;   // Command Set Supported
        let to = ((cap >> 24) & 0xFF) as u32 * 500; // Timeout in ms
        
        serial_println!("NVMe: Max queue entries: {}, CSS: 0x{:x}, Timeout: {}ms", 
                      mqes, css, to);
        
        // Create admin queue
        let admin_queue = NvmeQueue::new(0, 64)?;
        
        Ok(Self {
            base_addr,
            admin_queue,
            io_queues: Vec::new(),
            identify_controller: None,
            namespaces: Vec::new(),
            max_transfer_size: 4096, // Default
            version,
        })
    }
    
    unsafe fn read32(base_addr: u64, offset: usize) -> u32 {
        let addr = (PHYS_MEM_OFFSET + base_addr + offset as u64) as *const u32;
        addr.read_volatile()
    }
    
    unsafe fn write32(base_addr: u64, offset: usize, value: u32) {
        let addr = (PHYS_MEM_OFFSET + base_addr + offset as u64) as *mut u32;
        addr.write_volatile(value);
    }
    
    unsafe fn read64(base_addr: u64, offset: usize) -> u64 {
        let low = Self::read32(base_addr, offset) as u64;
        let high = Self::read32(base_addr, offset + 4) as u64;
        high << 32 | low
    }
    
    unsafe fn write64(base_addr: u64, offset: usize, value: u64) {
        Self::write32(base_addr, offset, value as u32);
        Self::write32(base_addr, offset + 4, (value >> 32) as u32);
    }
    
    pub fn init(&mut self) -> Result<(), &'static str> {
        unsafe {
            // Disable controller
            self.disable()?;
            
            // Configure admin queues
            let aqa = ((self.admin_queue.size as u32 - 1) << 16) | 
                     (self.admin_queue.size as u32 - 1);
            Self::write32(self.base_addr, NVME_AQA, aqa);
            
            // Set admin queue base addresses
            Self::write64(self.base_addr, NVME_ASQ, self.admin_queue.submission_queue);
            Self::write64(self.base_addr, NVME_ACQ, self.admin_queue.completion_queue);
            
            // Configure and enable controller
            let mut cc = Self::read32(self.base_addr, NVME_CC);
            cc &= !0xFFFFFF;  // Clear all bits except reserved
            cc |= NVME_CC_CSS_NVM;           // NVM command set
            cc |= NVME_CC_AMS_RR;             // Round robin arbitration
            cc |= (0 << NVME_CC_MPS_SHIFT);  // 4KB memory page size
            cc |= NVME_CC_IOSQES;             // 64 byte submission queue entries
            cc |= NVME_CC_IOCQES;             // 16 byte completion queue entries
            cc |= NVME_CC_EN;                 // Enable
            
            Self::write32(self.base_addr, NVME_CC, cc);
            
            // Wait for controller ready
            self.wait_ready(true)?;
            
            serial_println!("NVMe: Controller enabled");
            
            // Identify controller
            self.identify_controller()?;
            
            // Identify namespaces
            self.identify_namespaces()?;
            
            // Create I/O queues
            self.create_io_queues()?;
        }
        
        Ok(())
    }
    
    unsafe fn disable(&mut self) -> Result<(), &'static str> {
        let cc = Self::read32(self.base_addr, NVME_CC);
        if cc & NVME_CC_EN != 0 {
            Self::write32(self.base_addr, NVME_CC, cc & !NVME_CC_EN);
            self.wait_ready(false)?;
        }
        Ok(())
    }
    
    unsafe fn wait_ready(&self, ready: bool) -> Result<(), &'static str> {
        let expected = if ready { NVME_CSTS_RDY } else { 0 };
        
        for _ in 0..1000 {
            let csts = Self::read32(self.base_addr, NVME_CSTS);
            
            if csts & NVME_CSTS_CFS != 0 {
                return Err("NVMe controller fatal status");
            }
            
            if (csts & NVME_CSTS_RDY) == expected {
                return Ok(());
            }
            
            // Wait 10ms
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
        
        Err("NVMe controller ready timeout")
    }
    
    fn identify_controller(&mut self) -> Result<(), &'static str> {
        let mut data = [0u8; 4096];
        
        let mut cmd = NvmeCommand::new();
        cmd.opcode = NVME_ADMIN_IDENTIFY;
        cmd.nsid = 0;
        cmd.prp1 = data.as_ptr() as u64;
        cmd.cdw10 = 1; // Controller identify
        
        self.admin_command(&cmd)?;
        
        let identify = unsafe {
            *(data.as_ptr() as *const NvmeIdentifyController)
        };
        
        // Parse controller info
        let mut serial = String::new();
        for &byte in &identify.sn {
            if byte != 0 && byte != 0x20 {
                serial.push(byte as char);
            }
        }
        
        let mut model = String::new();
        for &byte in &identify.mn {
            if byte != 0 && byte != 0x20 {
                model.push(byte as char);
            }
        }
        
        serial_println!("NVMe: Controller SN: {}, Model: {}", serial.trim(), model.trim());
        serial_println!("NVMe: {} namespace(s), MDTS: {}", identify.nn, identify.mdts);
        
        // Calculate max transfer size
        if identify.mdts != 0 {
            self.max_transfer_size = 4096 * (1 << identify.mdts);
        }
        
        self.identify_controller = Some(identify);
        
        Ok(())
    }
    
    fn identify_namespaces(&mut self) -> Result<(), &'static str> {
        let ctrl = self.identify_controller.ok_or("Controller not identified")?;
        
        for nsid in 1..=ctrl.nn {
            let mut data = [0u8; 4096];
            
            let mut cmd = NvmeCommand::new();
            cmd.opcode = NVME_ADMIN_IDENTIFY;
            cmd.nsid = nsid;
            cmd.prp1 = data.as_ptr() as u64;
            cmd.cdw10 = 0; // Namespace identify
            
            if self.admin_command(&cmd).is_ok() {
                let identify = unsafe {
                    *(data.as_ptr() as *const NvmeIdentifyNamespace)
                };
                
                if identify.nsze == 0 {
                    continue; // Inactive namespace
                }
                
                // Get LBA format
                let lba_format = identify.lbaf[(identify.flbas & 0x0F) as usize];
                let block_size = 1u32 << lba_format.lbads;
                
                let namespace = NvmeNamespace {
                    id: nsid,
                    size: identify.nsze,
                    block_size,
                    capacity: identify.nsze * block_size as u64,
                    features: identify.nsfeat,
                };
                
                serial_println!("NVMe: Namespace {}: {} blocks, {} bytes/block, {} MB total",
                              nsid, namespace.size, namespace.block_size,
                              namespace.capacity / (1024 * 1024));
                
                self.namespaces.push(namespace);
            }
        }
        
        Ok(())
    }
    
    fn create_io_queues(&mut self) -> Result<(), &'static str> {
        // Create I/O completion queue
        let io_cq = NvmeQueue::new(1, 256)?;
        
        let mut cmd = NvmeCommand::new();
        cmd.opcode = NVME_ADMIN_CREATE_CQ;
        cmd.prp1 = io_cq.completion_queue;
        cmd.cdw10 = ((io_cq.size as u32 - 1) << 16) | 1; // Queue size and queue ID
        cmd.cdw11 = 1; // Physically contiguous, interrupts enabled
        
        self.admin_command(&cmd)?;
        
        // Create I/O submission queue
        let io_sq = NvmeQueue::new(1, 256)?;
        
        let mut cmd = NvmeCommand::new();
        cmd.opcode = NVME_ADMIN_CREATE_SQ;
        cmd.prp1 = io_sq.submission_queue;
        cmd.cdw10 = ((io_sq.size as u32 - 1) << 16) | 1; // Queue size and queue ID
        cmd.cdw11 = (1 << 16) | 1; // CQ ID and physically contiguous
        
        self.admin_command(&cmd)?;
        
        self.io_queues.push(io_sq);
        
        serial_println!("NVMe: Created I/O queue pair");
        
        Ok(())
    }
    
    fn admin_command(&mut self, cmd: &NvmeCommand) -> Result<(), &'static str> {
        self.admin_queue.submit_command(cmd, self.base_addr)?;
        self.admin_queue.wait_completion(self.base_addr)
    }
    
    pub fn read_blocks(&mut self, namespace_id: u32, start_lba: u64, count: u32, buffer: &mut [u8]) -> Result<(), &'static str> {
        let ns = self.namespaces.iter()
            .find(|n| n.id == namespace_id)
            .ok_or("Invalid namespace")?;
        
        if buffer.len() < (count as usize * ns.block_size as usize) {
            return Err("Buffer too small");
        }
        
        let mut cmd = NvmeCommand::new();
        cmd.opcode = NVME_IO_READ;
        cmd.nsid = namespace_id;
        cmd.prp1 = buffer.as_ptr() as u64;
        
        // Handle multi-page transfers
        if count * ns.block_size > 4096 {
            // Need PRP list
            cmd.prp2 = self.create_prp_list(buffer)?;
        }
        
        cmd.cdw10 = start_lba as u32;
        cmd.cdw11 = (start_lba >> 32) as u32;
        cmd.cdw12 = count - 1; // 0-based
        
        if self.io_queues.is_empty() {
            return Err("No I/O queues available");
        }
        
        self.io_queues[0].submit_command(&cmd, self.base_addr)?;
        self.io_queues[0].wait_completion(self.base_addr)
    }
    
    pub fn write_blocks(&mut self, namespace_id: u32, start_lba: u64, count: u32, data: &[u8]) -> Result<(), &'static str> {
        let ns = self.namespaces.iter()
            .find(|n| n.id == namespace_id)
            .ok_or("Invalid namespace")?;
        
        if data.len() < (count as usize * ns.block_size as usize) {
            return Err("Data too small");
        }
        
        let mut cmd = NvmeCommand::new();
        cmd.opcode = NVME_IO_WRITE;
        cmd.nsid = namespace_id;
        cmd.prp1 = data.as_ptr() as u64;
        
        // Handle multi-page transfers
        if count * ns.block_size > 4096 {
            // Need PRP list
            cmd.prp2 = self.create_prp_list(data)?;
        }
        
        cmd.cdw10 = start_lba as u32;
        cmd.cdw11 = (start_lba >> 32) as u32;
        cmd.cdw12 = count - 1; // 0-based
        
        if self.io_queues.is_empty() {
            return Err("No I/O queues available");
        }
        
        self.io_queues[0].submit_command(&cmd, self.base_addr)?;
        self.io_queues[0].wait_completion(self.base_addr)
    }
    
    fn create_prp_list(&self, buffer: &[u8]) -> Result<u64, &'static str> {
        // Simplified PRP list creation
        // In a real implementation, this would allocate memory for the PRP list
        // and populate it with physical addresses
        Ok(0)
    }
}

impl NvmeQueue {
    pub fn new(id: u16, size: u16) -> Result<Self, &'static str> {
        // Allocate memory for queues (simplified)
        let sq_size = size as usize * mem::size_of::<NvmeCommand>();
        let cq_size = size as usize * mem::size_of::<NvmeCompletion>();
        
        // These would be properly allocated in a real implementation
        let submission_queue = unsafe { allocate_aligned(sq_size, 4096) };
        let completion_queue = unsafe { allocate_aligned(cq_size, 4096) };
        
        // Clear queues
        unsafe {
            core::ptr::write_bytes((PHYS_MEM_OFFSET + submission_queue) as *mut u8, 0, sq_size);
            core::ptr::write_bytes((PHYS_MEM_OFFSET + completion_queue) as *mut u8, 0, cq_size);
        }
        
        Ok(Self {
            id,
            size,
            submission_queue,
            completion_queue,
            sq_tail: 0,
            cq_head: 0,
            cq_phase: true,
            doorbell_addr: 0x1000 + (id as u64 * 2 * 4), // Doorbell stride
        })
    }
    
    pub fn submit_command(&mut self, cmd: &NvmeCommand, base_addr: u64) -> Result<(), &'static str> {
        unsafe {
            // Write command to submission queue
            let sq_ptr = (PHYS_MEM_OFFSET + self.submission_queue + 
                         (self.sq_tail as u64 * mem::size_of::<NvmeCommand>() as u64)) as *mut NvmeCommand;
            *sq_ptr = *cmd;
            
            // Update tail doorbell
            self.sq_tail = (self.sq_tail + 1) % self.size;
            let doorbell = (PHYS_MEM_OFFSET + base_addr + self.doorbell_addr) as *mut u32;
            doorbell.write_volatile(self.sq_tail as u32);
        }
        
        Ok(())
    }
    
    pub fn wait_completion(&mut self, base_addr: u64) -> Result<(), &'static str> {
        unsafe {
            loop {
                let cq_ptr = (PHYS_MEM_OFFSET + self.completion_queue + 
                             (self.cq_head as u64 * mem::size_of::<NvmeCompletion>() as u64)) as *mut NvmeCompletion;
                let entry = cq_ptr.read_volatile();
                
                // Check phase bit
                if entry.get_phase() != self.cq_phase {
                    // No new completion
                    continue;
                }
                
                // Check for errors
                if entry.is_error() {
                    return Err("NVMe command failed");
                }
                
                // Update head and phase
                self.cq_head = (self.cq_head + 1) % self.size;
                if self.cq_head == 0 {
                    self.cq_phase = !self.cq_phase;
                }
                
                // Update completion queue head doorbell
                let doorbell = (PHYS_MEM_OFFSET + base_addr + self.doorbell_addr + 4) as *mut u32;
                doorbell.write_volatile(self.cq_head as u32);
                
                return Ok(());
            }
        }
    }
}

// Helper function to allocate aligned memory
unsafe fn allocate_aligned(size: usize, align: usize) -> u64 {
    // This is a simplified allocation - in production, use proper memory allocation
    static mut NEXT_ADDR: u64 = 0x20000000; // Start at 512MB
    let addr = (NEXT_ADDR + (align as u64 - 1)) & !(align as u64 - 1);
    NEXT_ADDR = addr + size as u64;
    addr
}

// NVMe Disk Driver Implementation
pub struct NvmeDisk {
    controller_idx: usize,
    namespace_id: u32,
    info: DiskInfo,
}

impl NvmeDisk {
    pub fn new(controller_idx: usize, namespace_id: u32) -> Result<Self, &'static str> {
        let controller = NVME_CONTROLLERS.lock();
        
        if controller_idx >= controller.len() {
            return Err("Invalid controller index");
        }
        
        let ctrl = &controller[controller_idx];
        let ns = ctrl.namespaces.iter()
            .find(|n| n.id == namespace_id)
            .ok_or("Invalid namespace")?;
        
        Ok(Self {
            controller_idx,
            namespace_id,
            info: DiskInfo {
                name: String::from("NVMe SSD"),
                sectors: ns.size,
                sector_size: ns.block_size as usize,
                model: String::from("NVMe Drive"),
                serial: String::from("N/A"),
            },
        })
    }
}

impl DiskDriver for NvmeDisk {
    fn read_sectors(&mut self, start_sector: u64, count: u32, buffer: &mut [u8]) -> Result<(), DiskError> {
        let mut controllers = NVME_CONTROLLERS.lock();
        
        if self.controller_idx >= controllers.len() {
            return Err(DiskError::InvalidSector);
        }
        
        controllers[self.controller_idx].read_blocks(self.namespace_id, start_sector, count, buffer)
            .map_err(|_| DiskError::IoError)
    }
    
    fn write_sectors(&mut self, start_sector: u64, count: u32, data: &[u8]) -> Result<(), DiskError> {
        let mut controllers = NVME_CONTROLLERS.lock();
        
        if self.controller_idx >= controllers.len() {
            return Err(DiskError::InvalidSector);
        }
        
        controllers[self.controller_idx].write_blocks(self.namespace_id, start_sector, count, data)
            .map_err(|_| DiskError::IoError)
    }
    
    fn get_info(&self) -> DiskInfo {
        self.info.clone()
    }
}

lazy_static! {
    pub static ref NVME_CONTROLLERS: Mutex<Vec<NvmeController>> = Mutex::new(Vec::new());
}

pub fn init() -> Result<(), &'static str> {
    serial_println!("NVMe: Initializing controllers");
    
    // This would come from PCI enumeration
    // Common NVMe controller base addresses
    let nvme_bases = [0xFEB10000u64]; // Example address
    
    let mut controllers = NVME_CONTROLLERS.lock();
    
    for &base in &nvme_bases {
        match unsafe { NvmeController::new(base) } {
            Ok(mut ctrl) => {
                if ctrl.init().is_ok() {
                    serial_println!("NVMe: Controller at 0x{:x} initialized", base);
                    controllers.push(ctrl);
                }
            }
            Err(e) => {
                serial_println!("NVMe: Failed to initialize controller at 0x{:x}: {}", base, e);
            }
        }
    }
    
    if controllers.is_empty() {
        serial_println!("NVMe: No controllers found");
    } else {
        serial_println!("NVMe: {} controller(s) initialized", controllers.len());
    }
    
    Ok(())
}