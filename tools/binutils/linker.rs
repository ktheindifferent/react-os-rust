#![no_std]
#![no_main]

use core::mem;
use core::ptr;
use core::slice;

pub struct Linker {
    output_format: OutputFormat,
    link_mode: LinkMode,
    symbol_resolver: SymbolResolver,
    relocation_processor: RelocationProcessor,
    sections: Vec<Section>,
    entry_point: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Executable,
    SharedLibrary,
    StaticLibrary,
    ObjectFile,
}

#[derive(Debug, Clone, Copy)]
pub enum LinkMode {
    Static,
    Dynamic,
    PartialLink,
}

pub struct SymbolResolver {
    global_symbols: Vec<GlobalSymbol>,
    undefined_symbols: Vec<String>,
    weak_symbols: Vec<WeakSymbol>,
}

pub struct GlobalSymbol {
    name: String,
    address: u64,
    size: u64,
    section: u32,
    visibility: SymbolVisibility,
}

pub struct WeakSymbol {
    name: String,
    default_value: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum SymbolVisibility {
    Default,
    Hidden,
    Protected,
    Internal,
}

pub struct RelocationProcessor {
    relocations: Vec<PendingRelocation>,
}

pub struct PendingRelocation {
    section: u32,
    offset: u64,
    symbol: String,
    reloc_type: RelocationType,
    addend: i64,
}

#[derive(Debug, Clone, Copy)]
pub enum RelocationType {
    Abs32,
    Abs64,
    Rel32,
    Rel64,
    Got,
    Plt,
    Copy,
    GlobDat,
    JumpSlot,
    Relative,
    TlsGd,
    TlsLd,
    TlsDtpMod,
    TlsDtpOff,
    TlsTpOff,
}

pub struct Section {
    name: String,
    virtual_address: u64,
    file_offset: u64,
    size: u64,
    data: Vec<u8>,
    flags: SectionFlags,
}

#[derive(Debug, Clone, Copy)]
pub struct SectionFlags {
    pub alloc: bool,
    pub write: bool,
    pub exec: bool,
    pub merge: bool,
    pub strings: bool,
}

impl Linker {
    pub fn new() -> Self {
        Self {
            output_format: OutputFormat::Executable,
            link_mode: LinkMode::Static,
            symbol_resolver: SymbolResolver::new(),
            relocation_processor: RelocationProcessor::new(),
            sections: Vec::new(),
            entry_point: None,
        }
    }

    pub fn link(&mut self, object_files: Vec<ObjectFile>) -> Result<Vec<u8>, LinkError> {
        self.collect_sections(object_files)?;
        self.resolve_symbols()?;
        self.layout_sections()?;
        self.process_relocations()?;
        self.generate_output()
    }

    fn collect_sections(&mut self, object_files: Vec<ObjectFile>) -> Result<(), LinkError> {
        for obj in object_files {
            for section in obj.sections {
                self.merge_section(section)?;
            }
            
            for symbol in obj.symbols {
                self.add_symbol(symbol)?;
            }
            
            for reloc in obj.relocations {
                self.relocation_processor.add_relocation(reloc);
            }
        }
        
        Ok(())
    }

    fn merge_section(&mut self, new_section: InputSection) -> Result<(), LinkError> {
        if let Some(existing) = self.sections.iter_mut().find(|s| s.name == new_section.name) {
            existing.data.extend_from_slice(&new_section.data);
            existing.size += new_section.data.len() as u64;
        } else {
            self.sections.push(Section {
                name: new_section.name,
                virtual_address: 0,
                file_offset: 0,
                size: new_section.data.len() as u64,
                data: new_section.data,
                flags: new_section.flags,
            });
        }
        
        Ok(())
    }

    fn add_symbol(&mut self, symbol: InputSymbol) -> Result<(), LinkError> {
        match symbol.binding {
            SymbolBinding::Global => {
                if self.symbol_resolver.has_symbol(&symbol.name) {
                    return Err(LinkError::DuplicateSymbol(symbol.name));
                }
                self.symbol_resolver.add_global(symbol);
            }
            SymbolBinding::Weak => {
                self.symbol_resolver.add_weak(symbol);
            }
            SymbolBinding::Local => {
            }
        }
        
        Ok(())
    }

    fn resolve_symbols(&mut self) -> Result<(), LinkError> {
        let undefined = self.symbol_resolver.get_undefined();
        if !undefined.is_empty() {
            return Err(LinkError::UndefinedSymbols(undefined));
        }
        
        if let Some(entry) = self.find_entry_point() {
            self.entry_point = Some(entry);
        } else {
            return Err(LinkError::NoEntryPoint);
        }
        
        Ok(())
    }

    fn find_entry_point(&self) -> Option<u64> {
        self.symbol_resolver.find_symbol("_start")
            .or_else(|| self.symbol_resolver.find_symbol("main"))
    }

