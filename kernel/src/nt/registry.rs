use super::{NtStatus, object::{Handle, ObjectHeader, ObjectTrait, ObjectType}};
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::{Vec, self};
use alloc::format;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::AtomicU64;

// Registry value types - matching Windows registry exactly
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryValueType {
    RegNone = 0,
    RegSz = 1,                    // Null-terminated string
    RegExpandSz = 2,              // Null-terminated string with environment variables
    RegBinary = 3,                // Binary data
    RegDword = 4,                 // 32-bit number
    RegDwordBigEndian = 5,        // 32-bit number (big-endian)
    RegLink = 6,                  // Symbolic link
    RegMultiSz = 7,               // Multiple null-terminated strings
    RegResourceList = 8,          // Resource list in hardware description
    RegFullResourceDescriptor = 9, // Resource descriptor in hardware description
    RegResourceRequirementsList = 10, // Resource requirements list
    RegQword = 11,                // 64-bit number
}

// Registry key disposition codes
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryDisposition {
    RegCreatedNewKey = 1,
    RegOpenedExistingKey = 2,
}

// Registry access rights
bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct RegistryRights: u32 {
        const KEY_QUERY_VALUE = 0x0001;
        const KEY_SET_VALUE = 0x0002;
        const KEY_CREATE_SUB_KEY = 0x0004;
        const KEY_ENUMERATE_SUB_KEYS = 0x0008;
        const KEY_NOTIFY = 0x0010;
        const KEY_CREATE_LINK = 0x0020;
        const KEY_WOW64_32KEY = 0x0200;
        const KEY_WOW64_64KEY = 0x0100;
        const KEY_WOW64_RES = 0x0300;
        
        const KEY_READ = Self::KEY_QUERY_VALUE.bits() |
                        Self::KEY_ENUMERATE_SUB_KEYS.bits() |
                        Self::KEY_NOTIFY.bits();
        
        const KEY_WRITE = Self::KEY_SET_VALUE.bits() |
                         Self::KEY_CREATE_SUB_KEY.bits();
        
        const KEY_EXECUTE = Self::KEY_READ.bits();
        
        const KEY_ALL_ACCESS = Self::KEY_QUERY_VALUE.bits() |
                              Self::KEY_SET_VALUE.bits() |
                              Self::KEY_CREATE_SUB_KEY.bits() |
                              Self::KEY_ENUMERATE_SUB_KEYS.bits() |
                              Self::KEY_NOTIFY.bits() |
                              Self::KEY_CREATE_LINK.bits() |
                              0x000F0000; // STANDARD_RIGHTS_ALL
    }
}

// Registry key creation options
bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct RegistryOptions: u32 {
        const REG_OPTION_RESERVED = 0x00000000;
        const REG_OPTION_NON_VOLATILE = 0x00000000;
        const REG_OPTION_VOLATILE = 0x00000001;
        const REG_OPTION_CREATE_LINK = 0x00000002;
        const REG_OPTION_BACKUP_RESTORE = 0x00000004;
        const REG_OPTION_OPEN_LINK = 0x00000008;
    }
}

// Registry value entry
#[derive(Debug, Clone)]
pub struct RegistryValue {
    pub name: String,
    pub value_type: RegistryValueType,
    pub data: Vec<u8>,
    pub data_size: u32,
}

impl RegistryValue {
    pub fn new_string(name: String, value: String) -> Self {
        let mut data = value.into_bytes();
        data.push(0); // Null terminator
        
        Self {
            name,
            value_type: RegistryValueType::RegSz,
            data_size: data.len() as u32,
            data,
        }
    }
    
    pub fn new_dword(name: String, value: u32) -> Self {
        Self {
            name,
            value_type: RegistryValueType::RegDword,
            data: value.to_le_bytes().to_vec(),
            data_size: 4,
        }
    }
    
    pub fn new_qword(name: String, value: u64) -> Self {
        Self {
            name,
            value_type: RegistryValueType::RegQword,
            data: value.to_le_bytes().to_vec(),
            data_size: 8,
        }
    }
    
