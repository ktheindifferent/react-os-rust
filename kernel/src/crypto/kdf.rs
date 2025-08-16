#![no_std]

use alloc::vec::Vec;
use super::errors::{CryptoError, CryptoResult};
use super::hash::{HashFunction, SHA256, SHA512};
use super::mac::{Hmac, Mac};
use super::CryptoProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KdfAlgorithm {
    PBKDF2SHA256,
    PBKDF2SHA512,
    Argon2id,
    Scrypt,
    HKDF,
}

pub trait KeyDerivation: Send + Sync {
    fn derive(&self, password: &[u8], salt: &[u8], iterations: u32, key_len: usize) -> CryptoResult<Vec<u8>>;
}

pub struct PBKDF2<H: HashFunction> {
    hasher: H,
}

impl<H: HashFunction> PBKDF2<H> {
    pub fn new(hasher: H) -> Self {
        Self { hasher }
    }
}

impl<H: HashFunction> KeyDerivation for PBKDF2<H> {
    fn derive(&self, password: &[u8], salt: &[u8], iterations: u32, key_len: usize) -> CryptoResult<Vec<u8>> {
        if iterations == 0 {
            return Err(CryptoError::InvalidParameter);
        }
        
        let hmac = Hmac::new(self.hasher.clone());
        let hash_len = hmac.tag_size();
        let blocks_needed = (key_len + hash_len - 1) / hash_len;
        
        let mut derived_key = Vec::with_capacity(key_len);
        
        for i in 1..=blocks_needed {
            let mut salt_with_counter = salt.to_vec();
            salt_with_counter.extend_from_slice(&(i as u32).to_be_bytes());
            
            let mut u = hmac.compute(password, &salt_with_counter)?;
            let mut f = u.clone();
            
            for _ in 1..iterations {
                u = hmac.compute(password, &u)?;
                for (f_byte, u_byte) in f.iter_mut().zip(u.iter()) {
                    *f_byte ^= u_byte;
                }
            }
            
            derived_key.extend_from_slice(&f);
        }
        
        derived_key.truncate(key_len);
        Ok(derived_key)
    }
}

pub struct Argon2id {
    memory_blocks: usize,
    iterations: usize,
    parallelism: usize,
}

impl Argon2id {
    pub fn new(memory_kb: usize, iterations: usize, parallelism: usize) -> Self {
        Self {
            memory_blocks: memory_kb,
            iterations,
            parallelism,
        }
    }
    
    fn blake2b_long(&self, input: &[u8], output_len: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(output_len);
        
        if output_len <= 64 {
            let hasher = super::hash::BLAKE2b::new(output_len);
            result.extend_from_slice(&hasher.hash(input));
        } else {
            let hasher = super::hash::BLAKE2b::new(64);
            let v0 = hasher.hash(input);
            result.extend_from_slice(&v0[..32]);
            
            let mut remaining = output_len - 32;
            let mut counter = 1u32;
            
            while remaining > 64 {
                let mut input_i = v0.clone();
                input_i.extend_from_slice(&counter.to_le_bytes());
                let vi = hasher.hash(&input_i);
                result.extend_from_slice(&vi);
                remaining -= 64;
                counter += 1;
            }
            
            if remaining > 0 {
                let hasher = super::hash::BLAKE2b::new(remaining);
                let mut input_i = v0.clone();
                input_i.extend_from_slice(&counter.to_le_bytes());
                let vi = hasher.hash(&input_i);
                result.extend_from_slice(&vi);
            }
        }
        
        result
    }
    
    fn g(&self, a: u64, b: u64, c: u64, d: u64) -> (u64, u64, u64, u64) {
        let a = a.wrapping_add(b).wrapping_add(2u64.wrapping_mul(a.wrapping_mul(b)));
        let d = (d ^ a).rotate_right(32);
        let c = c.wrapping_add(d).wrapping_add(2u64.wrapping_mul(c.wrapping_mul(d)));
        let b = (b ^ c).rotate_right(24);
        let a = a.wrapping_add(b).wrapping_add(2u64.wrapping_mul(a.wrapping_mul(b)));
        let d = (d ^ a).rotate_right(16);
        let c = c.wrapping_add(d).wrapping_add(2u64.wrapping_mul(c.wrapping_mul(d)));
        let b = (b ^ c).rotate_right(63);
        
        (a, b, c, d)
    }
}

