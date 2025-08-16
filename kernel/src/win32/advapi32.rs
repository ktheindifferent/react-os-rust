// ADVAPI32.DLL - Advanced Windows API implementation
use super::*;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;
use core::ffi::CStr;

// Define HKEY type locally if not defined elsewhere
type HKEY = usize;

// Registry root keys
const HKEY_CLASSES_ROOT: HKEY = 0x80000000;
const HKEY_CURRENT_USER: HKEY = 0x80000001;
const HKEY_LOCAL_MACHINE: HKEY = 0x80000002;
const HKEY_USERS: HKEY = 0x80000003;
const HKEY_CURRENT_CONFIG: HKEY = 0x80000005;

// Simple in-memory registry emulation
lazy_static! {
    static ref REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());
}

struct Registry {
    hives: BTreeMap<HKEY, RegistryHive>,
}

struct RegistryHive {
    keys: BTreeMap<String, RegistryKey>,
}

struct RegistryKey {
    values: BTreeMap<String, RegistryValue>,
    subkeys: BTreeMap<String, RegistryKey>,
}

#[derive(Clone)]
enum RegistryValue {
    String(String),
    Dword(u32),
    Binary(Vec<u8>),
}

impl Registry {
    fn new() -> Self {
        let mut registry = Self {
            hives: BTreeMap::new(),
        };
        
        // Initialize standard hives
        registry.hives.insert(HKEY_CLASSES_ROOT, RegistryHive::new());
        registry.hives.insert(HKEY_CURRENT_USER, RegistryHive::new());
        registry.hives.insert(HKEY_LOCAL_MACHINE, RegistryHive::new());
        registry.hives.insert(HKEY_USERS, RegistryHive::new());
        registry.hives.insert(HKEY_CURRENT_CONFIG, RegistryHive::new());
        
        // Add some default keys
        if let Some(hklm) = registry.hives.get_mut(&HKEY_LOCAL_MACHINE) {
            let mut software = RegistryKey::new();
            let mut microsoft = RegistryKey::new();
            let mut windows = RegistryKey::new();
            let mut current_version = RegistryKey::new();
            
            // Add version info
            current_version.values.insert(
                String::from("Version"),
                RegistryValue::String(String::from("10.0"))
            );
            current_version.values.insert(
                String::from("ProductName"),
                RegistryValue::String(String::from("Rust OS"))
            );
            current_version.values.insert(
                String::from("BuildNumber"),
                RegistryValue::Dword(19045)
            );
            
            windows.subkeys.insert(String::from("CurrentVersion"), current_version);
            microsoft.subkeys.insert(String::from("Windows"), windows);
            software.subkeys.insert(String::from("Microsoft"), microsoft);
            hklm.keys.insert(String::from("SOFTWARE"), software);
        }
        
        registry
    }
    
    fn open_key(&self, hkey: HKEY, subkey: &str) -> Option<HKEY> {
        // Simplified - just check if key exists
        if self.hives.contains_key(&hkey) {
            // Return a fake handle
            Some((hkey as usize + subkey.len()) as HKEY)
        } else {
            None
        }
    }
    
    fn query_value(&self, hkey: HKEY, value_name: &str) -> Option<RegistryValue> {
        // Simplified - return dummy values
        match value_name {
            "Version" => Some(RegistryValue::String(String::from("10.0"))),
            "ProductName" => Some(RegistryValue::String(String::from("Rust OS"))),
            _ => None,
        }
    }
    
    fn set_value(&mut self, hkey: HKEY, value_name: &str, value: RegistryValue) -> bool {
        // Simplified - just succeed
        true
    }
}

impl RegistryHive {
    fn new() -> Self {
        Self {
            keys: BTreeMap::new(),
        }
    }
}

impl RegistryKey {
    fn new() -> Self {
        Self {
            values: BTreeMap::new(),
            subkeys: BTreeMap::new(),
        }
    }
}

// Registry value types
pub const REG_NONE: DWORD = 0;
pub const REG_SZ: DWORD = 1;
pub const REG_EXPAND_SZ: DWORD = 2;
pub const REG_BINARY: DWORD = 3;
pub const REG_DWORD: DWORD = 4;
pub const REG_DWORD_BIG_ENDIAN: DWORD = 5;
pub const REG_LINK: DWORD = 6;
pub const REG_MULTI_SZ: DWORD = 7;
pub const REG_QWORD: DWORD = 11;

