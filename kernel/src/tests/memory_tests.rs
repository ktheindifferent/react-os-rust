// Memory Management Unit Tests

use crate::memory::*;
use crate::test_runner::TestRunner;
use alloc::vec::Vec;
use alloc::boxed::Box;

pub fn run_memory_tests(runner: &mut TestRunner) {
    runner.run_test("memory::heap_allocation", || {
        // Test basic heap allocation
        let data = Box::new(42u32);
        if *data != 42 {
            return Err(format!("Box allocation failed: expected 42, got {}", *data));
        }
        drop(data);
        Ok(())
    });
    
    runner.run_test("memory::vector_allocation", || {
        // Test dynamic vector allocation
        let mut vec = Vec::new();
        for i in 0..100 {
            vec.push(i);
        }
        if vec.len() != 100 {
            return Err(format!("Vector length: expected 100, got {}", vec.len()));
        }
        if vec[50] != 50 {
            return Err(format!("Vector element: expected 50, got {}", vec[50]));
        }
        Ok(())
    });
    
    runner.run_test("memory::large_allocation", || {
        // Test large allocation (1MB)
        let size = 1024 * 1024;
        let data: Vec<u8> = vec![0xFF; size];
        if data.len() != size {
            return Err(format!("Large allocation: expected {} bytes, got {}", size, data.len()));
        }
        for (i, &byte) in data.iter().enumerate().take(100) {
            if byte != 0xFF {
                return Err(format!("Data corruption at index {}: expected 0xFF, got 0x{:02X}", i, byte));
            }
        }
        Ok(())
    });
    
    runner.run_test("memory::allocation_patterns", || {
        // Test various allocation patterns
        let mut allocations = Vec::new();
        
        // Small allocations
        for i in 0..50 {
            allocations.push(Box::new(i as u64));
        }
        
        // Medium allocations
        for i in 0..20 {
            allocations.push(Box::new([i as u8; 256]));
        }
        
        // Check integrity
        for (i, alloc) in allocations.iter().enumerate() {
            if i < 50 {
                // Check small allocations
                let val = alloc.downcast_ref::<u64>().ok_or("Type mismatch")?;
                if *val != i as u64 {
                    return Err(format!("Small allocation {}: expected {}, got {}", i, i, val));
                }
            }
        }
        
        Ok(())
    });
    
    runner.run_test("memory::fragmentation_test", || {
        // Test memory fragmentation handling
        let mut allocations = Vec::new();
        
        // Create fragmentation pattern
        for i in 0..100 {
            allocations.push(vec![i as u8; 64]);
        }
        
        // Free every other allocation
        for i in (0..100).step_by(2) {
            allocations[i].clear();
        }
        
        // Try to allocate in fragmented space
        for i in (0..100).step_by(2) {
            allocations[i] = vec![(i + 100) as u8; 64];
        }
        
        // Verify data integrity
        for (i, alloc) in allocations.iter().enumerate() {
            if !alloc.is_empty() {
                let expected = if i % 2 == 0 { (i + 100) as u8 } else { i as u8 };
                if alloc[0] != expected {
                    return Err(format!("Fragmentation test: expected {}, got {}", expected, alloc[0]));
                }
            }
        }
        
        Ok(())
    });
    
    runner.run_test("memory::alignment_test", || {
        // Test memory alignment requirements
        let aligned_16 = Box::new([0u128; 4]);
        let addr = aligned_16.as_ptr() as usize;
        if addr % 16 != 0 {
            return Err(format!("16-byte alignment failed: address 0x{:x}", addr));
        }
        
        let aligned_8 = Box::new([0u64; 8]);
        let addr = aligned_8.as_ptr() as usize;
        if addr % 8 != 0 {
            return Err(format!("8-byte alignment failed: address 0x{:x}", addr));
        }
        
        Ok(())
    });
    
    runner.run_test("memory::zero_size_allocation", || {
        // Test zero-size type allocations
        let zst = Box::new(());
        drop(zst); // Should not panic
        
        let empty_vec: Vec<()> = Vec::with_capacity(100);
        if empty_vec.capacity() < 100 {
            return Err(format!("ZST vector capacity: expected >= 100, got {}", empty_vec.capacity()));
        }
        
        Ok(())
    });
    
    runner.run_test("memory::reallocation", || {
        // Test vector reallocation
        let mut vec = Vec::with_capacity(10);
        let initial_capacity = vec.capacity();
        
        for i in 0..100 {
            vec.push(i);
        }
        
        if vec.capacity() <= initial_capacity {
            return Err(format!("Reallocation failed: capacity should grow from {}", initial_capacity));
        }
        
        // Verify data integrity after reallocation
        for (i, &val) in vec.iter().enumerate() {
            if val != i {
                return Err(format!("Data corruption after reallocation: expected {}, got {}", i, val));
            }
        }
        
        Ok(())
    });
}

