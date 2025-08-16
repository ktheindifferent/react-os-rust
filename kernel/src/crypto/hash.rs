#![no_std]

use alloc::vec::Vec;
use core::convert::TryInto;
use super::errors::{CryptoError, CryptoResult};
use super::CryptoProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    SHA256,
    SHA512,
    SHA3_256,
    SHA3_512,
    BLAKE2b,
    BLAKE2s,
    MD5,
    SHA1,
}

pub trait HashFunction: Send + Sync {
    fn hash(&self, data: &[u8]) -> Vec<u8>;
    fn digest_size(&self) -> usize;
    fn block_size(&self) -> usize;
}

pub struct SHA256;

impl SHA256 {
    pub fn new() -> Self {
        Self
    }
    
    fn process_block(&self, block: &[u8], h: &mut [u32; 8]) {
        let mut w = [0u32; 64];
        
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[4 * i],
                block[4 * i + 1],
                block[4 * i + 2],
                block[4 * i + 3],
            ]);
        }
        
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_val = h[7];
        
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_val.wrapping_add(s1).wrapping_add(ch).wrapping_add(SHA256_K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            
            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_val);
    }
}

impl HashFunction for SHA256 {
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        let mut h = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
            0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
        ];
        
        let mut padded = data.to_vec();
        let bit_len = (data.len() as u64) * 8;
        
        padded.push(0x80);
        
        while (padded.len() % 64) != 56 {
            padded.push(0x00);
        }
        
        padded.extend_from_slice(&bit_len.to_be_bytes());
        
        for chunk in padded.chunks(64) {
            self.process_block(chunk, &mut h);
        }
        
        let mut result = Vec::with_capacity(32);
        for val in h.iter() {
            result.extend_from_slice(&val.to_be_bytes());
        }
        
        result
    }
    
    fn digest_size(&self) -> usize {
        32
    }
    
    fn block_size(&self) -> usize {
        64
    }
}

pub struct SHA512;

impl SHA512 {
    pub fn new() -> Self {
        Self
    }
    
    fn process_block(&self, block: &[u8], h: &mut [u64; 8]) {
        let mut w = [0u64; 80];
        
        for i in 0..16 {
            w[i] = u64::from_be_bytes([
                block[8 * i],
                block[8 * i + 1],
                block[8 * i + 2],
                block[8 * i + 3],
                block[8 * i + 4],
                block[8 * i + 5],
                block[8 * i + 6],
                block[8 * i + 7],
            ]);
        }
        
        for i in 16..80 {
            let s0 = w[i - 15].rotate_right(1) ^ w[i - 15].rotate_right(8) ^ (w[i - 15] >> 7);
            let s1 = w[i - 2].rotate_right(19) ^ w[i - 2].rotate_right(61) ^ (w[i - 2] >> 6);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_val = h[7];
        
        for i in 0..80 {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_val.wrapping_add(s1).wrapping_add(ch).wrapping_add(SHA512_K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            
            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_val);
    }
}

impl HashFunction for SHA512 {
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        let mut h = [
            0x6a09e667f3bcc908, 0xbb67ae8584caa73b, 0x3c6ef372fe94f82b, 0xa54ff53a5f1d36f1,
            0x510e527fade682d1, 0x9b05688c2b3e6c1f, 0x1f83d9abfb41bd6b, 0x5be0cd19137e2179,
        ];
        
        let mut padded = data.to_vec();
        let bit_len = (data.len() as u128) * 8;
        
        padded.push(0x80);
        
        while (padded.len() % 128) != 112 {
            padded.push(0x00);
        }
        
        padded.extend_from_slice(&0u64.to_be_bytes());
        padded.extend_from_slice(&(bit_len as u64).to_be_bytes());
        
        for chunk in padded.chunks(128) {
            self.process_block(chunk, &mut h);
        }
        
        let mut result = Vec::with_capacity(64);
        for val in h.iter() {
            result.extend_from_slice(&val.to_be_bytes());
        }
        
        result
    }
    
    fn digest_size(&self) -> usize {
        64
    }
    
    fn block_size(&self) -> usize {
        128
    }
}

pub struct SHA3_256 {
    rate: usize,
    capacity: usize,
}

impl SHA3_256 {
    pub fn new() -> Self {
        Self {
            rate: 136,
            capacity: 64,
        }
    }
    
