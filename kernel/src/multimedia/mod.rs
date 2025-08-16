pub mod pipeline;
pub mod buffer;
pub mod clock;
pub mod format;
pub mod plugin;
pub mod codecs;
pub mod streaming;
pub mod capture;
pub mod effects;
pub mod container;

use alloc::sync::Arc;
use spin::RwLock;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use core::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Audio,
    Video,
    Subtitle,
    Data,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    Stopped,
    Paused,
    Playing,
    Buffering,
    Error,
}

#[derive(Debug, Clone)]
pub struct MediaFormat {
    pub media_type: MediaType,
    pub codec: String,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub framerate: Option<f32>,
    pub pixel_format: Option<PixelFormat>,
    pub audio_format: Option<AudioFormat>,
    pub extra_data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    RGB24,
    RGBA32,
    BGR24,
    BGRA32,
    YUV420P,
    YUV422P,
    YUV444P,
    NV12,
    NV21,
    GRAY8,
    GRAY16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    U8,
    S16LE,
    S16BE,
    S24LE,
    S24BE,
    S32LE,
    S32BE,
    F32LE,
    F32BE,
    F64LE,
    F64BE,
}

#[derive(Debug, Clone)]
pub struct MediaPacket {
    pub stream_index: usize,
    pub pts: Option<i64>,
    pub dts: Option<i64>,
    pub duration: Option<i64>,
    pub data: Vec<u8>,
    pub flags: PacketFlags,
}

bitflags::bitflags! {
    pub struct PacketFlags: u32 {
        const KEY_FRAME = 1 << 0;
        const CORRUPT = 1 << 1;
        const DISCARD = 1 << 2;
        const TRUSTED = 1 << 3;
    }
}

#[derive(Debug)]
pub struct MediaFrame {
    pub media_type: MediaType,
    pub pts: i64,
    pub duration: i64,
    pub format: MediaFormat,
    pub data: FrameData,
}

#[derive(Debug)]
pub enum FrameData {
    Audio(AudioFrame),
    Video(VideoFrame),
    Subtitle(SubtitleFrame),
}

#[derive(Debug)]
pub struct AudioFrame {
    pub samples: Vec<u8>,
    pub sample_count: usize,
    pub channel_layout: ChannelLayout,
}

#[derive(Debug)]
pub struct VideoFrame {
    pub planes: Vec<Vec<u8>>,
    pub linesize: Vec<usize>,
    pub key_frame: bool,
    pub picture_type: PictureType,
}

#[derive(Debug)]
pub struct SubtitleFrame {
    pub text: Option<String>,
    pub bitmap: Option<Vec<u8>>,
    pub start_time: i64,
    pub end_time: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelLayout {
    Mono,
    Stereo,
    Surround21,
    Surround51,
    Surround71,
    Custom(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureType {
    I,
    P,
    B,
    S,
    SI,
    SP,
    BI,
}

pub struct MultimediaSystem {
    pipelines: RwLock<BTreeMap<usize, Arc<pipeline::Pipeline>>>,
    plugins: RwLock<Vec<Arc<plugin::Plugin>>>,
    next_pipeline_id: RwLock<usize>,
}

impl MultimediaSystem {
    pub fn new() -> Self {
        Self {
            pipelines: RwLock::new(BTreeMap::new()),
            plugins: RwLock::new(Vec::new()),
            next_pipeline_id: RwLock::new(0),
        }
    }

    pub fn create_pipeline(&self, name: &str) -> Result<usize, MediaError> {
        let mut id_lock = self.next_pipeline_id.write();
        let id = *id_lock;
        *id_lock += 1;
        
        let pipeline = Arc::new(pipeline::Pipeline::new(id, name));
        self.pipelines.write().insert(id, pipeline);
        
        Ok(id)
    }

    pub fn get_pipeline(&self, id: usize) -> Option<Arc<pipeline::Pipeline>> {
        self.pipelines.read().get(&id).cloned()
    }

    pub fn register_plugin(&self, plugin: Arc<plugin::Plugin>) {
        self.plugins.write().push(plugin);
    }

    pub fn list_plugins(&self) -> Vec<Arc<plugin::Plugin>> {
        self.plugins.read().clone()
    }
}

#[derive(Debug)]
pub enum MediaError {
    NotSupported,
    InvalidFormat,
    CodecNotFound,
    DecodingError,
    EncodingError,
    StreamingError,
    CaptureError,
    PipelineError,
    BufferOverflow,
    BufferUnderflow,
    Timeout,
    InvalidState,
    ResourceBusy,
    OutOfMemory,
}

pub static MULTIMEDIA_SYSTEM: spin::Once<Arc<MultimediaSystem>> = spin::Once::new();

pub fn init() {
    MULTIMEDIA_SYSTEM.call_once(|| Arc::new(MultimediaSystem::new()));
    
    plugin::init_builtin_plugins();
    
    log::info!("Multimedia system initialized");
}

pub fn get_system() -> Arc<MultimediaSystem> {
    MULTIMEDIA_SYSTEM.get().expect("Multimedia system not initialized").clone()
}