#![no_std]

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use spin::RwLock;
use crate::crypto::{CryptoEngine, CipherAlgorithm, AeadAlgorithm, KdfAlgorithm};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionMode {
    AES256XTS,
    AES256GCM,
    ChaCha20Poly1305,
    AES128CBC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDerivationMode {
    PBKDF2,
    Argon2id,
    Scrypt,
}

#[derive(Debug, Clone)]
pub struct EncryptionPolicy {
    pub mode: EncryptionMode,
    pub key_size: usize,
    pub iv_size: usize,
    pub kdf_mode: KeyDerivationMode,
    pub kdf_iterations: u32,
    pub authenticated: bool,
}

impl Default for EncryptionPolicy {
    fn default() -> Self {
        Self {
            mode: EncryptionMode::AES256XTS,
            key_size: 64,
            iv_size: 16,
            kdf_mode: KeyDerivationMode::Argon2id,
            kdf_iterations: 3,
            authenticated: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileEncryptionKey {
    pub inode: u64,
    pub master_key: Vec<u8>,
    pub file_key: Vec<u8>,
    pub tweak_key: Vec<u8>,
    pub policy: EncryptionPolicy,
}

pub struct FilesystemCrypto {
    policies: RwLock<BTreeMap<String, EncryptionPolicy>>,
    keys: RwLock<BTreeMap<u64, FileEncryptionKey>>,
    master_keys: RwLock<BTreeMap<String, Vec<u8>>>,
    crypto_engine: CryptoEngine,
}

impl FilesystemCrypto {
    pub fn new() -> Self {
        Self {
            policies: RwLock::new(BTreeMap::new()),
            keys: RwLock::new(BTreeMap::new()),
            master_keys: RwLock::new(BTreeMap::new()),
            crypto_engine: CryptoEngine::new(),
        }
    }
    
    pub fn set_policy(&self, path: &str, policy: EncryptionPolicy) -> Result<(), CryptoError> {
        let mut policies = self.policies.write();
        policies.insert(String::from(path), policy);
        Ok(())
    }
    
    pub fn add_master_key(&self, identifier: &str, key: Vec<u8>) -> Result<(), CryptoError> {
        if key.len() < 32 {
            return Err(CryptoError::InvalidKeySize);
        }
        
        let mut master_keys = self.master_keys.write();
        master_keys.insert(String::from(identifier), key);
        Ok(())
    }
    
    pub fn derive_file_key(
        &self,
        inode: u64,
        master_key: &[u8],
        policy: &EncryptionPolicy,
    ) -> Result<FileEncryptionKey, CryptoError> {
        let kdf = match policy.kdf_mode {
            KeyDerivationMode::PBKDF2 => self.crypto_engine.get_kdf(KdfAlgorithm::PBKDF2SHA256)?,
            KeyDerivationMode::Argon2id => self.crypto_engine.get_kdf(KdfAlgorithm::Argon2id)?,
            KeyDerivationMode::Scrypt => self.crypto_engine.get_kdf(KdfAlgorithm::Scrypt)?,
        };
        
        let salt = self.generate_salt(inode);
        let derived = kdf.derive(master_key, &salt, policy.kdf_iterations, policy.key_size)?;
        
        let (file_key, tweak_key) = if policy.mode == EncryptionMode::AES256XTS {
            let mid = derived.len() / 2;
            (derived[..mid].to_vec(), derived[mid..].to_vec())
        } else {
            (derived.clone(), Vec::new())
        };
        
        Ok(FileEncryptionKey {
            inode,
            master_key: master_key.to_vec(),
            file_key,
            tweak_key,
            policy: policy.clone(),
        })
    }
    
    pub fn encrypt_block(
        &self,
        data: &[u8],
        block_num: u64,
        key: &FileEncryptionKey,
    ) -> Result<Vec<u8>, CryptoError> {
        match key.policy.mode {
            EncryptionMode::AES256XTS => {
                self.encrypt_xts(data, block_num, &key.file_key, &key.tweak_key)
            }
            EncryptionMode::AES256GCM => {
                let nonce = self.generate_nonce(key.inode, block_num);
                let aead = self.crypto_engine.get_aead(AeadAlgorithm::AesGcm256)?;
                aead.encrypt(&key.file_key, &nonce, data, &[])
            }
            EncryptionMode::ChaCha20Poly1305 => {
                let nonce = self.generate_nonce(key.inode, block_num);
                let aead = self.crypto_engine.get_aead(AeadAlgorithm::ChaCha20Poly1305)?;
                aead.encrypt(&key.file_key, &nonce, data, &[])
            }
            EncryptionMode::AES128CBC => {
                let iv = self.generate_iv(key.inode, block_num);
                let cipher = self.crypto_engine.get_cipher(CipherAlgorithm::Aes128)?;
                cipher.encrypt(data, &key.file_key[..16], Some(&iv))
            }
        }
    }
    
    pub fn decrypt_block(
        &self,
        ciphertext: &[u8],
        block_num: u64,
        key: &FileEncryptionKey,
    ) -> Result<Vec<u8>, CryptoError> {
        match key.policy.mode {
            EncryptionMode::AES256XTS => {
                self.decrypt_xts(ciphertext, block_num, &key.file_key, &key.tweak_key)
            }
            EncryptionMode::AES256GCM => {
                let nonce = self.generate_nonce(key.inode, block_num);
                let aead = self.crypto_engine.get_aead(AeadAlgorithm::AesGcm256)?;
                aead.decrypt(&key.file_key, &nonce, ciphertext, &[])
            }
            EncryptionMode::ChaCha20Poly1305 => {
                let nonce = self.generate_nonce(key.inode, block_num);
                let aead = self.crypto_engine.get_aead(AeadAlgorithm::ChaCha20Poly1305)?;
                aead.decrypt(&key.file_key, &nonce, ciphertext, &[])
            }
            EncryptionMode::AES128CBC => {
                let iv = self.generate_iv(key.inode, block_num);
                let cipher = self.crypto_engine.get_cipher(CipherAlgorithm::Aes128)?;
                cipher.decrypt(ciphertext, &key.file_key[..16], Some(&iv))
            }
        }
    }
    
    fn encrypt_xts(
        &self,
        data: &[u8],
        sector_num: u64,
        key1: &[u8],
        key2: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if key1.len() != 32 || key2.len() != 32 {
            return Err(CryptoError::InvalidKeySize);
        }
        
        let cipher = self.crypto_engine.get_cipher(CipherAlgorithm::Aes256)?;
        let mut ciphertext = Vec::with_capacity(data.len());
        
        let mut tweak = [0u8; 16];
        tweak[..8].copy_from_slice(&sector_num.to_le_bytes());
        
        let encrypted_tweak = cipher.encrypt(&tweak, key2, None)?;
        let mut current_tweak = [0u8; 16];
        current_tweak.copy_from_slice(&encrypted_tweak[..16]);
        
        for chunk in data.chunks(16) {
            let mut block = [0u8; 16];
            block[..chunk.len()].copy_from_slice(chunk);
            
            for i in 0..16 {
                block[i] ^= current_tweak[i];
            }
            
            let encrypted = cipher.encrypt(&block, key1, None)?;
            
            for i in 0..16 {
                ciphertext.push(encrypted[i] ^ current_tweak[i]);
            }
            
            self.gf_multiply_128(&mut current_tweak);
        }
        
        Ok(ciphertext)
    }
    
    fn decrypt_xts(
        &self,
        ciphertext: &[u8],
        sector_num: u64,
        key1: &[u8],
        key2: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if key1.len() != 32 || key2.len() != 32 {
            return Err(CryptoError::InvalidKeySize);
        }
        
        let cipher = self.crypto_engine.get_cipher(CipherAlgorithm::Aes256)?;
        let mut plaintext = Vec::with_capacity(ciphertext.len());
        
        let mut tweak = [0u8; 16];
        tweak[..8].copy_from_slice(&sector_num.to_le_bytes());
        
        let encrypted_tweak = cipher.encrypt(&tweak, key2, None)?;
        let mut current_tweak = [0u8; 16];
        current_tweak.copy_from_slice(&encrypted_tweak[..16]);
        
        for chunk in ciphertext.chunks(16) {
            let mut block = [0u8; 16];
            block[..chunk.len()].copy_from_slice(chunk);
            
            for i in 0..16 {
                block[i] ^= current_tweak[i];
            }
            
            let decrypted = cipher.decrypt(&block, key1, None)?;
            
            for i in 0..16 {
                plaintext.push(decrypted[i] ^ current_tweak[i]);
            }
            
            self.gf_multiply_128(&mut current_tweak);
        }
        
        Ok(plaintext)
    }
    
    fn gf_multiply_128(&self, block: &mut [u8; 16]) {
        let mut carry = false;
        
        for i in (0..16).rev() {
            let new_carry = (block[i] & 0x80) != 0;
            block[i] = (block[i] << 1) | if carry { 1 } else { 0 };
            carry = new_carry;
        }
        
        if carry {
            block[0] ^= 0x87;
        }
    }
    
    fn generate_salt(&self, inode: u64) -> Vec<u8> {
        let mut salt = Vec::with_capacity(16);
        salt.extend_from_slice(b"fscrypt");
        salt.push(0x00);
        salt.extend_from_slice(&inode.to_le_bytes());
        salt
    }
    
    fn generate_nonce(&self, inode: u64, block_num: u64) -> Vec<u8> {
        let mut nonce = vec![0u8; 12];
        nonce[..8].copy_from_slice(&inode.to_le_bytes());
        nonce[8..12].copy_from_slice(&(block_num as u32).to_le_bytes());
        nonce
    }
    
    fn generate_iv(&self, inode: u64, block_num: u64) -> Vec<u8> {
        let mut iv = vec![0u8; 16];
        iv[..8].copy_from_slice(&inode.to_le_bytes());
        iv[8..16].copy_from_slice(&block_num.to_le_bytes());
        iv
    }
}

pub struct EncryptedVolume {
    pub uuid: String,
    pub name: String,
    pub cipher: EncryptionMode,
    pub key_size: usize,
    pub header: VolumeHeader,
    pub key_slots: [Option<KeySlot>; 8],
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct VolumeHeader {
    pub magic: [u8; 8],
    pub version: u32,
    pub cipher_name: String,
    pub cipher_mode: String,
    pub hash_spec: String,
    pub payload_offset: u64,
    pub key_bytes: u32,
    pub mk_digest: [u8; 32],
    pub mk_salt: [u8; 32],
    pub mk_iterations: u32,
}

#[derive(Debug, Clone)]
pub struct KeySlot {
    pub active: bool,
    pub iterations: u32,
    pub salt: [u8; 32],
    pub key_material_offset: u64,
    pub stripes: u32,
}

impl EncryptedVolume {
    pub fn create(
        name: &str,
        cipher: EncryptionMode,
        passphrase: &str,
    ) -> Result<Self, CryptoError> {
        let uuid = Self::generate_uuid();
        
        let key_size = match cipher {
            EncryptionMode::AES256XTS => 64,
            EncryptionMode::AES256GCM => 32,
            EncryptionMode::ChaCha20Poly1305 => 32,
            EncryptionMode::AES128CBC => 16,
        };
        
        let header = VolumeHeader {
            magic: *b"LUKS\xba\xbe\x00\x02",
            version: 2,
            cipher_name: String::from("aes"),
            cipher_mode: String::from("xts-plain64"),
            hash_spec: String::from("sha256"),
            payload_offset: 4096,
            key_bytes: key_size as u32,
            mk_digest: [0u8; 32],
            mk_salt: Self::generate_salt(),
            mk_iterations: 100000,
        };
        
        let mut volume = Self {
            uuid,
            name: String::from(name),
            cipher,
            key_size,
            header,
            key_slots: [None, None, None, None, None, None, None, None],
            active: false,
        };
        
        volume.add_key(0, passphrase)?;
        
        Ok(volume)
    }
    
    pub fn open(&mut self, passphrase: &str) -> Result<Vec<u8>, CryptoError> {
        for (i, slot) in self.key_slots.iter().enumerate() {
            if let Some(key_slot) = slot {
                if let Ok(master_key) = self.try_key_slot(i, passphrase, key_slot) {
                    self.active = true;
                    return Ok(master_key);
                }
            }
        }
        
        Err(CryptoError::AuthenticationFailed)
    }
    
    pub fn add_key(&mut self, slot: usize, passphrase: &str) -> Result<(), CryptoError> {
        if slot >= 8 {
            return Err(CryptoError::InvalidParameter);
        }
        
        let key_slot = KeySlot {
            active: true,
            iterations: 100000,
            salt: Self::generate_salt(),
            key_material_offset: 4096 + (slot as u64 * 256 * 1024),
            stripes: 4000,
        };
        
        self.key_slots[slot] = Some(key_slot);
        Ok(())
    }
    
    fn try_key_slot(
        &self,
        _slot_num: usize,
        passphrase: &str,
        key_slot: &KeySlot,
    ) -> Result<Vec<u8>, CryptoError> {
        let engine = CryptoEngine::new();
        let kdf = engine.get_kdf(KdfAlgorithm::PBKDF2SHA256)?;
        
        let derived = kdf.derive(
            passphrase.as_bytes(),
            &key_slot.salt,
            key_slot.iterations,
            self.key_size,
        )?;
        
        Ok(derived)
    }
    
    fn generate_uuid() -> String {
        String::from("12345678-1234-1234-1234-123456789abc")
    }
    
    fn generate_salt() -> [u8; 32] {
        let mut salt = [0u8; 32];
        for i in 0..32 {
            salt[i] = ((i * 7 + 13) % 256) as u8;
        }
        salt
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    InvalidKeySize,
    InvalidBlockSize,
    InvalidNonce,
    AuthenticationFailed,
    DecryptionFailed,
    UnsupportedAlgorithm,
    InvalidParameter,
}

pub struct DmCrypt {
    volumes: RwLock<BTreeMap<String, EncryptedVolume>>,
    crypto: FilesystemCrypto,
}

impl DmCrypt {
    pub fn new() -> Self {
        Self {
            volumes: RwLock::new(BTreeMap::new()),
            crypto: FilesystemCrypto::new(),
        }
    }
    
    pub fn create_volume(
        &self,
        name: &str,
        size: u64,
        cipher: EncryptionMode,
        passphrase: &str,
    ) -> Result<(), CryptoError> {
        let volume = EncryptedVolume::create(name, cipher, passphrase)?;
        
        let mut volumes = self.volumes.write();
        volumes.insert(String::from(name), volume);
        
        Ok(())
    }
    
    pub fn open_volume(&self, name: &str, passphrase: &str) -> Result<(), CryptoError> {
        let mut volumes = self.volumes.write();
        
        if let Some(volume) = volumes.get_mut(name) {
            let _master_key = volume.open(passphrase)?;
            Ok(())
        } else {
            Err(CryptoError::InvalidParameter)
        }
    }
    
    pub fn close_volume(&self, name: &str) -> Result<(), CryptoError> {
        let mut volumes = self.volumes.write();
        
        if let Some(volume) = volumes.get_mut(name) {
            volume.active = false;
            Ok(())
        } else {
            Err(CryptoError::InvalidParameter)
        }
    }
}