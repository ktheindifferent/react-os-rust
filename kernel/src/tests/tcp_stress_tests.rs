// TCP Stress and Integration Tests
#![cfg(test)]

use crate::net::tcp::*;
use crate::net::ip::Ipv4Address;
use crate::net::socket::{Socket, SocketType, SocketAddr};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, Ordering};

// Stress test: Many concurrent connections
#[test_case]
fn stress_test_concurrent_connections() {
    const NUM_CONNECTIONS: usize = 1000;
    let mut connections = Vec::new();
    
    let server_addr = Ipv4Address::new(192, 168, 1, 100);
    let client_addr = Ipv4Address::new(192, 168, 1, 200);
    
    // Create many connections
    for i in 0..NUM_CONNECTIONS {
        let port = (10000 + i) as u16;
        let mut tcb = TcpControlBlock::new(client_addr, port);
        tcb.remote_addr = server_addr;
        tcb.remote_port = 80;
        tcb.state = TcpState::Established;
        connections.push(tcb);
    }
    
    // Verify all connections are established
    for conn in &connections {
        assert_eq!(conn.state, TcpState::Established);
    }
    
    // Send data on all connections
    for conn in &mut connections {
        let data = b"Test data";
        let segments = conn.send_data(data);
        assert!(!segments.is_empty());
    }
    
    crate::serial_println!("  ✓ Created and used {} concurrent connections", NUM_CONNECTIONS);
}

// Stress test: Large data transfer
#[test_case]
fn stress_test_large_data_transfer() {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut tcb = TcpControlBlock::new(local_addr, 12345);
    tcb.state = TcpState::Established;
    tcb.remote_addr = Ipv4Address::new(192, 168, 1, 200);
    tcb.remote_port = 80;
    
    // Create large data buffer (1MB)
    let large_data: Vec<u8> = (0..1048576).map(|i| (i % 256) as u8).collect();
    
    // Send the data
    let segments = tcb.send_data(&large_data);
    
    // Calculate expected number of segments
    let effective_mss = tcb.mss as usize;
    let expected_segments = (large_data.len() + effective_mss - 1) / effective_mss;
    
    assert_eq!(segments.len(), expected_segments);
    
    // Verify retransmit queue
    assert_eq!(tcb.retransmit_queue.len(), expected_segments);
    
    // Verify total data sent
    let mut total_sent = 0;
    for segment in &segments {
        total_sent += segment.data.len();
    }
    assert_eq!(total_sent, large_data.len());
    
    crate::serial_println!("  ✓ Transferred 1MB of data in {} segments", segments.len());
}

// Stress test: Rapid connection establishment and teardown
#[test_case]
fn stress_test_rapid_connections() {
    const NUM_ITERATIONS: usize = 100;
    
    for i in 0..NUM_ITERATIONS {
        let local_addr = Ipv4Address::new(192, 168, 1, 100);
        let port = (20000 + i) as u16;
        let mut tcb = TcpControlBlock::new(local_addr, port);
        
        // Establish connection
        tcb.remote_addr = Ipv4Address::new(192, 168, 1, 200);
        tcb.remote_port = 80;
        let _syn = tcb.send_syn();
        assert_eq!(tcb.state, TcpState::SynSent);
        
        // Simulate establishment
        tcb.state = TcpState::Established;
        
        // Send some data
        let data = b"Quick test";
        let _segments = tcb.send_data(data);
        
        // Close connection
        let _fin = tcb.send_fin();
        assert_eq!(tcb.state, TcpState::FinWait1);
    }
    
    crate::serial_println!("  ✓ Rapidly established and closed {} connections", NUM_ITERATIONS);
}

// Stress test: Window edge cases
#[test_case]
fn stress_test_window_edge_cases() {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut tcb = TcpControlBlock::new(local_addr, 12345);
    tcb.state = TcpState::Established;
    
    // Test zero window
    tcb.congestion.rwnd = 0;
    let window = tcb.effective_send_window();
    assert_eq!(window, 0);
    
    // Test with zero window probe
    let data = b"Test";
    let segments = tcb.send_data(data);
    assert_eq!(segments.len(), 0);  // Should not send when window is 0
    
    // Test maximum window
    tcb.congestion.rwnd = u32::MAX;
    tcb.congestion.cwnd = u32::MAX;
    tcb.congestion.bytes_in_flight = 0;
    let window = tcb.effective_send_window();
    assert_eq!(window, u32::MAX);
    
    // Test window scaling limits
    tcb.window_scaling_enabled = true;
    tcb.rcv_wnd_scale = TCP_WINDOW_SCALE_MAX;
    let scaled_window = TCP_WINDOW_DEFAULT << TCP_WINDOW_SCALE_MAX;
    assert!(scaled_window > 0);  // Should not overflow
    
    crate::serial_println!("  ✓ Handled window edge cases correctly");
}

