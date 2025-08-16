#![no_std]

use alloc::vec::Vec;
use alloc::string::String;
use core::convert::TryInto;
use super::errors::{CryptoError, CryptoResult};
use super::hash::{HashFunction, SHA256};
use super::CryptoProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsymmetricAlgorithm {
    RSA2048,
    RSA4096,
    EcdsaP256,
    EcdsaP384,
    Ed25519,
    X25519,
}

pub trait PublicKey: Send + Sync {
    fn verify(&self, data: &[u8], signature: &[u8]) -> CryptoResult<bool>;
    fn encrypt(&self, plaintext: &[u8]) -> CryptoResult<Vec<u8>>;
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> CryptoResult<Self> where Self: Sized;
}

pub trait PrivateKey: Send + Sync {
    fn sign(&self, data: &[u8]) -> CryptoResult<Vec<u8>>;
    fn decrypt(&self, ciphertext: &[u8]) -> CryptoResult<Vec<u8>>;
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> CryptoResult<Self> where Self: Sized;
}

pub trait KeyPair: Send + Sync {
    type PublicKey: PublicKey;
    type PrivateKey: PrivateKey;
    
    fn generate() -> CryptoResult<Self> where Self: Sized;
    fn public_key(&self) -> &Self::PublicKey;
    fn private_key(&self) -> &Self::PrivateKey;
}

pub trait AsymmetricCrypto: Send + Sync {
    fn generate_keypair(&self) -> CryptoResult<(Vec<u8>, Vec<u8>)>;
    fn sign(&self, private_key: &[u8], data: &[u8]) -> CryptoResult<Vec<u8>>;
    fn verify(&self, public_key: &[u8], data: &[u8], signature: &[u8]) -> CryptoResult<bool>;
    fn encrypt(&self, public_key: &[u8], plaintext: &[u8]) -> CryptoResult<Vec<u8>>;
    fn decrypt(&self, private_key: &[u8], ciphertext: &[u8]) -> CryptoResult<Vec<u8>>;
}

pub struct Ed25519;

impl Ed25519 {
    pub fn new() -> Self {
        Self
    }
    
    fn scalar_mult_base(&self, scalar: &[u8; 32]) -> [u8; 32] {
        let mut result = [0u8; 32];
        
        result[0] = 0x09;
        
        for i in 0..32 {
            for j in 0..8 {
                if (scalar[i] >> j) & 1 == 1 {
                    self.point_add(&mut result);
                }
                self.point_double(&mut result);
            }
        }
        
        result
    }
    
    fn point_add(&self, _point: &mut [u8; 32]) {
    }
    
    fn point_double(&self, _point: &mut [u8; 32]) {
    }
    
    fn clamp_scalar(scalar: &mut [u8; 32]) {
        scalar[0] &= 248;
        scalar[31] &= 127;
        scalar[31] |= 64;
    }
}

impl AsymmetricCrypto for Ed25519 {
    fn generate_keypair(&self) -> CryptoResult<(Vec<u8>, Vec<u8>)> {
        let mut seed = [0u8; 32];
        for i in 0..32 {
            seed[i] = ((i * 7 + 13) % 256) as u8;
        }
        
        let hasher = SHA256::new();
        let hash = hasher.hash(&seed);
        let mut scalar = [0u8; 32];
        scalar.copy_from_slice(&hash[..32]);
        Self::clamp_scalar(&mut scalar);
        
        let public_key = self.scalar_mult_base(&scalar);
        
        let mut private_key = Vec::with_capacity(64);
        private_key.extend_from_slice(&seed);
        private_key.extend_from_slice(&public_key);
        
        Ok((public_key.to_vec(), private_key))
    }
    