    pub fn new_binary(name: String, data: Vec<u8>) -> Self {
        let size = data.len() as u32;
        Self {
            name,
            value_type: RegistryValueType::RegBinary,
            data_size: size,
            data,
        }
    }
    
    pub fn as_string(&self) -> Option<String> {
        if self.value_type == RegistryValueType::RegSz || 
           self.value_type == RegistryValueType::RegExpandSz {
            let null_pos = self.data.iter().position(|&b| b == 0).unwrap_or(self.data.len());
            String::from_utf8(self.data[..null_pos].to_vec()).ok()
        } else {
            None
        }
    }
    
    pub fn as_dword(&self) -> Option<u32> {
        if self.value_type == RegistryValueType::RegDword && self.data.len() >= 4 {
            Some(u32::from_le_bytes([self.data[0], self.data[1], self.data[2], self.data[3]]))
        } else {
            None
        }
    }
    
    pub fn as_qword(&self) -> Option<u64> {
        if self.value_type == RegistryValueType::RegQword && self.data.len() >= 8 {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&self.data[..8]);
            Some(u64::from_le_bytes(bytes))
        } else {
            None
        }
    }
}

// Registry key
#[derive(Debug)]
pub struct RegistryKey {
    pub header: ObjectHeader,
    pub name: String,
    pub path: String,
    pub parent: Option<Handle>,
    pub subkeys: BTreeMap<String, Handle>,
    pub values: BTreeMap<String, RegistryValue>,
    pub last_write_time: u64,
    pub security_descriptor: Option<Vec<u8>>,
    pub flags: RegistryOptions,
}

impl RegistryKey {
    pub fn new(name: String, path: String) -> Self {
        Self {
            header: ObjectHeader::new(ObjectType::Key),
            name,
            path,
            parent: None,
            subkeys: BTreeMap::new(),
            values: BTreeMap::new(),
            last_write_time: 0, // Would be current NT time
            security_descriptor: None,
            flags: RegistryOptions::REG_OPTION_NON_VOLATILE,
        }
    }
    
    pub fn add_subkey(&mut self, name: String, handle: Handle) {
        self.subkeys.insert(name, handle);
        self.update_last_write_time();
    }
    
    pub fn remove_subkey(&mut self, name: &str) -> Option<Handle> {
        let result = self.subkeys.remove(name);
        if result.is_some() {
            self.update_last_write_time();
        }
        result
    }
    
    pub fn set_value(&mut self, name: String, value: RegistryValue) {
        self.values.insert(name, value);
        self.update_last_write_time();
    }
    
    pub fn get_value(&self, name: &str) -> Option<&RegistryValue> {
        self.values.get(name)
    }
    
    pub fn remove_value(&mut self, name: &str) -> Option<RegistryValue> {
        let result = self.values.remove(name);
        if result.is_some() {
            self.update_last_write_time();
        }
        result
    }
    
    pub fn enumerate_subkeys(&self) -> Vec<String> {
        self.subkeys.keys().cloned().collect()
    }
    
    pub fn enumerate_values(&self) -> Vec<String> {
        self.values.keys().cloned().collect()
    }
    
    fn update_last_write_time(&mut self) {
        // In a real implementation, this would be current NT FILETIME
        self.last_write_time = 0x01D6E1A0E1A0E1A0;
    }
}

impl ObjectTrait for RegistryKey {
    fn get_header(&self) -> &ObjectHeader {
        &self.header
    }
    
    fn get_header_mut(&mut self) -> &mut ObjectHeader {
        &mut self.header
    }
}

// Registry predefined keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PredefinedKey {
    HkeyClassesRoot,
    HkeyCurrentUser,
    HkeyLocalMachine,
    HkeyUsers,
    HkeyPerformanceData,
    HkeyCurrentConfig,
    HkeyDynData,
}

impl PredefinedKey {
    pub fn to_path(&self) -> &'static str {
        match self {
            PredefinedKey::HkeyClassesRoot => "\\Registry\\Machine\\Software\\Classes",
            PredefinedKey::HkeyCurrentUser => "\\Registry\\User\\CurrentUser",
            PredefinedKey::HkeyLocalMachine => "\\Registry\\Machine",
            PredefinedKey::HkeyUsers => "\\Registry\\User",
            PredefinedKey::HkeyPerformanceData => "\\Registry\\PerfData",
            PredefinedKey::HkeyCurrentConfig => "\\Registry\\Machine\\System\\CurrentControlSet\\Hardware Profiles\\Current",
            PredefinedKey::HkeyDynData => "\\Registry\\DynData",
        }
    }
}