// Registry access rights
pub const KEY_QUERY_VALUE: DWORD = 0x0001;
pub const KEY_SET_VALUE: DWORD = 0x0002;
pub const KEY_CREATE_SUB_KEY: DWORD = 0x0004;
pub const KEY_ENUMERATE_SUB_KEYS: DWORD = 0x0008;
pub const KEY_NOTIFY: DWORD = 0x0010;
pub const KEY_CREATE_LINK: DWORD = 0x0020;
pub const KEY_READ: DWORD = 0x20019;
pub const KEY_WRITE: DWORD = 0x20006;
pub const KEY_EXECUTE: DWORD = 0x20019;
pub const KEY_ALL_ACCESS: DWORD = 0xF003F;

/// RegOpenKeyExA - Open registry key
#[no_mangle]
pub extern "C" fn RegOpenKeyExA(
    hkey: HKEY,
    subkey: LPCSTR,
    options: DWORD,
    sam_desired: DWORD,
    phk_result: *mut HKEY,
) -> DWORD {
    if subkey.is_null() || phk_result.is_null() {
        return 87; // ERROR_INVALID_PARAMETER
    }
    
    let key_name = unsafe {
        match CStr::from_ptr(subkey as *const i8).to_str() {
            Ok(s) => s,
            Err(_) => return 87, // ERROR_INVALID_PARAMETER
        }
    };
    
    let registry = REGISTRY.lock();
    if let Some(handle) = registry.open_key(hkey, key_name) {
        unsafe {
            *phk_result = handle;
        }
        0 // ERROR_SUCCESS
    } else {
        2 // ERROR_FILE_NOT_FOUND
    }
}

/// RegCloseKey - Close registry key
#[no_mangle]
pub extern "C" fn RegCloseKey(hkey: HKEY) -> DWORD {
    // Simplified - always succeed
    0 // ERROR_SUCCESS
}

/// RegQueryValueExA - Query registry value
#[no_mangle]
pub extern "C" fn RegQueryValueExA(
    hkey: HKEY,
    value_name: LPCSTR,
    reserved: *mut DWORD,
    type_ptr: *mut DWORD,
    data: *mut u8,
    cb_data: *mut DWORD,
) -> DWORD {
    let name = if value_name.is_null() {
        ""
    } else {
        unsafe {
            match CStr::from_ptr(value_name as *const i8).to_str() {
                Ok(s) => s,
                Err(_) => return 87, // ERROR_INVALID_PARAMETER
            }
        }
    };
    
    let registry = REGISTRY.lock();
    if let Some(value) = registry.query_value(hkey, name) {
        match value {
            RegistryValue::String(s) => {
                if !type_ptr.is_null() {
                    unsafe { *type_ptr = REG_SZ; }
                }
                if !data.is_null() && !cb_data.is_null() {
                    let bytes = s.as_bytes();
                    let needed_size = bytes.len() + 1; // +1 for null terminator
                    
                    unsafe {
                        if *cb_data >= needed_size as DWORD {
                            core::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
                            *data.add(bytes.len()) = 0; // Null terminator
                        }
                        *cb_data = needed_size as DWORD;
                    }
                }
            }
            RegistryValue::Dword(d) => {
                if !type_ptr.is_null() {
                    unsafe { *type_ptr = REG_DWORD; }
                }
                if !data.is_null() && !cb_data.is_null() {
                    unsafe {
                        if *cb_data >= 4 {
                            *(data as *mut DWORD) = d;
                        }
                        *cb_data = 4;
                    }
                }
            }
            RegistryValue::Binary(b) => {
                if !type_ptr.is_null() {
                    unsafe { *type_ptr = REG_BINARY; }
                }
                if !data.is_null() && !cb_data.is_null() {
                    unsafe {
                        let needed_size = b.len() as DWORD;
                        if *cb_data >= needed_size {
                            core::ptr::copy_nonoverlapping(b.as_ptr(), data, b.len());
                        }
                        *cb_data = needed_size;
                    }
                }
            }
        }
        0 // ERROR_SUCCESS
    } else {
        2 // ERROR_FILE_NOT_FOUND
    }
}

