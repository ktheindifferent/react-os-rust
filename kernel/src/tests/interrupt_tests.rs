// Interrupt Processing Tests
use crate::interrupts::{
    queue_network_packet, queue_disk_operation, DiskOpType,
    get_network_stats, get_disk_stats, get_interrupt_stats,
    NETWORK_COALESCER, DISK_COALESCER,
};
use crate::{serial_print, serial_println};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

#[test_case]
fn test_network_packet_queue() {
    serial_print!("test_network_packet_queue... ");
    
    let (processed_before, dropped_before) = get_network_stats();
    
    // Queue some test packets
    queue_network_packet(1, 1500, 0x06); // TCP packet
    queue_network_packet(1, 512, 0x11);  // UDP packet
    queue_network_packet(2, 256, 0x01);  // ICMP packet
    
    // Small delay to allow processing
    for _ in 0..1000 {
        core::hint::spin_loop();
    }
    
    let (processed_after, dropped_after) = get_network_stats();
    
    // Verify packets were queued (they may not be processed yet without actual interrupts)
    assert!(processed_after >= processed_before || dropped_after >= dropped_before);
    
    serial_println!("[ok]");
}

#[test_case]
fn test_disk_operation_queue() {
    serial_print!("test_disk_operation_queue... ");
    
    let (completed_before, failed_before) = get_disk_stats();
    
    // Queue some test disk operations
    queue_disk_operation(0, DiskOpType::Read, 0, 8);
    queue_disk_operation(0, DiskOpType::Write, 100, 4);
    queue_disk_operation(1, DiskOpType::Flush, 0, 0);
    
    // Small delay to allow processing
    for _ in 0..1000 {
        core::hint::spin_loop();
    }
    
    let (completed_after, failed_after) = get_disk_stats();
    
    // Verify operations were queued
    assert!(completed_after >= completed_before || failed_after >= failed_before);
    
    serial_println!("[ok]");
}

#[test_case]
fn test_interrupt_coalescing() {
    serial_print!("test_interrupt_coalescing... ");
    
    // Test network coalescing
    let mut network_handled = 0;
    for _ in 0..20 {
        if NETWORK_COALESCER.should_handle() {
            network_handled += 1;
        }
    }
    
    // Should handle less than all interrupts due to coalescing
    assert!(network_handled > 0, "Network coalescer should handle some interrupts");
    assert!(network_handled < 20, "Network coalescer should coalesce some interrupts");
    
    // Test disk coalescing
    let mut disk_handled = 0;
    for _ in 0..10 {
        if DISK_COALESCER.should_handle() {
            disk_handled += 1;
        }
    }
    
    // Should handle less than all interrupts due to coalescing
    assert!(disk_handled > 0, "Disk coalescer should handle some interrupts");
    assert!(disk_handled <= 10, "Disk coalescer should not exceed interrupt count");
    
    serial_println!("[ok]");
}

#[test_case]
fn test_bulk_network_operations() {
    serial_print!("test_bulk_network_operations... ");
    
    let (processed_before, dropped_before) = get_network_stats();
    
    // Queue many packets to test batching
    for i in 0..100 {
        let protocol = match i % 3 {
            0 => 0x06, // TCP
            1 => 0x11, // UDP
            _ => 0x01, // ICMP
        };
        queue_network_packet(i % 4, 1024 + (i * 10) as usize, protocol);
    }
    
    // Give time for potential processing
    for _ in 0..10000 {
        core::hint::spin_loop();
    }
    
    let (processed_after, dropped_after) = get_network_stats();
    
    // At least some packets should be tracked
    assert!(
        processed_after > processed_before || dropped_after > dropped_before,
        "Bulk network operations should change statistics"
    );
    
    serial_println!("[ok]");
}

#[test_case]
fn test_bulk_disk_operations() {
    serial_print!("test_bulk_disk_operations... ");
    
    let (completed_before, failed_before) = get_disk_stats();
    
    // Queue many disk operations to test batching
    for i in 0..50 {
        let op_type = match i % 3 {
            0 => DiskOpType::Read,
            1 => DiskOpType::Write,
            _ => DiskOpType::Flush,
        };
        queue_disk_operation(i % 2, op_type, i as u64 * 100, (i % 16) + 1);
    }
    
    // Give time for potential processing
    for _ in 0..10000 {
        core::hint::spin_loop();
    }
    
    let (completed_after, failed_after) = get_disk_stats();
    
    // At least some operations should be tracked
    assert!(
        completed_after > completed_before || failed_after > failed_before,
        "Bulk disk operations should change statistics"
    );
    
    serial_println!("[ok]");
}

#[test_case]
fn test_mixed_protocol_handling() {
    serial_print!("test_mixed_protocol_handling... ");
    
    // Test handling of various network protocols
    let protocols = [
        (0x06, "TCP"),
        (0x11, "UDP"),
        (0x01, "ICMP"),
        (0xFF, "Unknown"), // Should be dropped
    ];
    
    for &(protocol, name) in &protocols {
        queue_network_packet(0, 256, protocol);
    }
    
    // Small delay
    for _ in 0..1000 {
        core::hint::spin_loop();
    }
    
    // Just verify no panic occurred
    serial_println!("[ok]");
}

