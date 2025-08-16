use super::NtStatus;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU64, Ordering};

// Object handle value
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Handle(pub u64);

impl Handle {
    pub const INVALID: Handle = Handle(0xFFFFFFFFFFFFFFFF);
    pub const NULL: Handle = Handle(0);
    
    pub fn new() -> Self {
        static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);
        Handle(NEXT_HANDLE.fetch_add(1, Ordering::SeqCst))
    }
    
    pub fn is_valid(&self) -> bool {
        self.0 != 0 && self.0 != Self::INVALID.0
    }
    
    pub fn from_raw(value: u64) -> Self {
        Handle(value)
    }
}

// Object types - must match Windows NT object types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Directory = 1,
    SymbolicLink = 2,
    Token = 3,
    Process = 4,
    Thread = 5,
    Job = 6,
    DebugObject = 7,
    Event = 8,
    EventPair = 9,
    Mutant = 10,
    Callback = 11,
    Semaphore = 12,
    Timer = 13,
    Profile = 14,
    KeyedEvent = 15,
    WindowStation = 16,
    Desktop = 17,
    Section = 18,
    Key = 19,
    Port = 20,
    WaitablePort = 21,
    Adapter = 22,
    Controller = 23,
    Device = 24,
    Driver = 25,
    IoCompletion = 26,
    File = 27,
    TmTx = 28,
    TmTm = 29,
    TmRm = 30,
    TmEn = 31,
    Type = 32,
    Session = 33,
    Partition = 34,
    FilterConnectionPort = 35,
    FilterCommunicationPort = 36,
    PcwObject = 37,
}

// Object attributes structure - compatible with Windows OBJECT_ATTRIBUTES
#[repr(C)]
#[derive(Debug)]
pub struct ObjectAttributes {
    pub length: u32,
    pub root_directory: Handle,
    pub object_name: Option<String>,
    pub attributes: ObjectAttributeFlags,
    pub security_descriptor: Option<*const u8>,
    pub security_quality_of_service: Option<*const u8>,
}

impl ObjectAttributes {
    pub fn new() -> Self {
        Self {
            length: core::mem::size_of::<ObjectAttributes>() as u32,
            root_directory: Handle::NULL,
            object_name: None,
            attributes: ObjectAttributeFlags::empty(),
            security_descriptor: None,
            security_quality_of_service: None,
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug)]
    pub struct ObjectAttributeFlags: u32 {
        const INHERIT = 0x00000002;
        const PERMANENT = 0x00000010;
        const EXCLUSIVE = 0x00000020;
        const CASE_INSENSITIVE = 0x00000040;
        const OPENIF = 0x00000080;
        const OPENLINK = 0x00000100;
        const KERNEL_HANDLE = 0x00000200;
        const FORCE_ACCESS_CHECK = 0x00000400;
        const IGNORE_IMPERSONATED_DEVICEMAP = 0x00000800;
        const DONT_REPARSE = 0x00001000;
    }
}

// Generic object header - every NT object starts with this
#[derive(Debug)]
pub struct ObjectHeader {
    pub object_type: ObjectType,
    pub reference_count: AtomicU64,
    pub handle_count: AtomicU64,
    pub name: Option<String>,
    pub parent_directory: Option<Handle>,
    pub security_descriptor: Option<Vec<u8>>,
}

impl ObjectHeader {
    pub fn new(object_type: ObjectType) -> Self {
        Self {
            object_type,
            reference_count: AtomicU64::new(1),
            handle_count: AtomicU64::new(0),
            name: None,
            parent_directory: None,
            security_descriptor: None,
        }
    }

