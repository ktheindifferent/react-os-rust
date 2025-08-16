use crate::serial_println;
use x86_64::VirtAddr;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::collections::BTreeSet;
use alloc::{vec, format};
use spin::Mutex;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::capabilities::{CapabilitySet, Capability};

static SANDBOXING_ENABLED: AtomicBool = AtomicBool::new(false);
static NEXT_SANDBOX_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct Sandbox {
    pub id: u64,
    pub name: String,
    pub policy: SandboxPolicy,
    pub capabilities: CapabilitySet,
    pub resource_limits: ResourceLimits,
    pub allowed_syscalls: BTreeSet<u32>,
    pub filesystem_view: FilesystemView,
    pub network_policy: NetworkPolicy,
}

#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    pub allow_network: bool,
    pub allow_filesystem: bool,
    pub allow_ipc: bool,
    pub allow_devices: bool,
    pub allow_exec: bool,
    pub strict_mode: bool,
}

impl SandboxPolicy {
    pub fn strict() -> Self {
        Self {
            allow_network: false,
            allow_filesystem: false,
            allow_ipc: false,
            allow_devices: false,
            allow_exec: false,
            strict_mode: true,
        }
    }
    
    pub fn relaxed() -> Self {
        Self {
            allow_network: true,
            allow_filesystem: true,
            allow_ipc: true,
            allow_devices: false,
            allow_exec: true,
            strict_mode: false,
        }
    }
    
