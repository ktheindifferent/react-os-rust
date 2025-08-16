#![no_std]
#![no_main]

use core::mem;
use core::slice;

pub struct Assembler {
    arch: Architecture,
    syntax: SyntaxStyle,
    output_format: ObjectFormat,
    symbol_table: SymbolTable,
    sections: Vec<Section>,
}

#[derive(Debug, Clone, Copy)]
pub enum Architecture {
    X86_64,
    AArch64,
    RiscV64,
}

#[derive(Debug, Clone, Copy)]
pub enum SyntaxStyle {
    Intel,
    Att,
}

#[derive(Debug, Clone, Copy)]
pub enum ObjectFormat {
    Elf64,
    Pe32Plus,
    MachO64,
}

pub struct SymbolTable {
    symbols: Vec<Symbol>,
}

pub struct Symbol {
    name: String,
    section: u32,
    offset: u64,
    size: u64,
    binding: SymbolBinding,
    visibility: SymbolVisibility,
}

#[derive(Debug, Clone, Copy)]
pub enum SymbolBinding {
    Local,
    Global,
    Weak,
}

#[derive(Debug, Clone, Copy)]
pub enum SymbolVisibility {
    Default,
    Hidden,
    Protected,
}

pub struct Section {
    name: String,
    kind: SectionKind,
    flags: SectionFlags,
    data: Vec<u8>,
    relocations: Vec<Relocation>,
}

#[derive(Debug, Clone, Copy)]
pub enum SectionKind {
    Code,
    Data,
    ReadOnlyData,
    Bss,
    Debug,
}

#[derive(Debug, Clone, Copy)]
pub struct SectionFlags {
    pub alloc: bool,
    pub write: bool,
    pub exec: bool,
    pub merge: bool,
    pub strings: bool,
}

pub struct Relocation {
    offset: u64,
    symbol: u32,
    kind: RelocationType,
    addend: i64,
}

#[derive(Debug, Clone, Copy)]
pub enum RelocationType {
    Abs64,
    Rel32,
    Rel64,
    GotRel,
    PltRel,
    TlsGd,
    TlsLd,
    TlsLe,
}

impl Assembler {
    pub fn new(arch: Architecture) -> Self {
        Self {
            arch,
            syntax: SyntaxStyle::Intel,
            output_format: ObjectFormat::Elf64,
            symbol_table: SymbolTable::new(),
            sections: Vec::new(),
        }
    }

    pub fn assemble(&mut self, source: &str) -> Result<Vec<u8>, AssemblerError> {
        let lines = self.preprocess(source)?;
        let instructions = self.parse_instructions(lines)?;
        let machine_code = self.generate_machine_code(instructions)?;
        let object_file = self.create_object_file(machine_code)?;
        Ok(object_file)
    }

    fn preprocess(&self, source: &str) -> Result<Vec<String>, AssemblerError> {
        let mut lines = Vec::new();
        let mut current_line = String::new();
        
        for line in source.lines() {
            let trimmed = line.trim();
            
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
                continue;
            }
            
            if trimmed.ends_with('\\') {
                current_line.push_str(&trimmed[..trimmed.len() - 1]);
                current_line.push(' ');
            } else {
                current_line.push_str(trimmed);
                lines.push(current_line.clone());
                current_line.clear();
            }
        }
        