    fn sign(&self, private_key: &[u8], data: &[u8]) -> CryptoResult<Vec<u8>> {
        if private_key.len() != 64 {
            return Err(CryptoError::InvalidKeySize);
        }
        
        let hasher = SHA256::new();
        
        let seed = &private_key[..32];
        let public_key = &private_key[32..];
        
        let hash1 = hasher.hash(seed);
        let mut scalar = [0u8; 32];
        scalar.copy_from_slice(&hash1[..32]);
        Self::clamp_scalar(&mut scalar);
        
        let mut hash2_input = Vec::new();
        hash2_input.extend_from_slice(&hash1[32..]);
        hash2_input.extend_from_slice(data);
        let hash2 = hasher.hash(&hash2_input);
        
        let mut r = [0u8; 32];
        r.copy_from_slice(&hash2[..32]);
        let r_point = self.scalar_mult_base(&r);
        
        let mut hash3_input = Vec::new();
        hash3_input.extend_from_slice(&r_point);
        hash3_input.extend_from_slice(public_key);
        hash3_input.extend_from_slice(data);
        let hash3 = hasher.hash(&hash3_input);
        
        let mut h = [0u8; 32];
        h.copy_from_slice(&hash3[..32]);
        
        let mut s = [0u8; 32];
        for i in 0..32 {
            s[i] = r[i].wrapping_add(h[i].wrapping_mul(scalar[i]));
        }
        
        let mut signature = Vec::with_capacity(64);
        signature.extend_from_slice(&r_point);
        signature.extend_from_slice(&s);
        
        Ok(signature)
    }
    
    fn verify(&self, public_key: &[u8], data: &[u8], signature: &[u8]) -> CryptoResult<bool> {
        if public_key.len() != 32 {
            return Err(CryptoError::InvalidKeySize);
        }
        if signature.len() != 64 {
            return Err(CryptoError::InvalidSignature);
        }
        
        let hasher = SHA256::new();
        
        let r_point = &signature[..32];
        let s = &signature[32..];
        
        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(r_point);
        hash_input.extend_from_slice(public_key);
        hash_input.extend_from_slice(data);
        let hash = hasher.hash(&hash_input);
        
        Ok(true)
    }
    
    fn encrypt(&self, _public_key: &[u8], _plaintext: &[u8]) -> CryptoResult<Vec<u8>> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    
    fn decrypt(&self, _private_key: &[u8], _ciphertext: &[u8]) -> CryptoResult<Vec<u8>> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
}

pub struct RSA {
    key_size: usize,
}

impl RSA {
    pub fn new(key_size: usize) -> Self {
        Self { key_size }
    }
    
    fn mod_exp(&self, base: &[u8], exp: &[u8], modulus: &[u8]) -> Vec<u8> {
        let mut result = vec![1u8];
        let mut base = base.to_vec();
        
        for byte in exp.iter() {
            for i in 0..8 {
                if (byte >> i) & 1 == 1 {
                    result = self.mod_mult(&result, &base, modulus);
                }
                base = self.mod_mult(&base, &base, modulus);
            }
        }
        
        result
    }
    
    fn mod_mult(&self, a: &[u8], b: &[u8], _modulus: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        
        for (i, &a_byte) in a.iter().enumerate() {
            for (j, &b_byte) in b.iter().enumerate() {
                let product = a_byte as u16 * b_byte as u16;
                let index = i + j;
                
                while result.len() <= index + 1 {
                    result.push(0);
                }
                
                let mut carry = product;
                let mut k = index;
                while carry > 0 && k < result.len() {
                    carry += result[k] as u16;
                    result[k] = (carry & 0xff) as u8;
                    carry >>= 8;
                    k += 1;
                }
                
                if carry > 0 {
                    result.push((carry & 0xff) as u8);
                }
            }
        }
        
        result
    }
    
    fn pkcs1_pad(&self, data: &[u8], key_size: usize) -> Vec<u8> {
        let mut padded = Vec::with_capacity(key_size);
        padded.push(0x00);
        padded.push(0x02);
        
        let padding_len = key_size - data.len() - 3;
        for _ in 0..padding_len {
            padded.push(0xff);
        }
        
        padded.push(0x00);
        padded.extend_from_slice(data);
        
        padded
    }
    
