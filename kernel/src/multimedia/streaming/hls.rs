use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use super::{StreamingProtocol, ProtocolType, StreamDescriptor};
use crate::multimedia::{MediaError, MediaPacket, MediaFormat};

pub struct HlsStreamer {
    playlist_url: String,
    segments: Vec<Segment>,
    current_segment: usize,
    target_duration: u32,
    sequence_number: u64,
    is_live: bool,
}

#[derive(Debug, Clone)]
struct Segment {
    url: String,
    duration: f32,
    sequence: u64,
    discontinuity: bool,
}

impl HlsStreamer {
    pub fn new() -> Self {
        Self {
            playlist_url: String::new(),
            segments: Vec::new(),
            current_segment: 0,
            target_duration: 10,
            sequence_number: 0,
            is_live: false,
        }
    }

    pub fn parse_playlist(&mut self, content: &str) -> Result<(), MediaError> {
        let lines: Vec<&str> = content.lines().collect();
        
        if lines.is_empty() || !lines[0].starts_with("#EXTM3U") {
            return Err(MediaError::InvalidFormat);
        }

        let mut i = 1;
        while i < lines.len() {
            let line = lines[i].trim();
            
            if line.starts_with("#EXT-X-TARGETDURATION:") {
                self.target_duration = line.split(':').nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10);
            } else if line.starts_with("#EXT-X-MEDIA-SEQUENCE:") {
                self.sequence_number = line.split(':').nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("#EXTINF:") {
                let duration = line.split(':').nth(1)
                    .and_then(|s| s.split(',').next())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                
                if i + 1 < lines.len() {
                    let url = lines[i + 1].trim();
                    if !url.starts_with("#") {
                        self.segments.push(Segment {
                            url: String::from(url),
                            duration,
                            sequence: self.sequence_number + self.segments.len() as u64,
                            discontinuity: false,
                        });
                        i += 1;
                    }
                }
            } else if line == "#EXT-X-ENDLIST" {
                self.is_live = false;
            }
            
            i += 1;
        }

        Ok(())
    }

    pub fn generate_playlist(&self) -> String {
        let mut playlist = String::from("#EXTM3U\n");
        playlist.push_str("#EXT-X-VERSION:3\n");
        playlist.push_str(&format!("#EXT-X-TARGETDURATION:{}\n", self.target_duration));
        playlist.push_str(&format!("#EXT-X-MEDIA-SEQUENCE:{}\n", self.sequence_number));
        
        for segment in &self.segments {
            if segment.discontinuity {
                playlist.push_str("#EXT-X-DISCONTINUITY\n");
            }
            playlist.push_str(&format!("#EXTINF:{:.3},\n", segment.duration));
            playlist.push_str(&format!("{}\n", segment.url));
        }
        
        if !self.is_live {
            playlist.push_str("#EXT-X-ENDLIST\n");
        }
        
        playlist
    }
}

impl StreamingProtocol for HlsStreamer {
    fn name(&self) -> &str {
        "HLS"
    }

    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::HLS
    }

    fn connect(&mut self, url: &str) -> Result<(), MediaError> {
        self.playlist_url = String::from(url);
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), MediaError> {
        self.segments.clear();
        self.current_segment = 0;
        Ok(())
    }

    fn send_packet(&mut self, _packet: &MediaPacket) -> Result<(), MediaError> {
        // HLS is primarily for playback, not sending
        Err(MediaError::NotSupported)
    }

    fn receive_packet(&mut self) -> Result<MediaPacket, MediaError> {
        if self.current_segment >= self.segments.len() {
            return Err(MediaError::StreamingError);
        }
        
        // Would fetch and decode segment here
        self.current_segment += 1;
        
        Ok(MediaPacket {
            stream_index: 0,
            pts: Some(0),
            dts: Some(0),
            duration: Some(0),
            data: Vec::new(),
            flags: crate::multimedia::PacketFlags::empty(),
        })
    }

    fn get_streams(&self) -> Vec<StreamDescriptor> {
        vec![StreamDescriptor {
            stream_id: 0,
            format: MediaFormat {
                media_type: crate::multimedia::MediaType::Video,
                codec: String::from("h264"),
                bitrate: Some(2_000_000),
                sample_rate: None,
                channels: None,
                width: Some(1920),
                height: Some(1080),
                framerate: Some(30.0),
                pixel_format: None,
                audio_format: None,
                extra_data: Vec::new(),
            },
            bandwidth: Some(2_500_000),
            language: None,
            name: Some(String::from("Main")),
        }]
    }

    fn set_bitrate(&mut self, _bitrate: u32) -> Result<(), MediaError> {
        // Would switch to different quality playlist
        Ok(())
    }
}

pub struct MasterPlaylist {
    variants: Vec<Variant>,
    audio_groups: BTreeMap<String, Vec<AudioTrack>>,
    subtitle_groups: BTreeMap<String, Vec<SubtitleTrack>>,
}

#[derive(Debug, Clone)]
struct Variant {
    bandwidth: u32,
    resolution: Option<(u32, u32)>,
    codecs: String,
    url: String,
    audio_group: Option<String>,
    subtitle_group: Option<String>,
}

#[derive(Debug, Clone)]
struct AudioTrack {
    name: String,
    language: String,
    url: String,
    default: bool,
}

#[derive(Debug, Clone)]
struct SubtitleTrack {
    name: String,
    language: String,
    url: String,
    forced: bool,
}

impl MasterPlaylist {
    pub fn new() -> Self {
        Self {
            variants: Vec::new(),
            audio_groups: BTreeMap::new(),
            subtitle_groups: BTreeMap::new(),
        }
    }

    pub fn add_variant(&mut self, bandwidth: u32, resolution: Option<(u32, u32)>, url: &str) {
        self.variants.push(Variant {
            bandwidth,
            resolution,
            codecs: String::from("avc1.42E01E,mp4a.40.2"),
            url: String::from(url),
            audio_group: None,
            subtitle_group: None,
        });
    }

    pub fn generate(&self) -> String {
        let mut playlist = String::from("#EXTM3U\n");
        
        for variant in &self.variants {
            let mut line = format!("#EXT-X-STREAM-INF:BANDWIDTH={}", variant.bandwidth);
            
            if let Some((width, height)) = variant.resolution {
                line.push_str(&format!(",RESOLUTION={}x{}", width, height));
            }
            
            line.push_str(&format!(",CODECS=\"{}\"", variant.codecs));
            
            playlist.push_str(&format!("{}\n{}\n", line, variant.url));
        }
        
        playlist
    }
}