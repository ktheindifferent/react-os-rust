// AHCI Command Processing

use super::fis;
use core::mem;

// ATA Command Set
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum AtaCommand {
    ReadPio = 0x20,
    ReadPioExt = 0x24,
    ReadDma = 0xC8,
    ReadDmaExt = 0x25,
    WritePio = 0x30,
    WritePioExt = 0x34,
    WriteDma = 0xCA,
    WriteDmaExt = 0x35,
    CacheFlush = 0xE7,
    CacheFlushExt = 0xEA,
    Packet = 0xA0,
    IdentifyPacket = 0xA1,
    Identify = 0xEC,
    SetFeatures = 0xEF,
}

// ATAPI Command Set
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum AtapiCommand {
    Read = 0xA8,
    TestUnitReady = 0x00,
    RequestSense = 0x03,
    FormatUnit = 0x04,
    Inquiry = 0x12,
    StartStop = 0x1B,
    PreventAllow = 0x1E,
    ReadFormatCapacities = 0x23,
    ReadCapacity = 0x25,
    Read10 = 0x28,
    Write10 = 0x2A,
    Seek10 = 0x2B,
    WriteSame10 = 0x41,
    Verify10 = 0x2F,
    ReadToc = 0x43,
}

// Build a READ DMA EXT command
pub fn build_read_dma_ext(lba: u64, count: u16) -> fis::FisRegH2D {
    fis::FisRegH2D {
        fis_type: fis::FIS_TYPE_REG_H2D,
        pmport_c: 0x80, // Command bit set
        command: AtaCommand::ReadDmaExt as u8,
        featurel: 0,
        
        lba0: (lba & 0xFF) as u8,
        lba1: ((lba >> 8) & 0xFF) as u8,
        lba2: ((lba >> 16) & 0xFF) as u8,
        device: 0x40, // LBA mode
        
        lba3: ((lba >> 24) & 0xFF) as u8,
        lba4: ((lba >> 32) & 0xFF) as u8,
        lba5: ((lba >> 40) & 0xFF) as u8,
        featureh: 0,
        
        countl: (count & 0xFF) as u8,
        counth: ((count >> 8) & 0xFF) as u8,
        icc: 0,
        control: 0,
        
        rsv1: [0; 4],
    }
}

// Build a WRITE DMA EXT command
pub fn build_write_dma_ext(lba: u64, count: u16) -> fis::FisRegH2D {
    fis::FisRegH2D {
        fis_type: fis::FIS_TYPE_REG_H2D,
        pmport_c: 0x80, // Command bit set
        command: AtaCommand::WriteDmaExt as u8,
        featurel: 0,
        
        lba0: (lba & 0xFF) as u8,
        lba1: ((lba >> 8) & 0xFF) as u8,
        lba2: ((lba >> 16) & 0xFF) as u8,
        device: 0x40, // LBA mode
        
        lba3: ((lba >> 24) & 0xFF) as u8,
        lba4: ((lba >> 32) & 0xFF) as u8,
        lba5: ((lba >> 40) & 0xFF) as u8,
        featureh: 0,
        
        countl: (count & 0xFF) as u8,
        counth: ((count >> 8) & 0xFF) as u8,
        icc: 0,
        control: 0,
        
        rsv1: [0; 4],
    }
}

// Build an IDENTIFY command
pub fn build_identify() -> fis::FisRegH2D {
    fis::FisRegH2D {
        fis_type: fis::FIS_TYPE_REG_H2D,
        pmport_c: 0x80, // Command bit set
        command: AtaCommand::Identify as u8,
        featurel: 0,
        
        lba0: 0,
        lba1: 0,
        lba2: 0,
        device: 0,
        
        lba3: 0,
        lba4: 0,
        lba5: 0,
        featureh: 0,
        
        countl: 0,
        counth: 0,
        icc: 0,
        control: 0,
        
        rsv1: [0; 4],
    }
}

// Build a FLUSH CACHE EXT command
pub fn build_flush_cache_ext() -> fis::FisRegH2D {
    fis::FisRegH2D {
        fis_type: fis::FIS_TYPE_REG_H2D,
        pmport_c: 0x80, // Command bit set
        command: AtaCommand::CacheFlushExt as u8,
        featurel: 0,
        
        lba0: 0,
        lba1: 0,
        lba2: 0,
        device: 0x40, // LBA mode
        
        lba3: 0,
        lba4: 0,
        lba5: 0,
        featureh: 0,
        
        countl: 0,
        counth: 0,
        icc: 0,
        control: 0,
        
        rsv1: [0; 4],
    }
}

// Check if a command completed successfully
pub fn check_command_status(tfd: u32) -> Result<(), &'static str> {
    let status = (tfd & 0xFF) as u8;
    let error = ((tfd >> 8) & 0xFF) as u8;
    
    if status & fis::ATA_SR_ERR != 0 {
        if error & fis::ATA_ER_AMNF != 0 {
            return Err("Address mark not found");
        }
        if error & fis::ATA_ER_TK0NF != 0 {
            return Err("Track 0 not found");
        }
        if error & fis::ATA_ER_ABRT != 0 {
            return Err("Command aborted");
        }
        if error & fis::ATA_ER_MCR != 0 {
            return Err("Media change requested");
        }
        if error & fis::ATA_ER_IDNF != 0 {
            return Err("ID not found");
        }
        if error & fis::ATA_ER_MC != 0 {
            return Err("Media changed");
        }
        if error & fis::ATA_ER_UNC != 0 {
            return Err("Uncorrectable data error");
        }
        if error & fis::ATA_ER_BBK != 0 {
            return Err("Bad block detected");
        }
        return Err("Unknown error");
    }
    
    if status & fis::ATA_SR_DF != 0 {
        return Err("Drive fault");
    }
    
    if status & fis::ATA_SR_DRQ != 0 {
        return Err("DRQ still set");
    }
    
    if status & fis::ATA_SR_BSY != 0 {
        return Err("Device still busy");
    }
    
    Ok(())
}