#![no_std]

use alloc::vec::Vec;
use core::arch::x86_64::*;

#[derive(Debug, Clone, Copy)]
pub struct CryptoFeatures {
    pub aes_ni: bool,
    pub sha_ni: bool,
    pub rdrand: bool,
    pub rdseed: bool,
    pub avx: bool,
    pub avx2: bool,
    pub avx512: bool,
    pub vaes: bool,
    pub vpclmulqdq: bool,
}

impl CryptoFeatures {
    pub fn detect() -> Self {
        let mut features = Self {
            aes_ni: false,
            sha_ni: false,
            rdrand: false,
            rdseed: false,
            avx: false,
            avx2: false,
            avx512: false,
            vaes: false,
            vpclmulqdq: false,
        };
        
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let cpuid_1 = __cpuid(1);
            let cpuid_7 = __cpuid_count(7, 0);
            
            features.aes_ni = (cpuid_1.ecx & (1 << 25)) != 0;
            features.rdrand = (cpuid_1.ecx & (1 << 30)) != 0;
            features.avx = (cpuid_1.ecx & (1 << 28)) != 0;
            
            features.avx2 = (cpuid_7.ebx & (1 << 5)) != 0;
            features.rdseed = (cpuid_7.ebx & (1 << 18)) != 0;
            features.sha_ni = (cpuid_7.ebx & (1 << 29)) != 0;
            features.avx512 = (cpuid_7.ebx & (1 << 16)) != 0;
            features.vaes = (cpuid_7.ecx & (1 << 9)) != 0;
            features.vpclmulqdq = (cpuid_7.ecx & (1 << 10)) != 0;
        }
        
        features
    }
}

pub fn detect_hardware_crypto() -> bool {
    let features = CryptoFeatures::detect();
    features.aes_ni || features.sha_ni || features.rdrand
}

pub fn init_hardware_crypto() {
    let features = CryptoFeatures::detect();
    
    if features.aes_ni {
        log::info!("AES-NI hardware acceleration available");
    }
    if features.sha_ni {
        log::info!("SHA-NI hardware acceleration available");
    }
    if features.rdrand {
        log::info!("RDRAND hardware RNG available");
    }
    if features.rdseed {
        log::info!("RDSEED hardware entropy available");
    }
    if features.avx2 {
        log::info!("AVX2 SIMD acceleration available");
    }
    if features.avx512 {
        log::info!("AVX-512 SIMD acceleration available");
    }
}

#[cfg(target_arch = "x86_64")]
pub mod aes_ni {
    use super::*;
    use core::arch::x86_64::*;
    
