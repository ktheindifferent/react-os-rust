pub mod hls;
pub mod dash;
pub mod rtsp;
pub mod webrtc;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use super::{MediaError, MediaPacket, MediaFormat};

pub trait StreamingProtocol: Send + Sync {
    fn name(&self) -> &str;
    fn protocol_type(&self) -> ProtocolType;
    fn connect(&mut self, url: &str) -> Result<(), MediaError>;
    fn disconnect(&mut self) -> Result<(), MediaError>;
    fn send_packet(&mut self, packet: &MediaPacket) -> Result<(), MediaError>;
    fn receive_packet(&mut self) -> Result<MediaPacket, MediaError>;
    fn get_streams(&self) -> Vec<StreamDescriptor>;
    fn set_bitrate(&mut self, bitrate: u32) -> Result<(), MediaError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    HLS,
    DASH,
    RTSP,
    WebRTC,
    RTMP,
    SRT,
}

#[derive(Debug, Clone)]
pub struct StreamDescriptor {
    pub stream_id: usize,
    pub format: MediaFormat,
    pub bandwidth: Option<u32>,
    pub language: Option<String>,
    pub name: Option<String>,
}

pub struct AdaptiveBitrateManager {
    current_bitrate: u32,
    available_bitrates: Vec<u32>,
    network_bandwidth: u32,
    buffer_level: f32,
}

impl AdaptiveBitrateManager {
    pub fn new() -> Self {
        Self {
            current_bitrate: 1_000_000,
            available_bitrates: vec![500_000, 1_000_000, 2_000_000, 4_000_000],
            network_bandwidth: 10_000_000,
            buffer_level: 0.0,
        }
    }

    pub fn update_network_bandwidth(&mut self, bandwidth: u32) {
        self.network_bandwidth = bandwidth;
        self.adapt_bitrate();
    }

    pub fn update_buffer_level(&mut self, level: f32) {
        self.buffer_level = level;
        self.adapt_bitrate();
    }

    fn adapt_bitrate(&mut self) {
        let target_bitrate = (self.network_bandwidth as f32 * 0.8) as u32;
        
        if self.buffer_level < 0.3 {
            // Low buffer, switch to lower quality
            for &bitrate in self.available_bitrates.iter().rev() {
                if bitrate < self.current_bitrate && bitrate <= target_bitrate {
                    self.current_bitrate = bitrate;
                    break;
                }
            }
        } else if self.buffer_level > 0.7 {
            // High buffer, can try higher quality
            for &bitrate in &self.available_bitrates {
                if bitrate > self.current_bitrate && bitrate <= target_bitrate {
                    self.current_bitrate = bitrate;
                    break;
                }
            }
        }
    }

    pub fn get_current_bitrate(&self) -> u32 {
        self.current_bitrate
    }
}

pub struct StreamingSession {
    session_id: String,
    protocol: ProtocolType,
    url: String,
    streams: Vec<StreamDescriptor>,
    state: SessionState,
    stats: SessionStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Connecting,
    Connected,
    Streaming,
    Paused,
    Disconnected,
    Error,
}

#[derive(Debug, Clone)]
pub struct SessionStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_lost: u64,
    pub jitter: f32,
    pub latency: f32,
    pub bitrate: u32,
}

impl StreamingSession {
    pub fn new(protocol: ProtocolType, url: &str) -> Self {
        Self {
            session_id: Self::generate_session_id(),
            protocol,
            url: String::from(url),
            streams: Vec::new(),
            state: SessionState::Idle,
            stats: SessionStats {
                bytes_sent: 0,
                bytes_received: 0,
                packets_sent: 0,
                packets_received: 0,
                packets_lost: 0,
                jitter: 0.0,
                latency: 0.0,
                bitrate: 0,
            },
        }
    }

    fn generate_session_id() -> String {
        // Simple session ID generation
        String::from("session_12345")
    }

    pub fn connect(&mut self) -> Result<(), MediaError> {
        if self.state != SessionState::Idle {
            return Err(MediaError::InvalidState);
        }
        
        self.state = SessionState::Connecting;
        // Connection logic would go here
        self.state = SessionState::Connected;
        
        Ok(())
    }

    pub fn start_streaming(&mut self) -> Result<(), MediaError> {
        if self.state != SessionState::Connected {
            return Err(MediaError::InvalidState);
        }
        
        self.state = SessionState::Streaming;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), MediaError> {
        if self.state != SessionState::Streaming {
            return Err(MediaError::InvalidState);
        }
        
        self.state = SessionState::Paused;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), MediaError> {
        if self.state != SessionState::Paused {
            return Err(MediaError::InvalidState);
        }
        
        self.state = SessionState::Streaming;
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), MediaError> {
        self.state = SessionState::Disconnected;
        Ok(())
    }

    pub fn get_stats(&self) -> SessionStats {
        self.stats.clone()
    }

    pub fn update_stats(&mut self, stats: SessionStats) {
        self.stats = stats;
    }
}