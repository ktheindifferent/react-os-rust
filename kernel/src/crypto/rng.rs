#![no_std]

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use super::CryptoProvider;
use super::hash::{HashFunction, SHA256};

pub trait SecureRandom: Send + Sync {
    fn generate(&self, length: usize) -> Vec<u8>;
    fn generate_range(&self, min: u64, max: u64) -> u64;
    fn reseed(&self, entropy: &[u8]);
}

pub trait RandomSource: Send + Sync {
    fn get_entropy(&self, length: usize) -> Vec<u8>;
}

pub struct ChaCha20Rng {
    state: [AtomicU64; 8],
    counter: AtomicU64,
}

impl ChaCha20Rng {
    pub fn new(seed: &[u8]) -> Self {
        let mut state = [0u64; 8];
        
        if seed.len() >= 32 {
            for i in 0..4 {
                state[i] = u64::from_le_bytes([
                    seed[8*i], seed[8*i+1], seed[8*i+2], seed[8*i+3],
                    seed[8*i+4], seed[8*i+5], seed[8*i+6], seed[8*i+7],
                ]);
            }
        } else {
            let hasher = SHA256::new();
            let hash = hasher.hash(seed);
            for i in 0..4 {
                state[i] = u64::from_le_bytes([
                    hash[8*i], hash[8*i+1], hash[8*i+2], hash[8*i+3],
                    hash[8*i+4], hash[8*i+5], hash[8*i+6], hash[8*i+7],
                ]);
            }
        }
        
        state[4] = 0x61707865_3320646e;
        state[5] = 0x79622d32_6b206574;
        
        let mut atomic_state = [
            AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
            AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
        ];
        
        for i in 0..8 {
            atomic_state[i].store(state[i], Ordering::SeqCst);
        }
        
        Self {
            state: atomic_state,
            counter: AtomicU64::new(0),
        }
    }
    