    fn keccak_f(&self, state: &mut [u64; 25]) {
        for round in 0..24 {
            self.theta(state);
            self.rho_pi(state);
            self.chi(state);
            self.iota(state, round);
        }
    }
    
    fn theta(&self, state: &mut [u64; 25]) {
        let mut c = [0u64; 5];
        let mut d = [0u64; 5];
        
        for x in 0..5 {
            c[x] = state[x] ^ state[x + 5] ^ state[x + 10] ^ state[x + 15] ^ state[x + 20];
        }
        
        for x in 0..5 {
            d[x] = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
        }
        
        for x in 0..5 {
            for y in 0..5 {
                state[y * 5 + x] ^= d[x];
            }
        }
    }
    
    fn rho_pi(&self, state: &mut [u64; 25]) {
        let mut temp = state[1];
        let mut x = 1;
        let mut y = 0;
        
        for t in 0..24 {
            let next_x = y;
            let next_y = (2 * x + 3 * y) % 5;
            let index = next_y * 5 + next_x;
            
            let rotation = ((t + 1) * (t + 2) / 2) % 64;
            let next_temp = state[index];
            state[index] = temp.rotate_left(rotation as u32);
            
            temp = next_temp;
            x = next_x;
            y = next_y;
        }
    }
    
    fn chi(&self, state: &mut [u64; 25]) {
        for y in 0..5 {
            let mut row = [0u64; 5];
            for x in 0..5 {
                row[x] = state[y * 5 + x];
            }
            for x in 0..5 {
                state[y * 5 + x] = row[x] ^ ((!row[(x + 1) % 5]) & row[(x + 2) % 5]);
            }
        }
    }
    
    fn iota(&self, state: &mut [u64; 25], round: usize) {
        state[0] ^= KECCAK_RC[round];
    }
}

impl HashFunction for SHA3_256 {
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        let mut state = [0u64; 25];
        let mut padded = data.to_vec();
        
        padded.push(0x06);
        
        while (padded.len() % self.rate) != 0 {
            padded.push(0x00);
        }
        padded[padded.len() - 1] |= 0x80;
        
        for chunk in padded.chunks(self.rate) {
            for (i, byte_chunk) in chunk.chunks(8).enumerate() {
                if byte_chunk.len() == 8 {
                    state[i] ^= u64::from_le_bytes(byte_chunk.try_into().unwrap());
                }
            }
            self.keccak_f(&mut state);
        }
        
        let mut output = Vec::with_capacity(32);
        for i in 0..4 {
            output.extend_from_slice(&state[i].to_le_bytes());
        }
        
        output
    }
    
    fn digest_size(&self) -> usize {
        32
    }
    
    fn block_size(&self) -> usize {
        self.rate
    }
}

pub struct BLAKE2b {
    output_size: usize,
}

impl BLAKE2b {
    pub fn new(output_size: usize) -> Self {
        Self { output_size }
    }
    
    fn g(&self, v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
        v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
        v[d] = (v[d] ^ v[a]).rotate_right(32);
        
        v[c] = v[c].wrapping_add(v[d]);
        v[b] = (v[b] ^ v[c]).rotate_right(24);
        
        v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
        v[d] = (v[d] ^ v[a]).rotate_right(16);
        
        v[c] = v[c].wrapping_add(v[d]);
        v[b] = (v[b] ^ v[c]).rotate_right(63);
    }
    