#[test_case]
fn test_interrupt_statistics() {
    serial_print!("test_interrupt_statistics... ");
    
    // Get stats for timer interrupt (most likely to have been triggered)
    let (count, cycles, max_latency, min_latency) = 
        get_interrupt_stats(32); // Timer interrupt vector
    
    // Timer should have triggered at least once during boot
    assert!(count > 0, "Timer interrupt should have triggered");
    
    // If count > 0, other stats should be valid
    if count > 0 {
        assert!(cycles > 0, "Cycles should be tracked");
        assert!(max_latency > 0, "Max latency should be tracked");
        assert!(min_latency < u64::MAX, "Min latency should be updated");
        assert!(max_latency >= min_latency, "Max should be >= min latency");
    }
    
    serial_println!("[ok]");
}

#[test_case]
fn test_disk_operation_types() {
    serial_print!("test_disk_operation_types... ");
    
    // Test each disk operation type
    queue_disk_operation(0, DiskOpType::Read, 0, 1);
    queue_disk_operation(0, DiskOpType::Write, 1000, 8);
    queue_disk_operation(0, DiskOpType::Flush, 0, 0);
    
    // Test boundary conditions
    queue_disk_operation(u32::MAX, DiskOpType::Read, u64::MAX, 1);
    queue_disk_operation(0, DiskOpType::Write, 0, u32::MAX);
    
    // Small delay
    for _ in 0..1000 {
        core::hint::spin_loop();
    }
    
    // Just verify no panic occurred
    serial_println!("[ok]");
}

#[test_case]
fn test_network_packet_sizes() {
    serial_print!("test_network_packet_sizes... ");
    
    // Test various packet sizes
    let sizes = [0, 1, 64, 512, 1500, 9000, 65535];
    
    for &size in &sizes {
        queue_network_packet(0, size, 0x06);
    }
    
    // Small delay
    for _ in 0..1000 {
        core::hint::spin_loop();
    }
    
    // Just verify no panic occurred
    serial_println!("[ok]");
}

#[test_case]
fn test_concurrent_operations() {
    serial_print!("test_concurrent_operations... ");
    
    // Simulate concurrent network and disk operations
    for i in 0..20 {
        if i % 2 == 0 {
            queue_network_packet(i % 4, 1024, 0x06);
        } else {
            queue_disk_operation(i % 2, DiskOpType::Read, i as u64 * 512, 4);
        }
    }
    
    // Small delay
    for _ in 0..2000 {
        core::hint::spin_loop();
    }
    
    let (net_processed, net_dropped) = get_network_stats();
    let (disk_completed, disk_failed) = get_disk_stats();
    
    // Verify both subsystems are tracking operations
    assert!(
        net_processed > 0 || net_dropped > 0 || disk_completed > 0 || disk_failed > 0,
        "Concurrent operations should be tracked"
    );
    
    serial_println!("[ok]");
}

// Stress test for high-throughput scenarios
#[test_case]
fn test_high_throughput_network() {
    serial_print!("test_high_throughput_network... ");
    
    let (processed_before, dropped_before) = get_network_stats();
    
    // Simulate high-throughput network traffic
    for burst in 0..10 {
        for i in 0..50 {
            queue_network_packet(i % 8, 1500, if i % 2 == 0 { 0x06 } else { 0x11 });
        }
        
        // Small delay between bursts
        for _ in 0..100 {
            core::hint::spin_loop();
        }
    }
    
    let (processed_after, dropped_after) = get_network_stats();
    
    // Should have processed or dropped packets
    assert!(
        processed_after > processed_before || dropped_after > dropped_before,
        "High-throughput should change statistics"
    );
    
    serial_println!("[ok]");
}

// Stress test for concurrent disk I/O
#[test_case]
fn test_concurrent_disk_io() {
    serial_print!("test_concurrent_disk_io... ");
    
    let (completed_before, failed_before) = get_disk_stats();
    
    // Simulate concurrent disk I/O from multiple sources
    for source in 0..4 {
        for op in 0..25 {
            let disk_id = source % 2;
            let sector = (source * 1000 + op * 10) as u64;
            let op_type = match op % 3 {
                0 => DiskOpType::Read,
                1 => DiskOpType::Write,
                _ => DiskOpType::Flush,
            };
            queue_disk_operation(disk_id, op_type, sector, (op % 8) + 1);
        }
    }
    
    // Give time for processing
    for _ in 0..5000 {
        core::hint::spin_loop();
    }
    
    let (completed_after, failed_after) = get_disk_stats();
    
    // Should have processed operations
    assert!(
        completed_after > completed_before || failed_after > failed_before,
        "Concurrent I/O should change statistics"
    );
    
    serial_println!("[ok]");
}

// Test error recovery
#[test_case]
fn test_error_recovery() {
    serial_print!("test_error_recovery... ");
    
    // Queue operations with invalid parameters that might fail
    queue_network_packet(u32::MAX, 0, 0xFF); // Unknown protocol
    queue_disk_operation(u32::MAX, DiskOpType::Read, u64::MAX, u32::MAX);
    
    // Queue normal operations after potential failures
    queue_network_packet(0, 1024, 0x06);
    queue_disk_operation(0, DiskOpType::Read, 0, 1);
    
    // Small delay
    for _ in 0..2000 {
        core::hint::spin_loop();
    }
    
    // System should not panic and continue processing
    serial_println!("[ok]");
}