use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

#[derive(Debug, Clone)]
pub enum RegistryValue {
    String(String),
    DWord(u32),
    Binary(Vec<u8>),
}

#[derive(Debug)]
pub struct RegistryKey {
    values: BTreeMap<String, RegistryValue>,
    subkeys: BTreeMap<String, RegistryKey>,
}

impl RegistryKey {
    pub fn new() -> Self {
        Self {
            values: BTreeMap::new(),
            subkeys: BTreeMap::new(),
        }
    }

    pub fn set_value(&mut self, name: String, value: RegistryValue) {
        self.values.insert(name, value);
    }

    pub fn get_value(&self, name: &str) -> Option<&RegistryValue> {
        self.values.get(name)
    }

    pub fn create_subkey(&mut self, name: String) -> &mut RegistryKey {
        self.subkeys.entry(name).or_insert_with(RegistryKey::new)
    }

    pub fn get_subkey(&self, name: &str) -> Option<&RegistryKey> {
        self.subkeys.get(name)
    }

    pub fn get_subkey_mut(&mut self, name: &str) -> Option<&mut RegistryKey> {
        self.subkeys.get_mut(name)
    }
}

pub struct Registry {
    hkey_local_machine: RegistryKey,
    hkey_current_user: RegistryKey,
    hkey_classes_root: RegistryKey,
}

impl Registry {
    pub fn new() -> Self {
        let mut registry = Self {
            hkey_local_machine: RegistryKey::new(),
            hkey_current_user: RegistryKey::new(),
            hkey_classes_root: RegistryKey::new(),
        };
        
        registry.initialize_default_keys();
        registry
    }

    fn initialize_default_keys(&mut self) {
        // Initialize HKEY_LOCAL_MACHINE
        let system_key = self.hkey_local_machine.create_subkey("SYSTEM".to_string());
        let current_control_set = system_key.create_subkey("CurrentControlSet".to_string());
        let control = current_control_set.create_subkey("Control".to_string());
        
        control.set_value(
            "SystemBootDevice".to_string(),
            RegistryValue::String("\\Device\\HarddiskVolume1".to_string())
        );

        // Initialize software key
        let software_key = self.hkey_local_machine.create_subkey("SOFTWARE".to_string());
        let microsoft = software_key.create_subkey("Microsoft".to_string());
        let windows_nt = microsoft.create_subkey("Windows NT".to_string());
        let current_version = windows_nt.create_subkey("CurrentVersion".to_string());
        
        current_version.set_value(
            "ProductName".to_string(),
            RegistryValue::String("Rust ReactOS".to_string())
        );
        current_version.set_value(
            "CurrentVersion".to_string(),
            RegistryValue::String("6.1".to_string())
        );
        current_version.set_value(
            "CurrentBuildNumber".to_string(),
            RegistryValue::String("7601".to_string())
        );

        // Initialize file associations in HKEY_CLASSES_ROOT
        let exe_key = self.hkey_classes_root.create_subkey(".exe".to_string());
        exe_key.set_value(
            "".to_string(),
            RegistryValue::String("exefile".to_string())
        );

        let exefile_key = self.hkey_classes_root.create_subkey("exefile".to_string());
        let shell = exefile_key.create_subkey("shell".to_string());
        let open = shell.create_subkey("open".to_string());
        let command = open.create_subkey("command".to_string());
        command.set_value(
            "".to_string(),
            RegistryValue::String("\"%1\" %*".to_string())
        );
    }

    pub fn get_key_by_path(&self, path: &str) -> Option<&RegistryKey> {
        let parts: Vec<&str> = path.split('\\').collect();
        if parts.is_empty() {
            return None;
        }

        let root_key = match parts[0] {
            "HKEY_LOCAL_MACHINE" | "HKLM" => &self.hkey_local_machine,
            "HKEY_CURRENT_USER" | "HKCU" => &self.hkey_current_user,
            "HKEY_CLASSES_ROOT" | "HKCR" => &self.hkey_classes_root,
            _ => return None,
        };

        let mut current_key = root_key;
        for part in &parts[1..] {
            if let Some(subkey) = current_key.get_subkey(part) {
                current_key = subkey;
            } else {
                return None;
            }
        }

        Some(current_key)
    }

    pub fn get_value(&self, key_path: &str, value_name: &str) -> Option<&RegistryValue> {
        self.get_key_by_path(key_path)?.get_value(value_name)
    }
}

lazy_static! {
    pub static ref REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());
}

// Windows Registry API functions
pub fn reg_query_value_ex(
    key_path: &str,
    value_name: &str,
) -> Result<RegistryValue, &'static str> {
    let registry = REGISTRY.lock();
    if let Some(value) = registry.get_value(key_path, value_name) {
        Ok(value.clone())
    } else {
        Err("Value not found")
    }
}