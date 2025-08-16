#![no_std]

use alloc::vec::Vec;
use core::convert::TryInto;
use super::errors::{CryptoError, CryptoResult};
use super::CryptoProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherAlgorithm {
    Aes128,
    Aes192,
    Aes256,
    ChaCha20,
    TripleDes,
    Blowfish,
    Twofish,
    Camellia128,
    Camellia256,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherMode {
    ECB,
    CBC,
    CTR,
    OFB,
    CFB,
    XTS,
}

pub trait SymmetricCipher: Send + Sync {
    fn encrypt(&self, plaintext: &[u8], key: &[u8], iv: Option<&[u8]>) -> CryptoResult<Vec<u8>>;
    fn decrypt(&self, ciphertext: &[u8], key: &[u8], iv: Option<&[u8]>) -> CryptoResult<Vec<u8>>;
    fn block_size(&self) -> usize;
    fn key_size(&self) -> usize;
    fn iv_size(&self) -> Option<usize>;
}

pub struct AesCipher {
    key_size: usize,
    mode: CipherMode,
}

impl AesCipher {
    pub fn new(key_size: usize, mode: CipherMode) -> Self {
        Self { key_size, mode }
    }
    
    fn aes_encrypt_block(&self, block: &[u8], round_keys: &[u32]) -> [u8; 16] {
        let mut state = [0u8; 16];
        state.copy_from_slice(block);
        
        let nr = match self.key_size {
            16 => 10,
            24 => 12,
            32 => 14,
            _ => panic!("Invalid AES key size"),
        };
        
        self.add_round_key(&mut state, &round_keys[0..4]);
        
        for round in 1..nr {
            self.sub_bytes(&mut state);
            self.shift_rows(&mut state);
            self.mix_columns(&mut state);
            self.add_round_key(&mut state, &round_keys[round * 4..(round + 1) * 4]);
        }
        
        self.sub_bytes(&mut state);
        self.shift_rows(&mut state);
        self.add_round_key(&mut state, &round_keys[nr * 4..(nr + 1) * 4]);
        
        state
    }
    
    fn aes_decrypt_block(&self, block: &[u8], round_keys: &[u32]) -> [u8; 16] {
        let mut state = [0u8; 16];
        state.copy_from_slice(block);
        
        let nr = match self.key_size {
            16 => 10,
            24 => 12,
            32 => 14,
            _ => panic!("Invalid AES key size"),
        };
        
        self.add_round_key(&mut state, &round_keys[nr * 4..(nr + 1) * 4]);
        
        for round in (1..nr).rev() {
            self.inv_shift_rows(&mut state);
            self.inv_sub_bytes(&mut state);
            self.add_round_key(&mut state, &round_keys[round * 4..(round + 1) * 4]);
            self.inv_mix_columns(&mut state);
        }
        
        self.inv_shift_rows(&mut state);
        self.inv_sub_bytes(&mut state);
        self.add_round_key(&mut state, &round_keys[0..4]);
        
        state
    }
    
    fn key_expansion(&self, key: &[u8]) -> Vec<u32> {
        let nk = self.key_size / 4;
        let nr = match self.key_size {
            16 => 10,
            24 => 12,
            32 => 14,
            _ => panic!("Invalid AES key size"),
        };
        
        let mut w = Vec::with_capacity(4 * (nr + 1));
        
        for i in 0..nk {
            w.push(u32::from_be_bytes([
                key[4 * i],
                key[4 * i + 1],
                key[4 * i + 2],
                key[4 * i + 3],
            ]));
        }
        
        for i in nk..(4 * (nr + 1)) {
            let mut temp = w[i - 1];
            if i % nk == 0 {
                temp = self.sub_word(self.rot_word(temp)) ^ self.rcon(i / nk);
            } else if nk > 6 && i % nk == 4 {
                temp = self.sub_word(temp);
            }
            w.push(w[i - nk] ^ temp);
        }
        
        w
    }
    
    fn add_round_key(&self, state: &mut [u8; 16], round_key: &[u32]) {
        for i in 0..4 {
            let key_bytes = round_key[i].to_be_bytes();
            for j in 0..4 {
                state[i * 4 + j] ^= key_bytes[j];
            }
        }
    }
    
    fn sub_bytes(&self, state: &mut [u8; 16]) {
        for byte in state.iter_mut() {
            *byte = AES_SBOX[*byte as usize];
        }
    }
    
    fn inv_sub_bytes(&self, state: &mut [u8; 16]) {
        for byte in state.iter_mut() {
            *byte = AES_INV_SBOX[*byte as usize];
        }
    }
    
    fn shift_rows(&self, state: &mut [u8; 16]) {
        let temp = state[1];
        state[1] = state[5];
        state[5] = state[9];
        state[9] = state[13];
        state[13] = temp;
        
        let temp = state[2];
        state[2] = state[10];
        state[10] = temp;
        let temp = state[6];
        state[6] = state[14];
        state[14] = temp;
        
        let temp = state[3];
        state[3] = state[15];
        state[15] = state[11];
        state[11] = state[7];
        state[7] = temp;
    }
    
    fn inv_shift_rows(&self, state: &mut [u8; 16]) {
        let temp = state[13];
        state[13] = state[9];
        state[9] = state[5];
        state[5] = state[1];
        state[1] = temp;
        
        let temp = state[2];
        state[2] = state[10];
        state[10] = temp;
        let temp = state[6];
        state[6] = state[14];
        state[14] = temp;
        
        let temp = state[7];
        state[7] = state[11];
        state[11] = state[15];
        state[15] = state[3];
        state[3] = temp;
    }
    
    fn mix_columns(&self, state: &mut [u8; 16]) {
        for i in 0..4 {
            let col = [
                state[i * 4],
                state[i * 4 + 1],
                state[i * 4 + 2],
                state[i * 4 + 3],
            ];
            
            state[i * 4] = gmul(0x02, col[0]) ^ gmul(0x03, col[1]) ^ col[2] ^ col[3];
            state[i * 4 + 1] = col[0] ^ gmul(0x02, col[1]) ^ gmul(0x03, col[2]) ^ col[3];
            state[i * 4 + 2] = col[0] ^ col[1] ^ gmul(0x02, col[2]) ^ gmul(0x03, col[3]);
            state[i * 4 + 3] = gmul(0x03, col[0]) ^ col[1] ^ col[2] ^ gmul(0x02, col[3]);
        }
    }
    
    fn inv_mix_columns(&self, state: &mut [u8; 16]) {
        for i in 0..4 {
            let col = [
                state[i * 4],
                state[i * 4 + 1],
                state[i * 4 + 2],
                state[i * 4 + 3],
            ];
            
            state[i * 4] = gmul(0x0e, col[0]) ^ gmul(0x0b, col[1]) ^ gmul(0x0d, col[2]) ^ gmul(0x09, col[3]);
            state[i * 4 + 1] = gmul(0x09, col[0]) ^ gmul(0x0e, col[1]) ^ gmul(0x0b, col[2]) ^ gmul(0x0d, col[3]);
            state[i * 4 + 2] = gmul(0x0d, col[0]) ^ gmul(0x09, col[1]) ^ gmul(0x0e, col[2]) ^ gmul(0x0b, col[3]);
            state[i * 4 + 3] = gmul(0x0b, col[0]) ^ gmul(0x0d, col[1]) ^ gmul(0x09, col[2]) ^ gmul(0x0e, col[3]);
        }
    }
    
    fn rot_word(&self, word: u32) -> u32 {
        (word << 8) | (word >> 24)
    }
    
    fn sub_word(&self, word: u32) -> u32 {
        let bytes = word.to_be_bytes();
        u32::from_be_bytes([
            AES_SBOX[bytes[0] as usize],
            AES_SBOX[bytes[1] as usize],
            AES_SBOX[bytes[2] as usize],
            AES_SBOX[bytes[3] as usize],
        ])
    }
    
    fn rcon(&self, i: usize) -> u32 {
        RCON[i - 1]
    }
}

impl SymmetricCipher for AesCipher {
    fn encrypt(&self, plaintext: &[u8], key: &[u8], iv: Option<&[u8]>) -> CryptoResult<Vec<u8>> {
        if key.len() != self.key_size {
            return Err(CryptoError::InvalidKeySize);
        }
        
        let round_keys = self.key_expansion(key);
        let mut ciphertext = Vec::new();
        
        match self.mode {
            CipherMode::ECB => {
                for chunk in plaintext.chunks(16) {
                    let mut block = [0u8; 16];
                    block[..chunk.len()].copy_from_slice(chunk);
                    if chunk.len() < 16 {
                        apply_pkcs7_padding(&mut block, chunk.len());
                    }
                    let encrypted = self.aes_encrypt_block(&block, &round_keys);
                    ciphertext.extend_from_slice(&encrypted);
                }
            }
            CipherMode::CBC => {
                let iv = iv.ok_or(CryptoError::InvalidNonce)?;
                if iv.len() != 16 {
                    return Err(CryptoError::InvalidNonce);
                }
                
                let mut prev_block = [0u8; 16];
                prev_block.copy_from_slice(iv);
                
                for chunk in plaintext.chunks(16) {
                    let mut block = [0u8; 16];
                    block[..chunk.len()].copy_from_slice(chunk);
                    if chunk.len() < 16 {
                        apply_pkcs7_padding(&mut block, chunk.len());
                    }
                    
                    for i in 0..16 {
                        block[i] ^= prev_block[i];
                    }
                    
                    let encrypted = self.aes_encrypt_block(&block, &round_keys);
                    ciphertext.extend_from_slice(&encrypted);
                    prev_block = encrypted;
                }
            }
            _ => return Err(CryptoError::UnsupportedAlgorithm),
        }
        
        Ok(ciphertext)
    }
    
    fn decrypt(&self, ciphertext: &[u8], key: &[u8], iv: Option<&[u8]>) -> CryptoResult<Vec<u8>> {
        if key.len() != self.key_size {
            return Err(CryptoError::InvalidKeySize);
        }
        
        if ciphertext.len() % 16 != 0 {
            return Err(CryptoError::InvalidBlockSize);
        }
        
        let round_keys = self.key_expansion(key);
        let mut plaintext = Vec::new();
        
        match self.mode {
            CipherMode::ECB => {
                for chunk in ciphertext.chunks(16) {
                    let decrypted = self.aes_decrypt_block(chunk, &round_keys);
                    plaintext.extend_from_slice(&decrypted);
                }
            }
            CipherMode::CBC => {
                let iv = iv.ok_or(CryptoError::InvalidNonce)?;
                if iv.len() != 16 {
                    return Err(CryptoError::InvalidNonce);
                }
                
                let mut prev_block = [0u8; 16];
                prev_block.copy_from_slice(iv);
                
                for chunk in ciphertext.chunks(16) {
                    let decrypted = self.aes_decrypt_block(chunk, &round_keys);
                    
                    let mut block = [0u8; 16];
                    for i in 0..16 {
                        block[i] = decrypted[i] ^ prev_block[i];
                    }
                    
                    plaintext.extend_from_slice(&block);
                    prev_block.copy_from_slice(chunk);
                }
            }
            _ => return Err(CryptoError::UnsupportedAlgorithm),
        }
        
        remove_pkcs7_padding(&mut plaintext)?;
        Ok(plaintext)
    }
    
    fn block_size(&self) -> usize {
        16
    }
    
    fn key_size(&self) -> usize {
        self.key_size
    }
    
    fn iv_size(&self) -> Option<usize> {
        match self.mode {
            CipherMode::ECB => None,
            _ => Some(16),
        }
    }
}

pub struct ChaCha20Cipher;

impl ChaCha20Cipher {
    pub fn new() -> Self {
        Self
    }
    
    fn quarter_round(&self, a: &mut u32, b: &mut u32, c: &mut u32, d: &mut u32) {
        *a = a.wrapping_add(*b);
        *d ^= *a;
        *d = d.rotate_left(16);
        
        *c = c.wrapping_add(*d);
        *b ^= *c;
        *b = b.rotate_left(12);
        
        *a = a.wrapping_add(*b);
        *d ^= *a;
        *d = d.rotate_left(8);
        
        *c = c.wrapping_add(*d);
        *b ^= *c;
        *b = b.rotate_left(7);
    }
    
    fn chacha20_block(&self, key: &[u8; 32], nonce: &[u8; 12], counter: u32) -> [u8; 64] {
        let mut state = [0u32; 16];
        
        state[0] = 0x61707865;
        state[1] = 0x3320646e;
        state[2] = 0x79622d32;
        state[3] = 0x6b206574;
        
        for i in 0..8 {
            state[4 + i] = u32::from_le_bytes([
                key[4 * i],
                key[4 * i + 1],
                key[4 * i + 2],
                key[4 * i + 3],
            ]);
        }
        
        state[12] = counter;
        
        for i in 0..3 {
            state[13 + i] = u32::from_le_bytes([
                nonce[4 * i],
                nonce[4 * i + 1],
                nonce[4 * i + 2],
                nonce[4 * i + 3],
            ]);
        }
        
        let mut working_state = state;
        
        for _ in 0..10 {
            self.quarter_round(&mut working_state[0], &mut working_state[4], &mut working_state[8], &mut working_state[12]);
            self.quarter_round(&mut working_state[1], &mut working_state[5], &mut working_state[9], &mut working_state[13]);
            self.quarter_round(&mut working_state[2], &mut working_state[6], &mut working_state[10], &mut working_state[14]);
            self.quarter_round(&mut working_state[3], &mut working_state[7], &mut working_state[11], &mut working_state[15]);
            
            self.quarter_round(&mut working_state[0], &mut working_state[5], &mut working_state[10], &mut working_state[15]);
            self.quarter_round(&mut working_state[1], &mut working_state[6], &mut working_state[11], &mut working_state[12]);
            self.quarter_round(&mut working_state[2], &mut working_state[7], &mut working_state[8], &mut working_state[13]);
            self.quarter_round(&mut working_state[3], &mut working_state[4], &mut working_state[9], &mut working_state[14]);
        }
        
        for i in 0..16 {
            working_state[i] = working_state[i].wrapping_add(state[i]);
        }
        
        let mut output = [0u8; 64];
        for i in 0..16 {
            let bytes = working_state[i].to_le_bytes();
            output[4 * i..4 * i + 4].copy_from_slice(&bytes);
        }
        
        output
    }
}

impl SymmetricCipher for ChaCha20Cipher {
    fn encrypt(&self, plaintext: &[u8], key: &[u8], iv: Option<&[u8]>) -> CryptoResult<Vec<u8>> {
        if key.len() != 32 {
            return Err(CryptoError::InvalidKeySize);
        }
        
        let nonce = iv.ok_or(CryptoError::InvalidNonce)?;
        if nonce.len() != 12 {
            return Err(CryptoError::InvalidNonce);
        }
        
        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(key);
        
        let mut nonce_array = [0u8; 12];
        nonce_array.copy_from_slice(nonce);
        
        let mut ciphertext = Vec::with_capacity(plaintext.len());
        let mut counter = 0u32;
        
        for chunk in plaintext.chunks(64) {
            let keystream = self.chacha20_block(&key_array, &nonce_array, counter);
            counter += 1;
            
            for (i, &byte) in chunk.iter().enumerate() {
                ciphertext.push(byte ^ keystream[i]);
            }
        }
        
        Ok(ciphertext)
    }
    
    fn decrypt(&self, ciphertext: &[u8], key: &[u8], iv: Option<&[u8]>) -> CryptoResult<Vec<u8>> {
        self.encrypt(ciphertext, key, iv)
    }
    
    fn block_size(&self) -> usize {
        64
    }
    
    fn key_size(&self) -> usize {
        32
    }
    
    fn iv_size(&self) -> Option<usize> {
        Some(12)
    }
}

fn gmul(a: u8, b: u8) -> u8 {
    let mut p = 0u8;
    let mut a = a;
    let mut b = b;
    
    for _ in 0..8 {
        if b & 1 != 0 {
            p ^= a;
        }
        
        let hi_bit = a & 0x80;
        a <<= 1;
        if hi_bit != 0 {
            a ^= 0x1b;
        }
        b >>= 1;
    }
    
    p
}

fn apply_pkcs7_padding(block: &mut [u8; 16], data_len: usize) {
    let padding_len = 16 - data_len;
    for i in data_len..16 {
        block[i] = padding_len as u8;
    }
}

fn remove_pkcs7_padding(data: &mut Vec<u8>) -> CryptoResult<()> {
    if data.is_empty() {
        return Err(CryptoError::InvalidPadding);
    }
    
    let padding_len = data[data.len() - 1] as usize;
    if padding_len == 0 || padding_len > 16 {
        return Err(CryptoError::InvalidPadding);
    }
    
    for i in 1..=padding_len {
        if data[data.len() - i] != padding_len as u8 {
            return Err(CryptoError::InvalidPadding);
        }
    }
    
    data.truncate(data.len() - padding_len);
    Ok(())
}

pub fn get_cipher(algorithm: CipherAlgorithm, _provider: CryptoProvider) -> CryptoResult<Box<dyn SymmetricCipher>> {
    match algorithm {
        CipherAlgorithm::Aes128 => Ok(Box::new(AesCipher::new(16, CipherMode::CBC))),
        CipherAlgorithm::Aes192 => Ok(Box::new(AesCipher::new(24, CipherMode::CBC))),
        CipherAlgorithm::Aes256 => Ok(Box::new(AesCipher::new(32, CipherMode::CBC))),
        CipherAlgorithm::ChaCha20 => Ok(Box::new(ChaCha20Cipher::new())),
        _ => Err(CryptoError::UnsupportedAlgorithm),
    }
}

const AES_SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

const AES_INV_SBOX: [u8; 256] = [
    0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7, 0xfb,
    0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde, 0xe9, 0xcb,
    0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42, 0xfa, 0xc3, 0x4e,
    0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49, 0x6d, 0x8b, 0xd1, 0x25,
    0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c, 0xcc, 0x5d, 0x65, 0xb6, 0x92,
    0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15, 0x46, 0x57, 0xa7, 0x8d, 0x9d, 0x84,
    0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7, 0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45, 0x06,
    0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02, 0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a, 0x6b,
    0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc, 0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6, 0x73,
    0x96, 0xac, 0x74, 0x22, 0xe7, 0xad, 0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf, 0x6e,
    0x47, 0xf1, 0x1a, 0x71, 0x1d, 0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe, 0x1b,
    0xfc, 0x56, 0x3e, 0x4b, 0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a, 0xf4,
    0x1f, 0xdd, 0xa8, 0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec, 0x5f,
    0x60, 0x51, 0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c, 0xef,
    0xa0, 0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99, 0x61,
    0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c, 0x7d,
];

const RCON: [u32; 10] = [
    0x01000000, 0x02000000, 0x04000000, 0x08000000, 0x10000000,
    0x20000000, 0x40000000, 0x80000000, 0x1b000000, 0x36000000,
];