#![no_std]

use alloc::vec::Vec;
use super::cipher::{ChaCha20Cipher, AesCipher, CipherMode, SymmetricCipher};
use super::mac::{Poly1305, Mac};
use super::errors::{CryptoError, CryptoResult};
use super::CryptoProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AeadAlgorithm {
    AesGcm128,
    AesGcm256,
    ChaCha20Poly1305,
    AesCcm,
    XChaCha20Poly1305,
}

pub trait Aead: Send + Sync {
    fn encrypt(&self, key: &[u8], nonce: &[u8], plaintext: &[u8], aad: &[u8]) -> CryptoResult<Vec<u8>>;
    fn decrypt(&self, key: &[u8], nonce: &[8], ciphertext: &[u8], aad: &[u8]) -> CryptoResult<Vec<u8>>;
    fn key_size(&self) -> usize;
    fn nonce_size(&self) -> usize;
    fn tag_size(&self) -> usize;
}

pub struct ChaCha20Poly1305Aead {
    cipher: ChaCha20Cipher,
    mac: Poly1305,
}

impl ChaCha20Poly1305Aead {
    pub fn new() -> Self {
        Self {
            cipher: ChaCha20Cipher::new(),
            mac: Poly1305::new(),
        }
    }
    
    fn poly1305_key(&self, key: &[u8], nonce: &[u8]) -> Vec<u8> {
        let mut counter = [0u8; 4];
        let mut full_nonce = Vec::with_capacity(16);
        full_nonce.extend_from_slice(&counter);
        full_nonce.extend_from_slice(nonce);
        
        let keystream = self.cipher.encrypt(&vec![0u8; 32], key, Some(&full_nonce[4..])).unwrap();
        keystream[..32].to_vec()
    }
    
    fn pad16(data: &[u8]) -> Vec<u8> {
        let mut padded = data.to_vec();
        let remainder = data.len() % 16;
        if remainder != 0 {
            padded.extend_from_slice(&vec![0u8; 16 - remainder]);
        }
        padded
    }
}

impl Aead for ChaCha20Poly1305Aead {
    fn encrypt(&self, key: &[u8], nonce: &[u8], plaintext: &[u8], aad: &[u8]) -> CryptoResult<Vec<u8>> {
        if key.len() != 32 {
            return Err(CryptoError::InvalidKeySize);
        }
        if nonce.len() != 12 {
            return Err(CryptoError::InvalidNonce);
        }
        
        let ciphertext = self.cipher.encrypt(plaintext, key, Some(nonce))?;
        
        let poly_key = self.poly1305_key(key, nonce);
        
        let mut auth_data = Vec::new();
        auth_data.extend_from_slice(&Self::pad16(aad));
        auth_data.extend_from_slice(&Self::pad16(&ciphertext));
        auth_data.extend_from_slice(&(aad.len() as u64).to_le_bytes());
        auth_data.extend_from_slice(&(ciphertext.len() as u64).to_le_bytes());
        
        let tag = self.mac.compute(&poly_key, &auth_data)?;
        
        let mut result = ciphertext;
        result.extend_from_slice(&tag);
        
        Ok(result)
    }
    
    fn decrypt(&self, key: &[u8], nonce: &[u8], ciphertext: &[u8], aad: &[u8]) -> CryptoResult<Vec<u8>> {
        if key.len() != 32 {
            return Err(CryptoError::InvalidKeySize);
        }
        if nonce.len() != 12 {
            return Err(CryptoError::InvalidNonce);
        }
        if ciphertext.len() < 16 {
            return Err(CryptoError::InvalidTag);
        }
        
        let (cipher_data, tag) = ciphertext.split_at(ciphertext.len() - 16);
        
        let poly_key = self.poly1305_key(key, nonce);
        
        let mut auth_data = Vec::new();
        auth_data.extend_from_slice(&Self::pad16(aad));
        auth_data.extend_from_slice(&Self::pad16(cipher_data));
        auth_data.extend_from_slice(&(aad.len() as u64).to_le_bytes());
        auth_data.extend_from_slice(&(cipher_data.len() as u64).to_le_bytes());
        
        if !self.mac.verify(&poly_key, &auth_data, tag)? {
            return Err(CryptoError::AuthenticationFailed);
        }
        
        self.cipher.decrypt(cipher_data, key, Some(nonce))
    }
    
    fn key_size(&self) -> usize {
        32
    }
    
    fn nonce_size(&self) -> usize {
        12
    }
    
    fn tag_size(&self) -> usize {
        16
    }
}

pub struct AesGcm {
    key_size: usize,
}

impl AesGcm {
    pub fn new(key_size: usize) -> Self {
        Self { key_size }
    }
    
    fn ghash(&self, h: &[u8; 16], data: &[u8]) -> [u8; 16] {
        let mut y = [0u8; 16];
        
        for chunk in data.chunks(16) {
            for i in 0..chunk.len() {
                y[i] ^= chunk[i];
            }
            
            y = self.gf_mult(&y, h);
        }
        
        y
    }
    
    fn gf_mult(&self, x: &[u8; 16], y: &[u8; 16]) -> [u8; 16] {
        let mut z = [0u8; 16];
        let mut v = *y;
        
        for i in 0..128 {
            let byte_idx = i / 8;
            let bit_idx = 7 - (i % 8);
            
            if (x[byte_idx] >> bit_idx) & 1 == 1 {
                for j in 0..16 {
                    z[j] ^= v[j];
                }
            }
            
            let lsb = v[15] & 1;
            
            for j in (1..16).rev() {
                v[j] = (v[j] >> 1) | ((v[j - 1] & 1) << 7);
            }
            v[0] >>= 1;
            
            if lsb == 1 {
                v[0] ^= 0xe1;
            }
        }
        
        z
    }
    
