use core::arch::x86_64::*;

pub unsafe fn memcpy_optimized(dest: *mut u8, src: *const u8, len: usize) {
    let cpu_info = crate::cpu::get_info();
    
    if len < 32 {
        // Small copies - use simple byte copy
        core::ptr::copy_nonoverlapping(src, dest, len);
        return;
    }
    
    if cpu_info.has_avx2() && len >= 256 {
        memcpy_avx2(dest, src, len);
    } else if cpu_info.has_sse2() && len >= 64 {
        memcpy_sse2(dest, src, len);
    } else {
        // Fallback to standard copy
        core::ptr::copy_nonoverlapping(src, dest, len);
    }
}

unsafe fn memcpy_avx2(mut dest: *mut u8, mut src: *const u8, mut len: usize) {
    // Align destination to 32 bytes
    let align_offset = dest.align_offset(32);
    if align_offset > 0 && align_offset < len {
        core::ptr::copy_nonoverlapping(src, dest, align_offset);
        dest = dest.add(align_offset);
        src = src.add(align_offset);
        len -= align_offset;
    }
    
    // Copy 256 bytes at a time using AVX2
    while len >= 256 {
        for i in 0..8 {
            let data = _mm256_loadu_si256(src.add(i * 32) as *const __m256i);
            _mm256_storeu_si256(dest.add(i * 32) as *mut __m256i, data);
        }
        dest = dest.add(256);
        src = src.add(256);
        len -= 256;
    }
    
    // Copy 32 bytes at a time
    while len >= 32 {
        let data = _mm256_loadu_si256(src as *const __m256i);
        _mm256_storeu_si256(dest as *mut __m256i, data);
        dest = dest.add(32);
        src = src.add(32);
        len -= 32;
    }
    
    // Copy remaining bytes
    if len > 0 {
        core::ptr::copy_nonoverlapping(src, dest, len);
    }
}

unsafe fn memcpy_sse2(mut dest: *mut u8, mut src: *const u8, mut len: usize) {
    // Align destination to 16 bytes
    let align_offset = dest.align_offset(16);
    if align_offset > 0 && align_offset < len {
        core::ptr::copy_nonoverlapping(src, dest, align_offset);
        dest = dest.add(align_offset);
        src = src.add(align_offset);
        len -= align_offset;
    }
    
    // Copy 64 bytes at a time using SSE2
    while len >= 64 {
        let data0 = _mm_loadu_si128(src as *const __m128i);
        let data1 = _mm_loadu_si128(src.add(16) as *const __m128i);
        let data2 = _mm_loadu_si128(src.add(32) as *const __m128i);
        let data3 = _mm_loadu_si128(src.add(48) as *const __m128i);
        
        _mm_storeu_si128(dest as *mut __m128i, data0);
        _mm_storeu_si128(dest.add(16) as *mut __m128i, data1);
        _mm_storeu_si128(dest.add(32) as *mut __m128i, data2);
        _mm_storeu_si128(dest.add(48) as *mut __m128i, data3);
        
        dest = dest.add(64);
        src = src.add(64);
        len -= 64;
    }
    
    // Copy 16 bytes at a time
    while len >= 16 {
        let data = _mm_loadu_si128(src as *const __m128i);
        _mm_storeu_si128(dest as *mut __m128i, data);
        dest = dest.add(16);
        src = src.add(16);
        len -= 16;
    }
    
    // Copy remaining bytes
    if len > 0 {
        core::ptr::copy_nonoverlapping(src, dest, len);
    }
}

pub unsafe fn memset_optimized(dest: *mut u8, val: u8, len: usize) {
    let cpu_info = crate::cpu::get_info();
    
    if len < 32 {
        // Small sets - use simple byte set
        core::ptr::write_bytes(dest, val, len);
        return;
    }
    
    if cpu_info.has_avx2() && len >= 256 {
        memset_avx2(dest, val, len);
    } else if cpu_info.has_sse2() && len >= 64 {
        memset_sse2(dest, val, len);
    } else {
        // Fallback to standard memset
        core::ptr::write_bytes(dest, val, len);
    }
}

unsafe fn memset_avx2(mut dest: *mut u8, val: u8, mut len: usize) {
    // Create 256-bit value pattern
    let val_256 = _mm256_set1_epi8(val as i8);
    
    // Align destination to 32 bytes
    let align_offset = dest.align_offset(32);
    if align_offset > 0 && align_offset < len {
        core::ptr::write_bytes(dest, val, align_offset);
        dest = dest.add(align_offset);
        len -= align_offset;
    }
    
    // Set 256 bytes at a time
    while len >= 256 {
        for i in 0..8 {
            _mm256_storeu_si256(dest.add(i * 32) as *mut __m256i, val_256);
        }
        dest = dest.add(256);
        len -= 256;
    }
    
    // Set 32 bytes at a time
    while len >= 32 {
        _mm256_storeu_si256(dest as *mut __m256i, val_256);
        dest = dest.add(32);
        len -= 32;
    }
    
    // Set remaining bytes
    if len > 0 {
        core::ptr::write_bytes(dest, val, len);
    }
}

