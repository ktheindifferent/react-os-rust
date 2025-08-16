use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;
use crate::serial_println;

use super::ContainerError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NamespaceType {
    Pid,
    Net,
    Mount,
    Ipc,
    User,
    Uts,
    Cgroup,
}

pub trait Namespace {
    fn ns_type(&self) -> NamespaceType;
    fn enter(&mut self) -> Result<(), ContainerError>;
    fn exit(&mut self) -> Result<(), ContainerError>;
    fn get_id(&self) -> u32;
}

static PID_NS_COUNTER: AtomicU32 = AtomicU32::new(1);
static NET_NS_COUNTER: AtomicU32 = AtomicU32::new(1);
static MOUNT_NS_COUNTER: AtomicU32 = AtomicU32::new(1);
static IPC_NS_COUNTER: AtomicU32 = AtomicU32::new(1);
static USER_NS_COUNTER: AtomicU32 = AtomicU32::new(1);
static UTS_NS_COUNTER: AtomicU32 = AtomicU32::new(1);

pub struct PidNamespace {
    id: u32,
    parent: Option<u32>,
    pids: Mutex<BTreeMap<u32, u32>>,
    next_pid: AtomicU32,
    active: bool,
}

impl PidNamespace {
    pub fn new() -> Result<Self, ContainerError> {
        Ok(Self {
            id: PID_NS_COUNTER.fetch_add(1, Ordering::SeqCst),
            parent: None,
            pids: Mutex::new(BTreeMap::new()),
            next_pid: AtomicU32::new(1),
            active: false,
        })
    }
    
    pub fn new_with_parent(parent_id: u32) -> Result<Self, ContainerError> {
        Ok(Self {
            id: PID_NS_COUNTER.fetch_add(1, Ordering::SeqCst),
            parent: Some(parent_id),
            pids: Mutex::new(BTreeMap::new()),
            next_pid: AtomicU32::new(1),
            active: false,
        })
    }
    
    pub fn allocate_pid(&self) -> u32 {
        self.next_pid.fetch_add(1, Ordering::SeqCst)
    }
    
    pub fn translate_pid(&self, ns_pid: u32) -> Option<u32> {
        self.pids.lock().get(&ns_pid).copied()
    }
    
    pub fn add_pid_mapping(&self, ns_pid: u32, host_pid: u32) {
        self.pids.lock().insert(ns_pid, host_pid);
    }
    
    pub fn remove_pid(&self, ns_pid: u32) {
        self.pids.lock().remove(&ns_pid);
    }
}

impl Namespace for PidNamespace {
    fn ns_type(&self) -> NamespaceType {
        NamespaceType::Pid
    }
    
    fn enter(&mut self) -> Result<(), ContainerError> {
        if self.active {
            return Err(ContainerError::NamespaceError("PID namespace already active".into()));
        }
        self.active = true;
        serial_println!("Entering PID namespace {}", self.id);
        Ok(())
    }
    
    fn exit(&mut self) -> Result<(), ContainerError> {
        if !self.active {
            return Err(ContainerError::NamespaceError("PID namespace not active".into()));
        }
        self.active = false;
        serial_println!("Exiting PID namespace {}", self.id);
        Ok(())
    }
    
    fn get_id(&self) -> u32 {
        self.id
    }
}

pub struct NetNamespace {
    id: u32,
    interfaces: Mutex<Vec<NetworkInterface>>,
    routes: Mutex<Vec<Route>>,
    iptables_rules: Mutex<Vec<IptablesRule>>,
    active: bool,
}

#[derive(Debug, Clone)]
struct NetworkInterface {
    name: String,
    index: u32,
    mac_address: [u8; 6],
    ip_addresses: Vec<IpAddress>,
    flags: u32,
}

#[derive(Debug, Clone)]
struct IpAddress {
    address: [u8; 4],
    prefix_len: u8,
}

#[derive(Debug, Clone)]
struct Route {
    destination: [u8; 4],
    prefix_len: u8,
    gateway: Option<[u8; 4]>,
    interface_index: u32,
    metric: u32,
}

#[derive(Debug, Clone)]
struct IptablesRule {
    table: String,
    chain: String,
    rule: String,
}

impl NetNamespace {
    pub fn new() -> Result<Self, ContainerError> {
        let mut ns = Self {
            id: NET_NS_COUNTER.fetch_add(1, Ordering::SeqCst),
            interfaces: Mutex::new(Vec::new()),
            routes: Mutex::new(Vec::new()),
            iptables_rules: Mutex::new(Vec::new()),
            active: false,
        };
        
        ns.create_loopback();
        Ok(ns)
    }
    
    fn create_loopback(&mut self) {
        let lo = NetworkInterface {
            name: "lo".to_string(),
            index: 1,
            mac_address: [0; 6],
            ip_addresses: vec![
                IpAddress {
                    address: [127, 0, 0, 1],
                    prefix_len: 8,
                }
            ],
            flags: 0x1 | 0x8,
        };
        self.interfaces.lock().push(lo);
    }
    
