// TCP Implementation Tests
#![cfg(test)]

use crate::net::tcp::*;
use crate::net::ip::Ipv4Address;
use crate::net::socket::{Socket, SocketType, SocketAddr, SocketOption};
use alloc::vec::Vec;

// Test TCP header creation and parsing
#[test_case]
fn test_tcp_header_creation() {
    let header = TcpHeader::new(
        1234,  // src_port
        5678,  // dst_port
        0x12345678,  // seq
        0x87654321,  // ack
        TCP_SYN | TCP_ACK,  // flags
        8192,  // window
    );
    
    assert_eq!(header.src_port(), 1234);
    assert_eq!(header.dst_port(), 5678);
    assert_eq!(header.seq_num(), 0x12345678);
    assert_eq!(header.ack_num(), 0x87654321);
    assert_eq!(header.window(), 8192);
    assert!(header.has_flag(TCP_SYN));
    assert!(header.has_flag(TCP_ACK));
    assert!(!header.has_flag(TCP_FIN));
}

// Test TCP segment creation and parsing
#[test_case]
fn test_tcp_segment_parsing() {
    let data = vec![1, 2, 3, 4, 5];
    let header = TcpHeader::new(80, 1234, 1000, 2000, TCP_ACK, 4096);
    let segment = TcpSegment::new(header, data.clone());
    
    let bytes = segment.to_bytes();
    let parsed = TcpSegment::from_bytes(&bytes).expect("Failed to parse segment");
    
    assert_eq!(parsed.header.src_port(), 80);
    assert_eq!(parsed.header.dst_port(), 1234);
    assert_eq!(parsed.header.seq_num(), 1000);
    assert_eq!(parsed.header.ack_num(), 2000);
    assert_eq!(parsed.data, data);
}

// Test TCP state machine - three-way handshake
#[test_case]
fn test_tcp_three_way_handshake() {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let remote_addr = Ipv4Address::new(192, 168, 1, 200);
    
    // Client side
    let mut client = TcpControlBlock::new(local_addr, 12345);
    client.remote_addr = remote_addr;
    client.remote_port = 80;
    
    // Server side
    let mut server = TcpControlBlock::new(local_addr, 80);
    server.state = TcpState::Listen;
    
    // Client sends SYN
    assert_eq!(client.state, TcpState::Closed);
    let syn = client.send_syn();
    assert_eq!(client.state, TcpState::SynSent);
    assert!(syn.header.has_flag(TCP_SYN));
    assert!(!syn.header.has_flag(TCP_ACK));
    
    // Server receives SYN and sends SYN-ACK
    server.remote_addr = remote_addr;
    server.remote_port = 12345;
    let syn_ack_opt = server.process_segment(&syn);
    assert!(syn_ack_opt.is_some());
    let syn_ack = syn_ack_opt.unwrap();
    assert_eq!(server.state, TcpState::SynReceived);
    assert!(syn_ack.header.has_flag(TCP_SYN));
    assert!(syn_ack.header.has_flag(TCP_ACK));
    
    // Client receives SYN-ACK and sends ACK
    let ack_opt = client.process_segment(&syn_ack);
    assert!(ack_opt.is_some());
    let ack = ack_opt.unwrap();
    assert_eq!(client.state, TcpState::Established);
    assert!(!ack.header.has_flag(TCP_SYN));
    assert!(ack.header.has_flag(TCP_ACK));
    
    // Server receives ACK
    server.process_segment(&ack);
    assert_eq!(server.state, TcpState::Established);
}

// Test TCP data transfer
#[test_case]
fn test_tcp_data_transfer() {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut tcb = TcpControlBlock::new(local_addr, 12345);
    tcb.state = TcpState::Established;
    tcb.remote_addr = Ipv4Address::new(192, 168, 1, 200);
    tcb.remote_port = 80;
    
    // Send data
    let data = b"Hello, TCP!";
    let segments = tcb.send_data(data);
    assert!(!segments.is_empty());
    
    let first_segment = &segments[0];
    assert!(first_segment.header.has_flag(TCP_ACK));
    assert!(first_segment.header.has_flag(TCP_PSH));
    assert_eq!(first_segment.data, data.to_vec());
    
    // Check retransmit queue
    assert!(!tcb.retransmit_queue.is_empty());
    assert_eq!(tcb.retransmit_queue.len(), 1);
}

