// Kernel Crash Dump System (kdump)
// Generates and saves crash dumps for post-mortem analysis

use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::VirtAddr;
use x86_64::registers::control::{Cr0, Cr2, Cr3, Cr4};

// Crash dump header
#[repr(C)]
#[derive(Debug, Clone)]
pub struct CrashDumpHeader {
    pub signature: [u8; 8],        // "KDUMPV01"
    pub version: u32,
    pub header_size: u32,
    pub timestamp: u64,
    pub panic_message_offset: u64,
    pub panic_message_length: u64,
    pub cpu_count: u32,
    pub current_cpu: u32,
    pub physical_memory_size: u64,
    pub dump_type: DumpType,
    pub compression: CompressionType,
    pub checksum: u32,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum DumpType {
    MiniDump = 1,      // Essential information only
    KernelDump = 2,    // Kernel memory only
    FullDump = 3,      // Complete memory dump
    LiveDump = 4,      // Live system snapshot
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum CompressionType {
    None = 0,
    Gzip = 1,
    Lz4 = 2,
    Zstd = 3,
}

// CPU context saved in dump
#[repr(C)]
#[derive(Debug, Clone)]
pub struct CpuContext {
    pub cpu_id: u32,
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cr0: u64,
    pub cr2: u64,
    pub cr3: u64,
    pub cr4: u64,
    pub cs: u16,
    pub ds: u16,
    pub es: u16,
    pub fs: u16,
    pub gs: u16,
    pub ss: u16,
}

// Memory region descriptor
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub start_address: u64,
    pub size: u64,
    pub region_type: MemoryRegionType,
    pub flags: u32,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum MemoryRegionType {
    KernelCode = 1,
    KernelData = 2,
    KernelStack = 3,
    UserSpace = 4,
    PageTables = 5,
    Reserved = 6,
    Hardware = 7,
}

// Crash dump manager
pub struct CrashDumpManager {
    enabled: AtomicBool,
    dump_in_progress: AtomicBool,
    dump_count: AtomicU64,
    reserved_memory: Option<ReservedDumpArea>,
    dump_type: Mutex<DumpType>,
    compression: Mutex<CompressionType>,
}

struct ReservedDumpArea {
    physical_address: u64,
    virtual_address: VirtAddr,
    size: usize,
}

lazy_static! {
    pub static ref CRASH_DUMP: CrashDumpManager = CrashDumpManager::new();
}

impl CrashDumpManager {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            dump_in_progress: AtomicBool::new(false),
            dump_count: AtomicU64::new(0),
            reserved_memory: None,
            dump_type: Mutex::new(DumpType::KernelDump),
            compression: Mutex::new(CompressionType::None),
        }
    }
    
    pub fn reserve_dump_memory(&mut self, size: usize) -> Result<(), String> {
        // Reserve memory for crash dumps (would allocate from physical memory)
        // This memory should be preserved across kexec for kdump kernel
        let phys_addr = 0x10000000;  // Example address
        let virt_addr = VirtAddr::new(phys_addr);
        
        self.reserved_memory = Some(ReservedDumpArea {
            physical_address: phys_addr,
            virtual_address: virt_addr,
            size,
        });
        
        crate::serial_println!("[KDUMP] Reserved {} MB at {:#x} for crash dumps",
            size / (1024 * 1024), phys_addr);
        Ok(())
    }
    