    pub fn add_interface(&self, name: String, mac: [u8; 6]) -> u32 {
        let mut interfaces = self.interfaces.lock();
        let index = interfaces.len() as u32 + 1;
        interfaces.push(NetworkInterface {
            name,
            index,
            mac_address: mac,
            ip_addresses: Vec::new(),
            flags: 0,
        });
        index
    }
    
    pub fn add_route(&self, dest: [u8; 4], prefix: u8, gateway: Option<[u8; 4]>, iface_idx: u32) {
        self.routes.lock().push(Route {
            destination: dest,
            prefix_len: prefix,
            gateway,
            interface_index: iface_idx,
            metric: 100,
        });
    }
}

impl Namespace for NetNamespace {
    fn ns_type(&self) -> NamespaceType {
        NamespaceType::Net
    }
    
    fn enter(&mut self) -> Result<(), ContainerError> {
        if self.active {
            return Err(ContainerError::NamespaceError("Network namespace already active".into()));
        }
        self.active = true;
        serial_println!("Entering network namespace {}", self.id);
        Ok(())
    }
    
    fn exit(&mut self) -> Result<(), ContainerError> {
        if !self.active {
            return Err(ContainerError::NamespaceError("Network namespace not active".into()));
        }
        self.active = false;
        serial_println!("Exiting network namespace {}", self.id);
        Ok(())
    }
    
    fn get_id(&self) -> u32 {
        self.id
    }
}

pub struct MountNamespace {
    id: u32,
    mounts: Mutex<Vec<MountPoint>>,
    root: String,
    active: bool,
}

#[derive(Debug, Clone)]
struct MountPoint {
    source: String,
    target: String,
    fstype: String,
    flags: u32,
    data: Option<String>,
}

impl MountNamespace {
    pub fn new() -> Result<Self, ContainerError> {
        Ok(Self {
            id: MOUNT_NS_COUNTER.fetch_add(1, Ordering::SeqCst),
            mounts: Mutex::new(Vec::new()),
            root: "/".to_string(),
            active: false,
        })
    }
    
    pub fn set_root(&mut self, root: String) {
        self.root = root;
    }
    
    pub fn add_mount(&self, source: String, target: String, fstype: String, flags: u32) {
        self.mounts.lock().push(MountPoint {
            source,
            target,
            fstype,
            flags,
            data: None,
        });
    }
    
    pub fn remove_mount(&self, target: &str) {
        self.mounts.lock().retain(|m| m.target != target);
    }
}

impl Namespace for MountNamespace {
    fn ns_type(&self) -> NamespaceType {
        NamespaceType::Mount
    }
    
    fn enter(&mut self) -> Result<(), ContainerError> {
        if self.active {
            return Err(ContainerError::NamespaceError("Mount namespace already active".into()));
        }
        self.active = true;
        serial_println!("Entering mount namespace {}", self.id);
        Ok(())
    }
    
    fn exit(&mut self) -> Result<(), ContainerError> {
        if !self.active {
            return Err(ContainerError::NamespaceError("Mount namespace not active".into()));
        }
        self.active = false;
        serial_println!("Exiting mount namespace {}", self.id);
        Ok(())
    }
    
    fn get_id(&self) -> u32 {
        self.id
    }
}

pub struct IpcNamespace {
    id: u32,
    message_queues: Mutex<BTreeMap<u32, MessageQueue>>,
    semaphores: Mutex<BTreeMap<u32, Semaphore>>,
    shared_memory: Mutex<BTreeMap<u32, SharedMemory>>,
    next_id: AtomicU32,
    active: bool,
}

#[derive(Debug)]
struct MessageQueue {
    id: u32,
    messages: Vec<Vec<u8>>,
    max_messages: usize,
    max_msg_size: usize,
}

#[derive(Debug)]
struct Semaphore {
    id: u32,
    value: i32,
    waiters: Vec<u32>,
}

#[derive(Debug)]
struct SharedMemory {
    id: u32,
    size: usize,
    data: Vec<u8>,
    attached_processes: Vec<u32>,
}

impl IpcNamespace {
    pub fn new() -> Result<Self, ContainerError> {
        Ok(Self {
            id: IPC_NS_COUNTER.fetch_add(1, Ordering::SeqCst),
            message_queues: Mutex::new(BTreeMap::new()),
            semaphores: Mutex::new(BTreeMap::new()),
            shared_memory: Mutex::new(BTreeMap::new()),
            next_id: AtomicU32::new(1),
            active: false,
        })
    }
    
    pub fn create_message_queue(&self, max_messages: usize, max_msg_size: usize) -> u32 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.message_queues.lock().insert(id, MessageQueue {
            id,
            messages: Vec::new(),
            max_messages,
            max_msg_size,
        });
        id
    }
    
    pub fn create_semaphore(&self, initial_value: i32) -> u32 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.semaphores.lock().insert(id, Semaphore {
            id,
            value: initial_value,
            waiters: Vec::new(),
        });
        id
    }
    
    pub fn create_shared_memory(&self, size: usize) -> u32 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.shared_memory.lock().insert(id, SharedMemory {
            id,
            size,
            data: vec![0; size],
            attached_processes: Vec::new(),
        });
        id
    }
}