// Registry manager
pub struct RegistryManager {
    keys: BTreeMap<Handle, RegistryKey>,
    path_to_handle: BTreeMap<String, Handle>,
    predefined_keys: BTreeMap<PredefinedKey, Handle>,
    next_handle: AtomicU64,
}

impl RegistryManager {
    pub fn new() -> Self {
        Self {
            keys: BTreeMap::new(),
            path_to_handle: BTreeMap::new(),
            predefined_keys: BTreeMap::new(),
            next_handle: AtomicU64::new(1),
        }
    }
    
    pub fn initialize_predefined_keys(&mut self) -> NtStatus {
        use crate::serial_println;
        
        serial_println!("Registry: Creating predefined registry keys");
        
        // Create root registry key
        let root_handle = match self.create_key_internal("Registry".to_string(), "\\Registry".to_string(), None) {
            Ok(handle) => handle,
            Err(status) => return status,
        };
        
        // Create main hives
        let machine_handle = match self.create_key_internal("Machine".to_string(), "\\Registry\\Machine".to_string(), Some(root_handle)) {
            Ok(handle) => handle,
            Err(status) => return status,
        };
        let user_handle = match self.create_key_internal("User".to_string(), "\\Registry\\User".to_string(), Some(root_handle)) {
            Ok(handle) => handle,
            Err(status) => return status,
        };
        
        // Create HKLM subkeys
        let _ = match self.create_key_internal("Software".to_string(), "\\Registry\\Machine\\Software".to_string(), Some(machine_handle)) {
            Ok(handle) => handle,
            Err(status) => return status,
        };
        let _ = match self.create_key_internal("System".to_string(), "\\Registry\\Machine\\System".to_string(), Some(machine_handle)) {
            Ok(handle) => handle,
            Err(status) => return status,
        };
        let _ = match self.create_key_internal("Hardware".to_string(), "\\Registry\\Machine\\Hardware".to_string(), Some(machine_handle)) {
            Ok(handle) => handle,
            Err(status) => return status,
        };
        let _ = match self.create_key_internal("Security".to_string(), "\\Registry\\Machine\\Security".to_string(), Some(machine_handle)) {
            Ok(handle) => handle,
            Err(status) => return status,
        };
        let _ = match self.create_key_internal("SAM".to_string(), "\\Registry\\Machine\\SAM".to_string(), Some(machine_handle)) {
            Ok(handle) => handle,
            Err(status) => return status,
        };
        
        // Create HKLM\Software subkeys
        let software_handle_opt = self.path_to_handle.get("\\Registry\\Machine\\Software").copied();
        if let Some(software_handle) = software_handle_opt {
            let _ = match self.create_key_internal("Classes".to_string(), "\\Registry\\Machine\\Software\\Classes".to_string(), Some(software_handle)) {
                Ok(handle) => handle,
                Err(status) => return status,
            };
            let _ = match self.create_key_internal("Microsoft".to_string(), "\\Registry\\Machine\\Software\\Microsoft".to_string(), Some(software_handle)) {
                Ok(handle) => handle,
                Err(status) => return status,
            };
        }
        
        // Create HKLM\System subkeys
        let system_handle_opt = self.path_to_handle.get("\\Registry\\Machine\\System").copied();
        if let Some(system_handle) = system_handle_opt {
            let _ = match self.create_key_internal("CurrentControlSet".to_string(), "\\Registry\\Machine\\System\\CurrentControlSet".to_string(), Some(system_handle)) {
                Ok(handle) => handle,
                Err(status) => return status,
            };
            let _ = match self.create_key_internal("ControlSet001".to_string(), "\\Registry\\Machine\\System\\ControlSet001".to_string(), Some(system_handle)) {
                Ok(handle) => handle,
                Err(status) => return status,
            };
        }
        
        // Map predefined keys
        self.predefined_keys.insert(PredefinedKey::HkeyLocalMachine, machine_handle);
        self.predefined_keys.insert(PredefinedKey::HkeyUsers, user_handle);
        
        if let Some(&classes_handle) = self.path_to_handle.get("\\Registry\\Machine\\Software\\Classes") {
            self.predefined_keys.insert(PredefinedKey::HkeyClassesRoot, classes_handle);
        }
        
        serial_println!("Registry: Predefined keys created successfully");
        match self.populate_system_values() {
            NtStatus::Success => {},
            status => return status,
        }
        
        NtStatus::Success
    }
    
