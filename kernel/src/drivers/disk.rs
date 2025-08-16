// Disk driver interface and ATA/IDE implementation
use alloc::{vec::Vec, string::{String, ToString}, boxed::Box};
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

// Disk sector size (standard)
pub const SECTOR_SIZE: usize = 512;

// ATA/IDE ports
const ATA_PRIMARY_BASE: u16 = 0x1F0;
const ATA_PRIMARY_CTRL: u16 = 0x3F6;
const ATA_SECONDARY_BASE: u16 = 0x170;
const ATA_SECONDARY_CTRL: u16 = 0x376;

// ATA commands
const ATA_CMD_READ_SECTORS: u8 = 0x20;
const ATA_CMD_WRITE_SECTORS: u8 = 0x30;
const ATA_CMD_IDENTIFY: u8 = 0xEC;

// ATA status bits
const ATA_STATUS_ERR: u8 = 0x01;
const ATA_STATUS_DRQ: u8 = 0x08;
const ATA_STATUS_BSY: u8 = 0x80;

// Disk information
#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub name: String,
    pub sectors: u64,
    pub sector_size: usize,
    pub model: String,
    pub serial: String,
}

// Generic disk driver trait
pub trait DiskDriver: Send + Sync {
    fn read_sectors(&mut self, start_sector: u64, count: u32, buffer: &mut [u8]) -> Result<(), DiskError>;
    fn write_sectors(&mut self, start_sector: u64, count: u32, data: &[u8]) -> Result<(), DiskError>;
    fn get_info(&self) -> DiskInfo;
}

#[derive(Debug)]
pub enum DiskError {
    NotFound,
    IoError,
    InvalidSector,
    BufferTooSmall,
}

// ATA/IDE disk driver
pub struct AtaDisk {
    base_port: u16,
    control_port: u16,
    is_master: bool,
    info: DiskInfo,
    
    // Port objects
    data_port: Port<u16>,
    error_port: PortReadOnly<u8>,
    features_port: PortWriteOnly<u8>,
    sector_count_port: Port<u8>,
    lba_low_port: Port<u8>,
    lba_mid_port: Port<u8>,
    lba_high_port: Port<u8>,
    drive_port: Port<u8>,
    status_port: PortReadOnly<u8>,
    command_port: PortWriteOnly<u8>,
}

impl AtaDisk {
    pub fn new(base_port: u16, control_port: u16, is_master: bool) -> Self {
        let mut disk = Self {
            base_port,
            control_port,
            is_master,
            info: DiskInfo {
                name: String::from(if is_master { "Primary Master" } else { "Primary Slave" }),
                sectors: 0,
                sector_size: SECTOR_SIZE,
                model: String::new(),
                serial: String::new(),
            },
            data_port: Port::new(base_port),
            error_port: PortReadOnly::new(base_port + 1),
            features_port: PortWriteOnly::new(base_port + 1),
            sector_count_port: Port::new(base_port + 2),
            lba_low_port: Port::new(base_port + 3),
            lba_mid_port: Port::new(base_port + 4),
            lba_high_port: Port::new(base_port + 5),
            drive_port: Port::new(base_port + 6),
            status_port: PortReadOnly::new(base_port + 7),
            command_port: PortWriteOnly::new(base_port + 7),
        };
        
        // Try to identify the disk
        match disk.identify() {
            Ok(()) => {
                crate::serial_println!("ATA disk found: {}", disk.info.name);
            }
            Err(e) => {
                crate::serial_println!("Disk {} not found or failed to identify: {:?}", disk.info.name, e);
            }
        }
        
        disk
    }
    
