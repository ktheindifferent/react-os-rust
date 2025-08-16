// ELF (Executable and Linkable Format) loader
use alloc::vec::{self, Vec};
use x86_64::{VirtAddr, PhysAddr};

// ELF header constants
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_LSB: u8 = 1;
const ELF_VERSION_CURRENT: u8 = 1;
const ELF_OSABI_NONE: u8 = 0;

// ELF file types
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
pub enum ElfType {
    None = 0,
    Relocatable = 1,
    Executable = 2,
    SharedObject = 3,
    Core = 4,
}

// ELF machine types
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
pub enum ElfMachine {
    None = 0,
    X86_64 = 62,
}

// ELF segment types
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum SegmentType {
    Null = 0,
    Load = 1,
    Dynamic = 2,
    Interp = 3,
    Note = 4,
    Shlib = 5,
    Phdr = 6,
    Tls = 7,
}

// ELF segment flags
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct SegmentFlags: u32 {
        const EXECUTE = 0x1;
        const WRITE = 0x2;
        const READ = 0x4;
    }
}

// ELF header (64-bit)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Elf64Header {
    pub magic: [u8; 4],
    pub class: u8,
    pub data: u8,
    pub version: u8,
    pub osabi: u8,
    pub abiversion: u8,
    pub pad: [u8; 7],
    pub elf_type: u16,
    pub machine: u16,
    pub version2: u32,
    pub entry: u64,          // Entry point virtual address
    pub phoff: u64,          // Program header table offset
    pub shoff: u64,          // Section header table offset
    pub flags: u32,
    pub ehsize: u16,         // ELF header size
    pub phentsize: u16,      // Program header table entry size
    pub phnum: u16,          // Program header table entry count
    pub shentsize: u16,      // Section header table entry size
    pub shnum: u16,          // Section header table entry count
    pub shstrndx: u16,       // Section header string table index
}

// Program header (64-bit)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Elf64ProgramHeader {
    pub segment_type: u32,
    pub flags: u32,
    pub offset: u64,         // Segment file offset
    pub vaddr: u64,          // Segment virtual address
    pub paddr: u64,          // Segment physical address
    pub filesz: u64,         // Segment size in file
    pub memsz: u64,          // Segment size in memory
    pub align: u64,          // Segment alignment
}

// Loaded ELF information
#[derive(Debug)]
pub struct LoadedElf {
    pub entry_point: VirtAddr,
    pub segments: Vec<LoadedSegment>,
    pub base_address: VirtAddr,
    pub end_address: VirtAddr,
}

#[derive(Debug)]
pub struct LoadedSegment {
    pub vaddr: VirtAddr,
    pub size: usize,
    pub flags: SegmentFlags,
    pub data: Vec<u8>,
}

// ELF loader
pub struct ElfLoader;

impl ElfLoader {
    pub fn parse_header(data: &[u8]) -> Result<Elf64Header, &'static str> {
        if data.len() < core::mem::size_of::<Elf64Header>() {
            return Err("File too small for ELF header");
        }
        
        let header = unsafe {
            *(data.as_ptr() as *const Elf64Header)
        };
        
        // Validate magic number
        if header.magic != ELF_MAGIC {
            return Err("Invalid ELF magic number");
        }
        
        // Validate ELF class (64-bit)
        if header.class != ELF_CLASS_64 {
            return Err("Not a 64-bit ELF file");
        }
        
        // Validate data encoding (little-endian)
        if header.data != ELF_DATA_LSB {
            return Err("Not a little-endian ELF file");
        }
        
        // Validate machine type
        if header.machine != ElfMachine::X86_64 as u16 {
            return Err("Not an x86_64 ELF file");
        }
        
