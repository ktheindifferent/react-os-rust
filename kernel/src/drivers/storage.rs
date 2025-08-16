// Storage Subsystem Implementation
use super::*;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use alloc::boxed::Box;
use alloc::format;
use crate::nt::NtStatus;

// Storage Device Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageDeviceType {
    Unknown,
    HardDisk,
    FloppyDisk,
    OpticalDisk,
    SolidStateDrive,
    USBDrive,
    NetworkDrive,
    VirtualDisk,
    TapeDrive,
}

// Storage Interface Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageInterface {
    Unknown,
    IDE,      // Integrated Drive Electronics
    SATA,     // Serial ATA
    SCSI,     // Small Computer System Interface
    USB,      // Universal Serial Bus
    NVMe,     // Non-Volatile Memory Express
    ATAPI,    // AT Attachment Packet Interface
    SAS,      // Serial Attached SCSI
    Fibre,    // Fibre Channel
    iSCSI,    // Internet SCSI
}

// Storage Media Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageMediaType {
    Unknown,
    Fixed,        // Non-removable
    Removable,    // Removable
    RemoteFixed,  // Network fixed
    RemoteRemovable, // Network removable
}

// Storage Bus Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageBusType {
    Unknown,
    SCSI,
    ATAPI,
    ATA,
    OneThreeNineFour, // IEEE 1394 (FireWire)
    SSA,             // Serial Storage Architecture
    Fibre,           // Fibre Channel
    USB,
    RAID,
    iSCSI,
    SAS,
    SATA,
    SD,              // Secure Digital
    MMC,             // MultiMediaCard
    Virtual,
    FileBackedVirtual,
    NVMe,
    SCM,             // Storage Class Memory
    UFS,             // Universal Flash Storage
    Max,
}

// SCSI Command Descriptor Block
#[derive(Debug, Clone)]
pub struct ScsiCdb {
    pub operation_code: u8,
    pub flags: u8,
    pub logical_block_address: u64,
    pub transfer_length: u32,
    pub control: u8,
    pub additional_data: Vec<u8>,
}