/// RegSetValueExA - Set registry value
#[no_mangle]
pub extern "C" fn RegSetValueExA(
    hkey: HKEY,
    value_name: LPCSTR,
    reserved: DWORD,
    type_: DWORD,
    data: *const u8,
    cb_data: DWORD,
) -> DWORD {
    if data.is_null() {
        return 87; // ERROR_INVALID_PARAMETER
    }
    
    let name = if value_name.is_null() {
        String::new()
    } else {
        unsafe {
            match CStr::from_ptr(value_name as *const i8).to_str() {
                Ok(s) => String::from(s),
                Err(_) => return 87, // ERROR_INVALID_PARAMETER
            }
        }
    };
    
    let value = match type_ {
        REG_SZ | REG_EXPAND_SZ => {
            let bytes = unsafe {
                core::slice::from_raw_parts(data, cb_data as usize)
            };
            // Find null terminator
            let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            let s = String::from_utf8_lossy(&bytes[..len]).to_string();
            RegistryValue::String(s)
        }
        REG_DWORD => {
            if cb_data < 4 {
                return 87; // ERROR_INVALID_PARAMETER
            }
            let dword = unsafe { *(data as *const DWORD) };
            RegistryValue::Dword(dword)
        }
        REG_BINARY | _ => {
            let bytes = unsafe {
                core::slice::from_raw_parts(data, cb_data as usize)
            };
            RegistryValue::Binary(bytes.to_vec())
        }
    };
    
    let mut registry = REGISTRY.lock();
    if registry.set_value(hkey, &name, value) {
        0 // ERROR_SUCCESS
    } else {
        5 // ERROR_ACCESS_DENIED
    }
}

/// RegCreateKeyExA - Create registry key
#[no_mangle]
pub extern "C" fn RegCreateKeyExA(
    hkey: HKEY,
    subkey: LPCSTR,
    reserved: DWORD,
    class: LPSTR,
    options: DWORD,
    sam_desired: DWORD,
    security_attributes: *mut u8,
    phk_result: *mut HKEY,
    disposition: *mut DWORD,
) -> DWORD {
    if subkey.is_null() || phk_result.is_null() {
        return 87; // ERROR_INVALID_PARAMETER
    }
    
    let key_name = unsafe {
        match CStr::from_ptr(subkey as *const i8).to_str() {
            Ok(s) => s,
            Err(_) => return 87, // ERROR_INVALID_PARAMETER
        }
    };
    
    // For now, just return a dummy handle
    unsafe {
        *phk_result = (hkey as usize + key_name.len()) as HKEY;
        if !disposition.is_null() {
            *disposition = 1; // REG_CREATED_NEW_KEY
        }
    }
    
    0 // ERROR_SUCCESS
}

/// RegDeleteKeyA - Delete registry key
#[no_mangle]
pub extern "C" fn RegDeleteKeyA(
    hkey: HKEY,
    subkey: LPCSTR,
) -> DWORD {
    if subkey.is_null() {
        return 87; // ERROR_INVALID_PARAMETER
    }
    
    // Simplified - always succeed
    0 // ERROR_SUCCESS
}

/// RegDeleteValueA - Delete registry value
#[no_mangle]
pub extern "C" fn RegDeleteValueA(
    hkey: HKEY,
    value_name: LPCSTR,
) -> DWORD {
    // Simplified - always succeed
    0 // ERROR_SUCCESS
}

/// RegEnumKeyExA - Enumerate registry subkeys
#[no_mangle]
pub extern "C" fn RegEnumKeyExA(
    hkey: HKEY,
    index: DWORD,
    name: LPSTR,
    cch_name: *mut DWORD,
    reserved: *mut DWORD,
    class: LPSTR,
    cch_class: *mut DWORD,
    last_write_time: *mut u64,
) -> DWORD {
    if name.is_null() || cch_name.is_null() {
        return 87; // ERROR_INVALID_PARAMETER
    }
    
    // For now, return no more items
    259 // ERROR_NO_MORE_ITEMS
}

/// RegEnumValueA - Enumerate registry values
#[no_mangle]
pub extern "C" fn RegEnumValueA(
    hkey: HKEY,
    index: DWORD,
    value_name: LPSTR,
    cch_value_name: *mut DWORD,
    reserved: *mut DWORD,
    type_ptr: *mut DWORD,
    data: *mut u8,
    cb_data: *mut DWORD,
) -> DWORD {
    if value_name.is_null() || cch_value_name.is_null() {
        return 87; // ERROR_INVALID_PARAMETER
    }
    
    // For now, return no more items
    259 // ERROR_NO_MORE_ITEMS
}