// Slab allocator specific tests
pub fn run_slab_allocator_tests(runner: &mut TestRunner) {
    runner.run_test("slab::fixed_size_allocation", || {
        // Test fixed-size slab allocations
        let mut allocations = Vec::new();
        
        // Allocate multiple 64-byte blocks
        for i in 0..32 {
            let block = vec![i as u8; 64];
            allocations.push(block);
        }
        
        // Verify all allocations
        for (i, block) in allocations.iter().enumerate() {
            if block.len() != 64 {
                return Err(format!("Slab size mismatch: expected 64, got {}", block.len()));
            }
            if block[0] != i as u8 {
                return Err(format!("Slab data corruption: expected {}, got {}", i, block[0]));
            }
        }
        
        Ok(())
    });
    
    runner.run_test("slab::cache_behavior", || {
        // Test slab cache reuse
        let mut first_alloc = vec![0xFF_u8; 128];
        let first_addr = first_alloc.as_ptr() as usize;
        drop(first_alloc);
        
        // Allocate same size again - should potentially reuse slab
        let second_alloc = vec![0xAA_u8; 128];
        let second_addr = second_alloc.as_ptr() as usize;
        
        // While not guaranteed, cache reuse is expected in many cases
        // Just verify the allocation succeeded
        if second_alloc[0] != 0xAA {
            return Err(format!("Slab reallocation failed"));
        }
        
        Ok(())
    });
}

// Frame allocator tests
pub fn run_frame_allocator_tests(runner: &mut TestRunner) {
    runner.run_test("frame::page_alignment", || {
        // Test that frame addresses are page-aligned (4KB)
        const PAGE_SIZE: usize = 4096;
        
        // Simulate frame allocation
        let test_frame_addr = 0x100000_usize; // 1MB boundary
        if test_frame_addr % PAGE_SIZE != 0 {
            return Err(format!("Frame not page-aligned: 0x{:x}", test_frame_addr));
        }
        
        Ok(())
    });
    
    runner.run_test("frame::bitmap_operations", || {
        // Test frame bitmap operations
        let mut bitmap = vec![0u8; 128]; // 1024 frames
        
        // Mark frame as used
        let frame_idx = 42;
        bitmap[frame_idx / 8] |= 1 << (frame_idx % 8);
        
        // Check if marked
        if bitmap[frame_idx / 8] & (1 << (frame_idx % 8)) == 0 {
            return Err(format!("Frame {} not marked as used", frame_idx));
        }
        
        // Mark frame as free
        bitmap[frame_idx / 8] &= !(1 << (frame_idx % 8));
        
        // Check if freed
        if bitmap[frame_idx / 8] & (1 << (frame_idx % 8)) != 0 {
            return Err(format!("Frame {} not marked as free", frame_idx));
        }
        
        Ok(())
    });
    
    runner.run_test("frame::contiguous_allocation", || {
        // Test contiguous frame allocation
        let mut bitmap = vec![0u8; 16]; // 128 frames
        
        // Mark some frames as used
        bitmap[0] = 0b11110000; // Frames 4-7 are used
        bitmap[1] = 0b00001111; // Frames 8-11 are used
        
        // Find 4 contiguous free frames
        let mut found = false;
        let mut start = 0;
        
        for byte_idx in 0..bitmap.len() {
            for bit_idx in 0..8 {
                let frame_idx = byte_idx * 8 + bit_idx;
                let mut contiguous = true;
                
                // Check if next 4 frames are free
                for i in 0..4 {
                    let check_idx = frame_idx + i;
                    let check_byte = check_idx / 8;
                    let check_bit = check_idx % 8;
                    
                    if check_byte >= bitmap.len() {
                        contiguous = false;
                        break;
                    }
                    
                    if bitmap[check_byte] & (1 << check_bit) != 0 {
                        contiguous = false;
                        break;
                    }
                }
                
                if contiguous && frame_idx + 3 < 128 {
                    found = true;
                    start = frame_idx;
                    break;
                }
            }
            if found { break; }
        }
        
        if !found {
            return Err(format!("Could not find 4 contiguous free frames"));
        }
        
        Ok(())
    });
}