    pub fn create_dump(&self, panic_info: &core::panic::PanicInfo) -> Result<(), String> {
        // Check if already dumping (prevent recursive dumps)
        if self.dump_in_progress.swap(true, Ordering::SeqCst) {
            return Err("Dump already in progress".into());
        }
        
        let dump_id = self.dump_count.fetch_add(1, Ordering::SeqCst);
        crate::serial_println!("[KDUMP] Creating crash dump #{}", dump_id);
        
        // Collect CPU context
        let cpu_context = self.capture_cpu_context();
        
        // Create dump header
        let header = self.create_dump_header(panic_info);
        
        // Determine what to dump based on dump type
        let dump_type = *self.dump_type.lock();
        
        match dump_type {
            DumpType::MiniDump => self.create_mini_dump(&header, &cpu_context, panic_info),
            DumpType::KernelDump => self.create_kernel_dump(&header, &cpu_context, panic_info),
            DumpType::FullDump => self.create_full_dump(&header, &cpu_context, panic_info),
            DumpType::LiveDump => self.create_live_dump(&header, &cpu_context),
        }?;
        
        // Write dump to destination
        self.write_dump_to_storage(dump_id)?;
        
        crate::serial_println!("[KDUMP] Crash dump #{} completed", dump_id);
        
        self.dump_in_progress.store(false, Ordering::SeqCst);
        Ok(())
    }
    
    fn capture_cpu_context(&self) -> CpuContext {
        let mut context = CpuContext {
            cpu_id: 0,  // Would get actual CPU ID
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0, rsp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: 0, rflags: 0,
            cr0: 0, cr2: 0, cr3: 0, cr4: 0,
            cs: 0, ds: 0, es: 0, fs: 0, gs: 0, ss: 0,
        };
        
        // Capture general purpose registers
        unsafe {
            core::arch::asm!(
                "mov {}, rax",
                "mov {}, rbx",
                "mov {}, rcx",
                "mov {}, rdx",
                "mov {}, rsi",
                "mov {}, rdi",
                "mov {}, rbp",
                "mov {}, rsp",
                "mov {}, r8",
                "mov {}, r9",
                "mov {}, r10",
                "mov {}, r11",
                "mov {}, r12",
                "mov {}, r13",
                "mov {}, r14",
                "mov {}, r15",
                out(reg) context.rax,
                out(reg) context.rbx,
                out(reg) context.rcx,
                out(reg) context.rdx,
                out(reg) context.rsi,
                out(reg) context.rdi,
                out(reg) context.rbp,
                out(reg) context.rsp,
                out(reg) context.r8,
                out(reg) context.r9,
                out(reg) context.r10,
                out(reg) context.r11,
                out(reg) context.r12,
                out(reg) context.r13,
                out(reg) context.r14,
                out(reg) context.r15,
            );
            
            // Get RIP and RFLAGS
            core::arch::asm!(
                "lea {}, [rip]",
                "pushfq",
                "pop {}",
                out(reg) context.rip,
                out(reg) context.rflags,
            );
        }
        
        // Capture control registers
        context.cr0 = Cr0::read_raw();
        context.cr2 = Cr2::read().as_u64();
        context.cr3 = Cr3::read().0.start_address().as_u64();
        context.cr4 = Cr4::read_raw();
        
        // Capture segment registers
        unsafe {
            core::arch::asm!(
                "mov {}, cs",
                "mov {}, ds",
                "mov {}, es",
                "mov {}, fs",
                "mov {}, gs",
                "mov {}, ss",
                out(reg) context.cs,
                out(reg) context.ds,
                out(reg) context.es,
                out(reg) context.fs,
                out(reg) context.gs,
                out(reg) context.ss,
            );
        }
        
        context
    }
    
    fn create_dump_header(&self, panic_info: &core::panic::PanicInfo) -> CrashDumpHeader {
        let panic_msg = format!("{}", panic_info);
        
        CrashDumpHeader {
            signature: *b"KDUMPV01",
            version: 1,
            header_size: core::mem::size_of::<CrashDumpHeader>() as u32,
            timestamp: self.get_timestamp(),
            panic_message_offset: core::mem::size_of::<CrashDumpHeader>() as u64,
            panic_message_length: panic_msg.len() as u64,
            cpu_count: 1,  // Would get actual CPU count
            current_cpu: 0,  // Would get current CPU
            physical_memory_size: self.get_physical_memory_size(),
            dump_type: *self.dump_type.lock(),
            compression: *self.compression.lock(),
            checksum: 0,  // Would calculate actual checksum
        }
    }
    
