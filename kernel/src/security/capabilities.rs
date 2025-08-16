use crate::serial_println;
use alloc::collections::{BTreeSet, BTreeMap};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;

static CAPABILITIES_ENABLED: AtomicBool = AtomicBool::new(false);

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
    // File capabilities
    FileRead = 1 << 0,
    FileWrite = 1 << 1,
    FileExecute = 1 << 2,
    FileDelete = 1 << 3,
    
    // Process capabilities
    ProcessCreate = 1 << 4,
    ProcessTerminate = 1 << 5,
    ProcessDebug = 1 << 6,
    ProcessSetPriority = 1 << 7,
    
    // Network capabilities
    NetworkBind = 1 << 8,
    NetworkConnect = 1 << 9,
    NetworkListen = 1 << 10,
    NetworkRaw = 1 << 11,
    
    // System capabilities
    SystemShutdown = 1 << 12,
    SystemReboot = 1 << 13,
    SystemTime = 1 << 14,
    SystemLoadDriver = 1 << 15,
    
    // Memory capabilities
    MemoryLock = 1 << 16,
    MemoryMap = 1 << 17,
    MemoryExecute = 1 << 18,
    
    // Security capabilities
    SecurityAdmin = 1 << 19,
    SecurityAudit = 1 << 20,
    SecurityBypass = 1 << 21,
    
    // Device capabilities
    DeviceAccess = 1 << 22,
    DeviceAdmin = 1 << 23,
    
    // IPC capabilities
    IpcCreate = 1 << 24,
    IpcConnect = 1 << 25,
    IpcAdmin = 1 << 26,
    
    // Root capability (all permissions)
    Root = u64::MAX,
}

#[derive(Debug, Clone)]
pub struct CapabilitySet {
    permitted: BTreeSet<Capability>,
    effective: BTreeSet<Capability>,
    inheritable: BTreeSet<Capability>,
    bounding: BTreeSet<Capability>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self {
            permitted: BTreeSet::new(),
            effective: BTreeSet::new(),
            inheritable: BTreeSet::new(),
            bounding: BTreeSet::new(),
        }
    }
    
    pub fn empty() -> Self {
        Self::new()
    }
    
    pub fn root() -> Self {
        let mut set = Self::new();
        set.permitted.insert(Capability::Root);
        set.effective.insert(Capability::Root);
        set.bounding.insert(Capability::Root);
        set
    }
    
    pub fn grant(&mut self, cap: Capability) -> bool {
        if self.bounding.contains(&cap) || self.bounding.contains(&Capability::Root) {
            self.permitted.insert(cap);
            true
        } else {
            false
        }
    }
    
    pub fn drop(&mut self, cap: Capability) {
        self.permitted.remove(&cap);
        self.effective.remove(&cap);
        self.inheritable.remove(&cap);
    }
    
    pub fn activate(&mut self, cap: Capability) -> bool {
        if self.permitted.contains(&cap) || self.permitted.contains(&Capability::Root) {
            self.effective.insert(cap);
            true
        } else {
            false
        }
    }
    
    pub fn deactivate(&mut self, cap: Capability) {
        self.effective.remove(&cap);
    }
    
    pub fn has_capability(&self, cap: Capability) -> bool {
        self.effective.contains(&cap) || self.effective.contains(&Capability::Root)
    }
    
    pub fn check(&self, cap: Capability) -> Result<(), SecurityError> {
        if self.has_capability(cap) {
            Ok(())
        } else {
            Err(SecurityError::InsufficientCapabilities)
        }
    }
    
    pub fn inherit_to_child(&self) -> Self {
        let mut child = Self::new();
        
        // Child gets inheritable capabilities as permitted
        for cap in &self.inheritable {
            child.permitted.insert(*cap);
        }
        
        // Bounding set is inherited
        child.bounding = self.bounding.clone();
        
        child
    }
}

#[derive(Debug)]
pub enum SecurityError {
    InsufficientCapabilities,
    CapabilityDenied,
    InvalidCapability,
}

pub struct ProcessCapabilities {
    pub pid: u64,
    pub capabilities: CapabilitySet,
    pub uid: u32,
    pub gid: u32,
}

static PROCESS_CAPS: Mutex<alloc::collections::BTreeMap<u64, ProcessCapabilities>> = 
    Mutex::new(alloc::collections::BTreeMap::new());