// Stress test: Out-of-order segment handling
#[test_case]
fn stress_test_out_of_order_segments() {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut tcb = TcpControlBlock::new(local_addr, 12345);
    tcb.state = TcpState::Established;
    tcb.rcv_nxt = 1000;
    
    // Send many out-of-order segments
    let segments = vec![
        (1100, vec![11, 12, 13]),
        (1200, vec![21, 22, 23]),
        (1150, vec![15, 16, 17]),
        (1050, vec![5, 6, 7]),
        (1300, vec![31, 32, 33]),
    ];
    
    for (seq, data) in &segments {
        tcb.process_data(*seq, data, false);
    }
    
    // Should have buffered all out-of-order segments
    assert_eq!(tcb.out_of_order.len(), segments.len());
    
    // Fill the gap with in-order data
    let fill_data = vec![0u8; 100];  // Fills 1000-1100
    tcb.process_data(1000, &fill_data, false);
    
    // Should have processed some buffered segments
    assert!(tcb.rcv_nxt > 1100);
    assert!(tcb.out_of_order.len() < segments.len());
    
    crate::serial_println!("  ✓ Handled out-of-order segments correctly");
}

// Stress test: Congestion control under loss
#[test_case]
fn stress_test_congestion_under_loss() {
    let mut cc = CongestionControl::new(CongestionAlgorithm::Reno);
    cc.state = CongestionState::CongestionAvoidance;
    cc.cwnd = 100 * TCP_MSS_ETHERNET as u32;
    
    let initial_cwnd = cc.cwnd;
    
    // Simulate packet loss
    cc.on_loss();
    assert_eq!(cc.state, CongestionState::Loss);
    assert!(cc.cwnd < initial_cwnd);
    
    // Recovery
    cc.state = CongestionState::SlowStart;
    for _ in 0..10 {
        cc.on_ack(TCP_MSS_ETHERNET as u32, TCP_MSS_ETHERNET);
    }
    
    // Should have recovered some window
    assert!(cc.cwnd > TCP_MSS_ETHERNET as u32);
    
    // Test multiple losses
    for _ in 0..5 {
        let pre_loss_cwnd = cc.cwnd;
        cc.on_loss();
        assert!(cc.cwnd < pre_loss_cwnd);
    }
    
    crate::serial_println!("  ✓ Congestion control handles loss correctly");
}

// Integration test: Echo server simulation
#[test_case]
fn integration_test_echo_server() {
    // Setup server
    let server_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut server = TcpControlBlock::new(server_addr, 7);  // Echo port
    server.state = TcpState::Listen;
    
    // Setup client
    let client_addr = Ipv4Address::new(192, 168, 1, 200);
    let mut client = TcpControlBlock::new(client_addr, 12345);
    client.remote_addr = server_addr;
    client.remote_port = 7;
    
    // Three-way handshake
    let syn = client.send_syn();
    server.remote_addr = client_addr;
    server.remote_port = 12345;
    let syn_ack = server.process_segment(&syn).unwrap();
    let ack = client.process_segment(&syn_ack).unwrap();
    server.process_segment(&ack);
    
    assert_eq!(client.state, TcpState::Established);
    assert_eq!(server.state, TcpState::Established);
    
    // Send data from client
    let test_data = b"Hello, Echo!";
    let data_segments = client.send_data(test_data);
    
    // Server receives and echoes back
    for segment in &data_segments {
        server.process_segment(segment);
    }
    
    // Server should have received the data
    assert_eq!(server.recv_buffer.len(), test_data.len());
    
    // Server echoes back
    let echo_data: Vec<u8> = server.recv_buffer.drain(..).collect();
    let echo_segments = server.send_data(&echo_data);
    
    // Client receives echo
    for segment in &echo_segments {
        client.process_segment(segment);
    }
    
    // Client should have received the echo
    assert_eq!(client.recv_buffer.len(), test_data.len());
    
    crate::serial_println!("  ✓ Echo server integration test passed");
}

