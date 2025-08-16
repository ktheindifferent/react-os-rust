// NTFS Attributes Implementation
use alloc::vec::Vec;
use alloc::string::String;

// Standard NTFS Attribute Types
pub const ATTR_TYPE_STANDARD_INFO: u32 = 0x10;
pub const ATTR_TYPE_ATTRIBUTE_LIST: u32 = 0x20;
pub const ATTR_TYPE_FILE_NAME: u32 = 0x30;
pub const ATTR_TYPE_OBJECT_ID: u32 = 0x40;
pub const ATTR_TYPE_SECURITY_DESCRIPTOR: u32 = 0x50;
pub const ATTR_TYPE_VOLUME_NAME: u32 = 0x60;
pub const ATTR_TYPE_VOLUME_INFO: u32 = 0x70;
pub const ATTR_TYPE_DATA: u32 = 0x80;
pub const ATTR_TYPE_INDEX_ROOT: u32 = 0x90;
pub const ATTR_TYPE_INDEX_ALLOCATION: u32 = 0xA0;
pub const ATTR_TYPE_BITMAP: u32 = 0xB0;
pub const ATTR_TYPE_REPARSE_POINT: u32 = 0xC0;
pub const ATTR_TYPE_EA_INFO: u32 = 0xD0;
pub const ATTR_TYPE_EA: u32 = 0xE0;
pub const ATTR_TYPE_PROPERTY_SET: u32 = 0xF0;
pub const ATTR_TYPE_LOGGED_UTIL_STREAM: u32 = 0x100;
pub const ATTR_TYPE_END: u32 = 0xFFFFFFFF;

// Attribute Header (common part)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct AttributeHeader {
    pub type_code: u32,
    pub length: u32,
    pub non_resident: u8,
    pub name_length: u8,
    pub name_offset: u16,
    pub flags: u16,
    pub attribute_id: u16,
}

// Resident Attribute Header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ResidentAttributeHeader {
    pub common: AttributeHeader,
    pub value_length: u32,
    pub value_offset: u16,
    pub indexed_flag: u8,
    pub padding: u8,
}

// Non-Resident Attribute Header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct NonResidentAttributeHeader {
    pub common: AttributeHeader,
    pub start_vcn: u64,
    pub last_vcn: u64,
    pub data_runs_offset: u16,
    pub compression_unit_size: u16,
    pub padding: [u8; 4],
    pub allocated_size: u64,
    pub real_size: u64,
    pub initialized_size: u64,
}

// Data Run
#[derive(Debug, Clone)]
pub struct DataRun {
    pub length: u64,      // Number of clusters
    pub start_lcn: u64,   // Logical Cluster Number
}

// Attribute
#[derive(Debug, Clone)]
pub struct Attribute {
    pub type_code: u32,
    pub name: String,
    pub flags: u16,
    pub content: AttributeContent,
}

#[derive(Debug, Clone)]
pub enum AttributeContent {
    Resident(Vec<u8>),
    NonResident(NonResidentAttribute),
}

#[derive(Debug, Clone)]
pub struct NonResidentAttribute {
    pub start_vcn: u64,
    pub last_vcn: u64,
    pub allocated_size: u64,
    pub real_size: u64,
    pub initialized_size: u64,
    pub data_runs: Vec<DataRun>,
}

// Parse attributes from raw data
pub fn parse_attributes(data: &[u8]) -> Result<Vec<Attribute>, &'static str> {
    let mut attributes = Vec::new();
    let mut offset = 0;
    
    while offset + 8 <= data.len() {
        // Read attribute type
        let type_code = u32::from_le_bytes([
            data[offset], data[offset + 1], 
            data[offset + 2], data[offset + 3],
        ]);
        
        if type_code == ATTR_TYPE_END || type_code == 0 {
            break;
        }
        
        // Read attribute length
        let length = u32::from_le_bytes([
            data[offset + 4], data[offset + 5],
            data[offset + 6], data[offset + 7],
        ]);
        
        if length == 0 || length > data.len() as u32 - offset as u32 {
            break;
        }
        
        // Parse attribute
        let attr_data = &data[offset..offset + length as usize];
        if let Ok(attr) = parse_attribute(attr_data) {
            attributes.push(attr);
        }
        
        // Move to next attribute (aligned to 8 bytes)
        offset += ((length + 7) & !7) as usize;
    }
    
    Ok(attributes)
}