    fn layout_sections(&mut self) -> Result<(), LinkError> {
        let mut current_vaddr = 0x400000;
        let mut current_offset = 0x1000;
        
        self.sections.sort_by_key(|s| {
            match (s.flags.exec, s.flags.write) {
                (true, false) => 0,
                (false, false) => 1,
                (false, true) => 2,
                (true, true) => 3,
            }
        });
        
        for section in &mut self.sections {
            section.virtual_address = current_vaddr;
            section.file_offset = current_offset;
            
            current_vaddr = Self::align_up(current_vaddr + section.size, 0x1000);
            current_offset = Self::align_up(current_offset + section.size, 0x100);
        }
        
        Ok(())
    }

    fn align_up(value: u64, alignment: u64) -> u64 {
        (value + alignment - 1) & !(alignment - 1)
    }

    fn process_relocations(&mut self) -> Result<(), LinkError> {
        for reloc in &self.relocation_processor.relocations {
            let symbol_addr = self.symbol_resolver.find_symbol(&reloc.symbol)
                .ok_or_else(|| LinkError::UnresolvedRelocation(reloc.symbol.clone()))?;
            
            let section = self.sections.get_mut(reloc.section as usize)
                .ok_or(LinkError::InvalidSection)?;
            
            self.apply_relocation(section, reloc, symbol_addr)?;
        }
        
        Ok(())
    }

    fn apply_relocation(&self, section: &mut Section, reloc: &PendingRelocation, symbol_addr: u64) -> Result<(), LinkError> {
        let offset = reloc.offset as usize;
        let target_addr = (symbol_addr as i64 + reloc.addend) as u64;
        
        match reloc.reloc_type {
            RelocationType::Abs32 => {
                if offset + 4 <= section.data.len() {
                    let bytes = (target_addr as u32).to_le_bytes();
                    section.data[offset..offset + 4].copy_from_slice(&bytes);
                }
            }
            RelocationType::Abs64 => {
                if offset + 8 <= section.data.len() {
                    let bytes = target_addr.to_le_bytes();
                    section.data[offset..offset + 8].copy_from_slice(&bytes);
                }
            }
            RelocationType::Rel32 => {
                let pc = section.virtual_address + reloc.offset;
                let rel = (target_addr as i64 - pc as i64 - 4) as i32;
                if offset + 4 <= section.data.len() {
                    let bytes = rel.to_le_bytes();
                    section.data[offset..offset + 4].copy_from_slice(&bytes);
                }
            }
            RelocationType::Rel64 => {
                let pc = section.virtual_address + reloc.offset;
                let rel = target_addr as i64 - pc as i64 - 8;
                if offset + 8 <= section.data.len() {
                    let bytes = rel.to_le_bytes();
                    section.data[offset..offset + 8].copy_from_slice(&bytes);
                }
            }
            _ => {}
        }
        
        Ok(())
    }

    fn generate_output(&self) -> Result<Vec<u8>, LinkError> {
        match self.output_format {
            OutputFormat::Executable => self.generate_executable(),
            OutputFormat::SharedLibrary => self.generate_shared_library(),
            OutputFormat::StaticLibrary => self.generate_static_library(),
            OutputFormat::ObjectFile => self.generate_object_file(),
        }
    }

    fn generate_executable(&self) -> Result<Vec<u8>, LinkError> {
        let mut output = Vec::new();
        
        self.write_elf_header(&mut output)?;
        self.write_program_headers(&mut output)?;
        self.write_section_headers(&mut output)?;
        
        for section in &self.sections {
            while output.len() < section.file_offset as usize {
                output.push(0);
            }
            output.extend_from_slice(&section.data);
        }
        
        Ok(output)
    }