    #[inline]
    pub unsafe fn aes_encrypt_block(block: &[u8; 16], round_keys: &[__m128i]) -> [u8; 16] {
        let mut state = _mm_loadu_si128(block.as_ptr() as *const __m128i);
        
        state = _mm_xor_si128(state, round_keys[0]);
        
        for i in 1..10 {
            state = _mm_aesenc_si128(state, round_keys[i]);
        }
        
        state = _mm_aesenclast_si128(state, round_keys[10]);
        
        let mut output = [0u8; 16];
        _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, state);
        output
    }
    
    #[inline]
    pub unsafe fn aes_decrypt_block(block: &[u8; 16], round_keys: &[__m128i]) -> [u8; 16] {
        let mut state = _mm_loadu_si128(block.as_ptr() as *const __m128i);
        
        state = _mm_xor_si128(state, round_keys[10]);
        
        for i in (1..10).rev() {
            state = _mm_aesdec_si128(state, round_keys[i]);
        }
        
        state = _mm_aesdeclast_si128(state, round_keys[0]);
        
        let mut output = [0u8; 16];
        _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, state);
        output
    }
    
    #[inline]
    pub unsafe fn aes_key_expansion_128(key: &[u8; 16]) -> Vec<__m128i> {
        let mut round_keys = Vec::with_capacity(11);
        
        let mut key_schedule = _mm_loadu_si128(key.as_ptr() as *const __m128i);
        round_keys.push(key_schedule);
        
        for rcon in [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36] {
            key_schedule = aes_128_key_expansion_assist(key_schedule, rcon);
            round_keys.push(key_schedule);
        }
        
        round_keys
    }
    
    #[inline]
    unsafe fn aes_128_key_expansion_assist(key: __m128i, rcon: u8) -> __m128i {
        let mut keygened = _mm_aeskeygenassist_si128(key, rcon);
        keygened = _mm_shuffle_epi32(keygened, 0xff);
        
        let mut key = key;
        key = _mm_xor_si128(key, _mm_slli_si128(key, 4));
        key = _mm_xor_si128(key, _mm_slli_si128(key, 4));
        key = _mm_xor_si128(key, _mm_slli_si128(key, 4));
        
        _mm_xor_si128(key, keygened)
    }
    
    #[inline]
    pub unsafe fn aes_key_expansion_256(key: &[u8; 32]) -> Vec<__m128i> {
        let mut round_keys = Vec::with_capacity(15);
        
        let mut key1 = _mm_loadu_si128(key.as_ptr() as *const __m128i);
        let mut key2 = _mm_loadu_si128(key[16..].as_ptr() as *const __m128i);
        
        round_keys.push(key1);
        round_keys.push(key2);
        
        for i in 0..6 {
            key1 = aes_256_key_expansion_assist_1(key1, key2, 1 << i);
            round_keys.push(key1);
            
            key2 = aes_256_key_expansion_assist_2(key1, key2);
            round_keys.push(key2);
        }
        
        key1 = aes_256_key_expansion_assist_1(key1, key2, 0x40);
        round_keys.push(key1);
        
        round_keys
    }
    
    #[inline]
    unsafe fn aes_256_key_expansion_assist_1(key1: __m128i, key2: __m128i, rcon: u8) -> __m128i {
        let mut keygened = _mm_aeskeygenassist_si128(key2, rcon);
        keygened = _mm_shuffle_epi32(keygened, 0xff);
        
        let mut key1 = key1;
        key1 = _mm_xor_si128(key1, _mm_slli_si128(key1, 4));
        key1 = _mm_xor_si128(key1, _mm_slli_si128(key1, 4));
        key1 = _mm_xor_si128(key1, _mm_slli_si128(key1, 4));
        
        _mm_xor_si128(key1, keygened)
    }
    
    #[inline]
    unsafe fn aes_256_key_expansion_assist_2(key1: __m128i, key2: __m128i) -> __m128i {
        let mut keygened = _mm_aeskeygenassist_si128(key1, 0);
        keygened = _mm_shuffle_epi32(keygened, 0xaa);
        
        let mut key2 = key2;
        key2 = _mm_xor_si128(key2, _mm_slli_si128(key2, 4));
        key2 = _mm_xor_si128(key2, _mm_slli_si128(key2, 4));
        key2 = _mm_xor_si128(key2, _mm_slli_si128(key2, 4));
        
        _mm_xor_si128(key2, keygened)
    }
    
    pub fn aes_gcm_encrypt(plaintext: &[u8], key: &[u8], nonce: &[u8; 12], aad: &[u8]) -> Vec<u8> {
        unsafe {
            let round_keys = if key.len() == 16 {
                aes_key_expansion_128(key.try_into().unwrap())
            } else {
                aes_key_expansion_256(key.try_into().unwrap())
            };
            
            let mut ciphertext = Vec::with_capacity(plaintext.len() + 16);
            
            let mut counter = [0u8; 16];
            counter[..12].copy_from_slice(nonce);
            counter[15] = 1;
            
            for chunk in plaintext.chunks(16) {
                counter[15] = counter[15].wrapping_add(1);
                let keystream = aes_encrypt_block(&counter, &round_keys);
                
                for (i, &byte) in chunk.iter().enumerate() {
                    ciphertext.push(byte ^ keystream[i]);
                }
            }
            
            let h = aes_encrypt_block(&[0u8; 16], &round_keys);
            let mut tag = ghash(&h, aad, &ciphertext);
            
            counter[15] = 1;
            let j0_encrypted = aes_encrypt_block(&counter, &round_keys);
            for i in 0..16 {
                tag[i] ^= j0_encrypted[i];
            }
            
            ciphertext.extend_from_slice(&tag);
            ciphertext
        }
    }
    
    fn ghash(h: &[u8; 16], aad: &[u8], ciphertext: &[u8]) -> [u8; 16] {
        let mut y = [0u8; 16];
        
        for chunk in aad.chunks(16) {
            for i in 0..chunk.len() {
                y[i] ^= chunk[i];
            }
            gf_mult_ni(&mut y, h);
        }
        
        for chunk in ciphertext.chunks(16) {
            for i in 0..chunk.len() {
                y[i] ^= chunk[i];
            }
            gf_mult_ni(&mut y, h);
        }
        
        let aad_bits = (aad.len() * 8) as u64;
        let ct_bits = (ciphertext.len() * 8) as u64;
        
        for (i, &byte) in aad_bits.to_be_bytes().iter().enumerate() {
            y[i] ^= byte;
        }
        for (i, &byte) in ct_bits.to_be_bytes().iter().enumerate() {
            y[8 + i] ^= byte;
        }
        
        gf_mult_ni(&mut y, h);
        y
    }
    
    #[inline]
    fn gf_mult_ni(x: &mut [u8; 16], y: &[u8; 16]) {
        unsafe {
            let a = _mm_loadu_si128(x.as_ptr() as *const __m128i);
            let b = _mm_loadu_si128(y.as_ptr() as *const __m128i);
            
            let tmp1 = _mm_clmulepi64_si128(a, b, 0x00);
            let tmp2 = _mm_clmulepi64_si128(a, b, 0x01);
            let tmp3 = _mm_clmulepi64_si128(a, b, 0x10);
            let tmp4 = _mm_clmulepi64_si128(a, b, 0x11);
            
            let tmp5 = _mm_xor_si128(tmp2, tmp3);
            let tmp6 = _mm_slli_si128(tmp5, 8);
            let tmp7 = _mm_srli_si128(tmp5, 8);
            
            let tmp8 = _mm_xor_si128(tmp1, tmp6);
            let tmp9 = _mm_xor_si128(tmp4, tmp7);
            
            let poly = _mm_set_epi64x(0, 0xc200000000000000u64 as i64);
            
            let tmp10 = _mm_clmulepi64_si128(tmp8, poly, 0x10);
            let tmp11 = _mm_shuffle_epi32(tmp8, 0x4e);
            let tmp12 = _mm_xor_si128(tmp10, tmp11);
            
            let tmp13 = _mm_clmulepi64_si128(tmp12, poly, 0x10);
            let tmp14 = _mm_shuffle_epi32(tmp12, 0x4e);
            let tmp15 = _mm_xor_si128(tmp13, tmp14);
            
            let result = _mm_xor_si128(tmp15, tmp9);
            _mm_storeu_si128(x.as_mut_ptr() as *mut __m128i, result);
        }
    }
}