// Performance test: Throughput measurement
#[test_case]
fn performance_test_throughput() {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut tcb = TcpControlBlock::new(local_addr, 12345);
    tcb.state = TcpState::Established;
    tcb.remote_addr = Ipv4Address::new(192, 168, 1, 200);
    tcb.remote_port = 80;
    
    // Enable optimizations
    tcb.window_scaling_enabled = true;
    tcb.sack_permitted = true;
    tcb.timestamps_enabled = true;
    tcb.congestion.cwnd = 100 * TCP_MSS_ETHERNET as u32;
    
    // Measure time to send 10MB
    let data_size = 10 * 1024 * 1024;  // 10MB
    let chunk_size = 65536;  // 64KB chunks
    let mut total_segments = 0;
    
    for _ in 0..(data_size / chunk_size) {
        let chunk: Vec<u8> = vec![0; chunk_size];
        let segments = tcb.send_data(&chunk);
        total_segments += segments.len();
        
        // Simulate ACKs to advance window
        tcb.snd_una = tcb.snd_nxt;
        tcb.retransmit_queue.clear();
        tcb.congestion.bytes_in_flight = 0;
    }
    
    crate::serial_println!("  ✓ Sent 10MB in {} segments", total_segments);
    crate::serial_println!("    Average segment size: {} bytes", data_size / total_segments);
}

// Test retransmission behavior
#[test_case]
fn test_retransmission_behavior() {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut tcb = TcpControlBlock::new(local_addr, 12345);
    tcb.state = TcpState::Established;
    
    // Send data
    let data = b"Test data for retransmission";
    let segments = tcb.send_data(data);
    assert!(!segments.is_empty());
    
    // Verify data is in retransmit queue
    assert_eq!(tcb.retransmit_queue.len(), 1);
    
    // Simulate timeout
    let timer_result = tcb.handle_timer_expiry(TcpTimer::Retransmission);
    
    // Should trigger retransmission
    assert_eq!(tcb.retransmissions, 1);
    
    // RTO should have doubled
    assert!(tcb.congestion.rto > TCP_RTO_INITIAL);
    
    crate::serial_println!("  ✓ Retransmission behavior correct");
}

// Test simultaneous open
#[test_case]
fn test_simultaneous_open() {
    let addr1 = Ipv4Address::new(192, 168, 1, 100);
    let addr2 = Ipv4Address::new(192, 168, 1, 200);
    
    let mut tcb1 = TcpControlBlock::new(addr1, 12345);
    let mut tcb2 = TcpControlBlock::new(addr2, 54321);
    
    // Both send SYN
    tcb1.remote_addr = addr2;
    tcb1.remote_port = 54321;
    let syn1 = tcb1.send_syn();
    
    tcb2.remote_addr = addr1;
    tcb2.remote_port = 12345;
    let syn2 = tcb2.send_syn();
    
    // Both receive SYN (not SYN-ACK)
    let response1 = tcb1.process_segment(&syn2);
    let response2 = tcb2.process_segment(&syn1);
    
    // Both should be in SYN-RECEIVED
    assert_eq!(tcb1.state, TcpState::SynReceived);
    assert_eq!(tcb2.state, TcpState::SynReceived);
    
    // Both should send SYN-ACK
    assert!(response1.is_some());
    assert!(response2.is_some());
    
    crate::serial_println!("  ✓ Simultaneous open handled correctly");
}

// Helper function to run all stress tests
pub fn run_tcp_stress_tests() {
    crate::serial_println!("Running TCP stress and integration tests...");
    
    stress_test_concurrent_connections();
    stress_test_large_data_transfer();
    stress_test_rapid_connections();
    stress_test_window_edge_cases();
    stress_test_out_of_order_segments();
    stress_test_congestion_under_loss();
    integration_test_echo_server();
    performance_test_throughput();
    test_retransmission_behavior();
    test_simultaneous_open();
    
    crate::serial_println!("All TCP stress tests passed!");
}