    pub fn add_reference(&self) -> u64 {
        self.reference_count.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn remove_reference(&self) -> u64 {
        let old_count = self.reference_count.fetch_sub(1, Ordering::SeqCst);
        old_count.saturating_sub(1)
    }

    pub fn add_handle(&self) -> u64 {
        self.handle_count.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn remove_handle(&self) -> u64 {
        let old_count = self.handle_count.fetch_sub(1, Ordering::SeqCst);
        old_count.saturating_sub(1)
    }
}

// Object directory entry
#[derive(Debug)]
pub struct ObjectDirectoryEntry {
    pub header: ObjectHeader,
    pub entries: BTreeMap<String, Handle>,
}

impl ObjectDirectoryEntry {
    pub fn new() -> Self {
        Self {
            header: ObjectHeader::new(ObjectType::Directory),
            entries: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, handle: Handle) {
        self.entries.insert(name, handle);
    }

    pub fn lookup(&self, name: &str) -> Option<Handle> {
        self.entries.get(name).copied()
    }

    pub fn remove(&mut self, name: &str) -> Option<Handle> {
        self.entries.remove(name)
    }
}

// Object manager - manages all kernel objects
pub struct ObjectManager {
    objects: BTreeMap<Handle, Arc<Mutex<dyn ObjectTrait>>>,
    root_directory: Handle,
    next_handle: AtomicU64,
}

pub trait ObjectTrait: Send + Sync {
    fn get_header(&self) -> &ObjectHeader;
    fn get_header_mut(&mut self) -> &mut ObjectHeader;
    fn get_type(&self) -> ObjectType {
        self.get_header().object_type
    }
}

impl ObjectManager {
    pub fn new() -> Self {
        let mut manager = Self {
            objects: BTreeMap::new(),
            root_directory: Handle::NULL,
            next_handle: AtomicU64::new(1),
        };

        // Create root object directory
        let root_dir = Arc::new(Mutex::new(ObjectDirectoryEntry::new()));
        let root_handle = Handle::new();
        manager.objects.insert(root_handle, root_dir);
        manager.root_directory = root_handle;

        manager
    }

    pub fn create_object<T>(&mut self, object: T) -> Handle 
    where 
        T: ObjectTrait + 'static
    {
        let handle = Handle::new();
        let object_arc = Arc::new(Mutex::new(object));
        self.objects.insert(handle, object_arc);
        handle
    }

    pub fn reference_object(&self, handle: Handle) -> Option<Arc<Mutex<dyn ObjectTrait>>> {
        if let Some(object) = self.objects.get(&handle) {
            object.lock().get_header().add_reference();
            Some(object.clone())
        } else {
            None
        }
    }

    pub fn dereference_object(&mut self, handle: Handle) -> NtStatus {
        if let Some(object) = self.objects.get(&handle) {
            let ref_count = object.lock().get_header().remove_reference();
            if ref_count == 0 {
                self.objects.remove(&handle);
            }
            NtStatus::Success
        } else {
            NtStatus::InvalidHandle
        }
    }

    pub fn create_handle(&mut self, object_handle: Handle) -> Result<Handle, NtStatus> {
        if let Some(object) = self.objects.get(&object_handle) {
            object.lock().get_header().add_handle();
            let handle = Handle::new();
            // In a real implementation, we'd have a separate handle table per process
            Ok(handle)
        } else {
            Err(NtStatus::InvalidHandle)
        }
    }

    pub fn close_handle(&mut self, handle: Handle) -> NtStatus {
        // Find the object for this handle
        if let Some(object) = self.objects.get(&handle) {
            let handle_count = object.lock().get_header().remove_handle();
            
            // If no more handles and no references, delete the object
            let ref_count = object.lock().get_header().reference_count.load(Ordering::SeqCst);
            if handle_count == 0 && ref_count == 0 {
                self.objects.remove(&handle);
            }
            
            NtStatus::Success
        } else {
            NtStatus::InvalidHandle
        }
    }

    pub fn lookup_object_by_name(&self, _name: &str) -> Option<Handle> {
        // Simplified implementation - just return None for now
        None
    }

    pub fn insert_object(&mut self, _name: String, _handle: Handle) -> NtStatus {
        // Simplified implementation - just return success
        NtStatus::Success
    }
}

// Extension trait for downcasting
pub trait ObjectDowncast {
    fn as_any(&self) -> &dyn core::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any;
}

impl<T: 'static> ObjectDowncast for T {
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

impl ObjectTrait for ObjectDirectoryEntry {
    fn get_header(&self) -> &ObjectHeader {
        &self.header
    }

    fn get_header_mut(&mut self) -> &mut ObjectHeader {
        &mut self.header
    }
}

lazy_static! {
    pub static ref OBJECT_MANAGER: Mutex<ObjectManager> = Mutex::new(ObjectManager::new());
}

// NT Object Manager API functions
pub fn nt_create_directory_object(
    directory_handle: &mut Handle,
    desired_access: u32,
    object_attributes: &ObjectAttributes,
) -> NtStatus {
    let mut om = OBJECT_MANAGER.lock();
    
    let directory = ObjectDirectoryEntry::new();
    let handle = om.create_object(directory);
    
    if let Some(name) = &object_attributes.object_name {
        let _ = om.insert_object(name.clone(), handle);
    }
    
    *directory_handle = handle;
    NtStatus::Success
}

pub fn nt_open_directory_object(
    directory_handle: &mut Handle,
    desired_access: u32,
    object_attributes: &ObjectAttributes,
) -> NtStatus {
    let om = OBJECT_MANAGER.lock();
    
    if let Some(name) = &object_attributes.object_name {
        if let Some(handle) = om.lookup_object_by_name(name) {
            *directory_handle = handle;
            return NtStatus::Success;
        }
    }
    
    NtStatus::ObjectNameNotFound
}

pub fn nt_close(handle: Handle) -> NtStatus {
    let mut om = OBJECT_MANAGER.lock();
    om.close_handle(handle)
}