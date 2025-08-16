// NTFS Boot Sector Structure
use core::convert::TryInto;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct NtfsBootSector {
    pub jump: [u8; 3],
    pub oem_id: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub unused1: [u8; 3],
    pub unused2: u16,
    pub media_descriptor: u8,
    pub unused3: u16,
    pub sectors_per_track: u16,
    pub heads: u16,
    pub hidden_sectors: u32,
    pub unused4: u32,
    pub unused5: u32,
    pub total_sectors: u64,
    pub mft_lcn: u64,           // Logical Cluster Number of $MFT
    pub mft_mirr_lcn: u64,      // Logical Cluster Number of $MFTMirr
    pub clusters_per_mft_record: i8,
    pub unused6: [u8; 3],
    pub clusters_per_index_block: i8,
    pub unused7: [u8; 3],
    pub volume_serial: u64,
    pub checksum: u32,
    pub boot_code: [u8; 426],
    pub end_marker: u16,
}

impl NtfsBootSector {
    pub fn parse(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 512 {
            return Err("Boot sector too small");
        }
        
        // Check end marker
        if data[510] != 0x55 || data[511] != 0xAA {
            return Err("Invalid boot sector signature");
        }
        
        // Parse fields manually to handle alignment
        let mut boot_sector = Self {
            jump: [data[0], data[1], data[2]],
            oem_id: data[3..11].try_into().unwrap(),
            bytes_per_sector: u16::from_le_bytes([data[11], data[12]]),
            sectors_per_cluster: data[13],
            reserved_sectors: u16::from_le_bytes([data[14], data[15]]),
            unused1: [data[16], data[17], data[18]],
            unused2: u16::from_le_bytes([data[19], data[20]]),
            media_descriptor: data[21],
            unused3: u16::from_le_bytes([data[22], data[23]]),
            sectors_per_track: u16::from_le_bytes([data[24], data[25]]),
            heads: u16::from_le_bytes([data[26], data[27]]),
            hidden_sectors: u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
            unused4: u32::from_le_bytes([data[32], data[33], data[34], data[35]]),
            unused5: u32::from_le_bytes([data[36], data[37], data[38], data[39]]),
            total_sectors: u64::from_le_bytes([
                data[40], data[41], data[42], data[43],
                data[44], data[45], data[46], data[47],
            ]),
            mft_lcn: u64::from_le_bytes([
                data[48], data[49], data[50], data[51],
                data[52], data[53], data[54], data[55],
            ]),
            mft_mirr_lcn: u64::from_le_bytes([
                data[56], data[57], data[58], data[59],
                data[60], data[61], data[62], data[63],
            ]),
            clusters_per_mft_record: data[64] as i8,
            unused6: [data[65], data[66], data[67]],
            clusters_per_index_block: data[68] as i8,
            unused7: [data[69], data[70], data[71]],
            volume_serial: u64::from_le_bytes([
                data[72], data[73], data[74], data[75],
                data[76], data[77], data[78], data[79],
            ]),
            checksum: u32::from_le_bytes([data[80], data[81], data[82], data[83]]),
            boot_code: [0; 426],
            end_marker: 0xAA55,
        };
        
        // Copy boot code
        boot_sector.boot_code.copy_from_slice(&data[84..510]);
        
        Ok(boot_sector)
    }
    
    pub fn get_mft_record_size(&self) -> u32 {
        if self.clusters_per_mft_record > 0 {
            self.clusters_per_mft_record as u32 * self.sectors_per_cluster as u32 * self.bytes_per_sector as u32
        } else {
            // Negative value means 2^(-value) bytes
            1u32 << (-self.clusters_per_mft_record as u32)
        }
    }
    
    pub fn get_index_block_size(&self) -> u32 {
        if self.clusters_per_index_block > 0 {
            self.clusters_per_index_block as u32 * self.sectors_per_cluster as u32 * self.bytes_per_sector as u32
        } else {
            1u32 << (-self.clusters_per_index_block as u32)
        }
    }
    
    pub fn get_cluster_size(&self) -> u32 {
        self.sectors_per_cluster as u32 * self.bytes_per_sector as u32
    }
}