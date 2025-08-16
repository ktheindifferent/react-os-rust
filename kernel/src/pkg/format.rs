use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::fmt;
use serde::{Serialize, Deserialize};

pub const RPK_MAGIC: &[u8; 4] = b"RPK\0";
pub const RPK_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageInfo {
    pub name: String,
    pub version: Version,
    pub description: String,
    pub maintainer: String,
    pub homepage: Option<String>,
    pub license: String,
    pub architecture: Architecture,
    pub size: u64,
    pub installed_size: u64,
    pub dependencies: Vec<Dependency>,
    pub conflicts: Vec<String>,
    pub provides: Vec<String>,
    pub replaces: Vec<String>,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    pub build_time: u64,
    pub install_time: Option<u64>,
    pub checksum: String,
    pub signature: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build_metadata: Option<String>,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
            build_metadata: None,
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 3 {
            return Err("Invalid version format".to_string());
        }

        let major = parts[0].parse().map_err(|_| "Invalid major version")?;
        let minor = parts[1].parse().map_err(|_| "Invalid minor version")?;
        
        let (patch_str, pre_release) = if let Some(idx) = parts[2].find('-') {
            let (p, pr) = parts[2].split_at(idx);
            (p, Some(pr[1..].to_string()))
        } else {
            (parts[2], None)
        };

        let (patch_str, build_metadata) = if let Some(idx) = patch_str.find('+') {
            let (p, bm) = patch_str.split_at(idx);
            (p, Some(bm[1..].to_string()))
        } else {
            (patch_str, None)
        };

        let patch = patch_str.parse().map_err(|_| "Invalid patch version")?;

        Ok(Self {
            major,
            minor,
            patch,
            pre_release,
            build_metadata,
        })
    }

    pub fn satisfies(&self, constraint: &VersionConstraint) -> bool {
        constraint.matches(self)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.pre_release {
            write!(f, "-{}", pre)?;
        }
        if let Some(ref build) = self.build_metadata {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    Aarch64,
    Riscv64,
    Any,
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::X86_64 => write!(f, "x86_64"),
            Self::Aarch64 => write!(f, "aarch64"),
            Self::Riscv64 => write!(f, "riscv64"),
            Self::Any => write!(f, "any"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub constraint: VersionConstraint,
    pub optional: bool,
}

impl Dependency {
    pub fn new(name: String, constraint: VersionConstraint) -> Self {
        Self {
            name,
            constraint,
            optional: false,
        }
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VersionConstraint {
    Exact(Version),
    GreaterThan(Version),
    GreaterThanOrEqual(Version),
    LessThan(Version),
    LessThanOrEqual(Version),
    Range(Version, Version),
    Any,
}

impl VersionConstraint {
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            Self::Exact(v) => version == v,
            Self::GreaterThan(v) => version > v,
            Self::GreaterThanOrEqual(v) => version >= v,
            Self::LessThan(v) => version < v,
            Self::LessThanOrEqual(v) => version <= v,
            Self::Range(min, max) => version >= min && version <= max,
            Self::Any => true,
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        if s == "*" || s.is_empty() {
            return Ok(Self::Any);
        }

        if s.starts_with(">=") {
            let version = Version::from_str(&s[2..])?;
            Ok(Self::GreaterThanOrEqual(version))
        } else if s.starts_with(">") {
            let version = Version::from_str(&s[1..])?;
            Ok(Self::GreaterThan(version))
        } else if s.starts_with("<=") {
            let version = Version::from_str(&s[2..])?;
            Ok(Self::LessThanOrEqual(version))
        } else if s.starts_with("<") {
            let version = Version::from_str(&s[1..])?;
            Ok(Self::LessThan(version))
        } else if s.starts_with("=") {
            let version = Version::from_str(&s[1..])?;
            Ok(Self::Exact(version))
        } else if s.contains("..") {
            let parts: Vec<&str> = s.split("..").collect();
            if parts.len() != 2 {
                return Err("Invalid range format".to_string());
            }
            let min = Version::from_str(parts[0])?;
            let max = Version::from_str(parts[1])?;
            Ok(Self::Range(min, max))
        } else {
            let version = Version::from_str(s)?;
            Ok(Self::Exact(version))
        }
    }
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(v) => write!(f, "={}", v),
            Self::GreaterThan(v) => write!(f, ">{}", v),
            Self::GreaterThanOrEqual(v) => write!(f, ">={}", v),
            Self::LessThan(v) => write!(f, "<{}", v),
            Self::LessThanOrEqual(v) => write!(f, "<={}", v),
            Self::Range(min, max) => write!(f, "{}..{}", min, max),
            Self::Any => write!(f, "*"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Package {
    pub info: PackageInfo,
    pub files: Vec<PackageFile>,
    pub scripts: PackageScripts,
    pub config_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PackageFile {
    pub path: String,
    pub size: u64,
    pub mode: u32,
    pub checksum: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct PackageScripts {
    pub pre_install: Option<String>,
    pub post_install: Option<String>,
    pub pre_remove: Option<String>,
    pub post_remove: Option<String>,
    pub pre_upgrade: Option<String>,
    pub post_upgrade: Option<String>,
}

#[repr(C)]
pub struct RpkHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub metadata_offset: u64,
    pub metadata_size: u64,
    pub files_offset: u64,
    pub files_size: u64,
    pub checksum: [u8; 32],
}

impl RpkHeader {
    pub fn new() -> Self {
        Self {
            magic: *RPK_MAGIC,
            version: RPK_VERSION,
            flags: 0,
            metadata_offset: 0,
            metadata_size: 0,
            files_offset: 0,
            files_size: 0,
            checksum: [0; 32],
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if &self.magic != RPK_MAGIC {
            return Err("Invalid RPK magic number".to_string());
        }
        if self.version > RPK_VERSION {
            return Err(format!("Unsupported RPK version: {}", self.version));
        }
        Ok(())
    }
}

pub const RPK_FLAG_COMPRESSED: u16 = 0x0001;
pub const RPK_FLAG_SIGNED: u16 = 0x0002;
pub const RPK_FLAG_ENCRYPTED: u16 = 0x0004;
pub const RPK_FLAG_SPLIT: u16 = 0x0008;

pub fn parse_package(data: &[u8]) -> Result<Package, String> {
    if data.len() < core::mem::size_of::<RpkHeader>() {
        return Err("Package data too small".to_string());
    }

    let header = unsafe {
        &*(data.as_ptr() as *const RpkHeader)
    };
    
    header.validate()?;

    let metadata_start = header.metadata_offset as usize;
    let metadata_end = metadata_start + header.metadata_size as usize;
    
    if metadata_end > data.len() {
        return Err("Invalid metadata bounds".to_string());
    }

    let metadata_data = &data[metadata_start..metadata_end];
    let info: PackageInfo = serde_json::from_slice(metadata_data)
        .map_err(|e| format!("Failed to parse metadata: {:?}", e))?;

    let files_start = header.files_offset as usize;
    let files_end = files_start + header.files_size as usize;
    
    if files_end > data.len() {
        return Err("Invalid files bounds".to_string());
    }

    let files_data = &data[files_start..files_end];
    let files = parse_files(files_data)?;

    Ok(Package {
        info,
        files,
        scripts: PackageScripts::default(),
        config_files: Vec::new(),
    })
}

fn parse_files(data: &[u8]) -> Result<Vec<PackageFile>, String> {
    let mut files = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        if offset + 8 > data.len() {
            break;
        }

        let path_len = u32::from_le_bytes([
            data[offset], data[offset + 1], 
            data[offset + 2], data[offset + 3]
        ]) as usize;
        
        let content_len = u32::from_le_bytes([
            data[offset + 4], data[offset + 5],
            data[offset + 6], data[offset + 7]
        ]) as usize;

        offset += 8;

        if offset + path_len + content_len + 40 > data.len() {
            return Err("Invalid file entry".to_string());
        }

        let path = String::from_utf8(data[offset..offset + path_len].to_vec())
            .map_err(|_| "Invalid file path")?;
        offset += path_len;

        let mode = u32::from_le_bytes([
            data[offset], data[offset + 1],
            data[offset + 2], data[offset + 3]
        ]);
        offset += 4;

        let size = u64::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
            data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]
        ]);
        offset += 8;

        let checksum = hex::encode(&data[offset..offset + 32]);
        offset += 32;

        let content = data[offset..offset + content_len].to_vec();
        offset += content_len;

        files.push(PackageFile {
            path,
            size,
            mode,
            checksum,
            content,
        });
    }

    Ok(files)
}

pub fn create_package(package: &Package) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    let mut header = RpkHeader::new();

    data.extend_from_slice(&[0u8; core::mem::size_of::<RpkHeader>()]);

    let metadata = serde_json::to_vec(&package.info)
        .map_err(|e| format!("Failed to serialize metadata: {:?}", e))?;
    
    header.metadata_offset = data.len() as u64;
    header.metadata_size = metadata.len() as u64;
    data.extend_from_slice(&metadata);

    let files_data = serialize_files(&package.files)?;
    header.files_offset = data.len() as u64;
    header.files_size = files_data.len() as u64;
    data.extend_from_slice(&files_data);

    let header_bytes = unsafe {
        core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<RpkHeader>()
        )
    };
    
    data[..core::mem::size_of::<RpkHeader>()].copy_from_slice(header_bytes);

    Ok(data)
}

fn serialize_files(files: &[PackageFile]) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();

    for file in files {
        let path_bytes = file.path.as_bytes();
        data.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(&(file.content.len() as u32).to_le_bytes());
        data.extend_from_slice(path_bytes);
        data.extend_from_slice(&file.mode.to_le_bytes());
        data.extend_from_slice(&file.size.to_le_bytes());
        
        let checksum_bytes = hex::decode(&file.checksum)
            .map_err(|_| "Invalid checksum")?;
        if checksum_bytes.len() != 32 {
            return Err("Invalid checksum length".to_string());
        }
        data.extend_from_slice(&checksum_bytes);
        data.extend_from_slice(&file.content);
    }

    Ok(data)
}

mod hex {
    use alloc::string::String;
    use alloc::vec::Vec;

    pub fn encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        if s.len() % 2 != 0 {
            return Err(());
        }

        let mut bytes = Vec::with_capacity(s.len() / 2);
        for i in (0..s.len()).step_by(2) {
            let byte = u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ())?;
            bytes.push(byte);
        }
        Ok(bytes)
    }
}