    fn populate_system_values(&mut self) -> NtStatus {
        use crate::serial_println;
        
        serial_println!("Registry: Populating system registry values");
        
        // Populate version information
        if let Some(&machine_handle) = self.predefined_keys.get(&PredefinedKey::HkeyLocalMachine) {
            // Create Windows version key
            let version_handle = match self.create_key_internal(
                "Windows NT".to_string(),
                "\\Registry\\Machine\\Software\\Microsoft\\Windows NT".to_string(),
                Some(machine_handle)
            ) {
                Ok(handle) => handle,
                Err(status) => return status,
            };
            
            let current_version_handle = match self.create_key_internal(
                "CurrentVersion".to_string(),
                "\\Registry\\Machine\\Software\\Microsoft\\Windows NT\\CurrentVersion".to_string(),
                Some(version_handle)
            ) {
                Ok(handle) => handle,
                Err(status) => return status,
            };
            
            if let Some(key) = self.keys.get_mut(&current_version_handle) {
                key.set_value("ProductName".to_string(), 
                    RegistryValue::new_string("ProductName".to_string(), "ReactOS Rust Kernel".to_string()));
                key.set_value("CurrentVersion".to_string(),
                    RegistryValue::new_string("CurrentVersion".to_string(), "6.3".to_string()));
                key.set_value("CurrentBuild".to_string(),
                    RegistryValue::new_string("CurrentBuild".to_string(), "9600".to_string()));
                key.set_value("BuildLab".to_string(),
                    RegistryValue::new_string("BuildLab".to_string(), "rust-reactos".to_string()));
                key.set_value("InstallDate".to_string(),
                    RegistryValue::new_dword("InstallDate".to_string(), 0x5F000000));
            }
        }
        
        // Populate hardware information
        if let Some(&hardware_handle) = self.path_to_handle.get("\\Registry\\Machine\\Hardware") {
            let desc_handle = match self.create_key_internal(
                "Description".to_string(),
                "\\Registry\\Machine\\Hardware\\Description".to_string(),
                Some(hardware_handle)
            ) {
                Ok(handle) => handle,
                Err(status) => return status,
            };
            
            let system_handle = match self.create_key_internal(
                "System".to_string(),
                "\\Registry\\Machine\\Hardware\\Description\\System".to_string(),
                Some(desc_handle)
            ) {
                Ok(handle) => handle,
                Err(status) => return status,
            };
            
            if let Some(key) = self.keys.get_mut(&system_handle) {
                key.set_value("Identifier".to_string(),
                    RegistryValue::new_string("Identifier".to_string(), "AT/AT COMPATIBLE".to_string()));
                key.set_value("Configuration Data".to_string(),
                    RegistryValue::new_binary("Configuration Data".to_string(), alloc::vec![0u8; 64]));
            }
        }
        
        serial_println!("Registry: System values populated");
        NtStatus::Success
    }
    
    fn create_key_internal(&mut self, name: String, path: String, parent: Option<Handle>) -> Result<Handle, NtStatus> {
        let handle = Handle::new();
        let mut key = RegistryKey::new(name.clone(), path.clone());
        key.parent = parent;
        
        // Add to parent's subkeys
        if let Some(parent_handle) = parent {
            if let Some(parent_key) = self.keys.get_mut(&parent_handle) {
                parent_key.add_subkey(name, handle);
            }
        }
        
        self.keys.insert(handle, key);
        self.path_to_handle.insert(path, handle);
        
        Ok(handle)
    }
    
