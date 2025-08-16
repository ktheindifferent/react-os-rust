#![no_std]

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    User,
    Session,
    Process,
    Thread,
    Trusted,
    Encrypted,
    Asymmetric,
    Certificate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPermission {
    View,
    Read,
    Write,
    Link,
    SetAttr,
    All,
}

#[derive(Debug, Clone)]
pub struct KeyMetadata {
    pub key_type: KeyType,
    pub description: String,
    pub uid: u32,
    pub gid: u32,
    pub permissions: u32,
    pub expiry: Option<u64>,
    pub quota_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct Key {
    pub id: u64,
    pub metadata: KeyMetadata,
    pub payload: Vec<u8>,
    pub linked_keys: Vec<u64>,
    pub revoked: bool,
}

pub struct Keyring {
    keys: RwLock<BTreeMap<u64, Key>>,
    next_key_id: AtomicU64,
    session_keyring: RwLock<Vec<u64>>,
    process_keyring: RwLock<Vec<u64>>,
    thread_keyring: RwLock<Vec<u64>>,
    user_keyring: RwLock<BTreeMap<u32, Vec<u64>>>,
}

impl Keyring {
    pub fn new() -> Self {
        Self {
            keys: RwLock::new(BTreeMap::new()),
            next_key_id: AtomicU64::new(1),
            session_keyring: RwLock::new(Vec::new()),
            process_keyring: RwLock::new(Vec::new()),
            thread_keyring: RwLock::new(Vec::new()),
            user_keyring: RwLock::new(BTreeMap::new()),
        }
    }
    
    pub fn add_key(
        &self,
        key_type: KeyType,
        description: String,
        payload: Vec<u8>,
        uid: u32,
        gid: u32,
        permissions: u32,
    ) -> Result<u64, KeyError> {
        let key_id = self.next_key_id.fetch_add(1, Ordering::SeqCst);
        
        let metadata = KeyMetadata {
            key_type,
            description,
            uid,
            gid,
            permissions,
            expiry: None,
            quota_bytes: payload.len(),
        };
        
        let key = Key {
            id: key_id,
            metadata,
            payload,
            linked_keys: Vec::new(),
            revoked: false,
        };
        
        let mut keys = self.keys.write();
        keys.insert(key_id, key);
        
        match key_type {
            KeyType::Session => {
                let mut session = self.session_keyring.write();
                session.push(key_id);
            }
            KeyType::Process => {
                let mut process = self.process_keyring.write();
                process.push(key_id);
            }
            KeyType::Thread => {
                let mut thread = self.thread_keyring.write();
                thread.push(key_id);
            }
            KeyType::User => {
                let mut user = self.user_keyring.write();
                user.entry(uid).or_insert_with(Vec::new).push(key_id);
            }
            _ => {}
        }
        
        Ok(key_id)
    }
    
    pub fn get_key(&self, key_id: u64, uid: u32) -> Result<Vec<u8>, KeyError> {
        let keys = self.keys.read();
        
        match keys.get(&key_id) {
            Some(key) => {
                if key.revoked {
                    return Err(KeyError::KeyRevoked);
                }
                
                if !self.check_permission(&key, uid, KeyPermission::Read) {
                    return Err(KeyError::PermissionDenied);
                }
                
                if let Some(expiry) = key.metadata.expiry {
                    if self.get_current_time() > expiry {
                        return Err(KeyError::KeyExpired);
                    }
                }
                
                Ok(key.payload.clone())
            }
            None => Err(KeyError::KeyNotFound),
        }
    }
    
    pub fn update_key(&self, key_id: u64, payload: Vec<u8>, uid: u32) -> Result<(), KeyError> {
        let mut keys = self.keys.write();
        
        match keys.get_mut(&key_id) {
            Some(key) => {
                if key.revoked {
                    return Err(KeyError::KeyRevoked);
                }
                
                if !self.check_permission(&key, uid, KeyPermission::Write) {
                    return Err(KeyError::PermissionDenied);
                }
                
                key.payload = payload;
                key.metadata.quota_bytes = key.payload.len();
                
                Ok(())
            }
            None => Err(KeyError::KeyNotFound),
        }
    }
    