// Test TCP congestion control - slow start
#[test_case]
fn test_tcp_congestion_slow_start() {
    let mut cc = CongestionControl::new(CongestionAlgorithm::Reno);
    assert_eq!(cc.state, CongestionState::SlowStart);
    
    let initial_cwnd = cc.cwnd;
    let mss = TCP_MSS_ETHERNET;
    
    // Simulate ACKs during slow start
    for _ in 0..5 {
        cc.on_ack(mss as u32, mss);
        assert!(cc.cwnd > initial_cwnd);
    }
    
    // Should grow exponentially in slow start
    assert!(cc.cwnd >= initial_cwnd + 5 * mss as u32);
}

// Test TCP congestion control - congestion avoidance
#[test_case]
fn test_tcp_congestion_avoidance() {
    let mut cc = CongestionControl::new(CongestionAlgorithm::Reno);
    cc.state = CongestionState::CongestionAvoidance;
    cc.cwnd = 10 * TCP_MSS_ETHERNET as u32;
    
    let initial_cwnd = cc.cwnd;
    let mss = TCP_MSS_ETHERNET;
    
    // Simulate ACKs during congestion avoidance
    for _ in 0..10 {
        cc.on_ack(mss as u32, mss);
    }
    
    // Should grow linearly (approximately 1 MSS per RTT)
    assert!(cc.cwnd > initial_cwnd);
    assert!(cc.cwnd < initial_cwnd + 10 * mss as u32);
}

// Test TCP congestion control - fast recovery
#[test_case]
fn test_tcp_fast_recovery() {
    let mut cc = CongestionControl::new(CongestionAlgorithm::Reno);
    cc.state = CongestionState::CongestionAvoidance;
    cc.cwnd = 10 * TCP_MSS_ETHERNET as u32;
    
    let initial_cwnd = cc.cwnd;
    let mss = TCP_MSS_ETHERNET;
    
    // Simulate 3 duplicate ACKs
    assert!(!cc.on_dup_ack(mss));  // 1st dup ACK
    assert!(!cc.on_dup_ack(mss));  // 2nd dup ACK
    assert!(cc.on_dup_ack(mss));   // 3rd dup ACK - triggers fast retransmit
    
    // Should enter fast recovery
    assert_eq!(cc.state, CongestionState::FastRecovery);
    // cwnd should be reduced
    assert!(cc.cwnd < initial_cwnd);
}

// Test TCP window scaling
#[test_case]
fn test_tcp_window_scaling() {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut tcb = TcpControlBlock::new(local_addr, 12345);
    
    tcb.window_scaling_enabled = true;
    tcb.rcv_wnd_scale = 7;  // Scale factor of 7 (multiply by 128)
    tcb.snd_wnd_scale = 7;
    
    // Test window calculation
    let base_window = 1024u32;
    tcb.rcv_wnd = base_window << tcb.rcv_wnd_scale;
    assert_eq!(tcb.rcv_wnd, base_window * 128);
}

// Test TCP options parsing
#[test_case]
fn test_tcp_options_parsing() {
    // MSS option
    let mss_option = vec![
        TCP_OPT_MSS, 4, 0x05, 0xb4,  // MSS = 1460
    ];
    let options = TcpControlBlock::parse_options(&mss_option);
    assert_eq!(options.len(), 1);
    match &options[0] {
        TcpOption::Mss(mss) => assert_eq!(*mss, 1460),
        _ => panic!("Expected MSS option"),
    }
    
    // Window scale option
    let ws_option = vec![
        TCP_OPT_NOP,
        TCP_OPT_WINDOW_SCALE, 3, 7,  // Window scale = 7
    ];
    let options = TcpControlBlock::parse_options(&ws_option);
    assert_eq!(options.len(), 2);  // NOP and WindowScale
    match &options[1] {
        TcpOption::WindowScale(scale) => assert_eq!(*scale, 7),
        _ => panic!("Expected WindowScale option"),
    }
    
    // SACK permitted option
    let sack_option = vec![
        TCP_OPT_SACK_PERMITTED, 2,
    ];
    let options = TcpControlBlock::parse_options(&sack_option);
    assert_eq!(options.len(), 1);
    match &options[0] {
        TcpOption::SackPermitted => {},
        _ => panic!("Expected SackPermitted option"),
    }
}