    fn write_elf_header(&self, output: &mut Vec<u8>) -> Result<(), LinkError> {
        output.extend_from_slice(&[
            0x7f, b'E', b'L', b'F',
            2,
            1,
            1,
            0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        
        output.extend_from_slice(&[
            2, 0,
            0x3e, 0,
            1, 0, 0, 0,
        ]);
        
        let entry = self.entry_point.unwrap_or(0x400000);
        output.extend_from_slice(&entry.to_le_bytes());
        
        output.extend_from_slice(&[
            0x40, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0x40, 0,
            0x38, 0,
            3, 0,
            0x40, 0,
            0, 0,
            0, 0,
        ]);
        
        Ok(())
    }

    fn write_program_headers(&self, output: &mut Vec<u8>) -> Result<(), LinkError> {
        for section in &self.sections {
            if !section.flags.alloc {
                continue;
            }
            
            let p_type = 1u32;
            let p_flags = Self::section_flags_to_phdr_flags(section.flags);
            
            output.extend_from_slice(&p_type.to_le_bytes());
            output.extend_from_slice(&p_flags.to_le_bytes());
            output.extend_from_slice(&section.file_offset.to_le_bytes());
            output.extend_from_slice(&section.virtual_address.to_le_bytes());
            output.extend_from_slice(&section.virtual_address.to_le_bytes());
            output.extend_from_slice(&section.size.to_le_bytes());
            output.extend_from_slice(&section.size.to_le_bytes());
            output.extend_from_slice(&0x1000u64.to_le_bytes());
        }
        
        Ok(())
    }

    fn section_flags_to_phdr_flags(flags: SectionFlags) -> u32 {
        let mut p_flags = 0u32;
        if flags.exec { p_flags |= 1; }
        if flags.write { p_flags |= 2; }
        p_flags |= 4;
        p_flags
    }

    fn write_section_headers(&self, _output: &mut Vec<u8>) -> Result<(), LinkError> {
        Ok(())
    }

    fn generate_shared_library(&self) -> Result<Vec<u8>, LinkError> {
        Err(LinkError::UnsupportedFormat)
    }

    fn generate_static_library(&self) -> Result<Vec<u8>, LinkError> {
        Err(LinkError::UnsupportedFormat)
    }

    fn generate_object_file(&self) -> Result<Vec<u8>, LinkError> {
        Err(LinkError::UnsupportedFormat)
    }
}

impl SymbolResolver {
    fn new() -> Self {
        Self {
            global_symbols: Vec::new(),
            undefined_symbols: Vec::new(),
            weak_symbols: Vec::new(),
        }
    }

    fn has_symbol(&self, name: &str) -> bool {
        self.global_symbols.iter().any(|s| s.name == name)
    }

    fn add_global(&mut self, symbol: InputSymbol) {
        self.global_symbols.push(GlobalSymbol {
            name: symbol.name,
            address: symbol.value,
            size: symbol.size,
            section: symbol.section,
            visibility: SymbolVisibility::Default,
        });
    }

    fn add_weak(&mut self, symbol: InputSymbol) {
        self.weak_symbols.push(WeakSymbol {
            name: symbol.name,
            default_value: symbol.value,
        });
    }

    fn find_symbol(&self, name: &str) -> Option<u64> {
        self.global_symbols.iter()
            .find(|s| s.name == name)
            .map(|s| s.address)
            .or_else(|| {
                self.weak_symbols.iter()
                    .find(|s| s.name == name)
                    .map(|s| s.default_value)
            })
    }

    fn get_undefined(&self) -> Vec<String> {
        self.undefined_symbols.clone()
    }
}

impl RelocationProcessor {
    fn new() -> Self {
        Self {
            relocations: Vec::new(),
        }
    }

    fn add_relocation(&mut self, reloc: InputRelocation) {
        self.relocations.push(PendingRelocation {
            section: reloc.section,
            offset: reloc.offset,
            symbol: reloc.symbol,
            reloc_type: reloc.reloc_type,
            addend: reloc.addend,
        });
    }
}

pub struct ObjectFile {
    sections: Vec<InputSection>,
    symbols: Vec<InputSymbol>,
    relocations: Vec<InputRelocation>,
}

pub struct InputSection {
    name: String,
    data: Vec<u8>,
    flags: SectionFlags,
}

pub struct InputSymbol {
    name: String,
    value: u64,
    size: u64,
    section: u32,
    binding: SymbolBinding,
}

#[derive(Debug, Clone, Copy)]
pub enum SymbolBinding {
    Local,
    Global,
    Weak,
}

pub struct InputRelocation {
    section: u32,
    offset: u64,
    symbol: String,
    reloc_type: RelocationType,
    addend: i64,
}

#[derive(Debug)]
pub enum LinkError {
    DuplicateSymbol(String),
    UndefinedSymbols(Vec<String>),
    UnresolvedRelocation(String),
    NoEntryPoint,
    InvalidSection,
    UnsupportedFormat,
}

pub struct DynamicLinker {
    got: GlobalOffsetTable,
    plt: ProcedureLinkageTable,
    dynamic_symbols: Vec<DynamicSymbol>,
}

pub struct GlobalOffsetTable {
    entries: Vec<GotEntry>,
}

pub struct GotEntry {
    symbol: String,
    offset: u64,
}

pub struct ProcedureLinkageTable {
    entries: Vec<PltEntry>,
}

pub struct PltEntry {
    symbol: String,
    offset: u64,
}

pub struct DynamicSymbol {
    name: String,
    version: String,
    library: String,
}

impl DynamicLinker {
    pub fn new() -> Self {
        Self {
            got: GlobalOffsetTable { entries: Vec::new() },
            plt: ProcedureLinkageTable { entries: Vec::new() },
            dynamic_symbols: Vec::new(),
        }
    }

    pub fn resolve_dynamic_symbol(&mut self, name: &str) -> Option<u64> {
        None
    }

    pub fn load_shared_library(&mut self, _path: &str) -> Result<(), LinkError> {
        Ok(())
    }
}