    pub fn custom() -> Self {
        Self {
            allow_network: false,
            allow_filesystem: true,
            allow_ipc: false,
            allow_devices: false,
            allow_exec: false,
            strict_mode: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_memory: u64,      // bytes
    pub max_cpu_time: u64,    // milliseconds
    pub max_file_size: u64,   // bytes
    pub max_open_files: u32,
    pub max_processes: u32,
    pub max_threads: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: 256 * 1024 * 1024,     // 256 MB
            max_cpu_time: 60 * 1000,           // 60 seconds
            max_file_size: 10 * 1024 * 1024,   // 10 MB
            max_open_files: 256,
            max_processes: 10,
            max_threads: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FilesystemView {
    pub root: String,
    pub allowed_paths: Vec<String>,
    pub denied_paths: Vec<String>,
    pub read_only_paths: Vec<String>,
}

impl FilesystemView {
    pub fn isolated(root: String) -> Self {
        Self {
            root,
            allowed_paths: vec![],
            denied_paths: vec!["/proc", "/sys", "/dev"].iter().map(|s| s.to_string()).collect(),
            read_only_paths: vec!["/usr", "/lib", "/lib64"].iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetworkPolicy {
    pub allow_outbound: bool,
    pub allow_inbound: bool,
    pub allowed_ports: Vec<u16>,
    pub allowed_addresses: Vec<String>,
    pub denied_addresses: Vec<String>,
}

impl NetworkPolicy {
    pub fn none() -> Self {
        Self {
            allow_outbound: false,
            allow_inbound: false,
            allowed_ports: vec![],
            allowed_addresses: vec![],
            denied_addresses: vec![],
        }
    }
    
    pub fn outbound_only() -> Self {
        Self {
            allow_outbound: true,
            allow_inbound: false,
            allowed_ports: vec![80, 443], // HTTP/HTTPS only
            allowed_addresses: vec![],
            denied_addresses: vec!["127.0.0.1", "::1"].iter().map(|s| s.to_string()).collect(),
        }
    }
}

static SANDBOXES: Mutex<alloc::collections::BTreeMap<u64, Sandbox>> = 
    Mutex::new(alloc::collections::BTreeMap::new());

static PROCESS_SANDBOXES: Mutex<alloc::collections::BTreeMap<u64, u64>> = 
    Mutex::new(alloc::collections::BTreeMap::new());

pub fn init() {
    if SANDBOXING_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    
    serial_println!("[SANDBOX] Initializing sandboxing system");
    
    // Create default sandboxes
    create_driver_sandbox();
    create_user_sandbox();
    
    SANDBOXING_ENABLED.store(true, Ordering::SeqCst);
    serial_println!("[SANDBOX] Sandboxing system initialized");
}

fn create_driver_sandbox() {
    let sandbox = Sandbox {
        id: allocate_sandbox_id(),
        name: String::from("driver_sandbox"),
        policy: SandboxPolicy {
            allow_network: false,
            allow_filesystem: false,
            allow_ipc: true,
            allow_devices: true,
            allow_exec: false,
            strict_mode: true,
        },
        capabilities: super::capabilities::create_driver_capabilities(),
        resource_limits: ResourceLimits {
            max_memory: 64 * 1024 * 1024,  // 64 MB
            max_cpu_time: u64::MAX,        // No limit
            max_file_size: 0,               // No files
            max_open_files: 0,
            max_processes: 0,
            max_threads: 10,
        },
        allowed_syscalls: get_driver_syscalls(),
        filesystem_view: FilesystemView {
            root: String::from("/"),
            allowed_paths: vec![],
            denied_paths: vec![String::from("/")],
            read_only_paths: vec![],
        },
        network_policy: NetworkPolicy::none(),
    };
    
    SANDBOXES.lock().insert(sandbox.id, sandbox);
}

fn create_user_sandbox() {
    let sandbox = Sandbox {
        id: allocate_sandbox_id(),
        name: String::from("user_sandbox"),
        policy: SandboxPolicy::custom(),
        capabilities: super::capabilities::create_sandbox_capabilities(),
        resource_limits: ResourceLimits::default(),
        allowed_syscalls: get_user_syscalls(),
        filesystem_view: FilesystemView::isolated(String::from("/home/sandbox")),
        network_policy: NetworkPolicy::outbound_only(),
    };
    
    SANDBOXES.lock().insert(sandbox.id, sandbox);
}

fn allocate_sandbox_id() -> u64 {
    NEXT_SANDBOX_ID.fetch_add(1, Ordering::SeqCst)
}

fn get_driver_syscalls() -> BTreeSet<u32> {
    let mut syscalls = BTreeSet::new();
    // Allow only essential driver syscalls
    syscalls.insert(0x00); // read
    syscalls.insert(0x01); // write
    syscalls.insert(0x09); // mmap
    syscalls.insert(0x0B); // munmap
    syscalls.insert(0x10); // ioctl
    syscalls
}

fn get_user_syscalls() -> BTreeSet<u32> {
    let mut syscalls = BTreeSet::new();
    // Allow common user syscalls
    syscalls.insert(0x00); // read
    syscalls.insert(0x01); // write
    syscalls.insert(0x02); // open
    syscalls.insert(0x03); // close
    syscalls.insert(0x04); // stat
    syscalls.insert(0x05); // fstat
    syscalls.insert(0x09); // mmap
    syscalls.insert(0x0B); // munmap
    syscalls.insert(0x0C); // brk
    syscalls.insert(0x27); // getpid
    syscalls.insert(0x66); // getuid
    syscalls.insert(0x68); // getgid
    syscalls.insert(0xE7); // exit_group
    syscalls
}

pub fn create_sandbox(name: String, policy: SandboxPolicy) -> u64 {
    let sandbox_id = allocate_sandbox_id();
    
    let sandbox = Sandbox {
        id: sandbox_id,
        name,
        policy: policy.clone(),
        capabilities: if policy.strict_mode {
            CapabilitySet::empty()
        } else {
            super::capabilities::create_sandbox_capabilities()
        },
        resource_limits: ResourceLimits::default(),
        allowed_syscalls: if policy.strict_mode {
            BTreeSet::new()
        } else {
            get_user_syscalls()
        },
        filesystem_view: FilesystemView::isolated(format!("/sandbox/{}", sandbox_id)),
        network_policy: if policy.allow_network {
            NetworkPolicy::outbound_only()
        } else {
            NetworkPolicy::none()
        },
    };
    
    SANDBOXES.lock().insert(sandbox_id, sandbox);
    
    sandbox_id
}

pub fn assign_process_to_sandbox(pid: u64, sandbox_id: u64) -> Result<(), SandboxError> {
    let sandboxes = SANDBOXES.lock();
    
    if !sandboxes.contains_key(&sandbox_id) {
        return Err(SandboxError::InvalidSandbox);
    }
    
    drop(sandboxes); // Release lock
    
    PROCESS_SANDBOXES.lock().insert(pid, sandbox_id);
    
    // Apply sandbox capabilities
    if let Some(sandbox) = get_sandbox(sandbox_id) {
        super::capabilities::set_process_capabilities(pid, sandbox.capabilities);
    }
    
    Ok(())
}

pub fn get_process_sandbox(pid: u64) -> Option<u64> {
    PROCESS_SANDBOXES.lock().get(&pid).copied()
}

pub fn get_sandbox(sandbox_id: u64) -> Option<Sandbox> {
    SANDBOXES.lock().get(&sandbox_id).cloned()
}

#[derive(Debug)]
pub enum SandboxError {
    InvalidSandbox,
    PolicyViolation,
    ResourceExceeded,
    SyscallDenied,
    PathDenied,
    NetworkDenied,
}

pub fn check_syscall(pid: u64, syscall_num: u32) -> Result<(), SandboxError> {
    if !SANDBOXING_ENABLED.load(Ordering::SeqCst) {
        return Ok(());
    }
    
    if let Some(sandbox_id) = get_process_sandbox(pid) {
        if let Some(sandbox) = get_sandbox(sandbox_id) {
            if !sandbox.allowed_syscalls.contains(&syscall_num) {
                serial_println!("[SANDBOX] Process {} denied syscall 0x{:x}", pid, syscall_num);
                return Err(SandboxError::SyscallDenied);
            }
        }
    }
    
    Ok(())
}

pub fn check_path_access(pid: u64, path: &str, write: bool) -> Result<(), SandboxError> {
    if !SANDBOXING_ENABLED.load(Ordering::SeqCst) {
        return Ok(());
    }
    
    if let Some(sandbox_id) = get_process_sandbox(pid) {
        if let Some(sandbox) = get_sandbox(sandbox_id) {
            // Check if path is denied
            for denied in &sandbox.filesystem_view.denied_paths {
                if path.starts_with(denied) {
                    return Err(SandboxError::PathDenied);
                }
            }
            
            // Check if path is read-only
            if write {
                for readonly in &sandbox.filesystem_view.read_only_paths {
                    if path.starts_with(readonly) {
                        return Err(SandboxError::PathDenied);
                    }
                }
            }
            
            // Check if path is outside sandbox root
            if !path.starts_with(&sandbox.filesystem_view.root) {
                let mut allowed = false;
                for allowed_path in &sandbox.filesystem_view.allowed_paths {
                    if path.starts_with(allowed_path) {
                        allowed = true;
                        break;
                    }
                }
                if !allowed {
                    return Err(SandboxError::PathDenied);
                }
            }
        }
    }
    
    Ok(())
}

pub fn check_network_access(pid: u64, address: &str, port: u16, outbound: bool) -> Result<(), SandboxError> {
    if !SANDBOXING_ENABLED.load(Ordering::SeqCst) {
        return Ok(());
    }
    
    if let Some(sandbox_id) = get_process_sandbox(pid) {
        if let Some(sandbox) = get_sandbox(sandbox_id) {
            let policy = &sandbox.network_policy;
            
            // Check direction
            if outbound && !policy.allow_outbound {
                return Err(SandboxError::NetworkDenied);
            }
            if !outbound && !policy.allow_inbound {
                return Err(SandboxError::NetworkDenied);
            }
            
            // Check port
            if !policy.allowed_ports.is_empty() && !policy.allowed_ports.contains(&port) {
                return Err(SandboxError::NetworkDenied);
            }
            
            // Check address
            for denied in &policy.denied_addresses {
                if address == denied {
                    return Err(SandboxError::NetworkDenied);
                }
            }
            
            if !policy.allowed_addresses.is_empty() {
                if !policy.allowed_addresses.contains(&address.to_string()) {
                    return Err(SandboxError::NetworkDenied);
                }
            }
        }
    }
    
    Ok(())
}

pub fn check_resource_limit(pid: u64, resource: ResourceType, amount: u64) -> Result<(), SandboxError> {
    if !SANDBOXING_ENABLED.load(Ordering::SeqCst) {
        return Ok(());
    }
    
    if let Some(sandbox_id) = get_process_sandbox(pid) {
        if let Some(sandbox) = get_sandbox(sandbox_id) {
            let limits = &sandbox.resource_limits;
            
            let exceeded = match resource {
                ResourceType::Memory => amount > limits.max_memory,
                ResourceType::CpuTime => amount > limits.max_cpu_time,
                ResourceType::FileSize => amount > limits.max_file_size,
                ResourceType::OpenFiles => amount > limits.max_open_files as u64,
                ResourceType::Processes => amount > limits.max_processes as u64,
                ResourceType::Threads => amount > limits.max_threads as u64,
            };
            
            if exceeded {
                serial_println!("[SANDBOX] Process {} exceeded {:?} limit", pid, resource);
                return Err(SandboxError::ResourceExceeded);
            }
        }
    }
    
    Ok(())
}

#[derive(Debug)]
pub enum ResourceType {
    Memory,
    CpuTime,
    FileSize,
    OpenFiles,
    Processes,
    Threads,
}

pub fn enforce_sandbox_policy(pid: u64) -> Result<(), SandboxError> {
    if let Some(sandbox_id) = get_process_sandbox(pid) {
        if let Some(sandbox) = get_sandbox(sandbox_id) {
            if sandbox.policy.strict_mode {
                // In strict mode, default deny everything
                return Err(SandboxError::PolicyViolation);
            }
        }
    }
    
    Ok(())
}