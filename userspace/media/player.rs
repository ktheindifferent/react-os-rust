use std::sync::Arc;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::Duration;

pub struct MediaPlayer {
    pipeline: Option<Pipeline>,
    state: PlayerState,
    position: Duration,
    duration: Option<Duration>,
    volume: f32,
    playback_rate: f32,
    event_sender: Sender<PlayerEvent>,
    event_receiver: Receiver<PlayerEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    Stopped,
    Playing,
    Paused,
    Buffering,
    Error,
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    StateChanged(PlayerState),
    PositionChanged(Duration),
    DurationChanged(Duration),
    EndOfStream,
    Error(String),
    BufferingProgress(u8),
}

struct Pipeline {
    source: Box<dyn MediaSource>,
    decoder: Box<dyn MediaDecoder>,
    renderer: Box<dyn MediaRenderer>,
}

trait MediaSource: Send {
    fn open(&mut self, url: &str) -> Result<(), MediaError>;
    fn read(&mut self) -> Result<MediaPacket, MediaError>;
    fn seek(&mut self, position: Duration) -> Result<(), MediaError>;
    fn get_duration(&self) -> Option<Duration>;
}

trait MediaDecoder: Send {
    fn decode(&mut self, packet: &MediaPacket) -> Result<MediaFrame, MediaError>;
    fn flush(&mut self);
}

trait MediaRenderer: Send {
    fn render(&mut self, frame: &MediaFrame) -> Result<(), MediaError>;
    fn set_volume(&mut self, volume: f32);
    fn get_latency(&self) -> Duration;
}

#[derive(Debug)]
struct MediaPacket {
    data: Vec<u8>,
    pts: i64,
    dts: i64,
    duration: i64,
}

#[derive(Debug)]
struct MediaFrame {
    data: Vec<u8>,
    pts: i64,
    duration: i64,
}

#[derive(Debug)]
struct MediaError {
    message: String,
}

impl MediaPlayer {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        
        Self {
            pipeline: None,
            state: PlayerState::Stopped,
            position: Duration::ZERO,
            duration: None,
            volume: 1.0,
            playback_rate: 1.0,
            event_sender: tx,
            event_receiver: rx,
        }
    }

    pub fn open(&mut self, url: &str) -> Result<(), String> {
        // Create pipeline based on URL/file type
        let source = Box::new(FileSource::new());
        let decoder = Box::new(UniversalDecoder::new());
        let renderer = Box::new(AudioVideoRenderer::new());
        
        let mut pipeline = Pipeline {
            source,
            decoder,
            renderer,
        };
        
        pipeline.source.open(url).map_err(|e| e.message)?;
        
        if let Some(duration) = pipeline.source.get_duration() {
            self.duration = Some(duration);
            self.send_event(PlayerEvent::DurationChanged(duration));
        }
        
        self.pipeline = Some(pipeline);
        self.state = PlayerState::Stopped;
        
        Ok(())
    }

    pub fn play(&mut self) -> Result<(), String> {
        if self.pipeline.is_none() {
            return Err("No media loaded".to_string());
        }
        
        self.state = PlayerState::Playing;
        self.send_event(PlayerEvent::StateChanged(PlayerState::Playing));
        
        // Start playback thread
        self.start_playback_thread();
        
        Ok(())
    }

    pub fn pause(&mut self) {
        if self.state == PlayerState::Playing {
            self.state = PlayerState::Paused;
            self.send_event(PlayerEvent::StateChanged(PlayerState::Paused));
        }
    }

    pub fn stop(&mut self) {
        self.state = PlayerState::Stopped;
        self.position = Duration::ZERO;
        self.send_event(PlayerEvent::StateChanged(PlayerState::Stopped));
    }

    pub fn seek(&mut self, position: Duration) -> Result<(), String> {
        if let Some(ref mut pipeline) = self.pipeline {
            pipeline.source.seek(position).map_err(|e| e.message)?;
            pipeline.decoder.flush();
            self.position = position;
            self.send_event(PlayerEvent::PositionChanged(position));
            Ok(())
        } else {
            Err("No media loaded".to_string())
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(ref mut pipeline) = self.pipeline {
            pipeline.renderer.set_volume(self.volume);
        }
    }

    pub fn set_playback_rate(&mut self, rate: f32) {
        self.playback_rate = rate.clamp(0.25, 4.0);
    }

    pub fn get_position(&self) -> Duration {
        self.position
    }

    pub fn get_duration(&self) -> Option<Duration> {
        self.duration
    }

    pub fn get_state(&self) -> PlayerState {
        self.state
    }

    pub fn poll_events(&self) -> Option<PlayerEvent> {
        self.event_receiver.try_recv().ok()
    }

    fn send_event(&self, event: PlayerEvent) {
        let _ = self.event_sender.send(event);
    }

    fn start_playback_thread(&self) {
        // In a real implementation, this would spawn a thread
        // that continuously reads, decodes, and renders frames
    }
}