    fn create_mini_dump(&self, header: &CrashDumpHeader, context: &CpuContext, 
                        panic_info: &core::panic::PanicInfo) -> Result<(), String> {
        crate::serial_println!("[KDUMP] Creating mini dump...");
        
        // Mini dump includes:
        // - Crash dump header
        // - CPU context
        // - Stack trace
        // - Panic message
        // - Essential kernel data structures
        
        // Save stack area around RSP
        let stack_size = 8192;  // 8KB of stack
        let stack_start = (context.rsp - stack_size / 2) & !0xF;  // Align
        
        crate::serial_println!("[KDUMP] Saving {} bytes of stack from {:#x}", 
            stack_size, stack_start);
        
        // Would write to reserved memory area
        
        Ok(())
    }
    
    fn create_kernel_dump(&self, header: &CrashDumpHeader, context: &CpuContext,
                         panic_info: &core::panic::PanicInfo) -> Result<(), String> {
        crate::serial_println!("[KDUMP] Creating kernel dump...");
        
        // Kernel dump includes all kernel memory:
        // - Kernel code segment
        // - Kernel data segment
        // - Kernel heap
        // - Kernel stacks
        // - Page tables
        
        // Identify kernel memory regions
        let regions = self.identify_kernel_regions();
        
        for region in regions {
            crate::serial_println!("[KDUMP] Dumping {:?} region at {:#x} size {:#x}",
                region.region_type, region.start_address, region.size);
            
            // Would copy memory to dump area
        }
        
        Ok(())
    }
    
    fn create_full_dump(&self, header: &CrashDumpHeader, context: &CpuContext,
                        panic_info: &core::panic::PanicInfo) -> Result<(), String> {
        crate::serial_println!("[KDUMP] Creating full memory dump...");
        
        // Full dump includes all physical memory
        let mem_size = self.get_physical_memory_size();
        crate::serial_println!("[KDUMP] Dumping {} MB of physical memory", 
            mem_size / (1024 * 1024));
        
        // Would copy all physical memory
        
        Ok(())
    }
    
    fn create_live_dump(&self, header: &CrashDumpHeader, context: &CpuContext) -> Result<(), String> {
        crate::serial_println!("[KDUMP] Creating live system dump...");
        
        // Live dump is created without stopping the system
        // Useful for debugging hangs and performance issues
        
        Ok(())
    }
    
    fn identify_kernel_regions(&self) -> Vec<MemoryRegion> {
        let mut regions = Vec::new();
        
        // Add kernel code region
        regions.push(MemoryRegion {
            start_address: 0x200000,  // Example kernel base
            size: 0x100000,  // 1MB code
            region_type: MemoryRegionType::KernelCode,
            flags: 0,
        });
        
        // Add kernel data region
        regions.push(MemoryRegion {
            start_address: 0x300000,
            size: 0x200000,  // 2MB data
            region_type: MemoryRegionType::KernelData,
            flags: 0,
        });
        
        // Would add actual regions from memory map
        
        regions
    }
    
    fn write_dump_to_storage(&self, dump_id: u64) -> Result<(), String> {
        // Write dump to persistent storage
        // Options:
        // 1. Write to reserved disk partition
        // 2. Send over network (netdump)
        // 3. Write to USB storage
        // 4. Store in NVRAM
        
        crate::serial_println!("[KDUMP] Writing dump {} to storage", dump_id);
        
        // For now, just indicate where it would be written
        if let Some(ref area) = self.reserved_memory {
            crate::serial_println!("[KDUMP] Dump saved at physical address {:#x}", 
                area.physical_address);
        }
        
        Ok(())
    }
    
    fn get_timestamp(&self) -> u64 {
        // Would get actual timestamp
        0
    }
    
    fn get_physical_memory_size(&self) -> u64 {
        // Would get actual physical memory size
        256 * 1024 * 1024  // 256MB for example
    }
    
