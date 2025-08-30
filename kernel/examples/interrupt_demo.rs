// Interrupt Processing Demo
// This demonstrates the network and disk interrupt processing functionality

use rust_kernel::interrupts::{
    queue_network_packet, queue_disk_operation, DiskOpType,
    get_network_stats, get_disk_stats, print_interrupt_stats,
};

fn main() {
    println!("=== Interrupt Processing Demo ===\n");
    
    println!("1. Simulating Network Traffic:");
    println!("   Queuing 50 network packets...");
    
    for i in 0..50 {
        let protocol = match i % 3 {
            0 => 0x06, // TCP
            1 => 0x11, // UDP
            _ => 0x01, // ICMP
        };
        let interface_id = i % 4;
        let packet_size = 500 + (i * 30) as usize;
        
        queue_network_packet(interface_id, packet_size, protocol);
    }
    
    // Simulate processing delay
    for _ in 0..100000 {
        core::hint::spin_loop();
    }
    
    let (processed, dropped) = get_network_stats();
    println!("   Network Stats:");
    println!("   - Packets Processed: {}", processed);
    println!("   - Packets Dropped: {}", dropped);
    println!();
    
    println!("2. Simulating Disk I/O:");
    println!("   Queuing 30 disk operations...");
    
    for i in 0..30 {
        let op_type = match i % 3 {
            0 => DiskOpType::Read,
            1 => DiskOpType::Write,
            _ => DiskOpType::Flush,
        };
        let disk_id = i % 2;
        let sector = (i as u64) * 100;
        let count = (i % 8) + 1;
        
        queue_disk_operation(disk_id, op_type, sector, count);
    }
    
    // Simulate processing delay
    for _ in 0..100000 {
        core::hint::spin_loop();
    }
    
    let (completed, failed) = get_disk_stats();
    println!("   Disk Stats:");
    println!("   - Operations Completed: {}", completed);
    println!("   - Operations Failed: {}", failed);
    println!();
    
    println!("3. Interrupt Statistics:");
    print_interrupt_stats();
    println!();
    
    println!("4. High-Throughput Test:");
    println!("   Simulating burst of 500 packets...");
    
    for i in 0..500 {
        queue_network_packet(i % 8, 1500, if i % 2 == 0 { 0x06 } else { 0x11 });
    }
    
    // Simulate processing delay
    for _ in 0..500000 {
        core::hint::spin_loop();
    }
    
    let (processed_after, dropped_after) = get_network_stats();
    println!("   After burst:");
    println!("   - Total Processed: {}", processed_after);
    println!("   - Total Dropped: {}", dropped_after);
    println!("   - Burst Processed: {}", processed_after - processed);
    println!("   - Burst Dropped: {}", dropped_after - dropped);
    println!();
    
    println!("5. Concurrent I/O Test:");
    println!("   Interleaving network and disk operations...");
    
    for i in 0..100 {
        if i % 2 == 0 {
            queue_network_packet(i % 4, 1024, 0x06);
        } else {
            queue_disk_operation(i % 2, DiskOpType::Read, i as u64 * 512, 4);
        }
    }
    
    // Simulate processing delay
    for _ in 0..200000 {
        core::hint::spin_loop();
    }
    
    let (net_final, _) = get_network_stats();
    let (disk_final, _) = get_disk_stats();
    
    println!("   Final Stats:");
    println!("   - Network Packets: {}", net_final);
    println!("   - Disk Operations: {}", disk_final);
    println!();
    
    println!("=== Demo Complete ===");
}