struct FileSource {
    file_path: String,
    position: u64,
}

impl FileSource {
    fn new() -> Self {
        Self {
            file_path: String::new(),
            position: 0,
        }
    }
}

impl MediaSource for FileSource {
    fn open(&mut self, url: &str) -> Result<(), MediaError> {
        self.file_path = url.to_string();
        self.position = 0;
        Ok(())
    }

    fn read(&mut self) -> Result<MediaPacket, MediaError> {
        Ok(MediaPacket {
            data: vec![0; 4096],
            pts: self.position as i64,
            dts: self.position as i64,
            duration: 1000,
        })
    }

    fn seek(&mut self, position: Duration) -> Result<(), MediaError> {
        self.position = position.as_millis() as u64;
        Ok(())
    }

    fn get_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs(180))
    }
}

struct UniversalDecoder;

impl UniversalDecoder {
    fn new() -> Self {
        Self
    }
}

impl MediaDecoder for UniversalDecoder {
    fn decode(&mut self, _packet: &MediaPacket) -> Result<MediaFrame, MediaError> {
        Ok(MediaFrame {
            data: vec![0; 1920 * 1080 * 3],
            pts: 0,
            duration: 40,
        })
    }

    fn flush(&mut self) {
        // Flush decoder buffers
    }
}

struct AudioVideoRenderer;

impl AudioVideoRenderer {
    fn new() -> Self {
        Self
    }
}

impl MediaRenderer for AudioVideoRenderer {
    fn render(&mut self, _frame: &MediaFrame) -> Result<(), MediaError> {
        Ok(())
    }

    fn set_volume(&mut self, _volume: f32) {
        // Set audio volume
    }

    fn get_latency(&self) -> Duration {
        Duration::from_millis(20)
    }
}

// Playlist support
pub struct Playlist {
    items: Vec<PlaylistItem>,
    current_index: Option<usize>,
    repeat_mode: RepeatMode,
    shuffle: bool,
}

#[derive(Debug, Clone)]
pub struct PlaylistItem {
    pub url: String,
    pub title: String,
    pub duration: Option<Duration>,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    None,
    One,
    All,
}

impl Playlist {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            current_index: None,
            repeat_mode: RepeatMode::None,
            shuffle: false,
        }
    }

    pub fn add(&mut self, item: PlaylistItem) {
        self.items.push(item);
    }

    pub fn remove(&mut self, index: usize) -> Option<PlaylistItem> {
        if index < self.items.len() {
            Some(self.items.remove(index))
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.current_index = None;
    }

    pub fn next(&mut self) -> Option<&PlaylistItem> {
        if self.items.is_empty() {
            return None;
        }

        let next_index = if let Some(current) = self.current_index {
            match self.repeat_mode {
                RepeatMode::One => current,
                RepeatMode::All => (current + 1) % self.items.len(),
                RepeatMode::None => {
                    if current + 1 < self.items.len() {
                        current + 1
                    } else {
                        return None;
                    }
                }
            }
        } else {
            0
        };

        self.current_index = Some(next_index);
        self.items.get(next_index)
    }

    pub fn previous(&mut self) -> Option<&PlaylistItem> {
        if self.items.is_empty() {
            return None;
        }

        let prev_index = if let Some(current) = self.current_index {
            if current > 0 {
                current - 1
            } else if self.repeat_mode == RepeatMode::All {
                self.items.len() - 1
            } else {
                0
            }
        } else {
            0
        };

        self.current_index = Some(prev_index);
        self.items.get(prev_index)
    }

    pub fn set_repeat_mode(&mut self, mode: RepeatMode) {
        self.repeat_mode = mode;
    }

    pub fn set_shuffle(&mut self, enabled: bool) {
        self.shuffle = enabled;
    }
}