        Ok(header)
    }
    
    pub fn load(data: &[u8]) -> Result<LoadedElf, &'static str> {
        let header = Self::parse_header(data)?;
        
        // Check if it's an executable
        if header.elf_type != ElfType::Executable as u16 {
            return Err("Not an executable ELF file");
        }
        
        let mut segments = Vec::new();
        let mut min_vaddr = u64::MAX;
        let mut max_vaddr = 0u64;
        
        // Parse program headers
        for i in 0..header.phnum {
            let ph_offset = header.phoff + (i as u64) * (header.phentsize as u64);
            
            if ph_offset + core::mem::size_of::<Elf64ProgramHeader>() as u64 > data.len() as u64 {
                return Err("Invalid program header offset");
            }
            
            let ph = unsafe {
                *(data.as_ptr().add(ph_offset as usize) as *const Elf64ProgramHeader)
            };
            
            // Only load LOAD segments
            if ph.segment_type != SegmentType::Load as u32 {
                continue;
            }
            
            // Validate segment
            if ph.offset + ph.filesz > data.len() as u64 {
                return Err("Invalid segment offset or size");
            }
            
            // Track address range
            min_vaddr = min_vaddr.min(ph.vaddr);
            max_vaddr = max_vaddr.max(ph.vaddr + ph.memsz);
            
            // Copy segment data
            let mut segment_data = Vec::new();
            segment_data.resize(ph.memsz as usize, 0u8);
            let file_data = &data[ph.offset as usize..(ph.offset + ph.filesz) as usize];
            segment_data[..file_data.len()].copy_from_slice(file_data);
            
            segments.push(LoadedSegment {
                vaddr: VirtAddr::new(ph.vaddr),
                size: ph.memsz as usize,
                flags: SegmentFlags::from_bits_truncate(ph.flags),
                data: segment_data,
            });
        }
        
        if segments.is_empty() {
            return Err("No loadable segments found");
        }
        
        Ok(LoadedElf {
            entry_point: VirtAddr::new(header.entry),
            segments,
            base_address: VirtAddr::new(min_vaddr),
            end_address: VirtAddr::new(max_vaddr),
        })
    }
    
    pub fn validate_elf(data: &[u8]) -> bool {
        Self::parse_header(data).is_ok()
    }
}

// Simple ELF builder for creating test executables
pub struct ElfBuilder {
    segments: Vec<(Vec<u8>, VirtAddr, SegmentFlags)>,
    entry_point: VirtAddr,
}

impl ElfBuilder {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            entry_point: VirtAddr::new(0x400000),  // Default entry
        }
    }
    
    pub fn set_entry_point(&mut self, addr: VirtAddr) {
        self.entry_point = addr;
    }
    
    pub fn add_code_segment(&mut self, code: Vec<u8>, vaddr: VirtAddr) {
        self.segments.push((code, vaddr, SegmentFlags::READ | SegmentFlags::EXECUTE));
    }
    
    pub fn add_data_segment(&mut self, data: Vec<u8>, vaddr: VirtAddr) {
        self.segments.push((data, vaddr, SegmentFlags::READ | SegmentFlags::WRITE));
    }
    
    pub fn build(&self) -> Vec<u8> {
        // This would build a complete ELF file
        // For now, return a minimal valid ELF
        let mut elf = Vec::new();
        
        // ELF header
        elf.extend_from_slice(&ELF_MAGIC);
        elf.push(ELF_CLASS_64);
        elf.push(ELF_DATA_LSB);
        elf.push(ELF_VERSION_CURRENT);
        elf.push(ELF_OSABI_NONE);
        elf.extend_from_slice(&[0; 8]); // padding
        
        // Rest of header (simplified)
        elf.extend_from_slice(&(ElfType::Executable as u16).to_le_bytes());
        elf.extend_from_slice(&(ElfMachine::X86_64 as u16).to_le_bytes());
        elf.extend_from_slice(&1u32.to_le_bytes()); // version
        elf.extend_from_slice(&self.entry_point.as_u64().to_le_bytes());
        
        // Program headers offset (right after ELF header)
        elf.extend_from_slice(&64u64.to_le_bytes());
        elf.extend_from_slice(&0u64.to_le_bytes()); // no section headers
        elf.extend_from_slice(&0u32.to_le_bytes()); // flags
        elf.extend_from_slice(&64u16.to_le_bytes()); // header size
        elf.extend_from_slice(&56u16.to_le_bytes()); // program header size
        elf.extend_from_slice(&(self.segments.len() as u16).to_le_bytes());
        elf.extend_from_slice(&0u16.to_le_bytes()); // section header size
        elf.extend_from_slice(&0u16.to_le_bytes()); // section count
        elf.extend_from_slice(&0u16.to_le_bytes()); // string table index
        
        // Add program headers and segments
        // (simplified - would need proper layout)
        
        elf
    }
}