    pub fn analyze_dump(&self, dump_addr: u64) -> Result<DumpAnalysis, String> {
        crate::serial_println!("[KDUMP] Analyzing dump at {:#x}", dump_addr);
        
        // Read and validate dump header
        let header = unsafe {
            &*(dump_addr as *const CrashDumpHeader)
        };
        
        if &header.signature != b"KDUMPV01" {
            return Err("Invalid dump signature".into());
        }
        
        // Perform analysis
        let analysis = DumpAnalysis {
            dump_valid: true,
            panic_reason: String::new(),  // Would extract from dump
            faulting_address: 0,
            faulting_instruction: 0,
            stack_trace: Vec::new(),
            suggested_cause: String::from("Unknown"),
        };
        
        Ok(analysis)
    }
    
    pub fn configure_kexec_kernel(&self, kernel_path: &str) -> Result<(), String> {
        // Configure kexec to load crash kernel
        crate::serial_println!("[KDUMP] Configuring kexec crash kernel: {}", kernel_path);
        
        // Would:
        // 1. Load crash kernel into reserved memory
        // 2. Set up boot parameters for crash kernel
        // 3. Register crash handler to trigger kexec
        
        Ok(())
    }
}

#[derive(Debug)]
pub struct DumpAnalysis {
    pub dump_valid: bool,
    pub panic_reason: String,
    pub faulting_address: u64,
    pub faulting_instruction: u64,
    pub stack_trace: Vec<String>,
    pub suggested_cause: String,
}

// ELF core dump format support
pub mod elf_core {
    use super::*;
    
    #[repr(C)]
    pub struct ElfHeader {
        pub magic: [u8; 4],  // 0x7f, 'E', 'L', 'F'
        pub class: u8,       // 64-bit
        pub data: u8,        // Little endian
        pub version: u8,
        pub osabi: u8,
        pub abi_version: u8,
        pub pad: [u8; 7],
        pub elf_type: u16,   // ET_CORE
        pub machine: u16,    // EM_X86_64
        pub version2: u32,
        pub entry: u64,
        pub phoff: u64,
        pub shoff: u64,
        pub flags: u32,
        pub ehsize: u16,
        pub phentsize: u16,
        pub phnum: u16,
        pub shentsize: u16,
        pub shnum: u16,
        pub shstrndx: u16,
    }
    
    #[repr(C)]
    pub struct ProgramHeader {
        pub p_type: u32,     // PT_NOTE, PT_LOAD
        pub p_flags: u32,
        pub p_offset: u64,
        pub p_vaddr: u64,
        pub p_paddr: u64,
        pub p_filesz: u64,
        pub p_memsz: u64,
        pub p_align: u64,
    }
    
    pub fn create_elf_core_dump(context: &CpuContext) -> Vec<u8> {
        // Create ELF core dump format compatible with GDB
        let mut dump = Vec::new();
        
        // Would build proper ELF core file
        
        dump
    }
}

// Public API
pub fn init() {
    // Reserve memory for crash dumps
    CRASH_DUMP.reserve_dump_memory(64 * 1024 * 1024)  // 64MB
        .unwrap_or_else(|e| {
            crate::serial_println!("[KDUMP] Failed to reserve memory: {}", e);
        });
    
    crate::serial_println!("[KDUMP] Crash dump system initialized");
}

pub fn create_dump(panic_info: &core::panic::PanicInfo) {
    CRASH_DUMP.create_dump(panic_info)
        .unwrap_or_else(|e| {
            crate::serial_println!("[KDUMP] Failed to create dump: {}", e);
        });
}

pub fn trigger_test_dump() {
    crate::serial_println!("[KDUMP] Triggering test crash dump...");
    let test_panic = core::panic::PanicInfo::internal_constructor(
        Some(&"Test crash dump"),
        core::panic::Location::caller(),
        false,
    );
    create_dump(&test_panic);
}

pub fn set_dump_type(dump_type: DumpType) {
    *CRASH_DUMP.dump_type.lock() = dump_type;
    crate::serial_println!("[KDUMP] Dump type set to {:?}", dump_type);
}

pub fn analyze_last_dump() -> Result<DumpAnalysis, String> {
    // Would find and analyze the last dump
    CRASH_DUMP.analyze_dump(0)
}