    pub fn create_key(
        &mut self,
        parent_key: Option<Handle>,
        key_name: &str,
        options: RegistryOptions,
        desired_access: RegistryRights,
    ) -> Result<(Handle, RegistryDisposition), NtStatus> {
        let parent_path = if let Some(parent) = parent_key {
            if let Some(key) = self.keys.get(&parent) {
                key.path.clone()
            } else {
                return Err(NtStatus::InvalidHandle);
            }
        } else {
            "\\Registry".to_string()
        };
        
        let full_path = if parent_path == "\\Registry" {
            format!("\\Registry\\{}", key_name)
        } else {
            format!("{}\\{}", parent_path, key_name)
        };
        
        // Check if key already exists
        if let Some(&existing_handle) = self.path_to_handle.get(&full_path) {
            return Ok((existing_handle, RegistryDisposition::RegOpenedExistingKey));
        }
        
        // Create new key
        let handle = self.create_key_internal(key_name.to_string(), full_path, parent_key)?;
        
        if let Some(key) = self.keys.get_mut(&handle) {
            key.flags = options;
        }
        
        Ok((handle, RegistryDisposition::RegCreatedNewKey))
    }
    
    pub fn open_key(&self, parent_key: Option<Handle>, key_name: &str) -> Result<Handle, NtStatus> {
        let full_path = if let Some(parent) = parent_key {
            if let Some(key) = self.keys.get(&parent) {
                format!("{}\\{}", key.path, key_name)
            } else {
                return Err(NtStatus::InvalidHandle);
            }
        } else {
            format!("\\Registry\\{}", key_name)
        };
        
        self.path_to_handle.get(&full_path)
            .copied()
            .ok_or(NtStatus::ObjectNameNotFound)
    }
    
    pub fn delete_key(&mut self, key_handle: Handle) -> NtStatus {
        if let Some(key) = self.keys.get(&key_handle) {
            // Check if key has subkeys (Windows doesn't allow deletion of keys with subkeys)
            if !key.subkeys.is_empty() {
                return NtStatus::AccessDenied;
            }
            
            let path = key.path.clone();
            let parent = key.parent;
            let name = key.name.clone();
            
            // Remove from parent's subkeys
            if let Some(parent_handle) = parent {
                if let Some(parent_key) = self.keys.get_mut(&parent_handle) {
                    parent_key.remove_subkey(&name);
                }
            }
            
            self.keys.remove(&key_handle);
            self.path_to_handle.remove(&path);
            
            NtStatus::Success
        } else {
            NtStatus::InvalidHandle
        }
    }
    
    pub fn set_value(&mut self, key_handle: Handle, value_name: &str, value: RegistryValue) -> NtStatus {
        if let Some(key) = self.keys.get_mut(&key_handle) {
            key.set_value(value_name.to_string(), value);
            NtStatus::Success
        } else {
            NtStatus::InvalidHandle
        }
    }
    
    pub fn get_value(&self, key_handle: Handle, value_name: &str) -> Result<&RegistryValue, NtStatus> {
        if let Some(key) = self.keys.get(&key_handle) {
            key.get_value(value_name).ok_or(NtStatus::ObjectNameNotFound)
        } else {
            Err(NtStatus::InvalidHandle)
        }
    }
    
    pub fn delete_value(&mut self, key_handle: Handle, value_name: &str) -> NtStatus {
        if let Some(key) = self.keys.get_mut(&key_handle) {
            if key.remove_value(value_name).is_some() {
                NtStatus::Success
            } else {
                NtStatus::ObjectNameNotFound
            }
        } else {
            NtStatus::InvalidHandle
        }
    }
    
    pub fn enumerate_key(&self, key_handle: Handle, index: u32) -> Result<String, NtStatus> {
        if let Some(key) = self.keys.get(&key_handle) {
            let subkeys = key.enumerate_subkeys();
            if (index as usize) < subkeys.len() {
                Ok(subkeys[index as usize].clone())
            } else {
                Err(NtStatus::NoMoreEntries)
            }
        } else {
            Err(NtStatus::InvalidHandle)
        }
    }
    
