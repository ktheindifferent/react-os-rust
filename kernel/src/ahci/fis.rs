// FIS (Frame Information Structure) Types for AHCI

// FIS Types
pub const FIS_TYPE_REG_H2D: u8 = 0x27;    // Register FIS - host to device
pub const FIS_TYPE_REG_D2H: u8 = 0x34;    // Register FIS - device to host
pub const FIS_TYPE_DMA_ACT: u8 = 0x39;    // DMA activate FIS - device to host
pub const FIS_TYPE_DMA_SETUP: u8 = 0x41;  // DMA setup FIS - bidirectional
pub const FIS_TYPE_DATA: u8 = 0x46;       // Data FIS - bidirectional
pub const FIS_TYPE_BIST: u8 = 0x58;       // BIST activate FIS - bidirectional
pub const FIS_TYPE_PIO_SETUP: u8 = 0x5F;  // PIO setup FIS - device to host
pub const FIS_TYPE_DEV_BITS: u8 = 0xA1;   // Set device bits FIS - device to host

// Register Host to Device FIS
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FisRegH2D {
    pub fis_type: u8,     // FIS_TYPE_REG_H2D
    pub pmport_c: u8,     // Port multiplier port and C bit
    pub command: u8,      // Command register
    pub featurel: u8,     // Feature register, 7:0
    
    pub lba0: u8,         // LBA low register, 7:0
    pub lba1: u8,         // LBA mid register, 15:8
    pub lba2: u8,         // LBA high register, 23:16
    pub device: u8,       // Device register
    
    pub lba3: u8,         // LBA register, 31:24
    pub lba4: u8,         // LBA register, 39:32
    pub lba5: u8,         // LBA register, 47:40
    pub featureh: u8,     // Feature register, 15:8
    
    pub countl: u8,       // Count register, 7:0
    pub counth: u8,       // Count register, 15:8
    pub icc: u8,          // Isochronous command completion
    pub control: u8,      // Control register
    
    pub rsv1: [u8; 4],    // Reserved
}

// Register Device to Host FIS
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FisRegD2H {
    pub fis_type: u8,     // FIS_TYPE_REG_D2H
    pub pmport_i: u8,     // Port multiplier port and interrupt bit
    pub status: u8,       // Status register
    pub error: u8,        // Error register
    
    pub lba0: u8,         // LBA low register, 7:0
    pub lba1: u8,         // LBA mid register, 15:8
    pub lba2: u8,         // LBA high register, 23:16
    pub device: u8,       // Device register
    
    pub lba3: u8,         // LBA register, 31:24
    pub lba4: u8,         // LBA register, 39:32
    pub lba5: u8,         // LBA register, 47:40
    pub rsv2: u8,         // Reserved
    
    pub countl: u8,       // Count register, 7:0
    pub counth: u8,       // Count register, 15:8
    pub rsv3: [u8; 2],    // Reserved
    
    pub rsv4: [u8; 4],    // Reserved
}

// Data FIS
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FisData {
    pub fis_type: u8,     // FIS_TYPE_DATA
    pub pmport: u8,       // Port multiplier port
    pub rsv1: [u8; 2],    // Reserved
    
    // Minimum 1, maximum 65536 dwords of data
    pub data: [u32; 1],   // Payload
}

// PIO Setup Device to Host FIS
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FisPioSetup {
    pub fis_type: u8,     // FIS_TYPE_PIO_SETUP
    pub pmport_d_i: u8,   // Port multiplier port, direction, and interrupt bit
    pub status: u8,       // Status register
    pub error: u8,        // Error register
    
    pub lba0: u8,         // LBA low register, 7:0
    pub lba1: u8,         // LBA mid register, 15:8
    pub lba2: u8,         // LBA high register, 23:16
    pub device: u8,       // Device register
    
    pub lba3: u8,         // LBA register, 31:24
    pub lba4: u8,         // LBA register, 39:32
    pub lba5: u8,         // LBA register, 47:40
    pub rsv2: u8,         // Reserved
    
    pub countl: u8,       // Count register, 7:0
    pub counth: u8,       // Count register, 15:8
    pub rsv3: u8,         // Reserved
    pub e_status: u8,     // New value of status register
    
    pub tc: u16,          // Transfer count
    pub rsv4: [u8; 2],    // Reserved
}