fn parse_attribute(data: &[u8]) -> Result<Attribute, &'static str> {
    if data.len() < 16 {
        return Err("Attribute too small");
    }
    
    let header = AttributeHeader {
        type_code: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
        length: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
        non_resident: data[8],
        name_length: data[9],
        name_offset: u16::from_le_bytes([data[10], data[11]]),
        flags: u16::from_le_bytes([data[12], data[13]]),
        attribute_id: u16::from_le_bytes([data[14], data[15]]),
    };
    
    // Parse name if present
    let name = if header.name_length > 0 {
        let name_offset = header.name_offset as usize;
        let name_len = header.name_length as usize * 2; // UTF-16
        
        if name_offset + name_len <= data.len() {
            parse_utf16_name(&data[name_offset..name_offset + name_len])
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    
    // Parse content
    let content = if header.non_resident == 0 {
        // Resident attribute
        parse_resident_content(data, &header)?
    } else {
        // Non-resident attribute
        parse_non_resident_content(data, &header)?
    };
    
    Ok(Attribute {
        type_code: header.type_code,
        name,
        flags: header.flags,
        content,
    })
}

fn parse_resident_content(data: &[u8], header: &AttributeHeader) -> Result<AttributeContent, &'static str> {
    if data.len() < 24 {
        return Err("Resident attribute too small");
    }
    
    let value_length = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
    let value_offset = u16::from_le_bytes([data[20], data[21]]);
    
    let value_start = value_offset as usize;
    let value_end = value_start + value_length as usize;
    
    if value_end <= data.len() {
        Ok(AttributeContent::Resident(data[value_start..value_end].to_vec()))
    } else {
        Err("Resident attribute value out of bounds")
    }
}

fn parse_non_resident_content(data: &[u8], header: &AttributeHeader) -> Result<AttributeContent, &'static str> {
    if data.len() < 64 {
        return Err("Non-resident attribute too small");
    }
    
    let start_vcn = u64::from_le_bytes([
        data[16], data[17], data[18], data[19],
        data[20], data[21], data[22], data[23],
    ]);
    
    let last_vcn = u64::from_le_bytes([
        data[24], data[25], data[26], data[27],
        data[28], data[29], data[30], data[31],
    ]);
    
    let data_runs_offset = u16::from_le_bytes([data[32], data[33]]);
    
    let allocated_size = u64::from_le_bytes([
        data[40], data[41], data[42], data[43],
        data[44], data[45], data[46], data[47],
    ]);
    
    let real_size = u64::from_le_bytes([
        data[48], data[49], data[50], data[51],
        data[52], data[53], data[54], data[55],
    ]);
    
    let initialized_size = u64::from_le_bytes([
        data[56], data[57], data[58], data[59],
        data[60], data[61], data[62], data[63],
    ]);
    
    // Parse data runs
    let data_runs = if data_runs_offset > 0 {
        parse_data_runs(&data[data_runs_offset as usize..])?
    } else {
        Vec::new()
    };
    
    Ok(AttributeContent::NonResident(NonResidentAttribute {
        start_vcn,
        last_vcn,
        allocated_size,
        real_size,
        initialized_size,
        data_runs,
    }))
}

fn parse_data_runs(data: &[u8]) -> Result<Vec<DataRun>, &'static str> {
    let mut runs = Vec::new();
    let mut offset = 0;
    let mut current_lcn = 0i64;
    
    while offset < data.len() {
        let header = data[offset];
        if header == 0 {
            break;
        }
        
        let length_bytes = (header & 0x0F) as usize;
        let offset_bytes = ((header >> 4) & 0x0F) as usize;
        
        offset += 1;
        
        if offset + length_bytes + offset_bytes > data.len() {
            break;
        }
        
        // Parse length
        let mut length = 0u64;
        for i in 0..length_bytes {
            length |= (data[offset + i] as u64) << (i * 8);
        }
        offset += length_bytes;
        
        // Parse offset (can be negative)
        let mut lcn_offset = 0i64;
        for i in 0..offset_bytes {
            lcn_offset |= (data[offset + i] as i64) << (i * 8);
        }
        
        // Sign extend if necessary
        if offset_bytes > 0 && (data[offset + offset_bytes - 1] & 0x80) != 0 {
            for i in offset_bytes..8 {
                lcn_offset |= 0xFF << (i * 8);
            }
        }
        offset += offset_bytes;
        
        current_lcn += lcn_offset;
        
        runs.push(DataRun {
            length,
            start_lcn: current_lcn as u64,
        });
    }
    
    Ok(runs)
}

fn parse_utf16_name(data: &[u8]) -> String {
    let mut name = String::new();
    
    for i in (0..data.len()).step_by(2) {
        if i + 1 < data.len() {
            let ch = u16::from_le_bytes([data[i], data[i + 1]]);
            if let Some(c) = char::from_u32(ch as u32) {
                name.push(c);
            }
        }
    }
    
    name
}