    fn identify(&mut self) -> Result<(), DiskError> {
        unsafe {
            crate::serial_println!("Identifying {} disk...", if self.is_master { "master" } else { "slave" });
            
            // Select drive
            self.drive_port.write(if self.is_master { 0xA0 } else { 0xB0 });
            
            // Small delay after selecting drive
            for _ in 0..100 {
                core::hint::spin_loop();
            }
            
            // Check if drive exists first (early detection)
            let initial_status = self.status_port.read();
            crate::serial_println!("Initial status: 0x{:02X}", initial_status);
            if initial_status == 0 || initial_status == 0xFF {
                // No drive connected
                crate::serial_println!("No drive detected (status 0x{:02X})", initial_status);
                return Err(DiskError::NotFound);
            }
            
            // Send IDENTIFY command
            self.command_port.write(ATA_CMD_IDENTIFY);
            
            // Wait for response with timeout
            match self.wait_ready() {
                Ok(false) | Err(_) => return Err(DiskError::NotFound),
                Ok(true) => {},
            }
            
            // Check if drive exists
            let status = self.status_port.read();
            if status == 0 || status == 0xFF {
                return Err(DiskError::NotFound);
            }
            
            // Wait for data to be ready
            self.wait_drq()?;
            
            // Read identification data
            let mut data = [0u16; 256];
            for i in 0..256 {
                data[i] = self.data_port.read();
            }
            
            // Parse identification data
            // Words 60-61: Total sectors (LBA28)
            self.info.sectors = ((data[61] as u64) << 16) | (data[60] as u64);
            
            // Words 27-46: Model string
            let mut model = String::new();
            for i in 27..=46 {
                let bytes = data[i].to_le_bytes();
                model.push(bytes[1] as char);
                model.push(bytes[0] as char);
            }
            self.info.model = model.trim().to_string();
            
            // Words 10-19: Serial number
            let mut serial = String::new();
            for i in 10..=19 {
                let bytes = data[i].to_le_bytes();
                serial.push(bytes[1] as char);
                serial.push(bytes[0] as char);
            }
            self.info.serial = serial.trim().to_string();
        }
        
        Ok(())
    }
    
    fn wait_ready(&mut self) -> Result<bool, DiskError> {
        unsafe {
            // Wait for BSY to clear (very short timeout to prevent hangs)
            for _ in 0..10 {  // Reduced timeout to prevent hangs in QEMU
                let status = self.status_port.read();
                
                // Check for invalid status (no device)
                if status == 0xFF || status == 0 {
                    return Err(DiskError::NotFound);
                }
                
                if status & ATA_STATUS_BSY == 0 {
                    if status & ATA_STATUS_ERR != 0 {
                        return Ok(false);
                    }
                    return Ok(true);
                }
                // Very small delay
                for _ in 0..5 {
                    core::hint::spin_loop();
                }
            }
        }
        Err(DiskError::IoError)
    }
    
    fn wait_drq(&mut self) -> Result<(), DiskError> {
        unsafe {
            for _ in 0..10 {  // Further reduced to prevent hangs
                let status = self.status_port.read();
                
                // Check for invalid status
                if status == 0xFF || status == 0 {
                    return Err(DiskError::NotFound);
                }
                
                if status & ATA_STATUS_DRQ != 0 {
                    return Ok(());
                }
                if status & ATA_STATUS_ERR != 0 {
                    return Err(DiskError::IoError);
                }
                // Small delay
                for _ in 0..50 {  // Reduced from 100
                    core::hint::spin_loop();
                }
            }
        }
        Err(DiskError::IoError)
    }
}

impl DiskDriver for AtaDisk {
    fn read_sectors(&mut self, start_sector: u64, count: u32, buffer: &mut [u8]) -> Result<(), DiskError> {
        if start_sector >= self.info.sectors {
            return Err(DiskError::InvalidSector);
        }
        
        if buffer.len() < (count as usize * SECTOR_SIZE) {
            return Err(DiskError::BufferTooSmall);
        }
        
        unsafe {
            // Select drive and LBA mode
            self.drive_port.write(
                (if self.is_master { 0xE0 } else { 0xF0 }) | 
                ((start_sector >> 24) & 0x0F) as u8
            );
            
            // Set sector count
            self.sector_count_port.write(count as u8);
            
            // Set LBA address
            self.lba_low_port.write(start_sector as u8);
            self.lba_mid_port.write((start_sector >> 8) as u8);
            self.lba_high_port.write((start_sector >> 16) as u8);
            
            // Send READ command
            self.command_port.write(ATA_CMD_READ_SECTORS);
            
            // Read sectors
            for sector in 0..count {
                // Wait for data with timeout
                if self.wait_drq().is_err() {
                    crate::serial_println!("Timeout waiting for disk data");
                    return Err(DiskError::IoError);
                }
                
                // Read sector data
                let offset = sector as usize * SECTOR_SIZE;
                for i in (0..SECTOR_SIZE).step_by(2) {
                    let word = self.data_port.read();
                    buffer[offset + i] = word as u8;
                    buffer[offset + i + 1] = (word >> 8) as u8;
                }
            }
        }
        
        Ok(())
    }
    