    pub fn revoke_key(&self, key_id: u64, uid: u32) -> Result<(), KeyError> {
        let mut keys = self.keys.write();
        
        match keys.get_mut(&key_id) {
            Some(key) => {
                if key.metadata.uid != uid && uid != 0 {
                    return Err(KeyError::PermissionDenied);
                }
                
                key.revoked = true;
                Ok(())
            }
            None => Err(KeyError::KeyNotFound),
        }
    }
    
    pub fn link_key(&self, keyring_id: u64, key_id: u64, uid: u32) -> Result<(), KeyError> {
        let mut keys = self.keys.write();
        
        if !keys.contains_key(&key_id) {
            return Err(KeyError::KeyNotFound);
        }
        
        match keys.get_mut(&keyring_id) {
            Some(keyring) => {
                if !self.check_permission(&keyring, uid, KeyPermission::Link) {
                    return Err(KeyError::PermissionDenied);
                }
                
                if !keyring.linked_keys.contains(&key_id) {
                    keyring.linked_keys.push(key_id);
                }
                
                Ok(())
            }
            None => Err(KeyError::KeyNotFound),
        }
    }
    
    pub fn unlink_key(&self, keyring_id: u64, key_id: u64, uid: u32) -> Result<(), KeyError> {
        let mut keys = self.keys.write();
        
        match keys.get_mut(&keyring_id) {
            Some(keyring) => {
                if !self.check_permission(&keyring, uid, KeyPermission::Link) {
                    return Err(KeyError::PermissionDenied);
                }
                
                keyring.linked_keys.retain(|&id| id != key_id);
                Ok(())
            }
            None => Err(KeyError::KeyNotFound),
        }
    }
    
    pub fn search_key(&self, key_type: KeyType, description: &str, uid: u32) -> Result<u64, KeyError> {
        let keys = self.keys.read();
        
        for (id, key) in keys.iter() {
            if key.metadata.key_type == key_type && key.metadata.description == description {
                if !key.revoked && self.check_permission(&key, uid, KeyPermission::View) {
                    return Ok(*id);
                }
            }
        }
        
        Err(KeyError::KeyNotFound)
    }
    
    pub fn set_key_timeout(&self, key_id: u64, timeout_secs: u64, uid: u32) -> Result<(), KeyError> {
        let mut keys = self.keys.write();
        
        match keys.get_mut(&key_id) {
            Some(key) => {
                if !self.check_permission(&key, uid, KeyPermission::SetAttr) {
                    return Err(KeyError::PermissionDenied);
                }
                
                key.metadata.expiry = Some(self.get_current_time() + timeout_secs);
                Ok(())
            }
            None => Err(KeyError::KeyNotFound),
        }
    }
    
    pub fn garbage_collect(&self) {
        let current_time = self.get_current_time();
        let mut keys = self.keys.write();
        
        keys.retain(|_, key| {
            if let Some(expiry) = key.metadata.expiry {
                expiry > current_time && !key.revoked
            } else {
                !key.revoked
            }
        });
    }
    
    fn check_permission(&self, key: &Key, uid: u32, permission: KeyPermission) -> bool {
        if uid == 0 {
            return true;
        }
        
        if uid == key.metadata.uid {
            return match permission {
                KeyPermission::View => (key.metadata.permissions & 0o400) != 0,
                KeyPermission::Read => (key.metadata.permissions & 0o200) != 0,
                KeyPermission::Write => (key.metadata.permissions & 0o100) != 0,
                KeyPermission::Link => (key.metadata.permissions & 0o010) != 0,
                KeyPermission::SetAttr => (key.metadata.permissions & 0o001) != 0,
                KeyPermission::All => key.metadata.permissions == 0o777,
            };
        }
        
        match permission {
            KeyPermission::View => (key.metadata.permissions & 0o004) != 0,
            KeyPermission::Read => (key.metadata.permissions & 0o002) != 0,
            KeyPermission::Write => (key.metadata.permissions & 0o001) != 0,
            _ => false,
        }
    }
    