// DMA Setup Device to Host FIS
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FisDmaSetup {
    pub fis_type: u8,     // FIS_TYPE_DMA_SETUP
    pub pmport_d_i_a: u8, // Port multiplier port, direction, interrupt, and auto-activate
    pub reserved1: [u8; 2], // Reserved
    
    pub dma_buffer_id_low: u32,  // DMA Buffer Identifier low
    pub dma_buffer_id_high: u32, // DMA Buffer Identifier high
    
    pub reserved2: u32,   // Reserved
    
    pub dma_buffer_offset: u32, // Byte offset into buffer
    
    pub transfer_count: u32,     // Number of bytes to transfer
    
    pub reserved3: u32,   // Reserved
}

// Set Device Bits FIS
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FisDevBits {
    pub fis_type: u8,     // FIS_TYPE_DEV_BITS
    pub pmport_i: u8,     // Port multiplier port and interrupt bit
    
    pub status: u8,       // Status register
    pub error: u8,        // Error register
    
    pub protocol: u32,    // Protocol specific
}

// HBA Received FIS Structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HbaFis {
    pub dsfis: FisDmaSetup,    // DMA Setup FIS
    pub pad0: [u8; 4],
    
    pub psfis: FisPioSetup,    // PIO Setup FIS
    pub pad1: [u8; 12],
    
    pub rfis: FisRegD2H,       // Register Device to Host FIS
    pub pad2: [u8; 4],
    
    pub sdbfis: FisDevBits,    // Set Device Bits FIS
    
    pub ufis: [u8; 64],        // Unknown FIS
    
    pub rsv: [u8; 0x100 - 0xA0], // Reserved
}

// Common ATA Commands
pub const ATA_CMD_READ_PIO: u8 = 0x20;
pub const ATA_CMD_READ_PIO_EXT: u8 = 0x24;
pub const ATA_CMD_READ_DMA: u8 = 0xC8;
pub const ATA_CMD_READ_DMA_EXT: u8 = 0x25;
pub const ATA_CMD_WRITE_PIO: u8 = 0x30;
pub const ATA_CMD_WRITE_PIO_EXT: u8 = 0x34;
pub const ATA_CMD_WRITE_DMA: u8 = 0xCA;
pub const ATA_CMD_WRITE_DMA_EXT: u8 = 0x35;
pub const ATA_CMD_CACHE_FLUSH: u8 = 0xE7;
pub const ATA_CMD_CACHE_FLUSH_EXT: u8 = 0xEA;
pub const ATA_CMD_PACKET: u8 = 0xA0;
pub const ATA_CMD_IDENTIFY_PACKET: u8 = 0xA1;
pub const ATA_CMD_IDENTIFY: u8 = 0xEC;

// ATAPI Commands
pub const ATAPI_CMD_READ: u8 = 0xA8;
pub const ATAPI_CMD_EJECT: u8 = 0x1B;

// ATA Status Register Bits
pub const ATA_SR_BSY: u8 = 0x80;  // Busy
pub const ATA_SR_DRDY: u8 = 0x40; // Drive ready
pub const ATA_SR_DF: u8 = 0x20;   // Drive write fault
pub const ATA_SR_DSC: u8 = 0x10;  // Drive seek complete
pub const ATA_SR_DRQ: u8 = 0x08;  // Data request ready
pub const ATA_SR_CORR: u8 = 0x04; // Corrected data
pub const ATA_SR_IDX: u8 = 0x02;  // Index
pub const ATA_SR_ERR: u8 = 0x01;  // Error

// ATA Error Register Bits
pub const ATA_ER_BBK: u8 = 0x80;  // Bad block
pub const ATA_ER_UNC: u8 = 0x40;  // Uncorrectable data
pub const ATA_ER_MC: u8 = 0x20;   // Media changed
pub const ATA_ER_IDNF: u8 = 0x10; // ID mark not found
pub const ATA_ER_MCR: u8 = 0x08;  // Media change request
pub const ATA_ER_ABRT: u8 = 0x04; // Command aborted
pub const ATA_ER_TK0NF: u8 = 0x02; // Track 0 not found
pub const ATA_ER_AMNF: u8 = 0x01; // No address mark