// Storage Request Block
#[derive(Debug, Clone)]
pub struct StorageRequestBlock {
    pub srb_flags: u32,
    pub function: ScsiFunction,
    pub srb_status: ScsiStatus,
    pub scsi_status: u8,
    pub path_id: u8,
    pub target_id: u8,
    pub lun: u8,
    pub queue_tag: u8,
    pub queue_action: u8,
    pub cdb_length: u8,
    pub sense_info_buffer_length: u8,
    pub data_transfer_length: u32,
    pub timeout_value: u32,
    pub data_buffer: Option<Vec<u8>>,
    pub sense_info_buffer: Option<Vec<u8>>,
    pub cdb: ScsiCdb,
    pub next_srb: Option<Box<StorageRequestBlock>>,
    pub original_request: Option<*mut Irp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScsiFunction {
    ExecuteScsi = 0x00,
    ClaimDevice = 0x01,
    IoControl = 0x02,
    ReceiveEvent = 0x03,
    ReleaseQueue = 0x04,
    AttachDevice = 0x05,
    ReleaseDevice = 0x06,
    ShutdownQueue = 0x07,
    FlushQueue = 0x08,
    AbortCommand = 0x10,
    ResetBus = 0x12,
    ResetDevice = 0x13,
    TerminateIo = 0x14,
    FlushAdapter = 0x15,
    RemoveDevice = 0x16,
    WmiExecuteMethod = 0x17,
    LockQueue = 0x18,
    UnlockQueue = 0x19,
    ResetLogicalUnit = 0x20,
    SetLinkTimeoutValue = 0x21,
    SetLinkTimeoutValueHw = 0x22,
    PowerManagement = 0x23,
    PnpQueryDeviceText = 0x24,
    ExecuteScsiEx = 0x25,
    SetAdapterProperty = 0x26,
    QueryAdapterProperty = 0x27,
    DumpPointers = 0x28,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScsiStatus {
    Pending = 0x00,
    Success = 0x01,
    Aborted = 0x02,
    AbortFailed = 0x03,
    Error = 0x04,
    Busy = 0x05,
    InvalidRequest = 0x06,
    InvalidPathId = 0x07,
    NoDevice = 0x08,
    Timeout = 0x09,
    SelectionTimeout = 0x0A,
    CommandTimeout = 0x0B,
    MessageRejected = 0x0D,
    BusReset = 0x0E,
    ParityError = 0x0F,
    RequestSenseFailure = 0x10,
    NoHbaFailure = 0x11,
    DataOverrun = 0x12,
    UnexpectedBusPhase = 0x13,
    BadFunction = 0x22,
    ErrorRecovery = 0x23,
    NotPowered = 0x24,
    LinkDown = 0x25,
}

// Storage Device Descriptor
#[derive(Debug, Clone)]
pub struct StorageDeviceDescriptor {
    pub version: u32,
    pub size: u32,
    pub device_type: u8,
    pub device_type_modifier: u8,
    pub removable_media: bool,
    pub command_queueing: bool,
    pub vendor_id_offset: u32,
    pub product_id_offset: u32,
    pub product_revision_offset: u32,
    pub serial_number_offset: u32,
    pub bus_type: StorageBusType,
    pub raw_properties_length: u32,
    pub raw_device_properties: Vec<u8>,
}

// Storage Adapter Descriptor
#[derive(Debug, Clone)]
pub struct StorageAdapterDescriptor {
    pub version: u32,
    pub size: u32,
    pub max_transfer_length: u32,
    pub max_physical_pages: u32,
    pub alignment_mask: u32,
    pub adapter_use_pio: bool,
    pub adapter_scans_down: bool,
    pub adapter_uses_dma: bool,
    pub adapter_uses_dma32: bool,
    pub adapter_uses_dma64: bool,
    pub bus_type: StorageBusType,
    pub bus_major_version: u16,
    pub bus_minor_version: u16,
}

// Disk Geometry
#[derive(Debug, Clone)]
pub struct DiskGeometry {
    pub cylinders: u64,
    pub media_type: MediaType,
    pub tracks_per_cylinder: u32,
    pub sectors_per_track: u32,
    pub bytes_per_sector: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MediaType {
    Unknown = 0x00,
    F5_1Pt2_512 = 0x01,     // 5.25", 1.2MB,  512 bytes/sector
    F3_1Pt44_512 = 0x02,    // 3.5",  1.44MB, 512 bytes/sector
    F3_2Pt88_512 = 0x03,    // 3.5",  2.88MB, 512 bytes/sector
    F3_20Pt8_512 = 0x04,    // 3.5",  20.8MB, 512 bytes/sector
    F3_720_512 = 0x05,      // 3.5",  720KB,  512 bytes/sector
    F5_360_512 = 0x06,      // 5.25", 360KB,  512 bytes/sector
    F5_320_512 = 0x07,      // 5.25", 320KB,  512 bytes/sector
    F5_320_1024 = 0x08,     // 5.25", 320KB,  1024 bytes/sector
    F5_180_512 = 0x09,      // 5.25", 180KB,  512 bytes/sector
    F5_160_512 = 0x0a,      // 5.25", 160KB,  512 bytes/sector
    RemovableMedia = 0x0b,   // Removable media other than floppy
    FixedMedia = 0x0c,       // Fixed hard disk media
    F3_120M_512 = 0x0d,     // 3.5", 120M Floppy
    F3_640_512 = 0x0e,      // 3.5" ,  640KB,  512 bytes/sector
    F5_640_512 = 0x0f,      // 5.25",  640KB,  512 bytes/sector
    F5_720_512 = 0x10,      // 5.25",  720KB,  512 bytes/sector
    F3_1Pt2_512 = 0x11,     // 3.5",   1.2Mb,  512 bytes/sector
    F3_1Pt23_1024 = 0x12,   // 3.5",   1.23Mb, 1024 bytes/sector
    F5_1Pt23_1024 = 0x13,   // 5.25",  1.23MB, 1024 bytes/sector
    F3_128Mb_512 = 0x14,    // 3.5" MO 128Mb   512 bytes/sector
    F3_230Mb_512 = 0x15,    // 3.5" MO 230Mb   512 bytes/sector
    F8_256_128 = 0x16,      // 8",     256KB,  128 bytes/sector
    F3_200Mb_512 = 0x17,    // 3.5",   200M Floppy (HiFD)
    F3_240M_512 = 0x18,     // 3.5",   240Mb Floppy (HiFD)
    F3_32M_512 = 0x19,      // 3.5",   32Mb Floppy
}

// Partition Information
#[derive(Debug, Clone)]
pub struct PartitionInformation {
    pub starting_offset: u64,
    pub partition_length: u64,
    pub hidden_sectors: u32,
    pub partition_number: u32,
    pub partition_type: u8,
    pub bootable: bool,
    pub recognized_partition: bool,
    pub rewrite_partition: bool,
}

// Storage Device
#[derive(Debug, Clone)]
pub struct StorageDevice {
    pub device_handle: Handle,
    pub device_type: StorageDeviceType,
    pub interface_type: StorageInterface,
    pub media_type: StorageMediaType,
    pub bus_type: StorageBusType,
    pub device_number: u32,
    pub path_id: u8,
    pub target_id: u8,
    pub lun: u8,
    pub vendor_id: String,
    pub product_id: String,
    pub revision: String,
    pub serial_number: String,
    pub geometry: DiskGeometry,
    pub capacity: u64,
    pub block_size: u32,
    pub partitions: Vec<PartitionInformation>,
    pub removable: bool,
    pub read_only: bool,
    pub online: bool,
    pub driver_handle: Handle,
}

// Storage Port Driver
pub trait StoragePortDriver {
    fn initialize(&mut self) -> NtStatus;
    fn find_adapter(&mut self, config: &StorageAdapterConfig) -> NtStatus;
    fn hw_initialize(&mut self) -> bool;
    fn start_io(&mut self, srb: &mut StorageRequestBlock) -> bool;
    fn interrupt(&mut self) -> bool;
    fn reset_bus(&mut self, path_id: u8) -> bool;
    fn adapter_control(&mut self, control_type: ScsiAdapterControlType, parameters: &[u8]) -> bool;
    fn build_io(&mut self, device_object: Handle, irp: &mut Irp) -> NtStatus;
}

#[derive(Debug, Clone)]
pub struct StorageAdapterConfig {
    pub max_transfer_length: u32,
    pub num_physical_breaks: u32,
    pub dma_width: u32,
    pub dma_speed: u32,
    pub alignment_mask: u32,
    pub num_access_ranges: u32,
    pub access_ranges: Vec<AccessRange>,
    pub bus_interrupt_level: u32,
    pub bus_interrupt_vector: u32,
    pub interrupt_mode: InterruptMode,
    pub dma_channel: u32,
    pub dma_port: u32,
    pub master: bool,
    pub cached_buffers: bool,
    pub adapter_interface_type: InterfaceType,
    pub bus_number: u32,
    pub slot_number: u32,
}

#[derive(Debug, Clone)]
pub struct AccessRange {
    pub range_start: u64,
    pub range_length: u32,
    pub range_in_memory: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterruptMode {
    LevelSensitive,
    Latched,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterfaceType {
    Internal,
    Isa,
    Eisa,
    MicroChannel,
    TurboChannel,
    PCIBus,
    VMEBus,
    NuBus,
    PCMCIABus,
    CBus,
    MPIBus,
    MPSABus,
    ProcessorInternal,
    InternalPowerBus,
    PNPISABus,
    PNPBus,
    MaximumInterfaceType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScsiAdapterControlType {
    QuerySupportedControlTypes,
    StopAdapter,
    RestartAdapter,
    SetBootConfig,
    SetRunningConfig,
    ScsiQuerySupportedControlTypes,
    ScsiStopAdapter,
    ScsiRestartAdapter,
    ScsiSetBootConfig,
    ScsiSetRunningConfig,
}

// IDE/ATA Controller
pub struct IdeController {
    pub base_address: u16,
    pub control_address: u16,
    pub irq: u8,
    pub devices: [Option<IdeDevice>; 2], // Master and Slave
    pub dma_enabled: bool,
    pub udma_mode: u8,
}

#[derive(Debug, Clone)]
pub struct IdeDevice {
    pub drive_number: u8,
    pub device_type: IdeDeviceType,
    pub cylinders: u32,
    pub heads: u32,
    pub sectors: u32,
    pub capacity: u64,
    pub model: String,
    pub serial: String,
    pub firmware: String,
    pub supports_lba: bool,
    pub supports_lba48: bool,
    pub supports_dma: bool,
    pub supports_udma: bool,
    pub removable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IdeDeviceType {
    Unknown,
    HardDisk,
    CDROM,
    ATAPI,
}

// AHCI (Advanced Host Controller Interface) SATA Controller
pub struct AhciController {
    pub base_address: u64,
    pub num_ports: u8,
    pub ports: Vec<AhciPort>,
    pub command_slots: u8,
    pub supports_64bit: bool,
    pub supports_ncq: bool, // Native Command Queuing
    pub supports_hotplug: bool,
}

#[derive(Debug, Clone)]
pub struct AhciPort {
    pub port_number: u8,
    pub device: Option<SataDevice>,
    pub command_list_base: u64,
    pub fis_base: u64,
    pub interrupt_status: u32,
    pub command_issue: u32,
    pub signature: u32,
    pub sata_status: u32,
    pub sata_control: u32,
    pub sata_error: u32,
}

#[derive(Debug, Clone)]
pub struct SataDevice {
    pub port: u8,
    pub device_type: SataDeviceType,
    pub model: String,
    pub serial: String,
    pub firmware: String,
    pub capacity: u64,
    pub sector_size: u32,
    pub supports_ncq: bool,
    pub queue_depth: u8,
    pub max_lba: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SataDeviceType {
    Unknown,
    HardDisk,
    SolidState,
    ATAPI,
    PacketDevice,
}

// NVMe Controller
pub struct NvmeController {
    pub base_address: u64,
    pub admin_queue: NvmeQueue,
    pub io_queues: Vec<NvmeQueue>,
    pub namespace_count: u32,
    pub namespaces: Vec<NvmeNamespace>,
    pub max_queue_entries: u16,
    pub doorbell_stride: u8,
}

#[derive(Debug, Clone)]
pub struct NvmeQueue {
    pub id: u16,
    pub size: u16,
    pub submission_queue: Vec<NvmeCommand>,
    pub completion_queue: Vec<NvmeCompletion>,
    pub sq_tail: u16,
    pub cq_head: u16,
    pub phase_bit: bool,
}

#[derive(Debug, Clone)]
pub struct NvmeNamespace {
    pub id: u32,
    pub size: u64,
    pub capacity: u64,
    pub utilization: u64,
    pub block_size: u32,
    pub metadata_size: u16,
    pub protection_type: u8,
    pub protection_info_location: u8,
}

#[derive(Debug, Clone)]
pub struct NvmeCommand {
    pub opcode: u8,
    pub flags: u8,
    pub command_id: u16,
    pub namespace_id: u32,
    pub metadata: u64,
    pub prp1: u64,
    pub prp2: u64,
    pub cdw10: u32,
    pub cdw11: u32,
    pub cdw12: u32,
    pub cdw13: u32,
    pub cdw14: u32,
    pub cdw15: u32,
}

#[derive(Debug, Clone)]
pub struct NvmeCompletion {
    pub command_specific: u32,
    pub reserved: u32,
    pub sq_head: u16,
    pub sq_id: u16,
    pub command_id: u16,
    pub status: u16,
}

// Storage Class Driver
pub struct StorageClassDriver {
    pub devices: Vec<StorageDevice>,
    pub next_device_number: u32,
}

impl StorageClassDriver {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            next_device_number: 0,
        }
    }

    pub fn add_device(&mut self, device: StorageDevice) -> NtStatus {
        crate::println!("Storage: Adding device {} - {} {}", 
            device.device_number, device.vendor_id, device.product_id);
        
        self.devices.push(device);
        NtStatus::Success
    }

    pub fn remove_device(&mut self, device_number: u32) -> NtStatus {
        self.devices.retain(|device| device.device_number != device_number);
        crate::println!("Storage: Removed device {}", device_number);
        NtStatus::Success
    }

    pub fn read(&mut self, device_number: u32, offset: u64, buffer: &mut [u8]) -> Result<usize, NtStatus> {
        if let Some(device) = self.devices.iter().find(|d| d.device_number == device_number) {
            if !device.online {
                return Err(NtStatus::DeviceNotReady);
            }

            if device.read_only && offset + buffer.len() as u64 > device.capacity {
                return Err(NtStatus::InvalidParameter);
            }

            crate::println!("Storage: Reading {} bytes from device {} at offset {}", 
                buffer.len(), device_number, offset);

            // Simulate read operation
            // In a real implementation, this would send SCSI commands or equivalent
            Ok(buffer.len())
        } else {
            Err(NtStatus::NoSuchDevice)
        }
    }

    pub fn write(&mut self, device_number: u32, offset: u64, buffer: &[u8]) -> Result<usize, NtStatus> {
        if let Some(device) = self.devices.iter().find(|d| d.device_number == device_number) {
            if !device.online {
                return Err(NtStatus::DeviceNotReady);
            }

            if device.read_only {
                return Err(NtStatus::MediaWriteProtected);
            }

            if offset + buffer.len() as u64 > device.capacity {
                return Err(NtStatus::InvalidParameter);
            }

            crate::println!("Storage: Writing {} bytes to device {} at offset {}", 
                buffer.len(), device_number, offset);

            // Simulate write operation
            Ok(buffer.len())
        } else {
            Err(NtStatus::NoSuchDevice)
        }
    }

    pub fn get_device_geometry(&self, device_number: u32) -> Option<&DiskGeometry> {
        self.devices.iter()
            .find(|d| d.device_number == device_number)
            .map(|d| &d.geometry)
    }

    pub fn get_partition_info(&self, device_number: u32) -> Option<&[PartitionInformation]> {
        self.devices.iter()
            .find(|d| d.device_number == device_number)
            .map(|d| d.partitions.as_slice())
    }

    pub fn flush(&mut self, device_number: u32) -> NtStatus {
        if let Some(device) = self.devices.iter().find(|d| d.device_number == device_number) {
            if !device.online {
                return NtStatus::DeviceNotReady;
            }

            crate::println!("Storage: Flushing device {}", device_number);
            // Send flush cache command
            NtStatus::Success
        } else {
            NtStatus::NoSuchDevice
        }
    }

    pub fn set_device_online(&mut self, device_number: u32, online: bool) -> NtStatus {
        if let Some(device) = self.devices.iter_mut().find(|d| d.device_number == device_number) {
            device.online = online;
            crate::println!("Storage: Device {} is now {}", device_number, 
                if online { "online" } else { "offline" });
            NtStatus::Success
        } else {
            NtStatus::NoSuchDevice
        }
    }
}

impl IdeController {
    pub fn new(base_address: u16, control_address: u16, irq: u8) -> Self {
        Self {
            base_address,
            control_address,
            irq,
            devices: [None, None],
            dma_enabled: false,
            udma_mode: 0,
        }
    }

    pub fn initialize(&mut self) -> NtStatus {
        crate::println!("IDE: Initializing controller at base 0x{:04X}", self.base_address);

        // Reset the controller
        self.reset_controller();

        // Identify devices
        self.identify_devices();

        // Enable DMA if supported
        self.setup_dma();

        crate::println!("IDE: Controller initialized");
        NtStatus::Success
    }

    fn reset_controller(&mut self) {
        crate::println!("IDE: Resetting controller");
        // Send reset command to control register
        // Wait for reset completion
    }

    fn identify_devices(&mut self) {
        // Check master device
        if let Some(device) = self.identify_device(0) {
            crate::println!("IDE: Found master device - {}", device.model);
            self.devices[0] = Some(device);
        }

        // Check slave device
        if let Some(device) = self.identify_device(1) {
            crate::println!("IDE: Found slave device - {}", device.model);
            self.devices[1] = Some(device);
        }
    }

    fn identify_device(&self, drive: u8) -> Option<IdeDevice> {
        crate::println!("IDE: Identifying drive {}", drive);

        // Select drive
        // Send IDENTIFY command
        // Read response

        // Simulated device for demonstration
        Some(IdeDevice {
            drive_number: drive,
            device_type: IdeDeviceType::HardDisk,
            cylinders: 16383,
            heads: 16,
            sectors: 63,
            capacity: 1024 * 1024 * 1024, // 1GB
            model: String::from("Simulated IDE Drive"),
            serial: String::from("SIM123456789"),
            firmware: String::from("1.0"),
            supports_lba: true,
            supports_lba48: false,
            supports_dma: true,
            supports_udma: false,
            removable: false,
        })
    }

    fn setup_dma(&mut self) {
        // Check if DMA is supported by the controller and devices
        let mut dma_supported = true;

        for device in &self.devices {
            if let Some(ref dev) = device {
                if !dev.supports_dma {
                    dma_supported = false;
                    break;
                }
            }
        }

        if dma_supported {
            self.dma_enabled = true;
            crate::println!("IDE: DMA enabled");
        } else {
            crate::println!("IDE: Using PIO mode");
        }
    }

    pub fn read_sectors(&self, drive: u8, lba: u64, count: u16, buffer: &mut [u8]) -> NtStatus {
        if drive >= 2 || self.devices[drive as usize].is_none() {
            return NtStatus::NoSuchDevice;
        }

        crate::println!("IDE: Reading {} sectors from drive {} at LBA {}", count, drive, lba);

        if self.dma_enabled {
            self.read_dma(drive, lba, count, buffer)
        } else {
            self.read_pio(drive, lba, count, buffer)
        }
    }

    pub fn write_sectors(&self, drive: u8, lba: u64, count: u16, buffer: &[u8]) -> NtStatus {
        if drive >= 2 || self.devices[drive as usize].is_none() {
            return NtStatus::NoSuchDevice;
        }

        let device = self.devices[drive as usize].as_ref().unwrap();
        if device.removable {
            // Check if media is present and writable
        }

        crate::println!("IDE: Writing {} sectors to drive {} at LBA {}", count, drive, lba);

        if self.dma_enabled {
            self.write_dma(drive, lba, count, buffer)
        } else {
            self.write_pio(drive, lba, count, buffer)
        }
    }

    fn read_pio(&self, drive: u8, lba: u64, count: u16, buffer: &mut [u8]) -> NtStatus {
        crate::println!("IDE: PIO read operation");
        // Implement PIO read
        NtStatus::Success
    }

    fn write_pio(&self, drive: u8, lba: u64, count: u16, buffer: &[u8]) -> NtStatus {
        crate::println!("IDE: PIO write operation");
        // Implement PIO write
        NtStatus::Success
    }

    fn read_dma(&self, drive: u8, lba: u64, count: u16, buffer: &mut [u8]) -> NtStatus {
        crate::println!("IDE: DMA read operation");
        // Implement DMA read
        NtStatus::Success
    }

    fn write_dma(&self, drive: u8, lba: u64, count: u16, buffer: &[u8]) -> NtStatus {
        crate::println!("IDE: DMA write operation");
        // Implement DMA write
        NtStatus::Success
    }
}

impl AhciController {
    pub fn new(base_address: u64) -> Self {
        Self {
            base_address,
            num_ports: 0,
            ports: Vec::new(),
            command_slots: 32,
            supports_64bit: true,
            supports_ncq: true,
            supports_hotplug: true,
        }
    }

    pub fn initialize(&mut self) -> NtStatus {
        crate::println!("AHCI: Initializing controller at 0x{:016X}", self.base_address);

        // Read capability register
        self.read_capabilities();

        // Reset HBA
        self.reset_hba();

        // Initialize ports
        self.initialize_ports();

        // Enable AHCI mode
        self.enable_ahci_mode();

        crate::println!("AHCI: Controller initialized with {} ports", self.num_ports);
        NtStatus::Success
    }

    fn read_capabilities(&mut self) {
        // Read HBA capabilities from memory-mapped registers
        self.num_ports = 4; // Simulated
        self.command_slots = 32;
        self.supports_64bit = true;
        self.supports_ncq = true;
        self.supports_hotplug = true;

        crate::println!("AHCI: Capabilities - {} ports, {} command slots, NCQ: {}, 64-bit: {}",
            self.num_ports, self.command_slots, self.supports_ncq, self.supports_64bit);
    }

    fn reset_hba(&mut self) {
        crate::println!("AHCI: Resetting HBA");
        // Perform HBA reset
    }

    fn initialize_ports(&mut self) {
        self.ports.clear();

        for port_num in 0..self.num_ports {
            let mut port = AhciPort {
                port_number: port_num,
                device: None,
                command_list_base: 0,
                fis_base: 0,
                interrupt_status: 0,
                command_issue: 0,
                signature: 0,
                sata_status: 0,
                sata_control: 0,
                sata_error: 0,
            };

            // Check if device is connected
            if self.is_port_connected(port_num) {
                if let Some(device) = self.identify_sata_device(port_num) {
                    crate::println!("AHCI: Port {} - {} {}", port_num, device.model, device.serial);
                    port.device = Some(device);
                }
            }

            self.ports.push(port);
        }
    }

    fn enable_ahci_mode(&mut self) {
        crate::println!("AHCI: Enabling AHCI mode");
        // Set AHCI enable bit in Global HBA Control register
    }

    fn is_port_connected(&self, port: u8) -> bool {
        // Check SATA status register for device presence
        // For simulation, assume ports 0 and 1 have devices
        port < 2
    }

    fn identify_sata_device(&self, port: u8) -> Option<SataDevice> {
        crate::println!("AHCI: Identifying device on port {}", port);

        // Send IDENTIFY DEVICE command
        // Parse response

        // Simulated device
        Some(SataDevice {
            port,
            device_type: if port == 0 { SataDeviceType::HardDisk } else { SataDeviceType::SolidState },
            model: String::from("Simulated SATA Device"),
            serial: String::from("SATA123456789"),
            firmware: String::from("1.0"),
            capacity: 500 * 1024 * 1024 * 1024, // 500GB
            sector_size: 512,
            supports_ncq: true,
            queue_depth: 32,
            max_lba: (500 * 1024 * 1024 * 1024) / 512 - 1,
        })
    }

    pub fn read_sectors(&self, port: u8, lba: u64, count: u16, buffer: &mut [u8]) -> NtStatus {
        if port >= self.num_ports {
            return NtStatus::InvalidParameter;
        }

        if let Some(ref port_info) = self.ports.get(port as usize) {
            if let Some(ref device) = port_info.device {
                crate::println!("AHCI: Reading {} sectors from port {} at LBA {}", count, port, lba);

                // Build command FIS
                // Submit to command list
                // Wait for completion

                return NtStatus::Success;
            }
        }

        NtStatus::NoSuchDevice
    }

    pub fn write_sectors(&self, port: u8, lba: u64, count: u16, buffer: &[u8]) -> NtStatus {
        if port >= self.num_ports {
            return NtStatus::InvalidParameter;
        }

        if let Some(ref port_info) = self.ports.get(port as usize) {
            if let Some(ref device) = port_info.device {
                crate::println!("AHCI: Writing {} sectors to port {} at LBA {}", count, port, lba);

                // Build command FIS
                // Submit to command list
                // Wait for completion

                return NtStatus::Success;
            }
        }

        NtStatus::NoSuchDevice
    }
}

// Storage Subsystem Manager
pub struct StorageSubsystem {
    class_driver: StorageClassDriver,
    ide_controllers: Vec<IdeController>,
    ahci_controllers: Vec<AhciController>,
    nvme_controllers: Vec<NvmeController>,
    device_map: BTreeMap<u32, Handle>,
}

impl StorageSubsystem {
    pub fn new() -> Self {
        Self {
            class_driver: StorageClassDriver::new(),
            ide_controllers: Vec::new(),
            ahci_controllers: Vec::new(),
            nvme_controllers: Vec::new(),
            device_map: BTreeMap::new(),
        }
    }

    pub fn initialize(&mut self) -> NtStatus {
        crate::println!("Storage: Initializing storage subsystem");

        // Detect and initialize storage controllers
        let status = self.detect_controllers();
        if status != NtStatus::Success {
            return status;
        }

        // Initialize IDE controllers
        for controller in &mut self.ide_controllers {
            let status = controller.initialize();
            if status != NtStatus::Success {
                return status;
            }
        }
        
        // Temporarily skip device registration to fix borrow issues
        // TODO: Implement proper device registration without borrowing conflicts
        
        // Initialize AHCI controllers
        for controller in &mut self.ahci_controllers {
            let status = controller.initialize();
            if status != NtStatus::Success {
                return status;
            }
        }

        // Initialize NVMe controllers
        for controller in &mut self.nvme_controllers {
            // controller.initialize()?;
            // self.register_nvme_devices(controller);
        }

        crate::println!("Storage: Subsystem initialized with {} devices", 
            self.class_driver.devices.len());
        NtStatus::Success
    }

    fn detect_controllers(&mut self) -> NtStatus {
        crate::println!("Storage: Detecting storage controllers");

        // Simulate detecting controllers
        // In a real implementation, this would scan PCI bus

        // Add primary IDE controller
        self.ide_controllers.push(IdeController::new(0x1F0, 0x3F6, 14));

        // Add secondary IDE controller  
        self.ide_controllers.push(IdeController::new(0x170, 0x376, 15));

        // Add AHCI controller
        self.ahci_controllers.push(AhciController::new(0xF0000000));

        crate::println!("Storage: Found {} IDE, {} AHCI, {} NVMe controllers",
            self.ide_controllers.len(),
            self.ahci_controllers.len(), 
            self.nvme_controllers.len());

        NtStatus::Success
    }

    fn register_ide_devices(&mut self, controller: &IdeController) {
        for (drive_num, device_opt) in controller.devices.iter().enumerate() {
            if let Some(ref device) = device_opt {
                let storage_device = StorageDevice {
                    device_handle: Handle(0),
                    device_type: match device.device_type {
                        IdeDeviceType::HardDisk => StorageDeviceType::HardDisk,
                        IdeDeviceType::CDROM => StorageDeviceType::OpticalDisk,
                        IdeDeviceType::ATAPI => StorageDeviceType::OpticalDisk,
                        _ => StorageDeviceType::Unknown,
                    },
                    interface_type: StorageInterface::IDE,
                    media_type: if device.removable { 
                        StorageMediaType::Removable 
                    } else { 
                        StorageMediaType::Fixed 
                    },
                    bus_type: StorageBusType::ATA,
                    device_number: self.class_driver.next_device_number,
                    path_id: controller.base_address as u8,
                    target_id: drive_num as u8,
                    lun: 0,
                    vendor_id: String::from("IDE"),
                    product_id: device.model.clone(),
                    revision: device.firmware.clone(),
                    serial_number: device.serial.clone(),
                    geometry: DiskGeometry {
                        cylinders: device.cylinders as u64,
                        media_type: MediaType::FixedMedia,
                        tracks_per_cylinder: device.heads,
                        sectors_per_track: device.sectors,
                        bytes_per_sector: 512,
                    },
                    capacity: device.capacity,
                    block_size: 512,
                    partitions: Vec::new(),
                    removable: device.removable,
                    read_only: false,
                    online: true,
                    driver_handle: Handle(0),
                };

                self.class_driver.next_device_number += 1;
                self.class_driver.add_device(storage_device);
            }
        }
    }

    fn register_ahci_devices(&mut self, controller: &AhciController) {
        for port in &controller.ports {
            if let Some(ref device) = port.device {
                let storage_device = StorageDevice {
                    device_handle: Handle(0),
                    device_type: match device.device_type {
                        SataDeviceType::HardDisk => StorageDeviceType::HardDisk,
                        SataDeviceType::SolidState => StorageDeviceType::SolidStateDrive,
                        SataDeviceType::ATAPI => StorageDeviceType::OpticalDisk,
                        _ => StorageDeviceType::Unknown,
                    },
                    interface_type: StorageInterface::SATA,
                    media_type: StorageMediaType::Fixed,
                    bus_type: StorageBusType::SATA,
                    device_number: self.class_driver.next_device_number,
                    path_id: 0,
                    target_id: device.port,
                    lun: 0,
                    vendor_id: String::from("SATA"),
                    product_id: device.model.clone(),
                    revision: device.firmware.clone(),
                    serial_number: device.serial.clone(),
                    geometry: DiskGeometry {
                        cylinders: 0, // Not applicable for SATA
                        media_type: MediaType::FixedMedia,
                        tracks_per_cylinder: 0,
                        sectors_per_track: 0,
                        bytes_per_sector: device.sector_size,
                    },
                    capacity: device.capacity,
                    block_size: device.sector_size,
                    partitions: Vec::new(),
                    removable: false,
                    read_only: false,
                    online: true,
                    driver_handle: Handle(0),
                };

                self.class_driver.next_device_number += 1;
                self.class_driver.add_device(storage_device);
            }
        }
    }

    pub fn get_device_count(&self) -> usize {
        self.class_driver.devices.len()
    }

    pub fn get_device_info(&self, index: usize) -> Option<String> {
        if let Some(device) = self.class_driver.devices.get(index) {
            Some(format!(
                "Storage Device {}: {} {} ({}GB, {:?})",
                device.device_number,
                device.vendor_id,
                device.product_id,
                device.capacity / (1024 * 1024 * 1024),
                device.device_type
            ))
        } else {
            None
        }
    }

    pub fn read_device(&mut self, device_number: u32, offset: u64, buffer: &mut [u8]) -> Result<usize, NtStatus> {
        self.class_driver.read(device_number, offset, buffer)
    }

    pub fn write_device(&mut self, device_number: u32, offset: u64, buffer: &[u8]) -> Result<usize, NtStatus> {
        self.class_driver.write(device_number, offset, buffer)
    }

    pub fn flush_device(&mut self, device_number: u32) -> NtStatus {
        self.class_driver.flush(device_number)
    }
}

// Global storage subsystem instance
static mut STORAGE_SUBSYSTEM: Option<StorageSubsystem> = None;

pub fn initialize_storage_subsystem() -> NtStatus {
    unsafe {
        STORAGE_SUBSYSTEM = Some(StorageSubsystem::new());
        
        if let Some(ref mut storage) = STORAGE_SUBSYSTEM {
            storage.initialize()
        } else {
            NtStatus::InsufficientResources
        }
    }
}

pub fn get_storage_device_count() -> usize {
    unsafe {
        STORAGE_SUBSYSTEM.as_ref().map_or(0, |storage| storage.get_device_count())
    }
}

pub fn get_storage_device_info(index: usize) -> Option<String> {
    unsafe {
        STORAGE_SUBSYSTEM.as_ref().and_then(|storage| storage.get_device_info(index))
    }
}

pub fn read_storage_device(device_number: u32, offset: u64, buffer: &mut [u8]) -> Result<usize, NtStatus> {
    unsafe {
        if let Some(ref mut storage) = STORAGE_SUBSYSTEM {
            storage.read_device(device_number, offset, buffer)
        } else {
            Err(NtStatus::DeviceNotReady)
        }
    }
}

pub fn write_storage_device(device_number: u32, offset: u64, buffer: &[u8]) -> Result<usize, NtStatus> {
    unsafe {
        if let Some(ref mut storage) = STORAGE_SUBSYSTEM {
            storage.write_device(device_number, offset, buffer)
        } else {
            Err(NtStatus::DeviceNotReady)
        }
    }
}

pub fn flush_storage_device(device_number: u32) -> NtStatus {
    unsafe {
        if let Some(ref mut storage) = STORAGE_SUBSYSTEM {
            storage.flush_device(device_number)
        } else {
            NtStatus::DeviceNotReady
        }
    }
}