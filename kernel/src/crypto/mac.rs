#![no_std]

use alloc::vec::Vec;
use super::errors::{CryptoError, CryptoResult};
use super::hash::{HashFunction, SHA256, SHA512};
use super::CryptoProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacAlgorithm {
    HmacSHA256,
    HmacSHA512,
    Poly1305,
    CMAC,
    SipHash,
}

pub trait Mac: Send + Sync {
    fn compute(&self, key: &[u8], data: &[u8]) -> CryptoResult<Vec<u8>>;
    fn verify(&self, key: &[u8], data: &[u8], tag: &[u8]) -> CryptoResult<bool>;
    fn tag_size(&self) -> usize;
}

pub struct Hmac<H: HashFunction> {
    hasher: H,
}

impl<H: HashFunction> Hmac<H> {
    pub fn new(hasher: H) -> Self {
        Self { hasher }
    }
    
    fn compute_hmac(&self, key: &[u8], data: &[u8]) -> Vec<u8> {
        let block_size = self.hasher.block_size();
        
        let mut key_block = if key.len() > block_size {
            let mut hashed = self.hasher.hash(key);
            hashed.resize(block_size, 0);
            hashed
        } else {
            let mut k = key.to_vec();
            k.resize(block_size, 0);
            k
        };
        
        let mut i_pad = Vec::with_capacity(block_size);
        let mut o_pad = Vec::with_capacity(block_size);
        
        for byte in key_block.iter() {
            i_pad.push(byte ^ 0x36);
            o_pad.push(byte ^ 0x5c);
        }
        
        i_pad.extend_from_slice(data);
        let inner_hash = self.hasher.hash(&i_pad);
        
        o_pad.extend_from_slice(&inner_hash);
        self.hasher.hash(&o_pad)
    }
}

impl<H: HashFunction> Mac for Hmac<H> {
    fn compute(&self, key: &[u8], data: &[u8]) -> CryptoResult<Vec<u8>> {
        Ok(self.compute_hmac(key, data))
    }
    
    fn verify(&self, key: &[u8], data: &[u8], tag: &[u8]) -> CryptoResult<bool> {
        let computed = self.compute_hmac(key, data);
        
        if computed.len() != tag.len() {
            return Ok(false);
        }
        
        let mut diff = 0u8;
        for (a, b) in computed.iter().zip(tag.iter()) {
            diff |= a ^ b;
        }
        
        Ok(diff == 0)
    }
    
    fn tag_size(&self) -> usize {
        self.hasher.digest_size()
    }
}

pub struct Poly1305;

impl Poly1305 {
    pub fn new() -> Self {
        Self
    }
    
    fn clamp(r: &mut [u8; 16]) {
        r[3] &= 0x0f;
        r[7] &= 0x0f;
        r[11] &= 0x0f;
        r[15] &= 0x0f;
        
        r[4] &= 0xfc;
        r[8] &= 0xfc;
        r[12] &= 0xfc;
    }
    
