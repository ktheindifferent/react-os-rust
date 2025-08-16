use alloc::sync::Arc;
use spin::RwLock;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use core::time::Duration;
use super::{MediaType, StreamState, MediaFormat, MediaPacket, MediaFrame, MediaError};

pub trait Element: Send + Sync {
    fn name(&self) -> &str;
    fn element_type(&self) -> ElementType;
    fn connect(&mut self, output: &str, target: Arc<dyn Element>, input: &str) -> Result<(), MediaError>;
    fn process(&mut self) -> Result<(), MediaError>;
    fn set_state(&mut self, state: StreamState) -> Result<(), MediaError>;
    fn get_state(&self) -> StreamState;
    fn query_caps(&self, pad: &str) -> Option<Caps>;
    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), MediaError>;
    fn get_property(&self, name: &str) -> Option<PropertyValue>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    Source,
    Sink,
    Filter,
    Demuxer,
    Muxer,
    Decoder,
    Encoder,
    Queue,
    Tee,
    Mixer,
}

#[derive(Debug, Clone)]
pub struct Caps {
    pub media_type: MediaType,
    pub format: Option<String>,
    pub width: Option<RangeValue>,
    pub height: Option<RangeValue>,
    pub framerate: Option<RangeValue>,
    pub sample_rate: Option<RangeValue>,
    pub channels: Option<RangeValue>,
    pub extra: BTreeMap<String, PropertyValue>,
}

#[derive(Debug, Clone)]
pub enum RangeValue {
    Fixed(i32),
    Range(i32, i32),
    List(Vec<i32>),
}

#[derive(Debug, Clone)]
pub enum PropertyValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Binary(Vec<u8>),
}

pub struct Pipeline {
    id: usize,
    name: String,
    elements: RwLock<Vec<Arc<RwLock<dyn Element>>>>,
    connections: RwLock<Vec<Connection>>,
    state: RwLock<StreamState>,
    clock: Arc<super::clock::Clock>,
    bus: Arc<Bus>,
}

struct Connection {
    source: Arc<RwLock<dyn Element>>,
    source_pad: String,
    sink: Arc<RwLock<dyn Element>>,
    sink_pad: String,
}

impl Pipeline {
    pub fn new(id: usize, name: &str) -> Self {
        Self {
            id,
            name: String::from(name),
            elements: RwLock::new(Vec::new()),
            connections: RwLock::new(Vec::new()),
            state: RwLock::new(StreamState::Stopped),
            clock: Arc::new(super::clock::Clock::new()),
            bus: Arc::new(Bus::new()),
        }
    }

    pub fn add_element(&self, element: Arc<RwLock<dyn Element>>) {
        self.elements.write().push(element);
    }

    pub fn link(&self, source: Arc<RwLock<dyn Element>>, sink: Arc<RwLock<dyn Element>>) -> Result<(), MediaError> {
        self.link_pads(source, "src", sink, "sink")
    }

    pub fn link_pads(
        &self,
        source: Arc<RwLock<dyn Element>>,
        source_pad: &str,
        sink: Arc<RwLock<dyn Element>>,
        sink_pad: &str,
    ) -> Result<(), MediaError> {
        source.write().connect(source_pad, sink.clone(), sink_pad)?;
        
        self.connections.write().push(Connection {
            source: source.clone(),
            source_pad: String::from(source_pad),
            sink: sink.clone(),
            sink_pad: String::from(sink_pad),
        });
        
        Ok(())
    }

    pub fn set_state(&self, state: StreamState) -> Result<(), MediaError> {
        for element in self.elements.read().iter() {
            element.write().set_state(state)?;
        }
        *self.state.write() = state;
        
        if state == StreamState::Playing {
            self.clock.start();
        } else if state == StreamState::Stopped {
            self.clock.reset();
        }
        
        Ok(())
    }

    pub fn get_state(&self) -> StreamState {
        *self.state.read()
    }

    pub fn get_clock(&self) -> Arc<super::clock::Clock> {
        self.clock.clone()
    }

    pub fn get_bus(&self) -> Arc<Bus> {
        self.bus.clone()
    }

    pub fn process(&self) -> Result<(), MediaError> {
        let state = self.get_state();
        if state != StreamState::Playing {
            return Ok(());
        }

        for element in self.elements.read().iter() {
            element.write().process()?;
        }
        
        Ok(())
    }
}

pub struct Bus {
    messages: RwLock<Vec<Message>>,
    handlers: RwLock<Vec<Arc<dyn Fn(&Message) + Send + Sync>>>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            messages: RwLock::new(Vec::new()),
            handlers: RwLock::new(Vec::new()),
        }
    }

    pub fn post(&self, message: Message) {
        self.messages.write().push(message.clone());
        
        for handler in self.handlers.read().iter() {
            handler(&message);
        }
    }

    pub fn add_handler<F>(&self, handler: F)
    where
        F: Fn(&Message) + Send + Sync + 'static,
    {
        self.handlers.write().push(Arc::new(handler));
    }

    pub fn poll(&self) -> Option<Message> {
        self.messages.write().pop()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    StateChanged {
        old: StreamState,
        new: StreamState,
    },
    Error {
        source: String,
        error: String,
    },
    Warning {
        source: String,
        warning: String,
    },
    Info {
        source: String,
        info: String,
    },
    Eos,
    Buffering {
        percent: u8,
    },
    StreamStart,
    Tag {
        tags: BTreeMap<String, String>,
    },
}

pub struct Pad {
    name: String,
    direction: PadDirection,
    caps: Option<Caps>,
    linked_to: Option<Arc<RwLock<Pad>>>,
    buffers: RwLock<Vec<Arc<super::buffer::Buffer>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadDirection {
    Source,
    Sink,
}

impl Pad {
    pub fn new(name: &str, direction: PadDirection) -> Self {
        Self {
            name: String::from(name),
            direction,
            caps: None,
            linked_to: None,
            buffers: RwLock::new(Vec::new()),
        }
    }

    pub fn set_caps(&mut self, caps: Caps) {
        self.caps = Some(caps);
    }

    pub fn link(&mut self, other: Arc<RwLock<Pad>>) -> Result<(), MediaError> {
        if self.direction == other.read().direction {
            return Err(MediaError::InvalidState);
        }
        
        self.linked_to = Some(other.clone());
        other.write().linked_to = Some(Arc::new(RwLock::new(self.clone())));
        
        Ok(())
    }

    pub fn push(&self, buffer: Arc<super::buffer::Buffer>) -> Result<(), MediaError> {
        if let Some(linked) = &self.linked_to {
            linked.write().buffers.write().push(buffer);
            Ok(())
        } else {
            Err(MediaError::InvalidState)
        }
    }

    pub fn pull(&self) -> Option<Arc<super::buffer::Buffer>> {
        self.buffers.write().pop()
    }
}

impl Clone for Pad {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            direction: self.direction,
            caps: self.caps.clone(),
            linked_to: None,
            buffers: RwLock::new(Vec::new()),
        }
    }
}