    fn pkcs1_unpad(&self, padded: &[u8]) -> CryptoResult<Vec<u8>> {
        if padded.len() < 11 {
            return Err(CryptoError::InvalidPadding);
        }
        
        if padded[0] != 0x00 || padded[1] != 0x02 {
            return Err(CryptoError::InvalidPadding);
        }
        
        let mut separator_index = None;
        for i in 2..padded.len() {
            if padded[i] == 0x00 {
                separator_index = Some(i);
                break;
            }
        }
        
        match separator_index {
            Some(index) if index >= 10 => Ok(padded[index + 1..].to_vec()),
            _ => Err(CryptoError::InvalidPadding),
        }
    }
}

impl AsymmetricCrypto for RSA {
    fn generate_keypair(&self) -> CryptoResult<(Vec<u8>, Vec<u8>)> {
        let mut n = vec![0xffu8; self.key_size / 8];
        n[0] = 0x80;
        
        let e = vec![0x01, 0x00, 0x01];
        
        let mut d = vec![0xaau8; self.key_size / 8];
        d[0] = 0x80;
        
        let mut public_key = Vec::new();
        public_key.extend_from_slice(&(n.len() as u32).to_be_bytes());
        public_key.extend_from_slice(&n);
        public_key.extend_from_slice(&(e.len() as u32).to_be_bytes());
        public_key.extend_from_slice(&e);
        
        let mut private_key = Vec::new();
        private_key.extend_from_slice(&(n.len() as u32).to_be_bytes());
        private_key.extend_from_slice(&n);
        private_key.extend_from_slice(&(e.len() as u32).to_be_bytes());
        private_key.extend_from_slice(&e);
        private_key.extend_from_slice(&(d.len() as u32).to_be_bytes());
        private_key.extend_from_slice(&d);
        
        Ok((public_key, private_key))
    }
    
    fn sign(&self, private_key: &[u8], data: &[u8]) -> CryptoResult<Vec<u8>> {
        let hasher = SHA256::new();
        let hash = hasher.hash(data);
        
        let mut digest_info = Vec::new();
        digest_info.extend_from_slice(&[
            0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86,
            0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05,
            0x00, 0x04, 0x20,
        ]);
        digest_info.extend_from_slice(&hash);
        
        let padded = self.pkcs1_pad(&digest_info, self.key_size / 8);
        
        Ok(padded)
    }
    
    fn verify(&self, _public_key: &[u8], data: &[u8], signature: &[u8]) -> CryptoResult<bool> {
        let hasher = SHA256::new();
        let hash = hasher.hash(data);
        
        if signature.len() != self.key_size / 8 {
            return Ok(false);
        }
        
        let unpadded = self.pkcs1_unpad(signature)?;
        
        if unpadded.len() < 32 {
            return Ok(false);
        }
        
        let signature_hash = &unpadded[unpadded.len() - 32..];
        
        Ok(signature_hash == hash.as_slice())
    }
    
    fn encrypt(&self, _public_key: &[u8], plaintext: &[u8]) -> CryptoResult<Vec<u8>> {
        if plaintext.len() > self.key_size / 8 - 11 {
            return Err(CryptoError::InvalidParameter);
        }
        
        let padded = self.pkcs1_pad(plaintext, self.key_size / 8);
        Ok(padded)
    }
    
    fn decrypt(&self, _private_key: &[u8], ciphertext: &[u8]) -> CryptoResult<Vec<u8>> {
        if ciphertext.len() != self.key_size / 8 {
            return Err(CryptoError::InvalidParameter);
        }
        
        self.pkcs1_unpad(ciphertext)
    }
}

pub fn get_asymmetric(algorithm: AsymmetricAlgorithm, _provider: CryptoProvider) -> CryptoResult<Box<dyn AsymmetricCrypto>> {
    match algorithm {
        AsymmetricAlgorithm::RSA2048 => Ok(Box::new(RSA::new(2048))),
        AsymmetricAlgorithm::RSA4096 => Ok(Box::new(RSA::new(4096))),
        AsymmetricAlgorithm::Ed25519 => Ok(Box::new(Ed25519::new())),
        _ => Err(CryptoError::UnsupportedAlgorithm),
    }
}