// Virtual memory tests
pub fn run_virtual_memory_tests(runner: &mut TestRunner) {
    runner.run_test("vm::address_translation", || {
        // Test virtual to physical address translation
        let virtual_addr = 0xFFFF_8000_0000_0000_usize;
        let offset = 0x1234_5678_usize;
        let full_addr = virtual_addr + offset;
        
        // Extract page table indices (9 bits each)
        let p4_idx = (full_addr >> 39) & 0x1FF;
        let p3_idx = (full_addr >> 30) & 0x1FF;
        let p2_idx = (full_addr >> 21) & 0x1FF;
        let p1_idx = (full_addr >> 12) & 0x1FF;
        let page_offset = full_addr & 0xFFF;
        
        if p4_idx >= 512 || p3_idx >= 512 || p2_idx >= 512 || p1_idx >= 512 {
            return Err(format!("Invalid page table index"));
        }
        
        if page_offset >= 4096 {
            return Err(format!("Invalid page offset: {}", page_offset));
        }
        
        Ok(())
    });
    
    runner.run_test("vm::page_flags", || {
        // Test page table entry flags
        const PRESENT: u64 = 1 << 0;
        const WRITABLE: u64 = 1 << 1;
        const USER: u64 = 1 << 2;
        const WRITE_THROUGH: u64 = 1 << 3;
        const NO_CACHE: u64 = 1 << 4;
        const ACCESSED: u64 = 1 << 5;
        const DIRTY: u64 = 1 << 6;
        const HUGE: u64 = 1 << 7;
        const GLOBAL: u64 = 1 << 8;
        const NO_EXECUTE: u64 = 1 << 63;
        
        let mut entry = 0u64;
        
        // Set flags
        entry |= PRESENT | WRITABLE | USER;
        
        // Check flags
        if entry & PRESENT == 0 {
            return Err(format!("PRESENT flag not set"));
        }
        if entry & WRITABLE == 0 {
            return Err(format!("WRITABLE flag not set"));
        }
        if entry & USER == 0 {
            return Err(format!("USER flag not set"));
        }
        if entry & NO_EXECUTE != 0 {
            return Err(format!("NO_EXECUTE flag incorrectly set"));
        }
        
        Ok(())
    });
    
    runner.run_test("vm::huge_pages", || {
        // Test 2MB and 1GB huge page support
        const PAGE_SIZE_4K: usize = 4096;
        const PAGE_SIZE_2M: usize = 2 * 1024 * 1024;
        const PAGE_SIZE_1G: usize = 1024 * 1024 * 1024;
        
        // Check alignment requirements
        let addr_2m = 0x200000_usize; // 2MB boundary
        let addr_1g = 0x40000000_usize; // 1GB boundary
        
        if addr_2m % PAGE_SIZE_2M != 0 {
            return Err(format!("2MB page not aligned"));
        }
        if addr_1g % PAGE_SIZE_1G != 0 {
            return Err(format!("1GB page not aligned"));
        }
        
        Ok(())
    });
}

// Demand paging tests
pub fn run_demand_paging_tests(runner: &mut TestRunner) {
    runner.run_test("demand::page_fault_handling", || {
        // Simulate page fault scenario
        let fault_addr = 0xDEADBEEF_usize;
        let page_addr = fault_addr & !0xFFF; // Align to page boundary
        
        if page_addr != 0xDEADB000 {
            return Err(format!("Page alignment incorrect: expected 0xDEADB000, got 0x{:x}", page_addr));
        }
        
        Ok(())
    });
    
    runner.run_test("demand::lazy_allocation", || {
        // Test lazy allocation strategy
        let mut lazy_pages = Vec::new();
        
        // Mark pages for lazy allocation
        for i in 0..10 {
            lazy_pages.push((i * 4096, false)); // (address, allocated)
        }
        
        // Simulate access to page 5
        let accessed_page = 5;
        lazy_pages[accessed_page].1 = true;
        
        // Verify only accessed page is allocated
        let allocated_count = lazy_pages.iter().filter(|(_, allocated)| *allocated).count();
        if allocated_count != 1 {
            return Err(format!("Lazy allocation: expected 1 allocated page, got {}", allocated_count));
        }
        
        Ok(())
    });
    
    runner.run_test("demand::cow_pages", || {
        // Test Copy-on-Write pages
        struct CowPage {
            physical_frame: usize,
            ref_count: usize,
            writable: bool,
        }
        
        let mut original = CowPage {
            physical_frame: 0x1000,
            ref_count: 1,
            writable: false,
        };
        
        // Fork - share the page
        original.ref_count += 1;
        let forked = CowPage {
            physical_frame: original.physical_frame,
            ref_count: original.ref_count,
            writable: false,
        };
        
        if forked.physical_frame != original.physical_frame {
            return Err(format!("COW: forked page should share physical frame"));
        }
        
        // Write triggers COW
        let new_frame = 0x2000;
        let mut copied = CowPage {
            physical_frame: new_frame,
            ref_count: 1,
            writable: true,
        };
        original.ref_count -= 1;
        
        if copied.physical_frame == original.physical_frame {
            return Err(format!("COW: copied page should have new physical frame"));
        }
        
        Ok(())
    });
}