unsafe fn memset_sse2(mut dest: *mut u8, val: u8, mut len: usize) {
    // Create 128-bit value pattern
    let val_128 = _mm_set1_epi8(val as i8);
    
    // Align destination to 16 bytes
    let align_offset = dest.align_offset(16);
    if align_offset > 0 && align_offset < len {
        core::ptr::write_bytes(dest, val, align_offset);
        dest = dest.add(align_offset);
        len -= align_offset;
    }
    
    // Set 64 bytes at a time
    while len >= 64 {
        _mm_storeu_si128(dest as *mut __m128i, val_128);
        _mm_storeu_si128(dest.add(16) as *mut __m128i, val_128);
        _mm_storeu_si128(dest.add(32) as *mut __m128i, val_128);
        _mm_storeu_si128(dest.add(48) as *mut __m128i, val_128);
        dest = dest.add(64);
        len -= 64;
    }
    
    // Set 16 bytes at a time
    while len >= 16 {
        _mm_storeu_si128(dest as *mut __m128i, val_128);
        dest = dest.add(16);
        len -= 16;
    }
    
    // Set remaining bytes
    if len > 0 {
        core::ptr::write_bytes(dest, val, len);
    }
}

pub unsafe fn memmove_optimized(dest: *mut u8, src: *const u8, len: usize) {
    if dest as usize == src as usize || len == 0 {
        return;
    }
    
    // Check for overlap
    if (dest as usize) < (src as usize) {
        // Forward copy (no overlap or dest before src)
        memcpy_optimized(dest, src, len);
    } else if (dest as usize) > (src as usize + len) {
        // No overlap, dest after src
        memcpy_optimized(dest, src, len);
    } else {
        // Overlapping, copy backwards
        let cpu_info = crate::cpu::get_info();
        
        if len < 32 {
            // Small copies - use simple byte copy backwards
            for i in (0..len).rev() {
                *dest.add(i) = *src.add(i);
            }
        } else if cpu_info.has_sse2() {
            memmove_sse2_backward(dest, src, len);
        } else {
            // Fallback to byte-by-byte backward copy
            for i in (0..len).rev() {
                *dest.add(i) = *src.add(i);
            }
        }
    }
}

unsafe fn memmove_sse2_backward(dest: *mut u8, src: *const u8, mut len: usize) {
    let mut dest_end = dest.add(len);
    let mut src_end = src.add(len);
    
    // Copy 16 bytes at a time backwards
    while len >= 16 {
        dest_end = dest_end.sub(16);
        src_end = src_end.sub(16);
        let data = _mm_loadu_si128(src_end as *const __m128i);
        _mm_storeu_si128(dest_end as *mut __m128i, data);
        len -= 16;
    }
    
    // Copy remaining bytes backwards
    while len > 0 {
        len -= 1;
        *dest.add(len) = *src.add(len);
    }
}

pub unsafe fn memcmp_optimized(s1: *const u8, s2: *const u8, len: usize) -> i32 {
    let cpu_info = crate::cpu::get_info();
    
    if len < 16 {
        // Small comparison - use byte-by-byte
        for i in 0..len {
            let b1 = *s1.add(i);
            let b2 = *s2.add(i);
            if b1 != b2 {
                return (b1 as i32) - (b2 as i32);
            }
        }
        return 0;
    }
    
    if cpu_info.has_sse2() {
        memcmp_sse2(s1, s2, len)
    } else {
        // Fallback to byte-by-byte comparison
        for i in 0..len {
            let b1 = *s1.add(i);
            let b2 = *s2.add(i);
            if b1 != b2 {
                return (b1 as i32) - (b2 as i32);
            }
        }
        0
    }
}

unsafe fn memcmp_sse2(mut s1: *const u8, mut s2: *const u8, mut len: usize) -> i32 {
    // Compare 16 bytes at a time
    while len >= 16 {
        let data1 = _mm_loadu_si128(s1 as *const __m128i);
        let data2 = _mm_loadu_si128(s2 as *const __m128i);
        
        let cmp = _mm_cmpeq_epi8(data1, data2);
        let mask = _mm_movemask_epi8(cmp) as u16;
        
        if mask != 0xFFFF {
            // Found difference
            let diff_pos = mask.trailing_ones() as usize;
            let b1 = *s1.add(diff_pos);
            let b2 = *s2.add(diff_pos);
            return (b1 as i32) - (b2 as i32);
        }
        
        s1 = s1.add(16);
        s2 = s2.add(16);
        len -= 16;
    }
    
    // Compare remaining bytes
    for i in 0..len {
        let b1 = *s1.add(i);
        let b2 = *s2.add(i);
        if b1 != b2 {
            return (b1 as i32) - (b2 as i32);
        }
    }
    
    0
}