// Test TCP RTT estimation
#[test_case]
fn test_tcp_rtt_estimation() {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut tcb = TcpControlBlock::new(local_addr, 12345);
    
    // First RTT measurement
    tcb.update_rtt(100);
    assert_eq!(tcb.congestion.srtt, 100);
    assert_eq!(tcb.congestion.rttvar, 50);
    
    // Subsequent measurements should smooth the RTT
    tcb.update_rtt(150);
    assert!(tcb.congestion.srtt > 100 && tcb.congestion.srtt < 150);
    
    tcb.update_rtt(120);
    assert!(tcb.congestion.rto >= TCP_RTO_MIN);
    assert!(tcb.congestion.rto <= TCP_RTO_MAX);
}

// Test TCP out-of-order handling
#[test_case]
fn test_tcp_out_of_order() {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut tcb = TcpControlBlock::new(local_addr, 12345);
    tcb.state = TcpState::Established;
    tcb.rcv_nxt = 1000;
    
    // Receive out-of-order segment
    let data = vec![1, 2, 3];
    tcb.process_data(1005, &data, false);
    
    // Should be buffered
    assert_eq!(tcb.out_of_order.len(), 1);
    assert!(tcb.out_of_order.contains_key(&1005));
    
    // Receive in-order segment that fills the gap
    let fill_data = vec![4, 5, 6, 7, 8];
    tcb.process_data(1000, &fill_data, false);
    
    // Should have processed both segments
    assert_eq!(tcb.rcv_nxt, 1008);
    assert_eq!(tcb.out_of_order.len(), 0);
    assert_eq!(tcb.recv_buffer.len(), 8);
}

// Test TCP connection teardown
#[test_case]
fn test_tcp_connection_teardown() {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut client = TcpControlBlock::new(local_addr, 12345);
    let mut server = TcpControlBlock::new(local_addr, 80);
    
    // Both in established state
    client.state = TcpState::Established;
    server.state = TcpState::Established;
    server.rcv_nxt = 1000;
    client.rcv_nxt = 2000;
    
    // Client initiates close
    let fin = client.send_fin();
    assert_eq!(client.state, TcpState::FinWait1);
    assert!(fin.header.has_flag(TCP_FIN));
    assert!(fin.header.has_flag(TCP_ACK));
    
    // Server receives FIN
    let ack_opt = server.process_segment(&fin);
    assert!(ack_opt.is_some());
    assert_eq!(server.state, TcpState::CloseWait);
    
    // Server sends its FIN
    let server_fin = server.send_fin();
    assert_eq!(server.state, TcpState::LastAck);
    assert!(server_fin.header.has_flag(TCP_FIN));
}

// Test TCP timer management
#[test_case]
fn test_tcp_timers() {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut tcb = TcpControlBlock::new(local_addr, 12345);
    
    // Set various timers
    tcb.set_timer(TcpTimer::Retransmission, 1000);
    tcb.set_timer(TcpTimer::KeepAlive, 7200000);
    tcb.set_timer(TcpTimer::DelayedAck, 200);
    
    assert_eq!(tcb.timers.len(), 3);
    
    // Cancel a timer
    tcb.cancel_timer(TcpTimer::DelayedAck);
    assert_eq!(tcb.timers.len(), 2);
    
    // Check for expired timers (none should expire immediately)
    let expired = tcb.check_timers();
    assert_eq!(expired.len(), 0);
}

// Test CUBIC congestion control
#[test_case]
fn test_cubic_congestion_control() {
    let mut cc = CongestionControl::new(CongestionAlgorithm::Cubic);
    cc.state = CongestionState::CongestionAvoidance;
    cc.cwnd = 10 * TCP_MSS_ETHERNET as u32;
    cc.cubic_last_max = cc.cwnd;
    cc.cubic_epoch_start = TcpControlBlock::get_timestamp();
    
    let initial_cwnd = cc.cwnd;
    let mss = TCP_MSS_ETHERNET;
    
    // Simulate ACKs over time
    for _ in 0..10 {
        cc.on_ack(mss as u32, mss);
    }
    
    // CUBIC should have different growth pattern than Reno
    assert!(cc.cwnd > initial_cwnd);
}