    fn compute_poly1305(&self, key: &[u8], data: &[u8]) -> CryptoResult<Vec<u8>> {
        if key.len() != 32 {
            return Err(CryptoError::InvalidKeySize);
        }
        
        let mut r = [0u8; 16];
        let mut s = [0u8; 16];
        r.copy_from_slice(&key[0..16]);
        s.copy_from_slice(&key[16..32]);
        
        Self::clamp(&mut r);
        
        let mut h0 = 0u64;
        let mut h1 = 0u64;
        let mut h2 = 0u64;
        
        let r0 = u64::from(u32::from_le_bytes([r[0], r[1], r[2], r[3]]));
        let r1 = u64::from(u32::from_le_bytes([r[4], r[5], r[6], r[7]]));
        let r2 = u64::from(u32::from_le_bytes([r[8], r[9], r[10], r[11]]));
        let r3 = u64::from(u32::from_le_bytes([r[12], r[13], r[14], r[15]]));
        
        let s0 = u64::from(u32::from_le_bytes([s[0], s[1], s[2], s[3]]));
        let s1 = u64::from(u32::from_le_bytes([s[4], s[5], s[6], s[7]]));
        let s2 = u64::from(u32::from_le_bytes([s[8], s[9], s[10], s[11]]));
        let s3 = u64::from(u32::from_le_bytes([s[12], s[13], s[14], s[15]]));
        
        for chunk in data.chunks(16) {
            let mut block = [0u8; 17];
            block[..chunk.len()].copy_from_slice(chunk);
            block[chunk.len()] = 1;
            
            let t0 = u64::from(u32::from_le_bytes([block[0], block[1], block[2], block[3]]));
            let t1 = u64::from(u32::from_le_bytes([block[4], block[5], block[6], block[7]]));
            let t2 = u64::from(u32::from_le_bytes([block[8], block[9], block[10], block[11]]));
            let t3 = u64::from(u32::from_le_bytes([block[12], block[13], block[14], block[15]]));
            let t4 = u64::from(block[16]);
            
            h0 = h0.wrapping_add(t0);
            h1 = h1.wrapping_add(t1);
            h2 = h2.wrapping_add(t2.wrapping_add(t3 << 32).wrapping_add(t4 << 40));
            
            let d0 = h0.wrapping_mul(r0)
                .wrapping_add(h1.wrapping_mul(5 * r3))
                .wrapping_add(h2.wrapping_mul(5 * r2));
            let d1 = h0.wrapping_mul(r1)
                .wrapping_add(h1.wrapping_mul(r0))
                .wrapping_add(h2.wrapping_mul(5 * r3));
            let d2 = h0.wrapping_mul(r2)
                .wrapping_add(h1.wrapping_mul(r1))
                .wrapping_add(h2.wrapping_mul(r0));
            
            h0 = d0 & 0xfffffffffff;
            h1 = (d0 >> 44) | ((d1 & 0xfffffffffff) << 20);
            h2 = ((d1 >> 24) | (d2 << 40)) & 0x3ffffffffff;
            
            let carry = h2 >> 42;
            h2 &= 0x3ffffffffff;
            h0 = h0.wrapping_add(carry.wrapping_mul(5));
            h1 = h1.wrapping_add(h0 >> 44);
            h0 &= 0xfffffffffff;
        }
        
        h2 = h2.wrapping_add(h1 >> 44);
        h1 &= 0xfffffffffff;
        
        let mut g0 = h0.wrapping_add(5);
        let mut g1 = h1.wrapping_add(g0 >> 44);
        g0 &= 0xfffffffffff;
        let mut g2 = h2.wrapping_add(g1 >> 44);
        g1 &= 0xfffffffffff;
        
        let mask = (g2 >> 42).wrapping_sub(1);
        g0 &= mask;
        g1 &= mask;
        g2 &= mask;
        let mask = !mask;
        h0 = (h0 & mask) | g0;
        h1 = (h1 & mask) | g1;
        h2 = (h2 & mask) | g2;
        
        h0 = h0.wrapping_add(s0);
        h1 = h1.wrapping_add(s1).wrapping_add(h0 >> 32);
        h2 = h2.wrapping_add(s2).wrapping_add(h1 >> 32);
        let h3 = s3.wrapping_add(h2 >> 32);
        
        let mut tag = Vec::with_capacity(16);
        tag.extend_from_slice(&(h0 as u32).to_le_bytes());
        tag.extend_from_slice(&(h1 as u32).to_le_bytes());
        tag.extend_from_slice(&(h2 as u32).to_le_bytes());
        tag.extend_from_slice(&(h3 as u32).to_le_bytes());
        
        Ok(tag)
    }
}

impl Mac for Poly1305 {
    fn compute(&self, key: &[u8], data: &[u8]) -> CryptoResult<Vec<u8>> {
        self.compute_poly1305(key, data)
    }
    
    fn verify(&self, key: &[u8], data: &[u8], tag: &[u8]) -> CryptoResult<bool> {
        let computed = self.compute_poly1305(key, data)?;
        
        if computed.len() != tag.len() {
            return Ok(false);
        }
        
        let mut diff = 0u8;
        for (a, b) in computed.iter().zip(tag.iter()) {
            diff |= a ^ b;
        }
        
        Ok(diff == 0)
    }
    
    fn tag_size(&self) -> usize {
        16
    }
}

pub fn get_mac(algorithm: MacAlgorithm, _provider: CryptoProvider) -> CryptoResult<Box<dyn Mac>> {
    match algorithm {
        MacAlgorithm::HmacSHA256 => Ok(Box::new(Hmac::new(SHA256::new()))),
        MacAlgorithm::HmacSHA512 => Ok(Box::new(Hmac::new(SHA512::new()))),
        MacAlgorithm::Poly1305 => Ok(Box::new(Poly1305::new())),
        _ => Err(CryptoError::UnsupportedAlgorithm),
    }
}