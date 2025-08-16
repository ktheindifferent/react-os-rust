use super::{NtStatus, object::{ObjectHeader, ObjectTrait, Handle, ObjectType}};
use crate::memory::{PageProtection, AllocationType};
use crate::memory::virtual_memory::VirtualAllocator;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::VirtAddr;
use core::sync::atomic::{AtomicU64, AtomicU32, Ordering};

// Process ID type
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProcessId(pub u32);

impl ProcessId {
    pub const SYSTEM: ProcessId = ProcessId(4);
    pub const IDLE: ProcessId = ProcessId(0);
    
    pub fn new() -> Self {
        static NEXT_PID: AtomicU32 = AtomicU32::new(8);
        ProcessId(NEXT_PID.fetch_add(4, Ordering::SeqCst))
    }
}

// Thread ID type
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ThreadId(pub u32);

impl ThreadId {
    pub fn new() -> Self {
        static NEXT_TID: AtomicU32 = AtomicU32::new(8);
        ThreadId(NEXT_TID.fetch_add(4, Ordering::SeqCst))
    }
}

// Process creation flags - compatible with Windows
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct ProcessCreateFlags: u32 {
        const BREAKAWAY_FROM_JOB = 0x01000000;
        const DEFAULT_ERROR_MODE = 0x04000000;
        const NEW_CONSOLE = 0x00000010;
        const NEW_PROCESS_GROUP = 0x00000200;
        const NO_WINDOW = 0x08000000;
        const PROTECTED_PROCESS = 0x00040000;
        const PRESERVE_CODE_AUTHZ_LEVEL = 0x02000000;
        const SECURE_PROCESS = 0x00400000;
        const SEPARATE_WOW_VDM = 0x00000800;
        const SHARED_WOW_VDM = 0x00001000;
        const SUSPENDED = 0x00000004;
        const UNICODE_ENVIRONMENT = 0x00000400;
        const DEBUG_ONLY_THIS_PROCESS = 0x00000002;
        const DEBUG_PROCESS = 0x00000001;
        const DETACHED_PROCESS = 0x00000008;
        const EXTENDED_STARTUPINFO_PRESENT = 0x00080000;
        const INHERIT_PARENT_AFFINITY = 0x00010000;
    }
}

// Process state
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Initializing = 0,
    Ready = 1,
    Running = 2,
    Standby = 3,
    Terminated = 4,
    Waiting = 5,
    Transition = 6,
    DeferredReady = 7,
}

// NT Process structure - core process object
pub struct NtProcess {
    pub header: ObjectHeader,
    pub process_id: ProcessId,
    pub parent_process_id: ProcessId,
    pub state: ProcessState,
    pub image_file_name: String,
    pub command_line: String,
    pub current_directory: String,
    pub threads: BTreeMap<ThreadId, Handle>,
    pub virtual_memory_manager: Option<VirtualAllocator>,
    pub handle_table: BTreeMap<Handle, Handle>, // Process-specific handle table
    pub exit_code: Option<u32>,
    pub priority_class: PriorityClass,
    pub affinity_mask: u64,
    pub minimum_working_set_size: u64,
    pub maximum_working_set_size: u64,
    pub page_fault_count: AtomicU64,
    pub peak_working_set_size: u64,
    pub working_set_size: u64,
    pub peak_paged_pool_usage: u64,
    pub paged_pool_usage: u64,
    pub peak_non_paged_pool_usage: u64,
    pub non_paged_pool_usage: u64,
    pub pagefile_usage: u64,
    pub peak_pagefile_usage: u64,
    pub private_page_count: u64,
    pub read_operation_count: AtomicU64,
    pub write_operation_count: AtomicU64,
    pub other_operation_count: AtomicU64,
    pub read_transfer_count: AtomicU64,
    pub write_transfer_count: AtomicU64,
    pub other_transfer_count: AtomicU64,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityClass {
    Idle = 1,
    BelowNormal = 2,
    Normal = 3,
    AboveNormal = 4,
    High = 5,
    RealTime = 6,
}

impl NtProcess {
    pub fn new(
        image_file_name: String,
        command_line: String,
        parent_process_id: ProcessId,
    ) -> Self {
        let process_id = ProcessId::new();
        
        Self {
            header: ObjectHeader::new(ObjectType::Process),
            process_id,
            parent_process_id,
            state: ProcessState::Initializing,
            image_file_name,
            command_line,
            current_directory: String::from("C:\\"),
            threads: BTreeMap::new(),
            virtual_memory_manager: None,
            handle_table: BTreeMap::new(),
            exit_code: None,
            priority_class: PriorityClass::Normal,
            affinity_mask: 1, // Single CPU for now
            minimum_working_set_size: 200 * 1024, // 200KB
            maximum_working_set_size: 1024 * 1024, // 1MB
            page_fault_count: AtomicU64::new(0),
            peak_working_set_size: 0,
            working_set_size: 0,
            peak_paged_pool_usage: 0,
            paged_pool_usage: 0,
            peak_non_paged_pool_usage: 0,
            non_paged_pool_usage: 0,
            pagefile_usage: 0,
            peak_pagefile_usage: 0,
            private_page_count: 0,
            read_operation_count: AtomicU64::new(0),
            write_operation_count: AtomicU64::new(0),
            other_operation_count: AtomicU64::new(0),
            read_transfer_count: AtomicU64::new(0),
            write_transfer_count: AtomicU64::new(0),
            other_transfer_count: AtomicU64::new(0),
        }
    }