        Ok(lines)
    }

    fn parse_instructions(&mut self, lines: Vec<String>) -> Result<Vec<Instruction>, AssemblerError> {
        let mut instructions = Vec::new();
        let mut current_section = String::from(".text");
        
        for line in lines {
            if line.starts_with('.') {
                self.handle_directive(line, &mut current_section)?;
            } else if line.contains(':') {
                let parts: Vec<&str> = line.split(':').collect();
                self.add_label(parts[0].trim(), &current_section)?;
                if parts.len() > 1 && !parts[1].trim().is_empty() {
                    instructions.push(self.parse_instruction(parts[1].trim())?);
                }
            } else {
                instructions.push(self.parse_instruction(&line)?);
            }
        }
        
        Ok(instructions)
    }

    fn handle_directive(&mut self, line: String, current_section: &mut String) -> Result<(), AssemblerError> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts[0] {
            ".section" => {
                if parts.len() > 1 {
                    *current_section = parts[1].to_string();
                    self.add_section(current_section.clone())?;
                }
            }
            ".global" | ".globl" => {
                if parts.len() > 1 {
                    self.symbol_table.add_global(parts[1]);
                }
            }
            ".text" => {
                *current_section = String::from(".text");
                self.add_section(current_section.clone())?;
            }
            ".data" => {
                *current_section = String::from(".data");
                self.add_section(current_section.clone())?;
            }
            ".bss" => {
                *current_section = String::from(".bss");
                self.add_section(current_section.clone())?;
            }
            _ => {}
        }
        Ok(())
    }

    fn add_section(&mut self, name: String) -> Result<(), AssemblerError> {
        if !self.sections.iter().any(|s| s.name == name) {
            let kind = match name.as_str() {
                ".text" => SectionKind::Code,
                ".data" => SectionKind::Data,
                ".rodata" => SectionKind::ReadOnlyData,
                ".bss" => SectionKind::Bss,
                _ => SectionKind::Data,
            };
            
            let flags = match kind {
                SectionKind::Code => SectionFlags {
                    alloc: true,
                    write: false,
                    exec: true,
                    merge: false,
                    strings: false,
                },
                SectionKind::Data => SectionFlags {
                    alloc: true,
                    write: true,
                    exec: false,
                    merge: false,
                    strings: false,
                },
                SectionKind::ReadOnlyData => SectionFlags {
                    alloc: true,
                    write: false,
                    exec: false,
                    merge: true,
                    strings: true,
                },
                SectionKind::Bss => SectionFlags {
                    alloc: true,
                    write: true,
                    exec: false,
                    merge: false,
                    strings: false,
                },
                _ => SectionFlags {
                    alloc: false,
                    write: false,
                    exec: false,
                    merge: false,
                    strings: false,
                },
            };
            
            self.sections.push(Section {
                name,
                kind,
                flags,
                data: Vec::new(),
                relocations: Vec::new(),
            });
        }
        Ok(())
    }

    fn add_label(&mut self, label: &str, section: &str) -> Result<(), AssemblerError> {
        let section_idx = self.sections.iter().position(|s| s.name == section).unwrap_or(0);
        let offset = self.sections.get(section_idx).map(|s| s.data.len() as u64).unwrap_or(0);
        
        self.symbol_table.symbols.push(Symbol {
            name: label.to_string(),
            section: section_idx as u32,
            offset,
            size: 0,
            binding: SymbolBinding::Local,
            visibility: SymbolVisibility::Default,
        });
        
        Ok(())
    }

    fn parse_instruction(&self, line: &str) -> Result<Instruction, AssemblerError> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return Err(AssemblerError::InvalidInstruction);
        }
        
        let mnemonic = parts[0].to_lowercase();
        let operands = if parts.len() > 1 {
            self.parse_operands(&parts[1..].join(" "))?
        } else {
            Vec::new()
        };
        
        Ok(Instruction {
            mnemonic,
            operands,
        })
    }

    fn parse_operands(&self, operands_str: &str) -> Result<Vec<Operand>, AssemblerError> {
        let mut operands = Vec::new();
        
        for op_str in operands_str.split(',') {
            let op = self.parse_operand(op_str.trim())?;
            operands.push(op);
        }
        
        Ok(operands)
    }

    fn parse_operand(&self, op_str: &str) -> Result<Operand, AssemblerError> {
        if op_str.starts_with('%') || op_str.starts_with('r') {
            Ok(Operand::Register(self.parse_register(op_str)?))
        } else if op_str.starts_with('$') {
            Ok(Operand::Immediate(self.parse_immediate(&op_str[1..])?))
        } else if op_str.contains('(') {
            Ok(Operand::Memory(self.parse_memory(op_str)?))
        } else if op_str.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            Ok(Operand::Immediate(self.parse_immediate(op_str)?))
        } else {
            Ok(Operand::Label(op_str.to_string()))
        }
    }

    fn parse_register(&self, reg_str: &str) -> Result<Register, AssemblerError> {
        let reg_name = if reg_str.starts_with('%') {
            &reg_str[1..]
        } else {
            reg_str
        };
        
        match self.arch {
            Architecture::X86_64 => X86_64Register::from_str(reg_name),
            _ => Err(AssemblerError::UnsupportedArch),
        }
    }

    fn parse_immediate(&self, imm_str: &str) -> Result<i64, AssemblerError> {
        if imm_str.starts_with("0x") {
            i64::from_str_radix(&imm_str[2..], 16).map_err(|_| AssemblerError::InvalidImmediate)
        } else {
            imm_str.parse().map_err(|_| AssemblerError::InvalidImmediate)
        }
    }

    fn parse_memory(&self, _mem_str: &str) -> Result<MemoryOperand, AssemblerError> {
        Ok(MemoryOperand {
            base: None,
            index: None,
            scale: 1,
            displacement: 0,
        })
    }

    fn generate_machine_code(&mut self, instructions: Vec<Instruction>) -> Result<Vec<u8>, AssemblerError> {
        let mut code = Vec::new();
        
        for inst in instructions {
            let bytes = match self.arch {
                Architecture::X86_64 => self.encode_x86_64(inst)?,
                _ => return Err(AssemblerError::UnsupportedArch),
            };
            code.extend_from_slice(&bytes);
        }
        
        Ok(code)
    }

    fn encode_x86_64(&self, inst: Instruction) -> Result<Vec<u8>, AssemblerError> {
        let encoder = X86_64Encoder::new();
        encoder.encode(inst)
    }

    fn create_object_file(&self, machine_code: Vec<u8>) -> Result<Vec<u8>, AssemblerError> {
        match self.output_format {
            ObjectFormat::Elf64 => self.create_elf64(machine_code),
            _ => Err(AssemblerError::UnsupportedFormat),
        }
    }

    fn create_elf64(&self, machine_code: Vec<u8>) -> Result<Vec<u8>, AssemblerError> {
        let mut elf = Elf64Builder::new();
        elf.add_section(".text", machine_code);
        
        for section in &self.sections {
            if section.name != ".text" {
                elf.add_section(&section.name, section.data.clone());
            }
        }
        
        elf.add_symbols(&self.symbol_table);
        elf.build()
    }
}

