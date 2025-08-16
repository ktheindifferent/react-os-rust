// Symbol Resolution for Stack Traces and Debugging
// Maps addresses to function names and source locations

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

pub struct SymbolTable {
    symbols: Mutex<BTreeMap<u64, Symbol>>,
    sorted_addresses: Mutex<Vec<u64>>,
}

#[derive(Clone)]
pub struct Symbol {
    pub address: u64,
    pub size: usize,
    pub name: String,
    pub module: String,
    pub source_file: Option<String>,
    pub line_number: Option<u32>,
    pub symbol_type: SymbolType,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SymbolType {
    Function,
    Global,
    Local,
    Weak,
    Section,
    File,
}

lazy_static! {
    pub static ref SYMBOLS: SymbolTable = SymbolTable::new();
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut table = Self {
            symbols: Mutex::new(BTreeMap::new()),
            sorted_addresses: Mutex::new(Vec::new()),
        };
        
        // Register built-in kernel symbols
        table.register_builtin_symbols();
        table
    }
    
    fn register_builtin_symbols(&mut self) {
        // Register known kernel symbols
        // In a real implementation, these would be loaded from the kernel binary
        
        self.add_symbol(Symbol {
            address: 0x200000,
            size: 0x1000,
            name: "_start".to_string(),
            module: "kernel".to_string(),
            source_file: Some("src/main.rs".to_string()),
            line_number: Some(1),
            symbol_type: SymbolType::Function,
        });
        
        self.add_symbol(Symbol {
            address: 0x201000,
            size: 0x100,
            name: "kernel_main".to_string(),
            module: "kernel".to_string(),
            source_file: Some("src/main.rs".to_string()),
            line_number: Some(50),
            symbol_type: SymbolType::Function,
        });
        
        self.add_symbol(Symbol {
            address: 0x202000,
            size: 0x200,
            name: "panic_handler".to_string(),
            module: "kernel".to_string(),
            source_file: Some("src/main.rs".to_string()),
            line_number: Some(100),
            symbol_type: SymbolType::Function,
        });
        
        // Add more symbols as needed
    }
    
    pub fn add_symbol(&self, symbol: Symbol) {
        let address = symbol.address;
        self.symbols.lock().insert(address, symbol);
        
        // Keep addresses sorted for binary search
        let mut addresses = self.sorted_addresses.lock();
        match addresses.binary_search(&address) {
            Ok(_) => {} // Already exists
            Err(pos) => addresses.insert(pos, address),
        }
    }
    
    pub fn resolve(&self, address: u64) -> Option<ResolvedSymbol> {
        let symbols = self.symbols.lock();
        let addresses = self.sorted_addresses.lock();
        
        // Find the symbol containing this address
        let pos = match addresses.binary_search(&address) {
            Ok(pos) => pos,
            Err(pos) => {
                if pos == 0 {
                    return None;
                }
                pos - 1
            }
        };
        
        // Check if address falls within symbol range
        if let Some(&sym_addr) = addresses.get(pos) {
            if let Some(symbol) = symbols.get(&sym_addr) {
                if address >= symbol.address && address < symbol.address + symbol.size as u64 {
                    let offset = address - symbol.address;
                    return Some(ResolvedSymbol {
                        symbol: symbol.clone(),
                        offset,
                    });
                }
            }
        }
        
        // Check next symbol as well
        if pos + 1 < addresses.len() {
            if let Some(&sym_addr) = addresses.get(pos + 1) {
                if let Some(symbol) = symbols.get(&sym_addr) {
                    if address >= symbol.address && address < symbol.address + symbol.size as u64 {
                        let offset = address - symbol.address;
                        return Some(ResolvedSymbol {
                            symbol: symbol.clone(),
                            offset,
                        });
                    }
                }
            }
        }
        
        None
    }
    
    pub fn format_address(&self, address: u64) -> String {
        if let Some(resolved) = self.resolve(address) {
            if resolved.offset == 0 {
                format!("{}+0x0", resolved.symbol.name)
            } else {
                format!("{}+{:#x}", resolved.symbol.name, resolved.offset)
            }
        } else {
            format!("{:#x}", address)
        }
    }
    
    pub fn load_symbols_from_elf(&self, elf_data: &[u8]) -> Result<usize, String> {
        // Parse ELF symbol table
        // This would parse the .symtab and .strtab sections
        
        crate::serial_println!("[SYMBOLS] Loading symbols from ELF...");
        
        // Simplified ELF parsing
        if elf_data.len() < 64 {
            return Err("Invalid ELF file".to_string());
        }
        
        // Check ELF magic
        if &elf_data[0..4] != b"\x7fELF" {
            return Err("Not an ELF file".to_string());
        }
        
        // Would parse actual ELF headers and symbol table here
        
        Ok(0)
    }
    