    fn compress(&self, h: &mut [u64; 8], m: &[u8], t: u64, last: bool) {
        let mut v = [0u64; 16];
        
        for i in 0..8 {
            v[i] = h[i];
            v[i + 8] = BLAKE2B_IV[i];
        }
        
        v[12] ^= t;
        
        if last {
            v[14] = !v[14];
        }
        
        let mut m_words = [0u64; 16];
        for i in 0..16 {
            if i * 8 < m.len() {
                let mut bytes = [0u8; 8];
                let len = core::cmp::min(8, m.len() - i * 8);
                bytes[..len].copy_from_slice(&m[i * 8..i * 8 + len]);
                m_words[i] = u64::from_le_bytes(bytes);
            }
        }
        
        for round in 0..12 {
            let s = &BLAKE2B_SIGMA[round];
            
            self.g(&mut v, 0, 4, 8, 12, m_words[s[0]], m_words[s[1]]);
            self.g(&mut v, 1, 5, 9, 13, m_words[s[2]], m_words[s[3]]);
            self.g(&mut v, 2, 6, 10, 14, m_words[s[4]], m_words[s[5]]);
            self.g(&mut v, 3, 7, 11, 15, m_words[s[6]], m_words[s[7]]);
            
            self.g(&mut v, 0, 5, 10, 15, m_words[s[8]], m_words[s[9]]);
            self.g(&mut v, 1, 6, 11, 12, m_words[s[10]], m_words[s[11]]);
            self.g(&mut v, 2, 7, 8, 13, m_words[s[12]], m_words[s[13]]);
            self.g(&mut v, 3, 4, 9, 14, m_words[s[14]], m_words[s[15]]);
        }
        
        for i in 0..8 {
            h[i] ^= v[i] ^ v[i + 8];
        }
    }
}

impl HashFunction for BLAKE2b {
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        let mut h = BLAKE2B_IV;
        h[0] ^= 0x01010000 ^ self.output_size as u64;
        
        let mut t = 0u64;
        let chunks: Vec<&[u8]> = data.chunks(128).collect();
        
        for (i, chunk) in chunks.iter().enumerate() {
            t += chunk.len() as u64;
            let last = i == chunks.len() - 1;
            self.compress(&mut h, chunk, t, last);
        }
        
        if chunks.is_empty() {
            self.compress(&mut h, &[], 0, true);
        }
        
        let mut output = Vec::with_capacity(self.output_size);
        for i in 0..(self.output_size + 7) / 8 {
            let bytes = h[i].to_le_bytes();
            let len = core::cmp::min(8, self.output_size - i * 8);
            output.extend_from_slice(&bytes[..len]);
        }
        
        output
    }
    
    fn digest_size(&self) -> usize {
        self.output_size
    }
    
    fn block_size(&self) -> usize {
        128
    }
}

pub fn get_hash(algorithm: HashAlgorithm, _provider: CryptoProvider) -> CryptoResult<Box<dyn HashFunction>> {
    match algorithm {
        HashAlgorithm::SHA256 => Ok(Box::new(SHA256::new())),
        HashAlgorithm::SHA512 => Ok(Box::new(SHA512::new())),
        HashAlgorithm::SHA3_256 => Ok(Box::new(SHA3_256::new())),
        HashAlgorithm::BLAKE2b => Ok(Box::new(BLAKE2b::new(64))),
        HashAlgorithm::BLAKE2s => Ok(Box::new(BLAKE2b::new(32))),
        _ => Err(CryptoError::UnsupportedAlgorithm),
    }
}

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

const SHA512_K: [u64; 80] = [
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
    0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
    0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
    0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
    0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
    0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
    0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
    0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
    0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
    0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
];

const KECCAK_RC: [u64; 24] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
    0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
];

const BLAKE2B_IV: [u64; 8] = [
    0x6a09e667f3bcc908, 0xbb67ae8584caa73b, 0x3c6ef372fe94f82b, 0xa54ff53a5f1d36f1,
    0x510e527fade682d1, 0x9b05688c2b3e6c1f, 0x1f83d9abfb41bd6b, 0x5be0cd19137e2179,
];

const BLAKE2B_SIGMA: [[usize; 16]; 12] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
];