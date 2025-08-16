pub mod scheduler;
pub mod thread;
pub mod pcb;
pub mod elf;
pub mod pe_loader;
pub mod context_switch;
pub mod executor;

use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::serial_println;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadId(pub u32);

#[derive(Debug, Clone, Copy)]
pub enum ProcessState {
    Running,
    Ready,
    Blocked,
    Terminated,
}

#[derive(Debug)]
pub struct Process {
    pub id: ProcessId,
    pub name: String,
    pub state: ProcessState,
    pub threads: Vec<ThreadId>,
    pub parent: Option<ProcessId>,
    pub children: Vec<ProcessId>,
}

impl Process {
    pub fn new(id: ProcessId, name: String, parent: Option<ProcessId>) -> Self {
        Self {
            id,
            name,
            state: ProcessState::Ready,
            threads: Vec::new(),
            parent,
            children: Vec::new(),
        }
    }

    pub fn add_thread(&mut self, thread_id: ThreadId) {
        self.threads.push(thread_id);
    }

    pub fn add_child(&mut self, child_id: ProcessId) {
        self.children.push(child_id);
    }
}

pub struct ProcessManager {
    processes: Vec<Process>,
    next_process_id: u32,
    pub current_process: Option<ProcessId>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Vec::new(),
            next_process_id: 1,
            current_process: None,
        }
    }

    pub fn init(&mut self) {
        // Create the system idle process
        let idle_process = self.create_process(String::from("System Idle"), None);
        self.current_process = Some(idle_process);
        
        // Create the system process
        let _system_process = self.create_process(String::from("System"), None);
    }

    pub fn create_process(&mut self, name: String, parent: Option<ProcessId>) -> ProcessId {
        let id = ProcessId(self.next_process_id);
        self.next_process_id += 1;

        let mut process = Process::new(id, name, parent);
        process.state = ProcessState::Ready;
        
        // Update parent's children list if needed
        if let Some(parent_id) = parent {
            for p in &mut self.processes {
                if p.id == parent_id {
                    p.add_child(id);
                    break;
                }
            }
        }
        
        self.processes.push(process);
        id
    }

    pub fn get_process(&self, id: ProcessId) -> Option<&Process> {
        self.processes.iter().find(|p| p.id == id)
    }

    pub fn get_process_mut(&mut self, id: ProcessId) -> Option<&mut Process> {
        self.processes.iter_mut().find(|p| p.id == id)
    }

    pub fn terminate_process(&mut self, id: ProcessId) {
        if let Some(process) = self.get_process_mut(id) {
            process.state = ProcessState::Terminated;
        }
    }

    pub fn list_processes(&self) -> &Vec<Process> {
        &self.processes
    }
}

lazy_static! {
    pub static ref PROCESS_MANAGER: Mutex<ProcessManager> = Mutex::new(ProcessManager::new());
}