    pub fn add_thread(&mut self, thread_id: ThreadId, thread_handle: Handle) {
        self.threads.insert(thread_id, thread_handle);
    }

    pub fn remove_thread(&mut self, thread_id: ThreadId) -> Option<Handle> {
        self.threads.remove(&thread_id)
    }

    pub fn get_thread_count(&self) -> usize {
        self.threads.len()
    }

    pub fn terminate(&mut self, exit_code: u32) {
        self.state = ProcessState::Terminated;
        self.exit_code = Some(exit_code);
    }

    pub fn allocate_virtual_memory(
        &mut self,
        _base_address: Option<VirtAddr>,
        _size: u64,
        _allocation_type: AllocationType,
        _protect: PageProtection,
    ) -> Result<VirtAddr, super::NtStatus> {
        // Simplified implementation - just return a dummy address
        Ok(VirtAddr::new(0x1000))
    }
}

impl ObjectTrait for NtProcess {
    fn get_header(&self) -> &ObjectHeader {
        &self.header
    }

    fn get_header_mut(&mut self) -> &mut ObjectHeader {
        &mut self.header
    }
}

// Process manager - manages all processes in the system
pub struct ProcessManager {
    processes: BTreeMap<ProcessId, Arc<Mutex<NtProcess>>>,
    current_process: Option<ProcessId>,
    system_process: Option<ProcessId>,
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut manager = Self {
            processes: BTreeMap::new(),
            current_process: None,
            system_process: None,
        };

        // Create the system process
        let system_process = NtProcess::new(
            String::from("System"),
            String::from(""),
            ProcessId::IDLE,
        );
        
        let system_pid = system_process.process_id;
        manager.processes.insert(system_pid, Arc::new(Mutex::new(system_process)));
        manager.system_process = Some(system_pid);
        manager.current_process = Some(system_pid);

        manager
    }

    pub fn create_process(
        &mut self,
        image_file_name: String,
        command_line: String,
        parent_process_id: Option<ProcessId>,
        creation_flags: ProcessCreateFlags,
    ) -> Result<(ProcessId, Handle), NtStatus> {
        let parent_pid = parent_process_id.unwrap_or(self.get_current_process_id());
        
        let process = NtProcess::new(image_file_name, command_line, parent_pid);
        let process_id = process.process_id;
        
        // Insert into process table
        let process_arc = Arc::new(Mutex::new(process));
        self.processes.insert(process_id, process_arc.clone());

        // Create a handle for the process
        let handle = Handle::new();

        // If not suspended, set state to ready
        if !creation_flags.contains(ProcessCreateFlags::SUSPENDED) {
            if let Some(mut proc) = process_arc.try_lock() {
                proc.state = ProcessState::Ready;
            }
        }

        Ok((process_id, handle))
    }

    pub fn terminate_process(&mut self, process_id: ProcessId, exit_code: u32) -> NtStatus {
        if let Some(process_arc) = self.processes.get(&process_id).cloned() {
            if let Some(mut process) = process_arc.try_lock() {
                // Set process state to terminated
                process.terminate(exit_code);
                
                // Terminate all threads in the process
                let thread_ids: Vec<ThreadId> = process.threads.keys().copied().collect();
                for thread_id in thread_ids {
                    // Remove thread from process's thread list
                    process.remove_thread(thread_id);
                    
                    // Notify thread manager to clean up the thread
                    self.terminate_thread(thread_id);
                }
                
                // Clean up virtual memory if allocated
                if process.virtual_memory_manager.is_some() {
                    process.virtual_memory_manager = None;
                }
                
                // Clear handle table
                process.handle_table.clear();
                
                // Update memory counters
                process.working_set_size = 0;
                process.paged_pool_usage = 0;
                process.non_paged_pool_usage = 0;
                process.pagefile_usage = 0;
                process.private_page_count = 0;
                
                drop(process);
                
                // Remove from process list if no references remain
                if Arc::strong_count(&process_arc) == 1 {
                    self.processes.remove(&process_id);
                }
                
                NtStatus::Success
            } else {
                NtStatus::AccessDenied
            }
        } else {
            NtStatus::InvalidCid
        }
    }
    
    fn terminate_thread(&self, thread_id: ThreadId) {
        // Notify the thread manager to clean up the thread
        // This would normally interact with the thread manager subsystem
        use crate::process::thread::THREAD_MANAGER;
        use crate::process::ThreadId as ProcessThreadId;
        
        if let Some(mut thread_manager) = THREAD_MANAGER.try_lock() {
            // Convert from NT ThreadId to process ThreadId
            let process_thread_id = ProcessThreadId(thread_id.0);
            if let Some(thread) = thread_manager.get_thread_mut(process_thread_id) {
                thread.state = crate::process::thread::ThreadState::Terminated;
            }
        }
    }