    fn generate_block(&self) -> [u8; 64] {
        let counter = self.counter.fetch_add(1, Ordering::SeqCst);
        
        let mut working_state = [0u32; 16];
        
        working_state[0] = 0x61707865;
        working_state[1] = 0x3320646e;
        working_state[2] = 0x79622d32;
        working_state[3] = 0x6b206574;
        
        for i in 0..4 {
            let val = self.state[i].load(Ordering::SeqCst);
            working_state[4 + i*2] = (val & 0xffffffff) as u32;
            working_state[4 + i*2 + 1] = (val >> 32) as u32;
        }
        
        working_state[12] = (counter & 0xffffffff) as u32;
        working_state[13] = (counter >> 32) as u32;
        
        let initial_state = working_state;
        
        for _ in 0..10 {
            working_state[0] = working_state[0].wrapping_add(working_state[4]);
            working_state[12] = (working_state[12] ^ working_state[0]).rotate_left(16);
            working_state[8] = working_state[8].wrapping_add(working_state[12]);
            working_state[4] = (working_state[4] ^ working_state[8]).rotate_left(12);
            working_state[0] = working_state[0].wrapping_add(working_state[4]);
            working_state[12] = (working_state[12] ^ working_state[0]).rotate_left(8);
            working_state[8] = working_state[8].wrapping_add(working_state[12]);
            working_state[4] = (working_state[4] ^ working_state[8]).rotate_left(7);
            
            working_state[1] = working_state[1].wrapping_add(working_state[5]);
            working_state[13] = (working_state[13] ^ working_state[1]).rotate_left(16);
            working_state[9] = working_state[9].wrapping_add(working_state[13]);
            working_state[5] = (working_state[5] ^ working_state[9]).rotate_left(12);
            working_state[1] = working_state[1].wrapping_add(working_state[5]);
            working_state[13] = (working_state[13] ^ working_state[1]).rotate_left(8);
            working_state[9] = working_state[9].wrapping_add(working_state[13]);
            working_state[5] = (working_state[5] ^ working_state[9]).rotate_left(7);
            
            working_state[2] = working_state[2].wrapping_add(working_state[6]);
            working_state[14] = (working_state[14] ^ working_state[2]).rotate_left(16);
            working_state[10] = working_state[10].wrapping_add(working_state[14]);
            working_state[6] = (working_state[6] ^ working_state[10]).rotate_left(12);
            working_state[2] = working_state[2].wrapping_add(working_state[6]);
            working_state[14] = (working_state[14] ^ working_state[2]).rotate_left(8);
            working_state[10] = working_state[10].wrapping_add(working_state[14]);
            working_state[6] = (working_state[6] ^ working_state[10]).rotate_left(7);
            
            working_state[3] = working_state[3].wrapping_add(working_state[7]);
            working_state[15] = (working_state[15] ^ working_state[3]).rotate_left(16);
            working_state[11] = working_state[11].wrapping_add(working_state[15]);
            working_state[7] = (working_state[7] ^ working_state[11]).rotate_left(12);
            working_state[3] = working_state[3].wrapping_add(working_state[7]);
            working_state[15] = (working_state[15] ^ working_state[3]).rotate_left(8);
            working_state[11] = working_state[11].wrapping_add(working_state[15]);
            working_state[7] = (working_state[7] ^ working_state[11]).rotate_left(7);
            
            working_state[0] = working_state[0].wrapping_add(working_state[5]);
            working_state[15] = (working_state[15] ^ working_state[0]).rotate_left(16);
            working_state[10] = working_state[10].wrapping_add(working_state[15]);
            working_state[5] = (working_state[5] ^ working_state[10]).rotate_left(12);
            working_state[0] = working_state[0].wrapping_add(working_state[5]);
            working_state[15] = (working_state[15] ^ working_state[0]).rotate_left(8);
            working_state[10] = working_state[10].wrapping_add(working_state[15]);
            working_state[5] = (working_state[5] ^ working_state[10]).rotate_left(7);
            
            working_state[1] = working_state[1].wrapping_add(working_state[6]);
            working_state[12] = (working_state[12] ^ working_state[1]).rotate_left(16);
            working_state[11] = working_state[11].wrapping_add(working_state[12]);
            working_state[6] = (working_state[6] ^ working_state[11]).rotate_left(12);
            working_state[1] = working_state[1].wrapping_add(working_state[6]);
            working_state[12] = (working_state[12] ^ working_state[1]).rotate_left(8);
            working_state[11] = working_state[11].wrapping_add(working_state[12]);
            working_state[6] = (working_state[6] ^ working_state[11]).rotate_left(7);
            
            working_state[2] = working_state[2].wrapping_add(working_state[7]);
            working_state[13] = (working_state[13] ^ working_state[2]).rotate_left(16);
            working_state[8] = working_state[8].wrapping_add(working_state[13]);
            working_state[7] = (working_state[7] ^ working_state[8]).rotate_left(12);
            working_state[2] = working_state[2].wrapping_add(working_state[7]);
            working_state[13] = (working_state[13] ^ working_state[2]).rotate_left(8);
            working_state[8] = working_state[8].wrapping_add(working_state[13]);
            working_state[7] = (working_state[7] ^ working_state[8]).rotate_left(7);
            
            working_state[3] = working_state[3].wrapping_add(working_state[4]);
            working_state[14] = (working_state[14] ^ working_state[3]).rotate_left(16);
            working_state[9] = working_state[9].wrapping_add(working_state[14]);
            working_state[4] = (working_state[4] ^ working_state[9]).rotate_left(12);
            working_state[3] = working_state[3].wrapping_add(working_state[4]);
            working_state[14] = (working_state[14] ^ working_state[3]).rotate_left(8);
            working_state[9] = working_state[9].wrapping_add(working_state[14]);
            working_state[4] = (working_state[4] ^ working_state[9]).rotate_left(7);
        }
        
        for i in 0..16 {
            working_state[i] = working_state[i].wrapping_add(initial_state[i]);
        }
        
        let mut output = [0u8; 64];
        for i in 0..16 {
            let bytes = working_state[i].to_le_bytes();
            output[i*4..(i+1)*4].copy_from_slice(&bytes);
        }
        
        output
    }
}