impl Namespace for IpcNamespace {
    fn ns_type(&self) -> NamespaceType {
        NamespaceType::Ipc
    }
    
    fn enter(&mut self) -> Result<(), ContainerError> {
        if self.active {
            return Err(ContainerError::NamespaceError("IPC namespace already active".into()));
        }
        self.active = true;
        serial_println!("Entering IPC namespace {}", self.id);
        Ok(())
    }
    
    fn exit(&mut self) -> Result<(), ContainerError> {
        if !self.active {
            return Err(ContainerError::NamespaceError("IPC namespace not active".into()));
        }
        self.active = false;
        serial_println!("Exiting IPC namespace {}", self.id);
        Ok(())
    }
    
    fn get_id(&self) -> u32 {
        self.id
    }
}

pub struct UserNamespace {
    id: u32,
    uid_map: Mutex<Vec<UidMapping>>,
    gid_map: Mutex<Vec<GidMapping>>,
    capabilities: u64,
    active: bool,
}

#[derive(Debug, Clone)]
struct UidMapping {
    inside_uid: u32,
    outside_uid: u32,
    count: u32,
}

#[derive(Debug, Clone)]
struct GidMapping {
    inside_gid: u32,
    outside_gid: u32,
    count: u32,
}

impl UserNamespace {
    pub fn new() -> Result<Self, ContainerError> {
        Ok(Self {
            id: USER_NS_COUNTER.fetch_add(1, Ordering::SeqCst),
            uid_map: Mutex::new(Vec::new()),
            gid_map: Mutex::new(Vec::new()),
            capabilities: 0,
            active: false,
        })
    }
    
    pub fn add_uid_mapping(&self, inside: u32, outside: u32, count: u32) {
        self.uid_map.lock().push(UidMapping {
            inside_uid: inside,
            outside_uid: outside,
            count,
        });
    }
    
    pub fn add_gid_mapping(&self, inside: u32, outside: u32, count: u32) {
        self.gid_map.lock().push(GidMapping {
            inside_gid: inside,
            outside_gid: outside,
            count,
        });
    }
    
    pub fn set_capabilities(&mut self, caps: u64) {
        self.capabilities = caps;
    }
}

impl Namespace for UserNamespace {
    fn ns_type(&self) -> NamespaceType {
        NamespaceType::User
    }
    
    fn enter(&mut self) -> Result<(), ContainerError> {
        if self.active {
            return Err(ContainerError::NamespaceError("User namespace already active".into()));
        }
        self.active = true;
        serial_println!("Entering user namespace {}", self.id);
        Ok(())
    }
    
    fn exit(&mut self) -> Result<(), ContainerError> {
        if !self.active {
            return Err(ContainerError::NamespaceError("User namespace not active".into()));
        }
        self.active = false;
        serial_println!("Exiting user namespace {}", self.id);
        Ok(())
    }
    
    fn get_id(&self) -> u32 {
        self.id
    }
}

pub struct UtsNamespace {
    id: u32,
    hostname: Mutex<String>,
    domainname: Mutex<String>,
    active: bool,
}

impl UtsNamespace {
    pub fn new(hostname: String) -> Result<Self, ContainerError> {
        Ok(Self {
            id: UTS_NS_COUNTER.fetch_add(1, Ordering::SeqCst),
            hostname: Mutex::new(hostname),
            domainname: Mutex::new("localdomain".to_string()),
            active: false,
        })
    }
    
    pub fn set_hostname(&self, hostname: String) {
        *self.hostname.lock() = hostname;
    }
    
    pub fn get_hostname(&self) -> String {
        self.hostname.lock().clone()
    }
    
    pub fn set_domainname(&self, domainname: String) {
        *self.domainname.lock() = domainname;
    }
    
    pub fn get_domainname(&self) -> String {
        self.domainname.lock().clone()
    }
}

impl Namespace for UtsNamespace {
    fn ns_type(&self) -> NamespaceType {
        NamespaceType::Uts
    }
    
    fn enter(&mut self) -> Result<(), ContainerError> {
        if self.active {
            return Err(ContainerError::NamespaceError("UTS namespace already active".into()));
        }
        self.active = true;
        serial_println!("Entering UTS namespace {} with hostname: {}", self.id, self.hostname.lock());
        Ok(())
    }
    
    fn exit(&mut self) -> Result<(), ContainerError> {
        if !self.active {
            return Err(ContainerError::NamespaceError("UTS namespace not active".into()));
        }
        self.active = false;
        serial_println!("Exiting UTS namespace {}", self.id);
        Ok(())
    }
    
    fn get_id(&self) -> u32 {
        self.id
    }
}