    fn gctr(&self, key: &[u8], iv: &[u8], data: &[u8]) -> Vec<u8> {
        let cipher = AesCipher::new(self.key_size, CipherMode::ECB);
        let mut result = Vec::with_capacity(data.len());
        
        let mut counter = [0u8; 16];
        counter[..12].copy_from_slice(iv);
        counter[15] = 1;
        
        for chunk in data.chunks(16) {
            counter[15] = counter[15].wrapping_add(1);
            let keystream = cipher.encrypt(&counter, key, None).unwrap();
            
            for (i, &byte) in chunk.iter().enumerate() {
                result.push(byte ^ keystream[i]);
            }
        }
        
        result
    }
}

impl Aead for AesGcm {
    fn encrypt(&self, key: &[u8], nonce: &[u8], plaintext: &[u8], aad: &[u8]) -> CryptoResult<Vec<u8>> {
        if key.len() != self.key_size {
            return Err(CryptoError::InvalidKeySize);
        }
        if nonce.len() != 12 {
            return Err(CryptoError::InvalidNonce);
        }
        
        let cipher = AesCipher::new(self.key_size, CipherMode::ECB);
        
        let h_bytes = cipher.encrypt(&[0u8; 16], key, None)?;
        let mut h = [0u8; 16];
        h.copy_from_slice(&h_bytes);
        
        let ciphertext = self.gctr(key, nonce, plaintext);
        
        let mut ghash_input = Vec::new();
        ghash_input.extend_from_slice(aad);
        ghash_input.resize((aad.len() + 15) / 16 * 16, 0);
        ghash_input.extend_from_slice(&ciphertext);
        ghash_input.resize(ghash_input.len() + (15 - (ciphertext.len() + 15) % 16), 0);
        ghash_input.extend_from_slice(&(aad.len() as u64 * 8).to_be_bytes());
        ghash_input.extend_from_slice(&(ciphertext.len() as u64 * 8).to_be_bytes());
        
        let ghash = self.ghash(&h, &ghash_input);
        
        let mut j0 = [0u8; 16];
        j0[..12].copy_from_slice(nonce);
        j0[15] = 1;
        
        let tag_mask = cipher.encrypt(&j0, key, None)?;
        let mut tag = [0u8; 16];
        for i in 0..16 {
            tag[i] = ghash[i] ^ tag_mask[i];
        }
        
        let mut result = ciphertext;
        result.extend_from_slice(&tag);
        
        Ok(result)
    }
    
    fn decrypt(&self, key: &[u8], nonce: &[u8], ciphertext: &[u8], aad: &[u8]) -> CryptoResult<Vec<u8>> {
        if key.len() != self.key_size {
            return Err(CryptoError::InvalidKeySize);
        }
        if nonce.len() != 12 {
            return Err(CryptoError::InvalidNonce);
        }
        if ciphertext.len() < 16 {
            return Err(CryptoError::InvalidTag);
        }
        
        let (cipher_data, tag) = ciphertext.split_at(ciphertext.len() - 16);
        
        let cipher = AesCipher::new(self.key_size, CipherMode::ECB);
        
        let h_bytes = cipher.encrypt(&[0u8; 16], key, None)?;
        let mut h = [0u8; 16];
        h.copy_from_slice(&h_bytes);
        
        let mut ghash_input = Vec::new();
        ghash_input.extend_from_slice(aad);
        ghash_input.resize((aad.len() + 15) / 16 * 16, 0);
        ghash_input.extend_from_slice(cipher_data);
        ghash_input.resize(ghash_input.len() + (15 - (cipher_data.len() + 15) % 16), 0);
        ghash_input.extend_from_slice(&(aad.len() as u64 * 8).to_be_bytes());
        ghash_input.extend_from_slice(&(cipher_data.len() as u64 * 8).to_be_bytes());
        
        let ghash = self.ghash(&h, &ghash_input);
        
        let mut j0 = [0u8; 16];
        j0[..12].copy_from_slice(nonce);
        j0[15] = 1;
        
        let tag_mask = cipher.encrypt(&j0, key, None)?;
        
        for i in 0..16 {
            if (ghash[i] ^ tag_mask[i]) != tag[i] {
                return Err(CryptoError::AuthenticationFailed);
            }
        }
        
        Ok(self.gctr(key, nonce, cipher_data))
    }
    
    fn key_size(&self) -> usize {
        self.key_size
    }
    
    fn nonce_size(&self) -> usize {
        12
    }
    
    fn tag_size(&self) -> usize {
        16
    }
}

pub fn get_aead(algorithm: AeadAlgorithm, _provider: CryptoProvider) -> CryptoResult<Box<dyn Aead>> {
    match algorithm {
        AeadAlgorithm::ChaCha20Poly1305 => Ok(Box::new(ChaCha20Poly1305Aead::new())),
        AeadAlgorithm::AesGcm128 => Ok(Box::new(AesGcm::new(16))),
        AeadAlgorithm::AesGcm256 => Ok(Box::new(AesGcm::new(32))),
        _ => Err(CryptoError::UnsupportedAlgorithm),
    }
}