impl SecureRandom for ChaCha20Rng {
    fn generate(&self, length: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(length);
        
        while result.len() < length {
            let block = self.generate_block();
            let remaining = length - result.len();
            let to_copy = core::cmp::min(remaining, 64);
            result.extend_from_slice(&block[..to_copy]);
        }
        
        result
    }
    
    fn generate_range(&self, min: u64, max: u64) -> u64 {
        if min >= max {
            return min;
        }
        
        let range = max - min;
        let mut mask = range;
        mask |= mask >> 1;
        mask |= mask >> 2;
        mask |= mask >> 4;
        mask |= mask >> 8;
        mask |= mask >> 16;
        mask |= mask >> 32;
        
        loop {
            let bytes = self.generate(8);
            let mut value = u64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3],
                bytes[4], bytes[5], bytes[6], bytes[7],
            ]);
            
            value &= mask;
            
            if value < range {
                return min + value;
            }
        }
    }
    
    fn reseed(&self, entropy: &[u8]) {
        let hasher = SHA256::new();
        let mut seed_material = Vec::new();
        
        for i in 0..4 {
            let current = self.state[i].load(Ordering::SeqCst);
            seed_material.extend_from_slice(&current.to_le_bytes());
        }
        seed_material.extend_from_slice(entropy);
        
        let hash = hasher.hash(&seed_material);
        
        for i in 0..4 {
            let val = u64::from_le_bytes([
                hash[8*i], hash[8*i+1], hash[8*i+2], hash[8*i+3],
                hash[8*i+4], hash[8*i+5], hash[8*i+6], hash[8*i+7],
            ]);
            self.state[i].store(val, Ordering::SeqCst);
        }
        
        self.counter.store(0, Ordering::SeqCst);
    }
}

pub struct HardwareRng;

impl HardwareRng {
    pub fn new() -> Self {
        Self
    }
    
    #[cfg(target_arch = "x86_64")]
    fn rdrand(&self) -> Option<u64> {
        let mut value: u64;
        let success: u8;
        
        unsafe {
            core::arch::asm!(
                "rdrand {}",
                "setc {}",
                out(reg) value,
                out(reg_byte) success,
                options(nomem, nostack)
            );
        }
        
        if success != 0 {
            Some(value)
        } else {
            None
        }
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    fn rdrand(&self) -> Option<u64> {
        None
    }
}

impl RandomSource for HardwareRng {
    fn get_entropy(&self, length: usize) -> Vec<u8> {
        let mut entropy = Vec::with_capacity(length);
        
        while entropy.len() < length {
            if let Some(value) = self.rdrand() {
                let bytes = value.to_le_bytes();
                let remaining = length - entropy.len();
                let to_copy = core::cmp::min(remaining, 8);
                entropy.extend_from_slice(&bytes[..to_copy]);
            } else {
                let timestamp = unsafe {
                    core::arch::x86_64::_rdtsc()
                };
                entropy.push((timestamp & 0xff) as u8);
            }
        }
        
        entropy
    }
}

static mut GLOBAL_RNG: Option<ChaCha20Rng> = None;

pub fn init_random_subsystem() {
    let hw_rng = HardwareRng::new();
    let initial_entropy = hw_rng.get_entropy(32);
    
    unsafe {
        GLOBAL_RNG = Some(ChaCha20Rng::new(&initial_entropy));
    }
}

pub fn get_secure_random(provider: CryptoProvider) -> Box<dyn SecureRandom> {
    match provider {
        CryptoProvider::Hardware => {
            let hw_rng = HardwareRng::new();
            let entropy = hw_rng.get_entropy(32);
            Box::new(ChaCha20Rng::new(&entropy))
        }
        _ => {
            unsafe {
                if GLOBAL_RNG.is_none() {
                    init_random_subsystem();
                }
                Box::new(ChaCha20Rng::new(&[0u8; 32]))
            }
        }
    }
}