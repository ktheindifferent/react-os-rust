// Process executor - manages process execution and scheduling
use super::{ProcessId, ProcessState, PROCESS_MANAGER};
use super::pcb::{ProcessControlBlock, CpuContext, KERNEL_STACK_SIZE, USER_STACK_SIZE};
use super::context_switch::{init_context, switch_context};
use super::elf::ElfLoader;
use super::pe_loader::PeLoader;
use alloc::{vec::Vec, string::{String, ToString}, boxed::Box, collections::BTreeMap};
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::{VirtAddr, structures::paging::PageTableFlags};
use crate::memory::paging;
use crate::interrupts::TIMER_TICKS;
use crate::serial_println;

lazy_static! {
    pub static ref EXECUTOR: Mutex<ProcessExecutor> = Mutex::new(ProcessExecutor::new());
}

pub struct ProcessExecutor {
    processes: BTreeMap<u32, Box<ProcessControlBlock>>,
    current_pid: Option<u32>,
    next_pid: u32,
    ready_queue: Vec<u32>,
    blocked_queue: Vec<u32>,
    time_quantum: u32,
    current_quantum: u32,
}

impl ProcessExecutor {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            current_pid: None,
            next_pid: 1,
            ready_queue: Vec::new(),
            blocked_queue: Vec::new(),
            time_quantum: 10,  // 10 timer ticks per process
            current_quantum: 0,
        }
    }
    
    pub fn init(&mut self) {
        // Skip creating processes during initialization to avoid memory issues
        // Just mark the executor as initialized
        serial_println!("Process executor initialized (deferred process creation)");
    }
    
    fn create_idle_process(&mut self) -> Box<ProcessControlBlock> {
        let mut pcb = Box::new(ProcessControlBlock::new(
            0,
            String::from("idle"),
            String::from("System Idle Process"),
        ));
        
        // Idle process runs at lowest priority
        pcb.priority = 0;
        
        // Set up idle loop entry point
        init_context(
            &mut pcb.context,
            idle_process_entry as u64,
            allocate_kernel_stack(),
            true,  // Kernel process
        );
        
        pcb
    }
    
    fn create_init_process(&mut self) -> Box<ProcessControlBlock> {
        let mut pcb = Box::new(ProcessControlBlock::new(
            1,
            String::from("init"),
            String::from("System Init Process"),
        ));
        
        // Init process runs at normal priority
        pcb.priority = 10;
        
        // Set up init entry point
        init_context(
            &mut pcb.context,
            init_process_entry as u64,
            allocate_kernel_stack(),
            true,  // Kernel process
        );
        
        pcb
    }
    
    pub fn create_process(&mut self, name: String, binary_data: &[u8]) -> Result<u32, &'static str> {
        // Detect format and load executable
        let (entry_point, is_pe) = if PeLoader::validate_pe(binary_data) {
            // Load PE/COFF executable
            let loaded_pe = PeLoader::load_pe(binary_data)?;
            
            // Check if it's a DLL
            if loaded_pe.is_dll {
                return Err("Cannot execute DLL as process");
            }
            
            (loaded_pe.entry_point, true)
        } else {
            // Try loading as ELF
            let loaded_elf = ElfLoader::load(binary_data)?;
            (loaded_elf.entry_point, false)
        };
        
        // Allocate PID
        let pid = self.next_pid;
        self.next_pid += 1;
        
        // Create PCB
        let mut pcb = Box::new(ProcessControlBlock::new(
            pid,
            name.clone(),
            name.clone(),
        ));
        
        // Allocate stacks
        let kernel_stack = allocate_kernel_stack();
        let user_stack = allocate_user_stack();
        
        pcb.kernel_stack = VirtAddr::new(kernel_stack);
        pcb.user_stack = VirtAddr::new(user_stack);
        
        // Initialize context for user process
        init_context(
            &mut pcb.context,
            entry_point.as_u64(),
            user_stack,
            false,  // User process
        );
        
        // Map segments into process address space
        if is_pe {
            let loaded_pe = PeLoader::load_pe(binary_data)?;
            for section in &loaded_pe.sections {
                let protection = if section.characteristics & 0x20000000 != 0 {
                    crate::memory::PageProtection::ExecuteReadWrite
                } else if section.characteristics & 0x80000000 != 0 {
                    crate::memory::PageProtection::ReadWrite
                } else {
                    crate::memory::PageProtection::ReadOnly
                };
                
                pcb.address_space.add_region(crate::process::pcb::MemoryRegion {
                    start: VirtAddr::new(section.virtual_address.as_u64()),
                    end: VirtAddr::new(section.virtual_address.as_u64() + section.virtual_size as u64),
                    protection,
                    name: section.name.clone(),
                });
            }
        } else {
            let loaded_elf = ElfLoader::load(binary_data)?;
            for segment in &loaded_elf.segments {
                pcb.address_space.add_region(crate::process::pcb::MemoryRegion {
                    start: segment.vaddr,
                    end: VirtAddr::new(segment.vaddr.as_u64() + segment.size as u64),
                    protection: crate::memory::PageProtection::ExecuteReadWrite,
                    name: String::from("code"),
                });
            }
        }
        
        // Add to process table and ready queue
        self.processes.insert(pid, pcb);
        self.ready_queue.push(pid);
        
        // Update process manager
        let mut pm = PROCESS_MANAGER.lock();
        pm.create_process(name, self.current_pid.map(|pid| ProcessId(pid)));
        
        serial_println!("Created process with PID {}", pid);
        Ok(pid)
    }
    
    pub fn terminate_process(&mut self, pid: u32, exit_code: i32) {
        if let Some(mut pcb) = self.processes.remove(&pid) {
            pcb.exit_code = Some(exit_code);
            
            // Remove from queues
            self.ready_queue.retain(|&p| p != pid);
            self.blocked_queue.retain(|&p| p != pid);
            
            // Free resources (stacks, memory regions, etc.)
            // This would deallocate memory
            
            serial_println!("Process {} terminated with exit code {}", pid, exit_code);
            
            // If this was current process, schedule next
            if self.current_pid == Some(pid) {
                self.current_pid = None;
                self.schedule_next();
            }
        }
    }
    
    pub fn schedule_next(&mut self) {
        // Save current process context if needed
        if let Some(current) = self.current_pid {
            if let Some(pcb) = self.processes.get_mut(&current) {
                // Context would be saved by interrupt handler
                // Move to back of ready queue if still ready
                if !self.blocked_queue.contains(&current) {
                    self.ready_queue.retain(|&p| p != current);
                    self.ready_queue.push(current);
                }
            }
        }
        
        // Pick next process from ready queue
        if let Some(&next_pid) = self.ready_queue.first() {
            self.current_pid = Some(next_pid);
            self.current_quantum = 0;
            
            // Switch to next process
            if let Some(next_pcb) = self.processes.get(&next_pid) {
                // This would perform actual context switch
                serial_println!("Scheduling process {} ({})", next_pid, next_pcb.name);
            }
        } else {
            // No ready processes, run idle
            self.current_pid = Some(0);
        }
    }
    
    pub fn timer_tick(&mut self) {
        self.current_quantum += 1;
        
        // Update CPU time for current process
        if let Some(pid) = self.current_pid {
            if let Some(pcb) = self.processes.get_mut(&pid) {
                pcb.cpu_time += 1;
            }
        }
        
        // Check if time quantum expired
        if self.current_quantum >= self.time_quantum {
            self.schedule_next();
        }
    }
    
    pub fn block_process(&mut self, pid: u32, reason: super::pcb::WaitReason) {
        if let Some(pcb) = self.processes.get_mut(&pid) {
            pcb.wait_reason = Some(reason);
            
            // Move from ready to blocked queue
            self.ready_queue.retain(|&p| p != pid);
            if !self.blocked_queue.contains(&pid) {
                self.blocked_queue.push(pid);
            }
            
            // Schedule if this was current process
            if self.current_pid == Some(pid) {
                self.schedule_next();
            }
        }
    }
    
    pub fn unblock_process(&mut self, pid: u32) {
        if let Some(pcb) = self.processes.get_mut(&pid) {
            pcb.wait_reason = None;
            
            // Move from blocked to ready queue
            self.blocked_queue.retain(|&p| p != pid);
            if !self.ready_queue.contains(&pid) {
                self.ready_queue.push(pid);
            }
        }
    }
    
    pub fn get_current_process(&self) -> Option<&ProcessControlBlock> {
        self.current_pid.and_then(|pid| self.processes.get(&pid).map(|b| b.as_ref()))
    }
    
    pub fn get_current_pid(&self) -> Option<u32> {
        self.current_pid
    }
    
    pub fn list_processes(&self) -> Vec<(u32, String, String)> {
        self.processes
            .iter()
            .map(|(&pid, pcb)| {
                let state = if self.current_pid == Some(pid) {
                    "Running"
                } else if self.ready_queue.contains(&pid) {
                    "Ready"
                } else if self.blocked_queue.contains(&pid) {
                    "Blocked"
                } else {
                    "Unknown"
                };
                (pid, pcb.name.clone(), state.to_string())
            })
            .collect()
    }
}