#[cfg(target_arch = "x86_64")]
pub mod sha_ni {
    use super::*;
    use core::arch::x86_64::*;
    
    #[inline]
    pub unsafe fn sha256_hw(data: &[u8]) -> [u8; 32] {
        let mut h0 = _mm_set_epi32(
            0x6a09e667i32, 0xbb67ae85i32, 0x3c6ef372i32, 0xa54ff53ai32,
        );
        let mut h1 = _mm_set_epi32(
            0x510e527fi32, 0x9b05688ci32, 0x1f83d9abi32, 0x5be0cd19i32,
        );
        
        let mut padded = data.to_vec();
        let bit_len = (data.len() as u64) * 8;
        
        padded.push(0x80);
        while (padded.len() % 64) != 56 {
            padded.push(0x00);
        }
        padded.extend_from_slice(&bit_len.to_be_bytes());
        
        for chunk in padded.chunks(64) {
            let msg = chunk.as_ptr() as *const __m128i;
            
            let abef_save = h0;
            let cdgh_save = h1;
            
            let mut msg0 = _mm_loadu_si128(msg.add(0));
            let mut msg1 = _mm_loadu_si128(msg.add(1));
            let mut msg2 = _mm_loadu_si128(msg.add(2));
            let mut msg3 = _mm_loadu_si128(msg.add(3));
            
            msg0 = _mm_shuffle_epi8(msg0, _mm_set_epi8(
                12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3
            ));
            msg1 = _mm_shuffle_epi8(msg1, _mm_set_epi8(
                12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3
            ));
            msg2 = _mm_shuffle_epi8(msg2, _mm_set_epi8(
                12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3
            ));
            msg3 = _mm_shuffle_epi8(msg3, _mm_set_epi8(
                12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3
            ));
            
            let k = SHA256_K.as_ptr() as *const __m128i;
            
            h1 = _mm_sha256rnds2_epu32(h1, h0, _mm_add_epi32(msg0, _mm_loadu_si128(k.add(0))));
            h0 = _mm_sha256rnds2_epu32(h0, h1, _mm_shuffle_epi32(
                _mm_add_epi32(msg0, _mm_loadu_si128(k.add(0))), 0x0e
            ));
            
            msg0 = _mm_sha256msg1_epu32(msg0, msg1);
            msg0 = _mm_sha256msg2_epu32(_mm_add_epi32(msg0, _mm_alignr_epi8(msg3, msg2, 4)), msg3);
            
            h0 = _mm_add_epi32(h0, abef_save);
            h1 = _mm_add_epi32(h1, cdgh_save);
        }
        
        let mut result = [0u8; 32];
        _mm_storeu_si128(result.as_mut_ptr() as *mut __m128i, h0);
        _mm_storeu_si128(result[16..].as_mut_ptr() as *mut __m128i, h1);
        
        result
    }
    
    const SHA256_K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
        0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
        0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
        0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
        0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
}