    pub fn get_process(&self, process_id: ProcessId) -> Option<Arc<Mutex<NtProcess>>> {
        self.processes.get(&process_id).cloned()
    }

    pub fn get_current_process_id(&self) -> ProcessId {
        self.current_process.unwrap_or(ProcessId::SYSTEM)
    }

    pub fn get_current_process(&self) -> Option<Arc<Mutex<NtProcess>>> {
        let current_pid = self.get_current_process_id();
        self.get_process(current_pid)
    }

    pub fn set_current_process(&mut self, process_id: ProcessId) -> NtStatus {
        if self.processes.contains_key(&process_id) {
            self.current_process = Some(process_id);
            NtStatus::Success
        } else {
            NtStatus::InvalidCid
        }
    }

    pub fn enumerate_processes(&self) -> Vec<ProcessId> {
        self.processes.keys().copied().collect()
    }

    pub fn get_process_info(&self, process_id: ProcessId) -> Option<ProcessInfo> {
        if let Some(process_arc) = self.processes.get(&process_id) {
            if let Some(process) = process_arc.try_lock() {
                return Some(ProcessInfo {
                    process_id: process.process_id,
                    parent_process_id: process.parent_process_id,
                    image_file_name: process.image_file_name.clone(),
                    command_line: process.command_line.clone(),
                    state: process.state,
                    thread_count: process.get_thread_count(),
                    working_set_size: process.working_set_size,
                    page_fault_count: process.page_fault_count.load(Ordering::SeqCst),
                    priority_class: process.priority_class,
                });
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub process_id: ProcessId,
    pub parent_process_id: ProcessId,
    pub image_file_name: String,
    pub command_line: String,
    pub state: ProcessState,
    pub thread_count: usize,
    pub working_set_size: u64,
    pub page_fault_count: u64,
    pub priority_class: PriorityClass,
}

lazy_static! {
    pub static ref PROCESS_MANAGER: Mutex<ProcessManager> = Mutex::new(ProcessManager::new());
}

// NT Process API functions
pub fn nt_create_process_ex(
    process_handle: &mut Handle,
    desired_access: u32,
    object_attributes: Option<&super::object::ObjectAttributes>,
    parent_process: Handle,
    inherit_object_table: bool,
    section_handle: Option<Handle>,
    debug_port: Option<Handle>,
    exception_port: Option<Handle>,
    in_job: bool,
) -> NtStatus {
    let mut pm = PROCESS_MANAGER.lock();
    
    // For now, create a simple process
    match pm.create_process(
        String::from("unknown.exe"),
        String::from(""),
        None,
        ProcessCreateFlags::empty(),
    ) {
        Ok((_, handle)) => {
            *process_handle = handle;
            NtStatus::Success
        }
        Err(status) => status,
    }
}

pub fn nt_create_user_process(
    process_handle: &mut Handle,
    thread_handle: &mut Handle,
    desired_access: u32,
    thread_desired_access: u32,
    process_object_attributes: Option<&super::object::ObjectAttributes>,
    thread_object_attributes: Option<&super::object::ObjectAttributes>,
    process_flags: ProcessCreateFlags,
    thread_flags: u32,
    parameters: Option<*const u8>, // RTL_USER_PROCESS_PARAMETERS
    create_info: Option<*const u8>, // PS_CREATE_INFO
    attribute_list: Option<*const u8>, // PS_ATTRIBUTE_LIST
) -> NtStatus {
    let mut pm = PROCESS_MANAGER.lock();
    
    let image_name = String::from("user_process.exe");
    let command_line = String::from("");
    
    match pm.create_process(image_name, command_line, None, process_flags) {
        Ok((process_id, proc_handle)) => {
            *process_handle = proc_handle;
            
            // TODO: Create initial thread
            *thread_handle = Handle::new();
            
            NtStatus::Success
        }
        Err(status) => status,
    }
}

pub fn nt_terminate_process(process_handle: Handle, exit_status: u32) -> NtStatus {
    // In a real implementation, we'd resolve the handle to a process ID
    let mut pm = PROCESS_MANAGER.lock();
    let current_pid = pm.get_current_process_id();
    pm.terminate_process(current_pid, exit_status)
}

pub fn nt_query_information_process(
    process_handle: Handle,
    process_information_class: u32,
    process_information: *mut u8,
    process_information_length: u32,
    return_length: Option<&mut u32>,
) -> NtStatus {
    // Placeholder implementation
    NtStatus::NotImplemented
}

pub fn get_current_process_id() -> ProcessId {
    PROCESS_MANAGER.lock().get_current_process_id()
}

pub fn get_current_process() -> Option<Arc<Mutex<NtProcess>>> {
    PROCESS_MANAGER.lock().get_current_process()
}