// Entry point for idle process
extern "C" fn idle_process_entry() -> ! {
    loop {
        // Just halt until next interrupt
        x86_64::instructions::hlt();
    }
}

// Entry point for init process
extern "C" fn init_process_entry() -> ! {
    serial_println!("Init process started");
    
    // Init would start system services here
    
    loop {
        // Wait for child processes
        x86_64::instructions::hlt();
    }
}

// Allocate a kernel stack
fn allocate_kernel_stack() -> u64 {
    // This would allocate actual memory
    // For now, return a dummy address
    static mut NEXT_STACK: u64 = 0xFFFF_8000_1000_0000;
    unsafe {
        let stack = NEXT_STACK;
        NEXT_STACK += KERNEL_STACK_SIZE as u64;
        stack
    }
}

// Allocate a user stack
fn allocate_user_stack() -> u64 {
    // This would allocate actual memory in user space
    // For now, return a dummy address
    static mut NEXT_STACK: u64 = 0x7FFF_FF00_0000;
    unsafe {
        let stack = NEXT_STACK;
        NEXT_STACK -= USER_STACK_SIZE as u64;
        stack
    }
}

// System call handler for process operations
pub fn handle_process_syscall(syscall: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    match syscall {
        // Fork
        0x01 => {
            // Would implement fork here
            -1
        },
        // Exec
        0x02 => {
            // Would implement exec here
            -1
        },
        // Exit
        0x03 => {
            let exit_code = arg1 as i32;
            let mut executor = EXECUTOR.lock();
            if let Some(pid) = executor.current_pid {
                executor.terminate_process(pid, exit_code);
            }
            0
        },
        // Wait
        0x04 => {
            // Would implement wait here
            -1
        },
        _ => -1,
    }
}