impl SymbolTable {
    fn new() -> Self {
        Self {
            symbols: Vec::new(),
        }
    }

    fn add_global(&mut self, name: &str) {
        if let Some(sym) = self.symbols.iter_mut().find(|s| s.name == name) {
            sym.binding = SymbolBinding::Global;
        }
    }
}

#[derive(Debug)]
pub enum AssemblerError {
    InvalidInstruction,
    InvalidOperand,
    InvalidImmediate,
    UnsupportedArch,
    UnsupportedFormat,
    SymbolNotFound,
}

pub struct Instruction {
    mnemonic: String,
    operands: Vec<Operand>,
}

pub enum Operand {
    Register(Register),
    Immediate(i64),
    Memory(MemoryOperand),
    Label(String),
}

pub struct MemoryOperand {
    base: Option<Register>,
    index: Option<Register>,
    scale: u8,
    displacement: i64,
}

pub type Register = u8;

struct X86_64Register;

impl X86_64Register {
    fn from_str(name: &str) -> Result<Register, AssemblerError> {
        match name {
            "rax" | "eax" | "ax" | "al" => Ok(0),
            "rcx" | "ecx" | "cx" | "cl" => Ok(1),
            "rdx" | "edx" | "dx" | "dl" => Ok(2),
            "rbx" | "ebx" | "bx" | "bl" => Ok(3),
            "rsp" | "esp" | "sp" | "spl" => Ok(4),
            "rbp" | "ebp" | "bp" | "bpl" => Ok(5),
            "rsi" | "esi" | "si" | "sil" => Ok(6),
            "rdi" | "edi" | "di" | "dil" => Ok(7),
            "r8" | "r8d" | "r8w" | "r8b" => Ok(8),
            "r9" | "r9d" | "r9w" | "r9b" => Ok(9),
            "r10" | "r10d" | "r10w" | "r10b" => Ok(10),
            "r11" | "r11d" | "r11w" | "r11b" => Ok(11),
            "r12" | "r12d" | "r12w" | "r12b" => Ok(12),
            "r13" | "r13d" | "r13w" | "r13b" => Ok(13),
            "r14" | "r14d" | "r14w" | "r14b" => Ok(14),
            "r15" | "r15d" | "r15w" | "r15b" => Ok(15),
            _ => Err(AssemblerError::InvalidOperand),
        }
    }
}