// Test socket API integration
#[test_case]
fn test_socket_api_tcp() {
    let mut socket = Socket::new(SocketType::Stream);
    
    // Bind socket
    let local_addr = SocketAddr::new(
        Ipv4Address::new(192, 168, 1, 100),
        8080
    );
    assert!(socket.bind(local_addr).is_ok());
    assert_eq!(socket.state, crate::net::socket::SocketState::Bound);
    
    // Listen
    assert!(socket.listen(10).is_ok());
    assert_eq!(socket.state, crate::net::socket::SocketState::Listening);
    
    // Set socket options
    assert!(socket.set_option(SocketOption::NoDelay(true)).is_ok());
    assert!(socket.set_option(SocketOption::KeepAlive(true)).is_ok());
    assert_eq!(socket.options.no_delay, true);
    assert_eq!(socket.options.keep_alive, true);
}

// Test effective send window calculation
#[test_case]
fn test_effective_send_window() {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut tcb = TcpControlBlock::new(local_addr, 12345);
    
    tcb.congestion.cwnd = 10000;
    tcb.congestion.rwnd = 8000;
    tcb.congestion.bytes_in_flight = 2000;
    
    // Effective window should be min(cwnd, rwnd) - bytes_in_flight
    let window = tcb.effective_send_window();
    assert_eq!(window, 6000);  // min(10000, 8000) - 2000
    
    // Test zero window
    tcb.congestion.bytes_in_flight = 8000;
    let window = tcb.effective_send_window();
    assert_eq!(window, 0);
}

// Test keepalive mechanism
#[test_case]
fn test_tcp_keepalive() {
    let local_addr = Ipv4Address::new(192, 168, 1, 100);
    let mut tcb = TcpControlBlock::new(local_addr, 12345);
    
    tcb.state = TcpState::Established;
    tcb.keepalive_enabled = true;
    tcb.start_keepalive();
    
    // Should have keepalive timer set
    assert!(tcb.timers.iter().any(|t| t.timer_type == TcpTimer::KeepAlive));
    
    // Reset on activity
    tcb.reset_keepalive();
    assert_eq!(tcb.keepalive_probes_sent, 0);
}

// Helper function to run all TCP tests
pub fn run_tcp_tests() {
    crate::serial_println!("Running TCP tests...");
    
    test_tcp_header_creation();
    crate::serial_println!("  ✓ TCP header creation");
    
    test_tcp_segment_parsing();
    crate::serial_println!("  ✓ TCP segment parsing");
    
    test_tcp_three_way_handshake();
    crate::serial_println!("  ✓ TCP three-way handshake");
    
    test_tcp_data_transfer();
    crate::serial_println!("  ✓ TCP data transfer");
    
    test_tcp_congestion_slow_start();
    crate::serial_println!("  ✓ TCP congestion slow start");
    
    test_tcp_congestion_avoidance();
    crate::serial_println!("  ✓ TCP congestion avoidance");
    
    test_tcp_fast_recovery();
    crate::serial_println!("  ✓ TCP fast recovery");
    
    test_tcp_window_scaling();
    crate::serial_println!("  ✓ TCP window scaling");
    
    test_tcp_options_parsing();
    crate::serial_println!("  ✓ TCP options parsing");
    
    test_tcp_rtt_estimation();
    crate::serial_println!("  ✓ TCP RTT estimation");
    
    test_tcp_out_of_order();
    crate::serial_println!("  ✓ TCP out-of-order handling");
    
    test_tcp_connection_teardown();
    crate::serial_println!("  ✓ TCP connection teardown");
    
    test_tcp_timers();
    crate::serial_println!("  ✓ TCP timer management");
    
    test_cubic_congestion_control();
    crate::serial_println!("  ✓ CUBIC congestion control");
    
    test_socket_api_tcp();
    crate::serial_println!("  ✓ Socket API integration");
    
    test_effective_send_window();
    crate::serial_println!("  ✓ Effective send window");
    
    test_tcp_keepalive();
    crate::serial_println!("  ✓ TCP keepalive");
    
    crate::serial_println!("All TCP tests passed!");
}