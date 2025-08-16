use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use super::{MediaError, MediaPacket, MediaFormat, MediaType};
use super::format::{FormatContext, StreamInfo, CodecId};

pub trait ContainerDemuxer: Send + Sync {
    fn probe(&mut self, data: &[u8]) -> bool;
    fn open(&mut self, data: &[u8]) -> Result<FormatContext, MediaError>;
    fn read_packet(&mut self) -> Result<MediaPacket, MediaError>;
    fn seek(&mut self, timestamp: i64) -> Result<(), MediaError>;
    fn get_duration(&self) -> Option<i64>;
    fn get_metadata(&self) -> BTreeMap<String, String>;
}

pub trait ContainerMuxer: Send + Sync {
    fn add_stream(&mut self, format: &MediaFormat) -> Result<usize, MediaError>;
    fn write_header(&mut self) -> Result<Vec<u8>, MediaError>;
    fn write_packet(&mut self, packet: &MediaPacket) -> Result<Vec<u8>, MediaError>;
    fn write_trailer(&mut self) -> Result<Vec<u8>, MediaError>;
    fn set_metadata(&mut self, key: &str, value: &str);
}

pub struct Mp4Demuxer {
    streams: Vec<StreamInfo>,
    current_position: u64,
    duration: Option<i64>,
    metadata: BTreeMap<String, String>,
}

impl Mp4Demuxer {
    pub fn new() -> Self {
        Self {
            streams: Vec::new(),
            current_position: 0,
            duration: None,
            metadata: BTreeMap::new(),
        }
    }

    fn parse_ftyp(&mut self, data: &[u8]) -> Result<(), MediaError> {
        if data.len() < 8 {
            return Err(MediaError::InvalidFormat);
        }
        
        let box_size = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let box_type = &data[4..8];
        
        if box_type != b"ftyp" {
            return Err(MediaError::InvalidFormat);
        }
        
        Ok(())
    }

    fn parse_moov(&mut self, data: &[u8]) -> Result<(), MediaError> {
        // Parse movie header box
        Ok(())
    }
}

impl ContainerDemuxer for Mp4Demuxer {
    fn probe(&mut self, data: &[u8]) -> bool {
        data.len() >= 12 && 
        (&data[4..8] == b"ftyp" || &data[4..8] == b"moov" || &data[4..8] == b"mdat")
    }

    fn open(&mut self, data: &[u8]) -> Result<FormatContext, MediaError> {
        self.parse_ftyp(data)?;
        
        Ok(FormatContext {
            streams: self.streams.clone(),
            duration: self.duration,
            bit_rate: None,
            metadata: self.metadata.clone(),
            format_name: String::from("mp4"),
            format_long_name: String::from("MPEG-4 Part 14"),
        })
    }

    fn read_packet(&mut self) -> Result<MediaPacket, MediaError> {
        Ok(MediaPacket {
            stream_index: 0,
            pts: Some(0),
            dts: Some(0),
            duration: Some(0),
            data: Vec::new(),
            flags: super::PacketFlags::empty(),
        })
    }

    fn seek(&mut self, timestamp: i64) -> Result<(), MediaError> {
        self.current_position = timestamp as u64;
        Ok(())
    }

    fn get_duration(&self) -> Option<i64> {
        self.duration
    }

    fn get_metadata(&self) -> BTreeMap<String, String> {
        self.metadata.clone()
    }
}

pub struct Mp4Muxer {
    streams: Vec<MediaFormat>,
    metadata: BTreeMap<String, String>,
}