struct X86_64Encoder;

impl X86_64Encoder {
    fn new() -> Self {
        Self
    }

    fn encode(&self, inst: Instruction) -> Result<Vec<u8>, AssemblerError> {
        match inst.mnemonic.as_str() {
            "nop" => Ok(vec![0x90]),
            "ret" => Ok(vec![0xc3]),
            "push" => self.encode_push(&inst.operands),
            "pop" => self.encode_pop(&inst.operands),
            "mov" => self.encode_mov(&inst.operands),
            "add" => self.encode_add(&inst.operands),
            "sub" => self.encode_sub(&inst.operands),
            "jmp" => self.encode_jmp(&inst.operands),
            "call" => self.encode_call(&inst.operands),
            _ => Err(AssemblerError::InvalidInstruction),
        }
    }

    fn encode_push(&self, _operands: &[Operand]) -> Result<Vec<u8>, AssemblerError> {
        Ok(vec![0x50])
    }

    fn encode_pop(&self, _operands: &[Operand]) -> Result<Vec<u8>, AssemblerError> {
        Ok(vec![0x58])
    }

    fn encode_mov(&self, _operands: &[Operand]) -> Result<Vec<u8>, AssemblerError> {
        Ok(vec![0x48, 0x89, 0xc0])
    }

    fn encode_add(&self, _operands: &[Operand]) -> Result<Vec<u8>, AssemblerError> {
        Ok(vec![0x48, 0x01, 0xc0])
    }

    fn encode_sub(&self, _operands: &[Operand]) -> Result<Vec<u8>, AssemblerError> {
        Ok(vec![0x48, 0x29, 0xc0])
    }

    fn encode_jmp(&self, _operands: &[Operand]) -> Result<Vec<u8>, AssemblerError> {
        Ok(vec![0xe9, 0x00, 0x00, 0x00, 0x00])
    }

    fn encode_call(&self, _operands: &[Operand]) -> Result<Vec<u8>, AssemblerError> {
        Ok(vec![0xe8, 0x00, 0x00, 0x00, 0x00])
    }
}

struct Elf64Builder {
    sections: Vec<(String, Vec<u8>)>,
}

impl Elf64Builder {
    fn new() -> Self {
        Self {
            sections: Vec::new(),
        }
    }

    fn add_section(&mut self, name: &str, data: Vec<u8>) {
        self.sections.push((name.to_string(), data));
    }

    fn add_symbols(&mut self, _symbols: &SymbolTable) {
    }

    fn build(&self) -> Result<Vec<u8>, AssemblerError> {
        let mut result = Vec::new();
        
        result.extend_from_slice(&[
            0x7f, b'E', b'L', b'F',
            2,
            1,
            1,
            0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        
        result.extend_from_slice(&[
            1, 0,
            0x3e, 0,
            1, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0x40, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0x40, 0,
            0x38, 0,
            0, 0,
            0, 0,
            0, 0,
            0, 0,
        ]);
        
        Ok(result)
    }
}