impl KeyDerivation for Argon2id {
    fn derive(&self, password: &[u8], salt: &[u8], _iterations: u32, key_len: usize) -> CryptoResult<Vec<u8>> {
        if salt.len() < 8 {
            return Err(CryptoError::InvalidParameter);
        }
        
        let mut h0_input = Vec::new();
        h0_input.extend_from_slice(&(self.parallelism as u32).to_le_bytes());
        h0_input.extend_from_slice(&(key_len as u32).to_le_bytes());
        h0_input.extend_from_slice(&(self.memory_blocks as u32).to_le_bytes());
        h0_input.extend_from_slice(&(self.iterations as u32).to_le_bytes());
        h0_input.extend_from_slice(&0x13u32.to_le_bytes());
        h0_input.extend_from_slice(&0u32.to_le_bytes());
        h0_input.extend_from_slice(&(password.len() as u32).to_le_bytes());
        h0_input.extend_from_slice(password);
        h0_input.extend_from_slice(&(salt.len() as u32).to_le_bytes());
        h0_input.extend_from_slice(salt);
        h0_input.extend_from_slice(&0u32.to_le_bytes());
        h0_input.extend_from_slice(&0u32.to_le_bytes());
        
        let h0 = self.blake2b_long(&h0_input, 64);
        
        let blocks_per_lane = self.memory_blocks / self.parallelism;
        let mut memory = vec![vec![0u64; 128]; self.memory_blocks];
        
        for lane in 0..self.parallelism {
            let mut h0_prime = h0.clone();
            h0_prime.extend_from_slice(&0u32.to_le_bytes());
            h0_prime.extend_from_slice(&(lane as u32).to_le_bytes());
            
            let block_data = self.blake2b_long(&h0_prime, 1024);
            for (i, chunk) in block_data.chunks(8).enumerate() {
                if chunk.len() == 8 {
                    memory[lane * blocks_per_lane][i] = u64::from_le_bytes(chunk.try_into().unwrap());
                }
            }
            
            let mut h0_prime = h0.clone();
            h0_prime.extend_from_slice(&1u32.to_le_bytes());
            h0_prime.extend_from_slice(&(lane as u32).to_le_bytes());
            
            let block_data = self.blake2b_long(&h0_prime, 1024);
            for (i, chunk) in block_data.chunks(8).enumerate() {
                if chunk.len() == 8 {
                    memory[lane * blocks_per_lane + 1][i] = u64::from_le_bytes(chunk.try_into().unwrap());
                }
            }
        }
        
        for pass in 0..self.iterations {
            for lane in 0..self.parallelism {
                for segment in 0..4 {
                    let start_idx = lane * blocks_per_lane + segment * (blocks_per_lane / 4);
                    let end_idx = start_idx + blocks_per_lane / 4;
                    
                    for idx in start_idx..end_idx {
                        let prev_idx = if idx == 0 {
                            self.memory_blocks - 1
                        } else {
                            idx - 1
                        };
                        
                        let pseudo_rand = memory[prev_idx][0];
                        let ref_lane = if pass == 0 && segment == 0 {
                            lane
                        } else {
                            (pseudo_rand as usize >> 32) % self.parallelism
                        };
                        
                        let ref_index = (pseudo_rand as usize) % blocks_per_lane;
                        let ref_block = ref_lane * blocks_per_lane + ref_index;
                        
                        for i in (0..128).step_by(16) {
                            let mut v = [0u64; 16];
                            for j in 0..16 {
                                v[j] = memory[prev_idx][i + j] ^ memory[ref_block][i + j];
                            }
                            
                            let (v[0], v[4], v[8], v[12]) = self.g(v[0], v[4], v[8], v[12]);
                            let (v[1], v[5], v[9], v[13]) = self.g(v[1], v[5], v[9], v[13]);
                            let (v[2], v[6], v[10], v[14]) = self.g(v[2], v[6], v[10], v[14]);
                            let (v[3], v[7], v[11], v[15]) = self.g(v[3], v[7], v[11], v[15]);
                            
                            let (v[0], v[5], v[10], v[15]) = self.g(v[0], v[5], v[10], v[15]);
                            let (v[1], v[6], v[11], v[12]) = self.g(v[1], v[6], v[11], v[12]);
                            let (v[2], v[7], v[8], v[13]) = self.g(v[2], v[7], v[8], v[13]);
                            let (v[3], v[4], v[9], v[14]) = self.g(v[3], v[4], v[9], v[14]);
                            
                            for j in 0..16 {
                                memory[idx][i + j] = memory[idx][i + j] ^ v[j];
                            }
                        }
                    }
                }
            }
        }
        
        let mut final_block = memory[self.memory_blocks - 1].clone();
        for lane in 0..self.parallelism {
            let last_block_idx = (lane + 1) * blocks_per_lane - 1;
            for i in 0..128 {
                final_block[i] ^= memory[last_block_idx][i];
            }
        }
        
        let mut final_bytes = Vec::with_capacity(1024);
        for val in final_block.iter() {
            final_bytes.extend_from_slice(&val.to_le_bytes());
        }
        
        Ok(self.blake2b_long(&final_bytes, key_len))
    }
}