    pub fn enumerate_value(&self, key_handle: Handle, index: u32) -> Result<String, NtStatus> {
        if let Some(key) = self.keys.get(&key_handle) {
            let values = key.enumerate_values();
            if (index as usize) < values.len() {
                Ok(values[index as usize].clone())
            } else {
                Err(NtStatus::NoMoreEntries)
            }
        } else {
            Err(NtStatus::InvalidHandle)
        }
    }
    
    pub fn get_predefined_key(&self, predefined: PredefinedKey) -> Option<Handle> {
        self.predefined_keys.get(&predefined).copied()
    }
}

// Global registry manager
lazy_static! {
    pub static ref REGISTRY_MANAGER: Mutex<RegistryManager> = Mutex::new(RegistryManager::new());
}

// Public API functions
pub fn initialize_registry() -> NtStatus {
    use crate::serial_println;
    
    serial_println!("Registry: Initializing Windows-compatible registry");
    
    let mut manager = REGISTRY_MANAGER.lock();
    manager.initialize_predefined_keys()
}

pub fn nt_create_key(
    key_handle: &mut Handle,
    desired_access: RegistryRights,
    object_attributes: &str, // Simplified - normally OBJECT_ATTRIBUTES
    options: RegistryOptions,
    disposition: &mut RegistryDisposition,
) -> NtStatus {
    let mut manager = REGISTRY_MANAGER.lock();
    
    match manager.create_key(None, object_attributes, options, desired_access) {
        Ok((handle, disp)) => {
            *key_handle = handle;
            *disposition = disp;
            NtStatus::Success
        }
        Err(status) => status,
    }
}

pub fn nt_open_key(
    key_handle: &mut Handle,
    desired_access: RegistryRights,
    object_attributes: &str,
) -> NtStatus {
    let manager = REGISTRY_MANAGER.lock();
    
    match manager.open_key(None, object_attributes) {
        Ok(handle) => {
            *key_handle = handle;
            NtStatus::Success
        }
        Err(status) => status,
    }
}

pub fn nt_set_value_key(
    key_handle: Handle,
    value_name: &str,
    value_type: RegistryValueType,
    data: &[u8],
) -> NtStatus {
    let mut manager = REGISTRY_MANAGER.lock();
    
    let value = RegistryValue {
        name: value_name.to_string(),
        value_type,
        data: data.to_vec(),
        data_size: data.len() as u32,
    };
    
    manager.set_value(key_handle, value_name, value)
}

pub fn nt_query_value_key(
    key_handle: Handle,
    value_name: &str,
    value_type: &mut RegistryValueType,
    data: &mut [u8],
    data_size: &mut u32,
) -> NtStatus {
    let manager = REGISTRY_MANAGER.lock();
    
    match manager.get_value(key_handle, value_name) {
        Ok(value) => {
            *value_type = value.value_type;
            *data_size = value.data_size;
            
            if data.len() >= value.data.len() {
                data[..value.data.len()].copy_from_slice(&value.data);
                NtStatus::Success
            } else {
                NtStatus::BufferTooSmall
            }
        }
        Err(status) => status,
    }
}

pub fn nt_delete_key(key_handle: Handle) -> NtStatus {
    let mut manager = REGISTRY_MANAGER.lock();
    manager.delete_key(key_handle)
}

pub fn nt_delete_value_key(key_handle: Handle, value_name: &str) -> NtStatus {
    let mut manager = REGISTRY_MANAGER.lock();
    manager.delete_value(key_handle, value_name)
}

pub fn nt_enumerate_key(
    key_handle: Handle,
    index: u32,
    key_name: &mut String,
) -> NtStatus {
    let manager = REGISTRY_MANAGER.lock();
    
    match manager.enumerate_key(key_handle, index) {
        Ok(name) => {
            *key_name = name;
            NtStatus::Success
        }
        Err(status) => status,
    }
}

pub fn nt_enumerate_value_key(
    key_handle: Handle,
    index: u32,
    value_name: &mut String,
) -> NtStatus {
    let manager = REGISTRY_MANAGER.lock();
    
    match manager.enumerate_value(key_handle, index) {
        Ok(name) => {
            *value_name = name;
            NtStatus::Success
        }
        Err(status) => status,
    }
}