    fn write_sectors(&mut self, start_sector: u64, count: u32, data: &[u8]) -> Result<(), DiskError> {
        if start_sector >= self.info.sectors {
            return Err(DiskError::InvalidSector);
        }
        
        if data.len() < (count as usize * SECTOR_SIZE) {
            return Err(DiskError::BufferTooSmall);
        }
        
        unsafe {
            // Select drive and LBA mode
            self.drive_port.write(
                (if self.is_master { 0xE0 } else { 0xF0 }) | 
                ((start_sector >> 24) & 0x0F) as u8
            );
            
            // Set sector count
            self.sector_count_port.write(count as u8);
            
            // Set LBA address
            self.lba_low_port.write(start_sector as u8);
            self.lba_mid_port.write((start_sector >> 8) as u8);
            self.lba_high_port.write((start_sector >> 16) as u8);
            
            // Send WRITE command
            self.command_port.write(ATA_CMD_WRITE_SECTORS);
            
            // Write sectors
            for sector in 0..count {
                self.wait_drq()?;
                
                // Write sector data
                let offset = sector as usize * SECTOR_SIZE;
                for i in (0..SECTOR_SIZE).step_by(2) {
                    let word = data[offset + i] as u16 | 
                              ((data[offset + i + 1] as u16) << 8);
                    self.data_port.write(word);
                }
                
                // Wait for write to complete
                self.wait_ready()?;
            }
        }
        
        Ok(())
    }
    
    fn get_info(&self) -> DiskInfo {
        self.info.clone()
    }
}

// Disk manager - manages all disk drivers
pub struct DiskManager {
    disks: Vec<Box<dyn DiskDriver>>,
}

impl DiskManager {
    pub fn new() -> Self {
        Self {
            disks: Vec::new(),
        }
    }
    
    pub fn init(&mut self) {
        crate::serial_println!("Initializing disk drivers...");
        
        // Skip disk detection for now to avoid hangs during boot
        // This can be enabled later when disk detection is more robust
        crate::serial_println!("Skipping disk detection to avoid boot hangs");
        
        // TODO: Implement timeout-based disk detection
        // The ATA identify command can hang on some systems/emulators
        // Need to implement proper timeout handling or async detection
        
        /*
        // Try to detect ATA disks - only check primary master for now
        // Checking non-existent drives causes timeouts
        crate::serial_println!("Checking for primary master disk...");
        let primary_master = AtaDisk::new(ATA_PRIMARY_BASE, ATA_PRIMARY_CTRL, true);
        if primary_master.info.sectors > 0 {
            crate::serial_println!("Found disk: {} ({} sectors)", 
                                   primary_master.info.model, 
                                   primary_master.info.sectors);
            self.disks.push(Box::new(primary_master));
        } else {
            crate::serial_println!("No primary master disk found");
        }
        */
        
        // Skip checking primary slave to avoid hang
        // In QEMU, usually only primary master is present
        // let primary_slave = AtaDisk::new(ATA_PRIMARY_BASE, ATA_PRIMARY_CTRL, false);
        // if primary_slave.info.sectors > 0 {
        //     crate::serial_println!("Found disk: {} ({} sectors)", 
        //                            primary_slave.info.model, 
        //                            primary_slave.info.sectors);
        //     self.disks.push(Box::new(primary_slave));
        // }
        
        crate::serial_println!("Disk driver initialization complete. Found {} disk(s)", self.disks.len());
    }
    
    pub fn get_disk(&mut self, index: usize) -> Option<&mut Box<dyn DiskDriver>> {
        self.disks.get_mut(index)
    }
    
    pub fn disk_count(&self) -> usize {
        self.disks.len()
    }
}

lazy_static! {
    pub static ref DISK_MANAGER: Mutex<DiskManager> = Mutex::new(DiskManager::new());
}