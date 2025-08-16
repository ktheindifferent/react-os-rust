// Process Control Block (PCB) - Core process data structure
use x86_64::{VirtAddr, structures::paging::PageTable};
use alloc::{vec::Vec, string::String, boxed::Box};
use crate::memory::PageProtection;

// x86_64 CPU context for process switching
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CpuContext {
    // General purpose registers
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
    
    // Instruction pointer
    pub rip: u64,
    
    // Flags register
    pub rflags: u64,
    
    // Segment registers
    pub cs: u16,
    pub ds: u16,
    pub es: u16,
    pub fs: u16,
    pub gs: u16,
    pub ss: u16,
    
    // Control registers
    pub cr3: u64,  // Page table base
}

impl CpuContext {
    pub fn new() -> Self {
        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0, rsp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: 0,
            rflags: 0x202,  // Interrupts enabled
            cs: 0x8,   // Kernel code segment
            ds: 0x10,  // Kernel data segment
            es: 0x10,
            fs: 0x10,
            gs: 0x10,
            ss: 0x10,
            cr3: 0,
        }
    }
}

// Memory region descriptor for process
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub start: VirtAddr,
    pub end: VirtAddr,
    pub protection: PageProtection,
    pub name: String,
}

// Process address space
#[derive(Debug)]
pub struct AddressSpace {
    pub page_table: Box<PageTable>,
    pub regions: Vec<MemoryRegion>,
    pub heap_start: VirtAddr,
    pub heap_end: VirtAddr,
    pub stack_start: VirtAddr,
    pub stack_end: VirtAddr,
}

impl AddressSpace {
    pub fn new() -> Self {
        Self {
            page_table: Box::new(PageTable::new()),
            regions: Vec::new(),
            heap_start: VirtAddr::new(0x4000_0000_0000),  // User heap at 256GB
            heap_end: VirtAddr::new(0x4000_0000_0000),
            stack_start: VirtAddr::new(0x7FFF_FFFF_F000),  // User stack near top
            stack_end: VirtAddr::new(0x7FFF_FF00_0000),
        }
    }
    
    pub fn add_region(&mut self, region: MemoryRegion) {
        self.regions.push(region);
    }
}

// File descriptor for process
#[derive(Debug, Clone)]
pub struct FileDescriptor {
    pub fd: i32,
    pub path: String,
    pub flags: u32,
    pub offset: u64,
}

// Process Control Block
#[derive(Debug)]
pub struct ProcessControlBlock {
    // Process identification
    pub pid: u32,
    pub ppid: Option<u32>,  // Parent PID
    pub name: String,
    pub command_line: String,
    
    // CPU state
    pub context: CpuContext,
    pub kernel_stack: VirtAddr,
    pub user_stack: VirtAddr,
    
    // Memory management
    pub address_space: AddressSpace,
    
    // File descriptors
    pub file_descriptors: Vec<FileDescriptor>,
    pub next_fd: i32,
    
    // Scheduling
    pub priority: u8,
    pub time_slice: u32,
    pub cpu_time: u64,
    
    // Process state
    pub exit_code: Option<i32>,
    pub wait_reason: Option<WaitReason>,
    
    // Security
    pub uid: u32,
    pub gid: u32,
    
    // Statistics
    pub creation_time: u64,
    pub user_time: u64,
    pub kernel_time: u64,
}

#[derive(Debug, Clone)]
pub enum WaitReason {
    None,
    Sleep(u64),          // Sleep until timestamp
    WaitPid(u32),        // Waiting for child process
    IO(i32),             // Waiting for I/O on file descriptor
    Mutex(usize),        // Waiting for mutex
    Signal,              // Waiting for signal
}

impl ProcessControlBlock {
    pub fn new(pid: u32, name: String, command_line: String) -> Self {
        Self {
            pid,
            ppid: None,
            name,
            command_line,
            context: CpuContext::new(),
            kernel_stack: VirtAddr::new(0),
            user_stack: VirtAddr::new(0),
            address_space: AddressSpace::new(),
            file_descriptors: Vec::new(),
            next_fd: 3,  // 0=stdin, 1=stdout, 2=stderr
            priority: 10,  // Default priority
            time_slice: 10,  // Default time slice in ms
            cpu_time: 0,
            exit_code: None,
            wait_reason: None,
            uid: 0,
            gid: 0,
            creation_time: 0,  // Would get from timer
            user_time: 0,
            kernel_time: 0,
        }
    }
    
    pub fn allocate_fd(&mut self, path: String, flags: u32) -> i32 {
        let fd = self.next_fd;
        self.next_fd += 1;
        
        self.file_descriptors.push(FileDescriptor {
            fd,
            path,
            flags,
            offset: 0,
        });
        
        fd
    }
    
    pub fn close_fd(&mut self, fd: i32) -> Result<(), &'static str> {
        if let Some(pos) = self.file_descriptors.iter().position(|f| f.fd == fd) {
            self.file_descriptors.remove(pos);
            Ok(())
        } else {
            Err("Invalid file descriptor")
        }
    }
    
    pub fn get_fd(&self, fd: i32) -> Option<&FileDescriptor> {
        self.file_descriptors.iter().find(|f| f.fd == fd)
    }
    
    pub fn get_fd_mut(&mut self, fd: i32) -> Option<&mut FileDescriptor> {
        self.file_descriptors.iter_mut().find(|f| f.fd == fd)
    }
}

// Kernel stack size for each process (8KB)
pub const KERNEL_STACK_SIZE: usize = 8192;

// User stack size for each process (1MB)
pub const USER_STACK_SIZE: usize = 1024 * 1024;