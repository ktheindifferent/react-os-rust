#![no_std]

pub mod cipher;
pub mod hash;
pub mod mac;
pub mod aead;
pub mod asymmetric;
pub mod kdf;
pub mod rng;
pub mod hw_accel;
pub mod errors;

use alloc::vec::Vec;
use alloc::string::String;
use core::fmt;

pub use cipher::{SymmetricCipher, CipherAlgorithm, CipherMode};
pub use hash::{HashAlgorithm, HashFunction};
pub use mac::{MacAlgorithm, Mac};
pub use aead::{AeadAlgorithm, Aead};
pub use asymmetric::{PublicKey, PrivateKey, KeyPair, AsymmetricAlgorithm};
pub use kdf::{KdfAlgorithm, KeyDerivation};
pub use rng::{SecureRandom, RandomSource};
pub use errors::CryptoError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoProvider {
    Software,
    Hardware,
    Hybrid,
}

pub struct CryptoEngine {
    provider: CryptoProvider,
    hw_available: bool,
    algorithms: Vec<String>,
}

impl CryptoEngine {
    pub fn new() -> Self {
        let hw_available = hw_accel::detect_hardware_crypto();
        
        Self {
            provider: if hw_available {
                CryptoProvider::Hybrid
            } else {
                CryptoProvider::Software
            },
            hw_available,
            algorithms: Self::enumerate_algorithms(),
        }
    }
    
    fn enumerate_algorithms() -> Vec<String> {
        let mut algos = Vec::new();
        
        algos.push(String::from("AES-128-CBC"));
        algos.push(String::from("AES-256-CBC"));
        algos.push(String::from("AES-128-GCM"));
        algos.push(String::from("AES-256-GCM"));
        algos.push(String::from("ChaCha20"));
        algos.push(String::from("ChaCha20-Poly1305"));
        algos.push(String::from("SHA-256"));
        algos.push(String::from("SHA-512"));
        algos.push(String::from("SHA3-256"));
        algos.push(String::from("SHA3-512"));
        algos.push(String::from("BLAKE2b"));
        algos.push(String::from("BLAKE2s"));
        algos.push(String::from("RSA-2048"));
        algos.push(String::from("RSA-4096"));
        algos.push(String::from("ECDSA-P256"));
        algos.push(String::from("Ed25519"));
        
        algos
    }
    
    pub fn get_cipher(&self, algorithm: CipherAlgorithm) -> Result<Box<dyn SymmetricCipher>, CryptoError> {
        cipher::get_cipher(algorithm, self.provider)
    }
    
    pub fn get_hash(&self, algorithm: HashAlgorithm) -> Result<Box<dyn HashFunction>, CryptoError> {
        hash::get_hash(algorithm, self.provider)
    }
    
    pub fn get_mac(&self, algorithm: MacAlgorithm) -> Result<Box<dyn Mac>, CryptoError> {
        mac::get_mac(algorithm, self.provider)
    }
    
    pub fn get_aead(&self, algorithm: AeadAlgorithm) -> Result<Box<dyn Aead>, CryptoError> {
        aead::get_aead(algorithm, self.provider)
    }
    
    pub fn get_asymmetric(&self, algorithm: AsymmetricAlgorithm) -> Result<Box<dyn asymmetric::AsymmetricCrypto>, CryptoError> {
        asymmetric::get_asymmetric(algorithm, self.provider)
    }
    
    pub fn get_kdf(&self, algorithm: KdfAlgorithm) -> Result<Box<dyn KeyDerivation>, CryptoError> {
        kdf::get_kdf(algorithm, self.provider)
    }
    
    pub fn get_random(&self) -> Box<dyn SecureRandom> {
        rng::get_secure_random(self.provider)
    }
}

pub fn init() {
    log::info!("Initializing kernel crypto subsystem");
    
    if hw_accel::detect_hardware_crypto() {
        log::info!("Hardware crypto acceleration available");
        hw_accel::init_hardware_crypto();
    }
    
    rng::init_random_subsystem();
    
    log::info!("Crypto subsystem initialized");
}

#[cfg(test)]
mod tests;