pub fn init() {
    if CAPABILITIES_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    
    serial_println!("[CAPABILITIES] Initializing capability-based security");
    
    // Initialize root process with full capabilities
    let root_caps = ProcessCapabilities {
        pid: 0,
        capabilities: CapabilitySet::root(),
        uid: 0,
        gid: 0,
    };
    
    PROCESS_CAPS.lock().insert(0, root_caps);
    
    CAPABILITIES_ENABLED.store(true, Ordering::SeqCst);
    serial_println!("[CAPABILITIES] Capability system initialized");
}

pub fn get_process_capabilities(pid: u64) -> Option<CapabilitySet> {
    PROCESS_CAPS.lock().get(&pid).map(|pc| pc.capabilities.clone())
}

pub fn set_process_capabilities(pid: u64, caps: CapabilitySet) {
    let mut process_caps = PROCESS_CAPS.lock();
    if let Some(pc) = process_caps.get_mut(&pid) {
        pc.capabilities = caps;
    }
}

pub fn check_capability(pid: u64, cap: Capability) -> Result<(), SecurityError> {
    let process_caps = PROCESS_CAPS.lock();
    
    if let Some(pc) = process_caps.get(&pid) {
        pc.capabilities.check(cap)
    } else {
        Err(SecurityError::InvalidCapability)
    }
}

pub fn drop_capability(pid: u64, cap: Capability) -> Result<(), SecurityError> {
    let mut process_caps = PROCESS_CAPS.lock();
    
    if let Some(pc) = process_caps.get_mut(&pid) {
        pc.capabilities.drop(cap);
        Ok(())
    } else {
        Err(SecurityError::InvalidCapability)
    }
}

pub fn create_sandbox_capabilities() -> CapabilitySet {
    let mut caps = CapabilitySet::new();
    
    // Minimal capabilities for sandboxed processes
    caps.grant(Capability::FileRead);
    caps.grant(Capability::ProcessCreate);
    caps.grant(Capability::MemoryMap);
    
    // Activate minimal set
    caps.activate(Capability::FileRead);
    caps.activate(Capability::MemoryMap);
    
    caps
}

pub fn create_driver_capabilities() -> CapabilitySet {
    let mut caps = CapabilitySet::new();
    
    // Driver-specific capabilities
    caps.grant(Capability::DeviceAccess);
    caps.grant(Capability::MemoryMap);
    caps.grant(Capability::MemoryLock);
    caps.grant(Capability::IpcCreate);
    
    // Activate necessary capabilities
    caps.activate(Capability::DeviceAccess);
    caps.activate(Capability::MemoryMap);
    
    caps
}

pub fn create_network_service_capabilities() -> CapabilitySet {
    let mut caps = CapabilitySet::new();
    
    // Network service capabilities
    caps.grant(Capability::NetworkBind);
    caps.grant(Capability::NetworkConnect);
    caps.grant(Capability::NetworkListen);
    caps.grant(Capability::FileRead);
    caps.grant(Capability::FileWrite);
    
    // Activate network capabilities
    caps.activate(Capability::NetworkBind);
    caps.activate(Capability::NetworkListen);
    
    caps
}

pub fn enforce_capability(cap: Capability) -> Result<(), SecurityError> {
    if !CAPABILITIES_ENABLED.load(Ordering::SeqCst) {
        return Ok(()); // Capabilities not enforced
    }
    
    // Get current process ID (would come from scheduler)
    let current_pid = get_current_pid();
    
    check_capability(current_pid, cap)
}

fn get_current_pid() -> u64 {
    // This would get the current process ID from the scheduler
    0 // Placeholder
}

pub fn transition_capabilities(from_pid: u64, to_pid: u64) {
    let mut process_caps = PROCESS_CAPS.lock();
    
    if let Some(parent_caps) = process_caps.get(&from_pid) {
        let child_caps = ProcessCapabilities {
            pid: to_pid,
            capabilities: parent_caps.capabilities.inherit_to_child(),
            uid: parent_caps.uid,
            gid: parent_caps.gid,
        };
        
        process_caps.insert(to_pid, child_caps);
    }
}

pub fn apply_capability_mask(pid: u64, mask: u64) {
    let mut process_caps = PROCESS_CAPS.lock();
    
    if let Some(pc) = process_caps.get_mut(&pid) {
        // Remove capabilities not in mask
        let mut to_remove = Vec::new();
        for cap in pc.capabilities.permitted.iter() {
            if (*cap as u64) & mask == 0 {
                to_remove.push(*cap);
            }
        }
        
        for cap in to_remove {
            pc.capabilities.drop(cap);
        }
    }
}