    pub fn print_symbols(&self, limit: Option<usize>) {
        let symbols = self.symbols.lock();
        let mut sorted: Vec<_> = symbols.values().collect();
        sorted.sort_by_key(|s| s.address);
        
        crate::serial_println!("\nKernel Symbol Table:");
        crate::serial_println!("====================");
        
        let count = limit.unwrap_or(sorted.len());
        for symbol in sorted.iter().take(count) {
            crate::serial_println!("{:#018x} {:6} {} [{}]",
                symbol.address,
                symbol.size,
                symbol.name,
                symbol.module);
            
            if let Some(ref file) = symbol.source_file {
                if let Some(line) = symbol.line_number {
                    crate::serial_println!("                      {}:{}", file, line);
                }
            }
        }
        
        if limit.is_some() && sorted.len() > count {
            crate::serial_println!("... and {} more symbols", sorted.len() - count);
        }
        
        crate::serial_println!("Total symbols: {}", sorted.len());
    }
    
    pub fn find_symbol_by_name(&self, name: &str) -> Option<Symbol> {
        self.symbols.lock()
            .values()
            .find(|s| s.name == name)
            .cloned()
    }
    
    pub fn get_function_bounds(&self, address: u64) -> Option<(u64, u64)> {
        if let Some(resolved) = self.resolve(address) {
            let start = resolved.symbol.address;
            let end = start + resolved.symbol.size as u64;
            Some((start, end))
        } else {
            None
        }
    }
}

pub struct ResolvedSymbol {
    pub symbol: Symbol,
    pub offset: u64,
}

impl ResolvedSymbol {
    pub fn format(&self) -> String {
        if self.offset == 0 {
            self.symbol.name.clone()
        } else {
            format!("{}+{:#x}", self.symbol.name, self.offset)
        }
    }
    
    pub fn format_with_source(&self) -> String {
        let mut result = self.format();
        
        if let Some(ref file) = self.symbol.source_file {
            if let Some(line) = self.symbol.line_number {
                result.push_str(&format!(" at {}:{}", file, line));
            }
        }
        
        result
    }
}

// Kallsyms-like interface for runtime symbol loading
pub mod kallsyms {
    use super::*;
    
    pub fn load_runtime_symbols() {
        crate::serial_println!("[KALLSYMS] Loading runtime symbols...");
        
        // In a real kernel, this would:
        // 1. Read symbols from a compressed table in the kernel image
        // 2. Decompress and parse the symbol data
        // 3. Build the runtime symbol table
        
        // Add some example runtime symbols
        SYMBOLS.add_symbol(Symbol {
            address: 0x210000,
            size: 0x50,
            name: "schedule".to_string(),
            module: "kernel".to_string(),
            source_file: Some("src/process/scheduler.rs".to_string()),
            line_number: None,
            symbol_type: SymbolType::Function,
        });
        
        SYMBOLS.add_symbol(Symbol {
            address: 0x211000,
            size: 0x100,
            name: "do_page_fault".to_string(),
            module: "kernel".to_string(),
            source_file: Some("src/memory/paging.rs".to_string()),
            line_number: None,
            symbol_type: SymbolType::Function,
        });
        
        crate::serial_println!("[KALLSYMS] Loaded runtime symbols");
    }
    
    pub fn sprint_symbol(address: u64) -> String {
        SYMBOLS.format_address(address)
    }
}

// DWARF debug info support for source-level debugging
pub mod dwarf {
    use super::*;
    
    pub struct DebugInfo {
        pub compile_units: Vec<CompileUnit>,
        pub line_info: BTreeMap<u64, LineInfo>,
    }
    
    pub struct CompileUnit {
        pub name: String,
        pub low_pc: u64,
        pub high_pc: u64,
        pub comp_dir: String,
    }
    
    pub struct LineInfo {
        pub file: String,
        pub line: u32,
        pub column: u32,
        pub is_stmt: bool,
    }
    
    pub fn load_debug_info(_debug_data: &[u8]) -> Result<DebugInfo, String> {
        // Would parse DWARF debug information
        // This enables source-level debugging with line numbers
        
        Ok(DebugInfo {
            compile_units: Vec::new(),
            line_info: BTreeMap::new(),
        })
    }
}

// Public API
pub fn init() {
    kallsyms::load_runtime_symbols();
    crate::serial_println!("[SYMBOLS] Symbol resolution initialized");
}

pub fn resolve_address(address: u64) -> Option<String> {
    SYMBOLS.resolve(address).map(|r| r.format())
}

pub fn format_address(address: u64) -> String {
    SYMBOLS.format_address(address)
}

pub fn print_symbols(limit: Option<usize>) {
    SYMBOLS.print_symbols(limit);
}

pub fn add_module_symbols(module_name: &str, symbols: Vec<Symbol>) {
    let count = symbols.len();
    for mut symbol in symbols {
        symbol.module = module_name.to_string();
        SYMBOLS.add_symbol(symbol);
    }
    
    crate::serial_println!("[SYMBOLS] Added {} symbols for module {}", 
        count, module_name);
}