    fn get_current_time(&self) -> u64 {
        0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyError {
    KeyNotFound,
    KeyExpired,
    KeyRevoked,
    PermissionDenied,
    QuotaExceeded,
    InvalidKeyType,
    InvalidOperation,
}

pub struct TrustedKey {
    pub sealed_data: Vec<u8>,
    pub pcr_mask: u32,
    pub pcr_values: Vec<[u8; 20]>,
}

impl TrustedKey {
    pub fn seal(data: &[u8], pcr_mask: u32) -> Result<Self, KeyError> {
        let mut sealed = Vec::new();
        sealed.push(0x01);
        sealed.extend_from_slice(&(data.len() as u32).to_le_bytes());
        sealed.extend_from_slice(data);
        
        let mut pcr_values = Vec::new();
        for i in 0..24 {
            if (pcr_mask & (1 << i)) != 0 {
                pcr_values.push([i as u8; 20]);
            }
        }
        
        Ok(Self {
            sealed_data: sealed,
            pcr_mask,
            pcr_values,
        })
    }
    
    pub fn unseal(&self) -> Result<Vec<u8>, KeyError> {
        if self.sealed_data.len() < 5 {
            return Err(KeyError::InvalidOperation);
        }
        
        if self.sealed_data[0] != 0x01 {
            return Err(KeyError::InvalidOperation);
        }
        
        let len = u32::from_le_bytes([
            self.sealed_data[1],
            self.sealed_data[2],
            self.sealed_data[3],
            self.sealed_data[4],
        ]) as usize;
        
        if self.sealed_data.len() < 5 + len {
            return Err(KeyError::InvalidOperation);
        }
        
        Ok(self.sealed_data[5..5 + len].to_vec())
    }
}

pub struct EncryptedKey {
    pub encrypted_data: Vec<u8>,
    pub master_key_id: u64,
    pub algorithm: String,
}

impl EncryptedKey {
    pub fn encrypt(data: &[u8], master_key_id: u64) -> Result<Self, KeyError> {
        let mut encrypted = Vec::new();
        encrypted.push(0x02);
        encrypted.extend_from_slice(&(data.len() as u32).to_le_bytes());
        
        for &byte in data {
            encrypted.push(byte ^ 0xaa);
        }
        
        Ok(Self {
            encrypted_data: encrypted,
            master_key_id,
            algorithm: String::from("AES-256-GCM"),
        })
    }
    
    pub fn decrypt(&self, _master_key: &[u8]) -> Result<Vec<u8>, KeyError> {
        if self.encrypted_data.len() < 5 {
            return Err(KeyError::InvalidOperation);
        }
        
        if self.encrypted_data[0] != 0x02 {
            return Err(KeyError::InvalidOperation);
        }
        
        let len = u32::from_le_bytes([
            self.encrypted_data[1],
            self.encrypted_data[2],
            self.encrypted_data[3],
            self.encrypted_data[4],
        ]) as usize;
        
        if self.encrypted_data.len() < 5 + len {
            return Err(KeyError::InvalidOperation);
        }
        
        let mut decrypted = Vec::with_capacity(len);
        for &byte in &self.encrypted_data[5..5 + len] {
            decrypted.push(byte ^ 0xaa);
        }
        
        Ok(decrypted)
    }
}

static GLOBAL_KEYRING: spin::Once<Keyring> = spin::Once::new();

pub fn init_keyring() {
    GLOBAL_KEYRING.call_once(|| Keyring::new());
}

pub fn get_keyring() -> &'static Keyring {
    GLOBAL_KEYRING.get().expect("Keyring not initialized")
}