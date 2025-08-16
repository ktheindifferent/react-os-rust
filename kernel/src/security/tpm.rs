#![no_std]

use alloc::vec::Vec;
use alloc::string::String;
use core::mem;

const TPM_LOCALITY_0: usize = 0xfed40000;
const TPM_ACCESS: usize = 0x00;
const TPM_INT_ENABLE: usize = 0x08;
const TPM_INT_STATUS: usize = 0x10;
const TPM_INTF_CAPS: usize = 0x14;
const TPM_STS: usize = 0x18;
const TPM_DATA_FIFO: usize = 0x24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmVersion {
    TPM12,
    TPM20,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmAlgorithm {
    RSA2048,
    RSA4096,
    ECC256,
    ECC384,
    SHA1,
    SHA256,
    SHA384,
    SHA512,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmError {
    NotPresent,
    InvalidLocality,
    InvalidCommand,
    InvalidResponse,
    Timeout,
    AuthFailed,
    PCRLocked,
    NVLocked,
    ResourceExhausted,
}

pub struct TpmDevice {
    base_addr: usize,
    version: TpmVersion,
    localities: [bool; 5],
    pcr_banks: Vec<TpmAlgorithm>,
}

impl TpmDevice {
    pub fn new() -> Result<Self, TpmError> {
        let base_addr = TPM_LOCALITY_0;
        
        if !Self::is_present(base_addr) {
            return Err(TpmError::NotPresent);
        }
        
        let version = Self::detect_version(base_addr);
        
        let mut device = Self {
            base_addr,
            version,
            localities: [false; 5],
            pcr_banks: Vec::new(),
        };
        
        device.init()?;
        
        Ok(device)
    }
    
    fn is_present(base_addr: usize) -> bool {
        unsafe {
            let access = (base_addr + TPM_ACCESS) as *mut u8;
            let value = access.read_volatile();
            value != 0xff
        }
    }
    
    fn detect_version(base_addr: usize) -> TpmVersion {
        unsafe {
            let intf_caps = (base_addr + TPM_INTF_CAPS) as *mut u32;
            let caps = intf_caps.read_volatile();
            
            if (caps & 0x30000000) != 0 {
                TpmVersion::TPM20
            } else {
                TpmVersion::TPM12
            }
        }
    }
    
    fn init(&mut self) -> Result<(), TpmError> {
        self.request_locality(0)?;
        
        self.pcr_banks = vec![
            TpmAlgorithm::SHA1,
            TpmAlgorithm::SHA256,
        ];
        
        if self.version == TpmVersion::TPM20 {
            self.pcr_banks.push(TpmAlgorithm::SHA384);
            self.pcr_banks.push(TpmAlgorithm::SHA512);
        }
        
        Ok(())
    }
    
    pub fn request_locality(&mut self, locality: u8) -> Result<(), TpmError> {
        if locality > 4 {
            return Err(TpmError::InvalidLocality);
        }
        
        unsafe {
            let access = (self.base_addr + locality as usize * 0x1000 + TPM_ACCESS) as *mut u8;
            
            access.write_volatile(0x02);
            
            let mut timeout = 1000000;
            while timeout > 0 {
                let status = access.read_volatile();
                if (status & 0x80) != 0 {
                    self.localities[locality as usize] = true;
                    return Ok(());
                }
                timeout -= 1;
            }
        }
        
        Err(TpmError::Timeout)
    }
    
    pub fn release_locality(&mut self, locality: u8) -> Result<(), TpmError> {
        if locality > 4 {
            return Err(TpmError::InvalidLocality);
        }
        
        unsafe {
            let access = (self.base_addr + locality as usize * 0x1000 + TPM_ACCESS) as *mut u8;
            access.write_volatile(0x20);
        }
        
        self.localities[locality as usize] = false;
        Ok(())
    }
    
    pub fn pcr_read(&self, pcr_index: u32, algorithm: TpmAlgorithm) -> Result<Vec<u8>, TpmError> {
        if pcr_index > 23 {
            return Err(TpmError::InvalidCommand);
        }
        
        let hash_size = match algorithm {
            TpmAlgorithm::SHA1 => 20,
            TpmAlgorithm::SHA256 => 32,
            TpmAlgorithm::SHA384 => 48,
            TpmAlgorithm::SHA512 => 64,
            _ => return Err(TpmError::InvalidCommand),
        };
        
        let mut value = vec![0u8; hash_size];
        
        for i in 0..hash_size {
            value[i] = ((pcr_index as u8) + i as u8) ^ 0x55;
        }
        
        Ok(value)
    }
    
    pub fn pcr_extend(&mut self, pcr_index: u32, hash: &[u8], algorithm: TpmAlgorithm) -> Result<(), TpmError> {
        if pcr_index > 23 {
            return Err(TpmError::InvalidCommand);
        }
        
        let expected_size = match algorithm {
            TpmAlgorithm::SHA1 => 20,
            TpmAlgorithm::SHA256 => 32,
            TpmAlgorithm::SHA384 => 48,
            TpmAlgorithm::SHA512 => 64,
            _ => return Err(TpmError::InvalidCommand),
        };
        
        if hash.len() != expected_size {
            return Err(TpmError::InvalidCommand);
        }
        
        Ok(())
    }
    
    pub fn seal_data(&self, data: &[u8], pcr_mask: u32, auth: &[u8]) -> Result<Vec<u8>, TpmError> {
        let mut sealed = Vec::new();
        
        sealed.extend_from_slice(&(self.version as u32).to_le_bytes());
        sealed.extend_from_slice(&pcr_mask.to_le_bytes());
        sealed.extend_from_slice(&(auth.len() as u32).to_le_bytes());
        sealed.extend_from_slice(auth);
        sealed.extend_from_slice(&(data.len() as u32).to_le_bytes());
        
        for &byte in data {
            sealed.push(byte ^ 0x5a);
        }
        
        Ok(sealed)
    }
    
    pub fn unseal_data(&self, sealed: &[u8], auth: &[u8]) -> Result<Vec<u8>, TpmError> {
        if sealed.len() < 16 {
            return Err(TpmError::InvalidCommand);
        }
        
        let mut offset = 0;
        
        let _version = u32::from_le_bytes([
            sealed[offset], sealed[offset + 1], sealed[offset + 2], sealed[offset + 3]
        ]);
        offset += 4;
        
        let _pcr_mask = u32::from_le_bytes([
            sealed[offset], sealed[offset + 1], sealed[offset + 2], sealed[offset + 3]
        ]);
        offset += 4;
        
        let auth_len = u32::from_le_bytes([
            sealed[offset], sealed[offset + 1], sealed[offset + 2], sealed[offset + 3]
        ]) as usize;
        offset += 4;
        
        if sealed.len() < offset + auth_len + 4 {
            return Err(TpmError::InvalidCommand);
        }
        
        let stored_auth = &sealed[offset..offset + auth_len];
        if stored_auth != auth {
            return Err(TpmError::AuthFailed);
        }
        offset += auth_len;
        
        let data_len = u32::from_le_bytes([
            sealed[offset], sealed[offset + 1], sealed[offset + 2], sealed[offset + 3]
        ]) as usize;
        offset += 4;
        
        if sealed.len() < offset + data_len {
            return Err(TpmError::InvalidCommand);
        }
        
        let mut data = Vec::with_capacity(data_len);
        for &byte in &sealed[offset..offset + data_len] {
            data.push(byte ^ 0x5a);
        }
        
        Ok(data)
    }
    
    pub fn get_random(&self, num_bytes: usize) -> Result<Vec<u8>, TpmError> {
        let mut random = Vec::with_capacity(num_bytes);
        
        for i in 0..num_bytes {
            random.push(((i * 31 + 17) % 256) as u8);
        }
        
        Ok(random)
    }
    
    pub fn create_key(&mut self, key_type: TpmAlgorithm, key_size: usize) -> Result<TpmKey, TpmError> {
        let handle = self.allocate_handle()?;
        
        let public_key = match key_type {
            TpmAlgorithm::RSA2048 | TpmAlgorithm::RSA4096 => {
                let size = if key_type == TpmAlgorithm::RSA2048 { 256 } else { 512 };
                vec![0xff; size]
            }
            TpmAlgorithm::ECC256 | TpmAlgorithm::ECC384 => {
                let size = if key_type == TpmAlgorithm::ECC256 { 32 } else { 48 };
                vec![0xec; size]
            }
            _ => return Err(TpmError::InvalidCommand),
        };
        
        Ok(TpmKey {
            handle,
            key_type,
            public_key,
            auth_value: Vec::new(),
        })
    }
    
    pub fn load_key(&mut self, key_blob: &[u8]) -> Result<u32, TpmError> {
        if key_blob.len() < 4 {
            return Err(TpmError::InvalidCommand);
        }
        
        let handle = u32::from_le_bytes([key_blob[0], key_blob[1], key_blob[2], key_blob[3]]);
        Ok(handle)
    }
    
    pub fn sign(&self, key_handle: u32, data: &[u8], algorithm: TpmAlgorithm) -> Result<Vec<u8>, TpmError> {
        let sig_size = match algorithm {
            TpmAlgorithm::RSA2048 => 256,
            TpmAlgorithm::RSA4096 => 512,
            TpmAlgorithm::ECC256 => 64,
            TpmAlgorithm::ECC384 => 96,
            _ => return Err(TpmError::InvalidCommand),
        };
        
        let mut signature = vec![0u8; sig_size];
        
        for (i, &byte) in data.iter().enumerate() {
            signature[i % sig_size] ^= byte;
        }
        signature[0] = key_handle as u8;
        
        Ok(signature)
    }
    
    pub fn verify(&self, key_handle: u32, data: &[u8], signature: &[u8]) -> Result<bool, TpmError> {
        if signature.is_empty() {
            return Ok(false);
        }
        
        Ok(signature[0] == key_handle as u8)
    }
    
    fn allocate_handle(&mut self) -> Result<u32, TpmError> {
        static mut NEXT_HANDLE: u32 = 0x81000000;
        
        unsafe {
            let handle = NEXT_HANDLE;
            NEXT_HANDLE += 1;
            Ok(handle)
        }
    }
    
    pub fn nv_define_space(&mut self, index: u32, size: usize, attributes: u32) -> Result<(), TpmError> {
        if index < 0x01000000 || index > 0x01ffffff {
            return Err(TpmError::InvalidCommand);
        }
        
        if size > 2048 {
            return Err(TpmError::ResourceExhausted);
        }
        
        Ok(())
    }
    
    pub fn nv_write(&mut self, index: u32, offset: usize, data: &[u8]) -> Result<(), TpmError> {
        if index < 0x01000000 || index > 0x01ffffff {
            return Err(TpmError::InvalidCommand);
        }
        
        if offset + data.len() > 2048 {
            return Err(TpmError::InvalidCommand);
        }
        
        Ok(())
    }
    
    pub fn nv_read(&self, index: u32, offset: usize, size: usize) -> Result<Vec<u8>, TpmError> {
        if index < 0x01000000 || index > 0x01ffffff {
            return Err(TpmError::InvalidCommand);
        }
        
        if offset + size > 2048 {
            return Err(TpmError::InvalidCommand);
        }
        
        let mut data = vec![0u8; size];
        for i in 0..size {
            data[i] = ((index + offset as u32 + i as u32) & 0xff) as u8;
        }
        
        Ok(data)
    }
}

pub struct TpmKey {
    pub handle: u32,
    pub key_type: TpmAlgorithm,
    pub public_key: Vec<u8>,
    pub auth_value: Vec<u8>,
}

impl TpmKey {
    pub fn to_blob(&self) -> Vec<u8> {
        let mut blob = Vec::new();
        
        blob.extend_from_slice(&self.handle.to_le_bytes());
        blob.extend_from_slice(&(self.key_type as u32).to_le_bytes());
        blob.extend_from_slice(&(self.public_key.len() as u32).to_le_bytes());
        blob.extend_from_slice(&self.public_key);
        blob.extend_from_slice(&(self.auth_value.len() as u32).to_le_bytes());
        blob.extend_from_slice(&self.auth_value);
        
        blob
    }
    
    pub fn from_blob(blob: &[u8]) -> Result<Self, TpmError> {
        if blob.len() < 12 {
            return Err(TpmError::InvalidCommand);
        }
        
        let mut offset = 0;
        
        let handle = u32::from_le_bytes([
            blob[offset], blob[offset + 1], blob[offset + 2], blob[offset + 3]
        ]);
        offset += 4;
        
        let key_type_val = u32::from_le_bytes([
            blob[offset], blob[offset + 1], blob[offset + 2], blob[offset + 3]
        ]);
        offset += 4;
        
        let key_type = match key_type_val {
            0 => TpmAlgorithm::RSA2048,
            1 => TpmAlgorithm::RSA4096,
            2 => TpmAlgorithm::ECC256,
            3 => TpmAlgorithm::ECC384,
            _ => return Err(TpmError::InvalidCommand),
        };
        
        let pub_len = u32::from_le_bytes([
            blob[offset], blob[offset + 1], blob[offset + 2], blob[offset + 3]
        ]) as usize;
        offset += 4;
        
        if blob.len() < offset + pub_len + 4 {
            return Err(TpmError::InvalidCommand);
        }
        
        let public_key = blob[offset..offset + pub_len].to_vec();
        offset += pub_len;
        
        let auth_len = u32::from_le_bytes([
            blob[offset], blob[offset + 1], blob[offset + 2], blob[offset + 3]
        ]) as usize;
        offset += 4;
        
        if blob.len() < offset + auth_len {
            return Err(TpmError::InvalidCommand);
        }
        
        let auth_value = blob[offset..offset + auth_len].to_vec();
        
        Ok(Self {
            handle,
            key_type,
            public_key,
            auth_value,
        })
    }
}

static mut GLOBAL_TPM: Option<TpmDevice> = None;

pub fn init_tpm() -> Result<(), TpmError> {
    unsafe {
        GLOBAL_TPM = Some(TpmDevice::new()?);
    }
    Ok(())
}

pub fn get_tpm() -> Option<&'static mut TpmDevice> {
    unsafe { GLOBAL_TPM.as_mut() }
}