pub struct HKDF<H: HashFunction> {
    hasher: H,
}

impl<H: HashFunction> HKDF<H> {
    pub fn new(hasher: H) -> Self {
        Self { hasher }
    }
    
    fn extract(&self, salt: &[u8], ikm: &[u8]) -> Vec<u8> {
        let hmac = Hmac::new(self.hasher.clone());
        hmac.compute(salt, ikm).unwrap()
    }
    
    fn expand(&self, prk: &[u8], info: &[u8], output_len: usize) -> CryptoResult<Vec<u8>> {
        let hmac = Hmac::new(self.hasher.clone());
        let hash_len = hmac.tag_size();
        
        if output_len > 255 * hash_len {
            return Err(CryptoError::InvalidParameter);
        }
        
        let n = (output_len + hash_len - 1) / hash_len;
        let mut okm = Vec::with_capacity(output_len);
        let mut t_prev = Vec::new();
        
        for i in 1..=n {
            let mut input = t_prev.clone();
            input.extend_from_slice(info);
            input.push(i as u8);
            
            t_prev = hmac.compute(prk, &input)?;
            okm.extend_from_slice(&t_prev);
        }
        
        okm.truncate(output_len);
        Ok(okm)
    }
}

impl<H: HashFunction> KeyDerivation for HKDF<H> {
    fn derive(&self, password: &[u8], salt: &[u8], _iterations: u32, key_len: usize) -> CryptoResult<Vec<u8>> {
        let prk = self.extract(salt, password);
        self.expand(&prk, b"", key_len)
    }
}

pub fn get_kdf(algorithm: KdfAlgorithm, _provider: CryptoProvider) -> CryptoResult<Box<dyn KeyDerivation>> {
    match algorithm {
        KdfAlgorithm::PBKDF2SHA256 => Ok(Box::new(PBKDF2::new(SHA256::new()))),
        KdfAlgorithm::PBKDF2SHA512 => Ok(Box::new(PBKDF2::new(SHA512::new()))),
        KdfAlgorithm::Argon2id => Ok(Box::new(Argon2id::new(4096, 3, 1))),
        KdfAlgorithm::HKDF => Ok(Box::new(HKDF::new(SHA256::new()))),
        _ => Err(CryptoError::UnsupportedAlgorithm),
    }
}