use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use spin::RwLock;
use super::{MediaType, MediaError, MediaFormat};
use super::pipeline::{Element, ElementType};
use super::format::CodecId;

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn version(&self) -> &str;
    fn plugin_type(&self) -> PluginType;
    fn create_element(&self, element_type: &str) -> Option<Box<dyn Element>>;
    fn get_supported_types(&self) -> Vec<String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    Codec,
    Container,
    Protocol,
    Effect,
    Device,
    Utility,
}

pub struct PluginRegistry {
    plugins: RwLock<BTreeMap<String, Arc<dyn Plugin>>>,
    element_factories: RwLock<BTreeMap<String, Arc<dyn ElementFactory>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(BTreeMap::new()),
            element_factories: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn register_plugin(&self, plugin: Arc<dyn Plugin>) {
        let name = String::from(plugin.name());
        self.plugins.write().insert(name, plugin);
    }

    pub fn register_element_factory(&self, name: &str, factory: Arc<dyn ElementFactory>) {
        self.element_factories.write().insert(String::from(name), factory);
    }

    pub fn get_plugin(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.read().get(name).cloned()
    }

    pub fn create_element(&self, factory_name: &str) -> Option<Box<dyn Element>> {
        self.element_factories.read()
            .get(factory_name)
            .and_then(|factory| factory.create())
    }

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins.read()
            .values()
            .map(|p| PluginInfo {
                name: String::from(p.name()),
                description: String::from(p.description()),
                version: String::from(p.version()),
                plugin_type: p.plugin_type(),
            })
            .collect()
    }

    pub fn find_decoder(&self, codec_id: CodecId) -> Option<Arc<dyn Codec>> {
        for plugin in self.plugins.read().values() {
            if plugin.plugin_type() == PluginType::Codec {
                for element_type in plugin.get_supported_types() {
                    if element_type.starts_with("decoder_") {
                        if let Some(element) = plugin.create_element(&element_type) {
                            // Check if this decoder supports the codec
                            return None; // Simplified for now
                        }
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    pub plugin_type: PluginType,
}

pub trait ElementFactory: Send + Sync {
    fn create(&self) -> Option<Box<dyn Element>>;
    fn get_metadata(&self) -> ElementMetadata;
}

#[derive(Debug, Clone)]
pub struct ElementMetadata {
    pub name: String,
    pub long_name: String,
    pub classification: String,
    pub description: String,
    pub author: String,
}

pub trait Codec: Send + Sync {
    fn name(&self) -> &str;
    fn codec_id(&self) -> CodecId;
    fn codec_type(&self) -> CodecType;
    fn supported_formats(&self) -> Vec<MediaFormat>;
    fn init(&mut self, format: &MediaFormat) -> Result<(), MediaError>;
    fn reset(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecType {
    Decoder,
    Encoder,
    Both,
}

pub trait Decoder: Codec {
    fn decode(&mut self, data: &[u8]) -> Result<DecodedFrame, MediaError>;
    fn flush(&mut self) -> Result<Vec<DecodedFrame>, MediaError>;
}

pub trait Encoder: Codec {
    fn encode(&mut self, frame: &EncoderInput) -> Result<Vec<u8>, MediaError>;
    fn flush(&mut self) -> Result<Vec<Vec<u8>>, MediaError>;
    fn set_bitrate(&mut self, bitrate: u32) -> Result<(), MediaError>;
    fn set_quality(&mut self, quality: f32) -> Result<(), MediaError>;
}

#[derive(Debug)]
pub enum DecodedFrame {
    Audio(AudioData),
    Video(VideoData),
}

#[derive(Debug)]
pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u32,
    pub pts: i64,
}

#[derive(Debug)]
pub struct VideoData {
    pub planes: Vec<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub pixel_format: super::PixelFormat,
    pub pts: i64,
}

#[derive(Debug)]
pub enum EncoderInput {
    Audio(AudioData),
    Video(VideoData),
}

pub static PLUGIN_REGISTRY: spin::Once<Arc<PluginRegistry>> = spin::Once::new();

pub fn init_builtin_plugins() {
    let registry = PLUGIN_REGISTRY.call_once(|| Arc::new(PluginRegistry::new()));
    
    // Register built-in plugins
    register_core_elements(&registry);
    register_basic_codecs(&registry);
    
    log::info!("Plugin system initialized with built-in plugins");
}

fn register_core_elements(registry: &PluginRegistry) {
    // Register core pipeline elements
    struct FileSourceFactory;
    impl ElementFactory for FileSourceFactory {
        fn create(&self) -> Option<Box<dyn Element>> {
            None // Implementation would go here
        }
        
        fn get_metadata(&self) -> ElementMetadata {
            ElementMetadata {
                name: String::from("filesrc"),
                long_name: String::from("File Source"),
                classification: String::from("Source/File"),
                description: String::from("Read from files"),
                author: String::from("TerragonOS"),
            }
        }
    }
    
    registry.register_element_factory("filesrc", Arc::new(FileSourceFactory));
}

fn register_basic_codecs(registry: &PluginRegistry) {
    // Register basic codec plugins
    struct BasicCodecPlugin;
    impl Plugin for BasicCodecPlugin {
        fn name(&self) -> &str { "basic-codecs" }
        fn description(&self) -> &str { "Basic audio/video codecs" }
        fn version(&self) -> &str { "1.0.0" }
        fn plugin_type(&self) -> PluginType { PluginType::Codec }
        
        fn create_element(&self, element_type: &str) -> Option<Box<dyn Element>> {
            None // Implementation would go here
        }
        
        fn get_supported_types(&self) -> Vec<String> {
            vec![
                String::from("decoder_mp3"),
                String::from("decoder_aac"),
                String::from("encoder_pcm"),
            ]
        }
    }
    
    registry.register_plugin(Arc::new(BasicCodecPlugin));
}

pub fn get_registry() -> Arc<PluginRegistry> {
    PLUGIN_REGISTRY.get().expect("Plugin registry not initialized").clone()
}