impl Mp4Muxer {
    pub fn new() -> Self {
        Self {
            streams: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    fn write_ftyp(&self) -> Vec<u8> {
        let mut data = Vec::new();
        
        // Box size (will be updated)
        data.extend_from_slice(&32u32.to_be_bytes());
        // Box type
        data.extend_from_slice(b"ftyp");
        // Major brand
        data.extend_from_slice(b"isom");
        // Minor version
        data.extend_from_slice(&0u32.to_be_bytes());
        // Compatible brands
        data.extend_from_slice(b"isomiso2mp41");
        
        // Update box size
        let size = data.len() as u32;
        data[0..4].copy_from_slice(&size.to_be_bytes());
        
        data
    }
}

impl ContainerMuxer for Mp4Muxer {
    fn add_stream(&mut self, format: &MediaFormat) -> Result<usize, MediaError> {
        let index = self.streams.len();
        self.streams.push(format.clone());
        Ok(index)
    }

    fn write_header(&mut self) -> Result<Vec<u8>, MediaError> {
        Ok(self.write_ftyp())
    }

    fn write_packet(&mut self, _packet: &MediaPacket) -> Result<Vec<u8>, MediaError> {
        Ok(Vec::new())
    }

    fn write_trailer(&mut self) -> Result<Vec<u8>, MediaError> {
        Ok(Vec::new())
    }

    fn set_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(String::from(key), String::from(value));
    }
}

pub struct MkvDemuxer {
    streams: Vec<StreamInfo>,
    duration: Option<i64>,
}

impl MkvDemuxer {
    pub fn new() -> Self {
        Self {
            streams: Vec::new(),
            duration: None,
        }
    }
}

impl ContainerDemuxer for MkvDemuxer {
    fn probe(&mut self, data: &[u8]) -> bool {
        data.len() >= 4 && &data[0..4] == &[0x1A, 0x45, 0xDF, 0xA3]
    }

    fn open(&mut self, _data: &[u8]) -> Result<FormatContext, MediaError> {
        Ok(FormatContext {
            streams: self.streams.clone(),
            duration: self.duration,
            bit_rate: None,
            metadata: BTreeMap::new(),
            format_name: String::from("matroska"),
            format_long_name: String::from("Matroska"),
        })
    }

    fn read_packet(&mut self) -> Result<MediaPacket, MediaError> {
        Ok(MediaPacket {
            stream_index: 0,
            pts: Some(0),
            dts: Some(0),
            duration: Some(0),
            data: Vec::new(),
            flags: super::PacketFlags::empty(),
        })
    }

    fn seek(&mut self, _timestamp: i64) -> Result<(), MediaError> {
        Ok(())
    }

    fn get_duration(&self) -> Option<i64> {
        self.duration
    }

    fn get_metadata(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }
}

pub struct WebMDemuxer {
    mkv_demuxer: MkvDemuxer,
}

impl WebMDemuxer {
    pub fn new() -> Self {
        Self {
            mkv_demuxer: MkvDemuxer::new(),
        }
    }
}

impl ContainerDemuxer for WebMDemuxer {
    fn probe(&mut self, data: &[u8]) -> bool {
        self.mkv_demuxer.probe(data)
    }

    fn open(&mut self, data: &[u8]) -> Result<FormatContext, MediaError> {
        let mut ctx = self.mkv_demuxer.open(data)?;
        ctx.format_name = String::from("webm");
        ctx.format_long_name = String::from("WebM");
        Ok(ctx)
    }

    fn read_packet(&mut self) -> Result<MediaPacket, MediaError> {
        self.mkv_demuxer.read_packet()
    }

    fn seek(&mut self, timestamp: i64) -> Result<(), MediaError> {
        self.mkv_demuxer.seek(timestamp)
    }

    fn get_duration(&self) -> Option<i64> {
        self.mkv_demuxer.get_duration()
    }

    fn get_metadata(&self) -> BTreeMap<String, String> {
        self.mkv_demuxer.get_metadata()
    }
}

pub struct ContainerRegistry {
    demuxers: BTreeMap<String, alloc::sync::Arc<dyn ContainerDemuxer>>,
    muxers: BTreeMap<String, alloc::sync::Arc<dyn ContainerMuxer>>,
}

impl ContainerRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            demuxers: BTreeMap::new(),
            muxers: BTreeMap::new(),
        };
        
        registry.register_builtin_containers();
        registry
    }

    fn register_builtin_containers(&mut self) {
        use alloc::sync::Arc;
        
        // Register MP4
        self.demuxers.insert(
            String::from("mp4"),
            Arc::new(Mp4Demuxer::new()) as Arc<dyn ContainerDemuxer>
        );
        
        // Register MKV
        self.demuxers.insert(
            String::from("mkv"),
            Arc::new(MkvDemuxer::new()) as Arc<dyn ContainerDemuxer>
        );
        
        // Register WebM
        self.demuxers.insert(
            String::from("webm"),
            Arc::new(WebMDemuxer::new()) as Arc<dyn ContainerDemuxer>
        );
    }

    pub fn probe_format(&self, data: &[u8]) -> Option<String> {
        for (name, demuxer) in &self.demuxers {
            let mut demuxer_clone = Mp4Demuxer::new();
            if demuxer_clone.probe(data) {
                return Some(name.clone());
            }
        }
        None
    }
}