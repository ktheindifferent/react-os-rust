use super::{NtStatus, object::{Handle, ObjectHeader, ObjectTrait, ObjectType}};
use super::process::{ProcessId, NtProcess, PROCESS_MANAGER};
use crate::memory::{PageProtection, AllocationType};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;
use core::mem;
use x86_64::VirtAddr;

// PE/COFF file format structures - exact Windows compatibility
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImageDosHeader {
    pub e_magic: u16,      // Magic number
    pub e_cblp: u16,       // Bytes on last page of file
    pub e_cp: u16,         // Pages in file
    pub e_crlc: u16,       // Relocations
    pub e_cparhdr: u16,    // Size of header in paragraphs
    pub e_minalloc: u16,   // Minimum extra paragraphs needed
    pub e_maxalloc: u16,   // Maximum extra paragraphs needed
    pub e_ss: u16,         // Initial relative SS value
    pub e_sp: u16,         // Initial SP value
    pub e_csum: u16,       // Checksum
    pub e_ip: u16,         // Initial IP value
    pub e_cs: u16,         // Initial relative CS value
    pub e_lfarlc: u16,     // File address of relocation table
    pub e_ovno: u16,       // Overlay number
    pub e_res: [u16; 4],   // Reserved words
    pub e_oemid: u16,      // OEM identifier (for e_oeminfo)
    pub e_oeminfo: u16,    // OEM information; e_oemid specific
    pub e_res2: [u16; 10], // Reserved words
    pub e_lfanew: u32,     // File address of new exe header
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImageNtHeaders64 {
    pub signature: u32,
    pub file_header: ImageFileHeader,
    pub optional_header: ImageOptionalHeader64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImageFileHeader {
    pub machine: u16,
    pub number_of_sections: u16,
    pub time_date_stamp: u32,
    pub pointer_to_symbol_table: u32,
    pub number_of_symbols: u32,
    pub size_of_optional_header: u16,
    pub characteristics: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImageOptionalHeader64 {
    pub magic: u16,
    pub major_linker_version: u8,
    pub minor_linker_version: u8,
    pub size_of_code: u32,
    pub size_of_initialized_data: u32,
    pub size_of_uninitialized_data: u32,
    pub address_of_entry_point: u32,
    pub base_of_code: u32,
    pub image_base: u64,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub major_operating_system_version: u16,
    pub minor_operating_system_version: u16,
    pub major_image_version: u16,
    pub minor_image_version: u16,
    pub major_subsystem_version: u16,
    pub minor_subsystem_version: u16,
    pub win32_version_value: u32,
    pub size_of_image: u32,
    pub size_of_headers: u32,
    pub checksum: u32,
    pub subsystem: u16,
    pub dll_characteristics: u16,
    pub size_of_stack_reserve: u64,
    pub size_of_stack_commit: u64,
    pub size_of_heap_reserve: u64,
    pub size_of_heap_commit: u64,
    pub loader_flags: u32,
    pub number_of_rva_and_sizes: u32,
    pub data_directory: [ImageDataDirectory; 16],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImageDataDirectory {
    pub virtual_address: u32,
    pub size: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImageSectionHeader {
    pub name: [u8; 8],
    pub virtual_size: u32,
    pub virtual_address: u32,
    pub size_of_raw_data: u32,
    pub pointer_to_raw_data: u32,
    pub pointer_to_relocations: u32,
    pub pointer_to_line_numbers: u32,
    pub number_of_relocations: u16,
    pub number_of_line_numbers: u16,
    pub characteristics: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImageImportDescriptor {
    pub original_first_thunk: u32,
    pub time_date_stamp: u32,
    pub forwarder_chain: u32,
    pub name: u32,
    pub first_thunk: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImageThunkData64 {
    pub u1: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImageImportByName {
    pub hint: u16,
    // name follows as null-terminated string
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImageExportDirectory {
    pub characteristics: u32,
    pub time_date_stamp: u32,
    pub major_version: u16,
    pub minor_version: u16,
    pub name: u32,
    pub base: u32,
    pub number_of_functions: u32,
    pub number_of_names: u32,
    pub address_of_functions: u32,
    pub address_of_names: u32,
    pub address_of_name_ordinals: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImageBaseRelocation {
    pub virtual_address: u32,
    pub size_of_block: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImageReloc {
    pub offset: u16, // Contains both offset and type
}

// PE constants
pub const IMAGE_DOS_SIGNATURE: u16 = 0x5A4D; // MZ
pub const IMAGE_NT_SIGNATURE: u32 = 0x00004550; // PE00

pub const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
pub const IMAGE_FILE_MACHINE_I386: u16 = 0x014c;

pub const IMAGE_NT_OPTIONAL_HDR64_MAGIC: u16 = 0x20b;
pub const IMAGE_NT_OPTIONAL_HDR32_MAGIC: u16 = 0x10b;

// Subsystem types
pub const IMAGE_SUBSYSTEM_NATIVE: u16 = 1;
pub const IMAGE_SUBSYSTEM_WINDOWS_GUI: u16 = 2;
pub const IMAGE_SUBSYSTEM_WINDOWS_CUI: u16 = 3;

// Section characteristics
pub const IMAGE_SCN_CNT_CODE: u32 = 0x00000020;
pub const IMAGE_SCN_CNT_INITIALIZED_DATA: u32 = 0x00000040;
pub const IMAGE_SCN_CNT_UNINITIALIZED_DATA: u32 = 0x00000080;
pub const IMAGE_SCN_MEM_EXECUTE: u32 = 0x20000000;
pub const IMAGE_SCN_MEM_READ: u32 = 0x40000000;
pub const IMAGE_SCN_MEM_WRITE: u32 = 0x80000000;

// Data directory entries
pub const IMAGE_DIRECTORY_ENTRY_EXPORT: usize = 0;
pub const IMAGE_DIRECTORY_ENTRY_IMPORT: usize = 1;
pub const IMAGE_DIRECTORY_ENTRY_RESOURCE: usize = 2;
pub const IMAGE_DIRECTORY_ENTRY_EXCEPTION: usize = 3;
pub const IMAGE_DIRECTORY_ENTRY_SECURITY: usize = 4;
pub const IMAGE_DIRECTORY_ENTRY_BASERELOC: usize = 5;
pub const IMAGE_DIRECTORY_ENTRY_DEBUG: usize = 6;
pub const IMAGE_DIRECTORY_ENTRY_ARCHITECTURE: usize = 7;
pub const IMAGE_DIRECTORY_ENTRY_GLOBALPTR: usize = 8;
pub const IMAGE_DIRECTORY_ENTRY_TLS: usize = 9;
pub const IMAGE_DIRECTORY_ENTRY_LOAD_CONFIG: usize = 10;
pub const IMAGE_DIRECTORY_ENTRY_BOUND_IMPORT: usize = 11;
pub const IMAGE_DIRECTORY_ENTRY_IAT: usize = 12;
pub const IMAGE_DIRECTORY_ENTRY_DELAY_IMPORT: usize = 13;
pub const IMAGE_DIRECTORY_ENTRY_COM_DESCRIPTOR: usize = 14;

// Relocation types
pub const IMAGE_REL_BASED_ABSOLUTE: u16 = 0;
pub const IMAGE_REL_BASED_HIGH: u16 = 1;
pub const IMAGE_REL_BASED_LOW: u16 = 2;
pub const IMAGE_REL_BASED_HIGHLOW: u16 = 3;
pub const IMAGE_REL_BASED_HIGHADJ: u16 = 4;
pub const IMAGE_REL_BASED_DIR64: u16 = 10;

// Loaded module information
#[derive(Debug, Clone)]
pub struct LoadedModule {
    pub base_address: VirtAddr,
    pub size: u32,
    pub entry_point: VirtAddr,
    pub name: String,
    pub file_path: String,
    pub exports: BTreeMap<String, VirtAddr>,
    pub imports: Vec<ImportInfo>,
    pub sections: Vec<SectionInfo>,
    pub is_dll: bool,
    pub reference_count: u32,
}

#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub dll_name: String,
    pub function_name: String,
    pub ordinal: Option<u16>,
    pub address: VirtAddr,
}

#[derive(Debug, Clone)]
pub struct SectionInfo {
    pub name: String,
    pub virtual_address: VirtAddr,
    pub virtual_size: u32,
    pub characteristics: u32,
}

// PE Loader implementation
pub struct PeLoader {
    loaded_modules: BTreeMap<String, LoadedModule>,
    module_load_order: Vec<String>,
    next_load_address: VirtAddr,
    system_dll_paths: Vec<String>,
}

impl PeLoader {
    pub fn new() -> Self {
        Self {
            loaded_modules: BTreeMap::new(),
            module_load_order: Vec::new(),
            next_load_address: VirtAddr::new(0x10000000), // Start at 256MB
            system_dll_paths: alloc::vec![
                "\\Windows\\System32\\".to_string(),
                "\\Windows\\SysWOW64\\".to_string(),
                "\\Windows\\".to_string(),
            ],
        }
    }

    pub fn load_executable(
        &mut self,
        process_id: ProcessId,
        file_data: &[u8],
        file_path: &str,
    ) -> Result<VirtAddr, NtStatus> {
        // Validate PE file format
        let pe_info = self.parse_pe_headers(file_data)?;
        
        // Allocate memory for the image
        let base_address = self.allocate_image_memory(process_id, &pe_info)?;
        
        // Map sections
        self.map_sections(process_id, file_data, &pe_info, base_address)?;
        
        // Process relocations
        self.process_relocations(process_id, file_data, &pe_info, base_address)?;
        
        // Resolve imports
        self.resolve_imports(process_id, file_data, &pe_info, base_address)?;
        
        // Create loaded module info
        let module = LoadedModule {
            base_address,
            size: pe_info.optional_header.size_of_image,
            entry_point: base_address + pe_info.optional_header.address_of_entry_point as u64,
            name: self.extract_filename(file_path),
            file_path: file_path.to_string(),
            exports: BTreeMap::new(), // Will be populated if needed
            imports: Vec::new(),      // Will be populated
            sections: Vec::new(),     // Will be populated
            is_dll: false,
            reference_count: 1,
        };
        
        self.loaded_modules.insert(module.name.clone(), module);
        self.module_load_order.push(self.extract_filename(file_path));
        
        Ok(base_address + pe_info.optional_header.address_of_entry_point as u64)
    }

    pub fn load_dll(
        &mut self,
        process_id: ProcessId,
        dll_name: &str,
    ) -> Result<VirtAddr, NtStatus> {
        // Check if DLL is already loaded
        if let Some(module) = self.loaded_modules.get(dll_name) {
            return Ok(module.base_address);
        }

        // Find DLL file
        let dll_path = self.find_dll_path(dll_name)?;
        
        // Load DLL file data (simplified - would read from filesystem)
        let file_data = self.load_file_data(&dll_path)?;
        
        // Parse PE headers
        let pe_info = self.parse_pe_headers(&file_data)?;
        
        // Allocate memory
        let base_address = self.allocate_image_memory(process_id, &pe_info)?;
        
        // Map sections
        self.map_sections(process_id, &file_data, &pe_info, base_address)?;
        
        // Process relocations
        self.process_relocations(process_id, &file_data, &pe_info, base_address)?;
        
        // Resolve imports (recursive)
        self.resolve_imports(process_id, &file_data, &pe_info, base_address)?;
        
        // Process exports
        let exports = self.process_exports(&file_data, &pe_info, base_address)?;
        
        // Create loaded module
        let module = LoadedModule {
            base_address,
            size: pe_info.optional_header.size_of_image,
            entry_point: if pe_info.optional_header.address_of_entry_point != 0 {
                base_address + pe_info.optional_header.address_of_entry_point as u64
            } else {
                VirtAddr::new(0)
            },
            name: dll_name.to_string(),
            file_path: dll_path,
            exports,
            imports: Vec::new(),
            sections: Vec::new(),
            is_dll: true,
            reference_count: 1,
        };
        
        self.loaded_modules.insert(dll_name.to_string(), module);
        self.module_load_order.push(dll_name.to_string());
        
        Ok(base_address)
    }

    pub fn get_export_address(&self, dll_name: &str, function_name: &str) -> Option<VirtAddr> {
        self.loaded_modules
            .get(dll_name)?
            .exports
            .get(function_name)
            .copied()
    }

    pub fn unload_module(&mut self, module_name: &str) -> NtStatus {
        if let Some(mut module) = self.loaded_modules.remove(module_name) {
            module.reference_count -= 1;
            if module.reference_count == 0 {
                // Actually unload the module
                self.module_load_order.retain(|name| name != module_name);
                // Free memory would happen here
                NtStatus::Success
            } else {
                // Still has references
                self.loaded_modules.insert(module_name.to_string(), module);
                NtStatus::Success
            }
        } else {
            NtStatus::ObjectNameNotFound
        }
    }

    fn parse_pe_headers(&self, file_data: &[u8]) -> Result<ParsedPeInfo, NtStatus> {
        if file_data.len() < mem::size_of::<ImageDosHeader>() {
            return Err(NtStatus::InvalidImageFormat);
        }

        // Parse DOS header
        let dos_header = unsafe {
            &*(file_data.as_ptr() as *const ImageDosHeader)
        };

        if dos_header.e_magic != IMAGE_DOS_SIGNATURE {
            return Err(NtStatus::InvalidImageNotMz);
        }

        let nt_headers_offset = dos_header.e_lfanew as usize;
        if nt_headers_offset + mem::size_of::<ImageNtHeaders64>() > file_data.len() {
            return Err(NtStatus::InvalidImageFormat);
        }

        // Parse NT headers
        let nt_headers = unsafe {
            &*(file_data.as_ptr().add(nt_headers_offset) as *const ImageNtHeaders64)
        };

        if nt_headers.signature != IMAGE_NT_SIGNATURE {
            return Err(NtStatus::InvalidImageFormat);
        }

        // Validate machine type
        if nt_headers.file_header.machine != IMAGE_FILE_MACHINE_AMD64 {
            return Err(NtStatus::InvalidImageFormat);
        }

        // Validate optional header magic
        if nt_headers.optional_header.magic != IMAGE_NT_OPTIONAL_HDR64_MAGIC {
            return Err(NtStatus::InvalidImageFormat);
        }

        Ok(ParsedPeInfo {
            dos_header: *dos_header,
            nt_headers: *nt_headers,
            file_header: nt_headers.file_header,
            optional_header: nt_headers.optional_header,
            sections_offset: nt_headers_offset + mem::size_of::<ImageNtHeaders64>(),
        })
    }

    fn allocate_image_memory(
        &mut self,
        _process_id: ProcessId,
        pe_info: &ParsedPeInfo,
    ) -> Result<VirtAddr, NtStatus> {
        // Simplified implementation - just use next available address
        let preferred_base = VirtAddr::new(pe_info.optional_header.image_base);
        let size = pe_info.optional_header.size_of_image as u64;
        
        // Try preferred base first
        let base_address = if preferred_base.as_u64() >= 0x10000000 {
            preferred_base
        } else {
            // Use fallback address
            let fallback_addr = self.next_load_address;
            self.next_load_address = VirtAddr::new(
                self.next_load_address.as_u64() + size + 0x10000
            );
            fallback_addr
        };
        
        Ok(base_address)
    }

    fn map_sections(
        &self,
        _process_id: ProcessId,
        file_data: &[u8],
        pe_info: &ParsedPeInfo,
        base_address: VirtAddr,
    ) -> Result<(), NtStatus> {
        let sections_ptr = unsafe {
            file_data.as_ptr().add(pe_info.sections_offset) as *const ImageSectionHeader
        };

        for i in 0..pe_info.file_header.number_of_sections {
            let section = unsafe { &*sections_ptr.add(i as usize) };
            
            let section_va = base_address.as_u64() + section.virtual_address as u64;
            let file_offset = section.pointer_to_raw_data as usize;
            let file_size = section.size_of_raw_data as usize;
            let virtual_size = section.virtual_size as usize;
            
            // Map the section data (simplified - in reality would use proper memory mapping)
            if file_offset + file_size <= file_data.len() && virtual_size > 0 {
                // Copy section data to virtual address
                // In a real implementation, this would use proper memory management
                let _source_data = &file_data[file_offset..file_offset + core::cmp::min(file_size, virtual_size)];
                // Copy to section_va...
            }
            
            // Set appropriate page protections based on section characteristics
            let _protection = if section.characteristics & IMAGE_SCN_MEM_EXECUTE != 0 {
                if section.characteristics & IMAGE_SCN_MEM_WRITE != 0 {
                    PageProtection::ExecuteReadWrite
                } else {
                    PageProtection::ExecuteRead
                }
            } else if section.characteristics & IMAGE_SCN_MEM_WRITE != 0 {
                PageProtection::ReadWrite
            } else {
                PageProtection::ReadOnly
            };
            
            // Apply protection (simplified)
        }

        Ok(())
    }

    fn process_relocations(
        &self,
        _process_id: ProcessId,
        file_data: &[u8],
        pe_info: &ParsedPeInfo,
        base_address: VirtAddr,
    ) -> Result<(), NtStatus> {
        let reloc_dir = &pe_info.optional_header.data_directory[IMAGE_DIRECTORY_ENTRY_BASERELOC];
        
        if reloc_dir.virtual_address == 0 || reloc_dir.size == 0 {
            return Ok(()); // No relocations needed
        }

        let image_base_delta = base_address.as_u64() as i64 - pe_info.optional_header.image_base as i64;
        
        if image_base_delta == 0 {
            return Ok(());  // Loaded at preferred base, no relocations needed
        }

        // Process relocation blocks
        let mut offset = reloc_dir.virtual_address as usize;
        let end_offset = offset + reloc_dir.size as usize;
        
        while offset < end_offset {
            if offset + mem::size_of::<ImageBaseRelocation>() > file_data.len() {
                break;
            }
            
            let reloc_block = unsafe {
                &*(file_data.as_ptr().add(offset) as *const ImageBaseRelocation)
            };
            
            if reloc_block.size_of_block < mem::size_of::<ImageBaseRelocation>() as u32 {
                break;
            }
            
            let num_entries = (reloc_block.size_of_block as usize - mem::size_of::<ImageBaseRelocation>()) / 2;
            let entries_ptr = unsafe {
                file_data.as_ptr().add(offset + mem::size_of::<ImageBaseRelocation>()) as *const u16
            };
            
            for i in 0..num_entries {
                let entry = unsafe { *entries_ptr.add(i) };
                let reloc_type = (entry >> 12) & 0xF;
                let reloc_offset = (entry & 0xFFF) as u32;
                
                let target_va = base_address.as_u64() + reloc_block.virtual_address as u64 + reloc_offset as u64;
                
                match reloc_type {
                    IMAGE_REL_BASED_ABSOLUTE => {
                        // No relocation needed
                    }
                    IMAGE_REL_BASED_DIR64 => {
                        // 64-bit relocation
                        // In reality, would modify the target memory location
                        let _target_addr = target_va;
                        // Modify memory at target_addr += image_base_delta
                    }
                    IMAGE_REL_BASED_HIGHLOW => {
                        // 32-bit relocation
                        let _target_addr = target_va;
                        // Modify memory at target_addr += image_base_delta (32-bit)
                    }
                    _ => {
                        // Unsupported relocation type
                        return Err(NtStatus::InvalidImageFormat);
                    }
                }
            }
            
            offset += reloc_block.size_of_block as usize;
        }

        Ok(())
    }

    fn resolve_imports(
        &mut self,
        process_id: ProcessId,
        file_data: &[u8],
        pe_info: &ParsedPeInfo,
        _base_address: VirtAddr,
    ) -> Result<(), NtStatus> {
        let import_dir = &pe_info.optional_header.data_directory[IMAGE_DIRECTORY_ENTRY_IMPORT];
        
        if import_dir.virtual_address == 0 || import_dir.size == 0 {
            return Ok(()); // No imports
        }

        let mut import_desc_offset = import_dir.virtual_address as usize;
        
        loop {
            if import_desc_offset + mem::size_of::<ImageImportDescriptor>() > file_data.len() {
                break;
            }
            
            let import_desc = unsafe {
                &*(file_data.as_ptr().add(import_desc_offset) as *const ImageImportDescriptor)
            };
            
            // Check for end of import descriptors
            if import_desc.name == 0 {
                break;
            }
            
            // Get DLL name
            let dll_name_offset = import_desc.name as usize;
            if dll_name_offset >= file_data.len() {
                break;
            }
            
            let dll_name = self.read_null_terminated_string(&file_data[dll_name_offset..])?;
            
            // Load the DLL if not already loaded
            let _dll_base = self.load_dll(process_id, &dll_name)?;
            
            // Process import thunks
            let mut thunk_offset = import_desc.first_thunk as usize;
            let mut original_thunk_offset = if import_desc.original_first_thunk != 0 {
                import_desc.original_first_thunk as usize
            } else {
                thunk_offset
            };
            
            loop {
                if thunk_offset + mem::size_of::<ImageThunkData64>() > file_data.len() ||
                   original_thunk_offset + mem::size_of::<ImageThunkData64>() > file_data.len() {
                    break;
                }
                
                let original_thunk = unsafe {
                    &*(file_data.as_ptr().add(original_thunk_offset) as *const ImageThunkData64)
                };
                
                if original_thunk.u1 == 0 {
                    break; // End of thunks
                }
                
                let function_address = if (original_thunk.u1 & 0x8000000000000000) != 0 {
                    // Import by ordinal
                    let ordinal = (original_thunk.u1 & 0xFFFF) as u16;
                    self.resolve_import_by_ordinal(&dll_name, ordinal)?
                } else {
                    // Import by name
                    let name_offset = original_thunk.u1 as usize;
                    if name_offset + mem::size_of::<ImageImportByName>() > file_data.len() {
                        return Err(NtStatus::InvalidImageFormat);
                    }
                    
                    let import_name = unsafe {
                        &*(file_data.as_ptr().add(name_offset) as *const ImageImportByName)
                    };
                    
                    let function_name = self.read_null_terminated_string(
                        &file_data[name_offset + mem::size_of::<ImageImportByName>()..]
                    )?;
                    
                    self.resolve_import_by_name(&dll_name, &function_name)?
                };
                
                // Write resolved address to import thunk
                // In reality, would write to the actual memory location
                let _thunk_va = thunk_offset;
                let _resolved_addr = function_address;
                
                thunk_offset += mem::size_of::<ImageThunkData64>();
                original_thunk_offset += mem::size_of::<ImageThunkData64>();
            }
            
            import_desc_offset += mem::size_of::<ImageImportDescriptor>();
        }

        Ok(())
    }

    fn process_exports(
        &self,
        file_data: &[u8],
        pe_info: &ParsedPeInfo,
        base_address: VirtAddr,
    ) -> Result<BTreeMap<String, VirtAddr>, NtStatus> {
        let mut exports = BTreeMap::new();
        
        let export_dir = &pe_info.optional_header.data_directory[IMAGE_DIRECTORY_ENTRY_EXPORT];
        
        if export_dir.virtual_address == 0 || export_dir.size == 0 {
            return Ok(exports); // No exports
        }
        
        let export_desc = unsafe {
            &*(file_data.as_ptr().add(export_dir.virtual_address as usize) as *const ImageExportDirectory)
        };
        
        let functions_ptr = unsafe {
            file_data.as_ptr().add(export_desc.address_of_functions as usize) as *const u32
        };
        
        let names_ptr = unsafe {
            file_data.as_ptr().add(export_desc.address_of_names as usize) as *const u32
        };
        
        let name_ordinals_ptr = unsafe {
            file_data.as_ptr().add(export_desc.address_of_name_ordinals as usize) as *const u16
        };
        
        // Process named exports
        for i in 0..export_desc.number_of_names {
            let name_rva = unsafe { *names_ptr.add(i as usize) };
            let name_ordinal = unsafe { *name_ordinals_ptr.add(i as usize) };
            let function_rva = unsafe { *functions_ptr.add(name_ordinal as usize) };
            
            if let Ok(function_name) = self.read_null_terminated_string(
                &file_data[name_rva as usize..]
            ) {
                let function_address = base_address + function_rva as u64;
                exports.insert(function_name, function_address);
            }
        }
        
        Ok(exports)
    }

    fn resolve_import_by_name(&self, dll_name: &str, function_name: &str) -> Result<VirtAddr, NtStatus> {
        if let Some(address) = self.get_export_address(dll_name, function_name) {
            Ok(address)
        } else {
            // Return a stub address for unresolved imports
            Ok(VirtAddr::new(0x1000)) // Stub
        }
    }

    fn resolve_import_by_ordinal(&self, _dll_name: &str, _ordinal: u16) -> Result<VirtAddr, NtStatus> {
        // Simplified - return stub
        Ok(VirtAddr::new(0x1000))
    }

    fn find_dll_path(&self, dll_name: &str) -> Result<String, NtStatus> {
        for path in &self.system_dll_paths {
            let full_path = format!("{}{}", path, dll_name);
            // In reality, would check if file exists
            return Ok(full_path);
        }
        Err(NtStatus::DllNotFound)
    }

    fn load_file_data(&self, _file_path: &str) -> Result<Vec<u8>, NtStatus> {
        // Simplified - return dummy data
        // In reality, would read from filesystem
        Ok(alloc::vec![0u8; 4096])
    }

    fn read_null_terminated_string(&self, data: &[u8]) -> Result<String, NtStatus> {
        let mut len = 0;
        for &byte in data {
            if byte == 0 {
                break;
            }
            len += 1;
        }
        
        String::from_utf8(data[..len].to_vec())
            .map_err(|_| NtStatus::InvalidParameter)
    }

    fn extract_filename(&self, path: &str) -> String {
        path.split('\\').last()
            .unwrap_or(path)
            .to_string()
    }

    pub fn enumerate_loaded_modules(&self) -> Vec<String> {
        self.module_load_order.clone()
    }

    pub fn get_module_info(&self, module_name: &str) -> Option<&LoadedModule> {
        self.loaded_modules.get(module_name)
    }
}

#[derive(Debug, Clone)]
struct ParsedPeInfo {
    dos_header: ImageDosHeader,
    nt_headers: ImageNtHeaders64,
    file_header: ImageFileHeader,
    optional_header: ImageOptionalHeader64,
    sections_offset: usize,
}

// Global PE loader
lazy_static! {
    pub static ref PE_LOADER: Mutex<PeLoader> = Mutex::new(PeLoader::new());
}

// Public API functions
pub fn load_executable(
    process_id: ProcessId,
    file_data: &[u8],
    file_path: &str,
) -> Result<VirtAddr, NtStatus> {
    let mut loader = PE_LOADER.lock();
    loader.load_executable(process_id, file_data, file_path)
}

pub fn load_dll(process_id: ProcessId, dll_name: &str) -> Result<VirtAddr, NtStatus> {
    let mut loader = PE_LOADER.lock();
    loader.load_dll(process_id, dll_name)
}

pub fn get_export_address(dll_name: &str, function_name: &str) -> Option<VirtAddr> {
    let loader = PE_LOADER.lock();
    loader.get_export_address(dll_name, function_name)
}

pub fn unload_module(module_name: &str) -> NtStatus {
    let mut loader = PE_LOADER.lock();
    loader.unload_module(module_name)
}

pub fn enumerate_loaded_modules() -> Vec<String> {
    let loader = PE_LOADER.lock();
    loader.enumerate_loaded_modules()
}

pub fn get_module_info(module_name: &str) -> Option<LoadedModule> {
    let loader = PE_LOADER.lock();
    loader.get_module_info(module_name).cloned()
}