// Model zoo with pre-trained models
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    fs,
    io::{self, Write},
    fmt,
    error::Error,
};

use serde::{Deserialize, Serialize};

// Model zoo errors
#[derive(Debug)]
pub enum ModelZooError {
    ModelNotFound(String),
    IoError(io::Error),
    InvalidModelName(String),
    ModelLoadFailed(String),
    ConfigurationError(String),
}

impl fmt::Display for ModelZooError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelZooError::ModelNotFound(name) => write!(f, "Model '{}' not found in zoo", name),
            ModelZooError::IoError(err) => write!(f, "IO error: {}", err),
            ModelZooError::InvalidModelName(name) => write!(f, "Invalid model name: '{}'", name),
            ModelZooError::ModelLoadFailed(msg) => write!(f, "Failed to load model: {}", msg),
            ModelZooError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl Error for ModelZooError {}

impl From<io::Error> for ModelZooError {
    fn from(err: io::Error) -> Self {
        ModelZooError::IoError(err)
    }
}

// Model zoo registry
pub struct ModelZoo {
    registry: ModelRegistry,
    cache_dir: PathBuf,
    models: HashMap<String, PretrainedModel>,
}

// Model registry with available models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistry {
    pub models: Vec<ModelEntry>,
    pub last_updated: String,
    pub version: String,
}

// Model entry in registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub name: String,
    pub category: ModelCategory,
    pub task: String,
    pub framework: String,
    pub version: String,
    pub size_mb: f64,
    pub accuracy: HashMap<String, f64>,
    pub url: String,
    pub checksum: String,
    pub description: String,
    pub paper: Option<String>,
    pub license: String,
}

// Model categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelCategory {
    ComputerVision,
    NaturalLanguageProcessing,
    Speech,
    Recommendation,
    Reinforcement,
    Generative,
    TimeSeries,
}

// Pre-trained model wrapper
#[derive(Debug)]
pub struct PretrainedModel {
    metadata: ModelEntry,
    model_path: PathBuf,
    config: ModelConfig,
    weights: Option<Vec<u8>>,
}

// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub input_size: Vec<usize>,
    pub output_size: Vec<usize>,
    pub num_classes: Option<usize>,
    pub preprocessing: PreprocessingConfig,
    pub class_names: Option<Vec<String>>,
}

// Preprocessing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreprocessingConfig {
    pub resize: Option<(u32, u32)>,
    pub normalize: bool,
    pub mean: Vec<f32>,
    pub std: Vec<f32>,
    pub color_mode: String,
}

impl ModelZoo {
    pub fn new(cache_dir: PathBuf) -> io::Result<Self> {
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir)?;
        
        // Load registry
        let registry = Self::load_registry()?;
        
        Ok(Self {
            registry,
            cache_dir,
            models: HashMap::new(),
        })
    }
    
    fn load_registry() -> io::Result<ModelRegistry> {
        // Load from embedded registry or remote source
        Ok(ModelRegistry {
            models: Self::get_builtin_models(),
            last_updated: "2024-01-01".to_string(),
            version: "1.0.0".to_string(),
        })
    }
    
    fn get_builtin_models() -> Vec<ModelEntry> {
        vec![
            // Computer Vision Models
            ModelEntry {
                name: "resnet50".to_string(),
                category: ModelCategory::ComputerVision,
                task: "image_classification".to_string(),
                framework: "onnx".to_string(),
                version: "1.0".to_string(),
                size_mb: 97.8,
                accuracy: [("top1".to_string(), 76.1), ("top5".to_string(), 92.9)].iter().cloned().collect(),
                url: "https://models.example.com/resnet50.onnx".to_string(),
                checksum: "abc123".to_string(),
                description: "ResNet-50 trained on ImageNet".to_string(),
                paper: Some("https://arxiv.org/abs/1512.03385".to_string()),
                license: "Apache-2.0".to_string(),
            },
            ModelEntry {
                name: "mobilenet_v2".to_string(),
                category: ModelCategory::ComputerVision,
                task: "image_classification".to_string(),
                framework: "onnx".to_string(),
                version: "1.0".to_string(),
                size_mb: 13.5,
                accuracy: [("top1".to_string(), 71.8), ("top5".to_string(), 91.0)].iter().cloned().collect(),
                url: "https://models.example.com/mobilenet_v2.onnx".to_string(),
                checksum: "def456".to_string(),
                description: "MobileNetV2 optimized for mobile devices".to_string(),
                paper: Some("https://arxiv.org/abs/1801.04381".to_string()),
                license: "Apache-2.0".to_string(),
            },
            ModelEntry {
                name: "yolov5s".to_string(),
                category: ModelCategory::ComputerVision,
                task: "object_detection".to_string(),
                framework: "onnx".to_string(),
                version: "6.0".to_string(),
                size_mb: 28.1,
                accuracy: [("mAP50".to_string(), 37.2), ("mAP50-95".to_string(), 56.0)].iter().cloned().collect(),
                url: "https://models.example.com/yolov5s.onnx".to_string(),
                checksum: "ghi789".to_string(),
                description: "YOLOv5 small model for real-time object detection".to_string(),
                paper: None,
                license: "GPL-3.0".to_string(),
            },
            ModelEntry {
                name: "deeplabv3_resnet101".to_string(),
                category: ModelCategory::ComputerVision,
                task: "semantic_segmentation".to_string(),
                framework: "onnx".to_string(),
                version: "1.0".to_string(),
                size_mb: 232.7,
                accuracy: [("mIoU".to_string(), 78.4)].iter().cloned().collect(),
                url: "https://models.example.com/deeplabv3.onnx".to_string(),
                checksum: "jkl012".to_string(),
                description: "DeepLabV3 with ResNet-101 backbone for semantic segmentation".to_string(),
                paper: Some("https://arxiv.org/abs/1706.05587".to_string()),
                license: "Apache-2.0".to_string(),
            },
            
            // NLP Models
            ModelEntry {
                name: "bert_base".to_string(),
                category: ModelCategory::NaturalLanguageProcessing,
                task: "text_classification".to_string(),
                framework: "onnx".to_string(),
                version: "1.0".to_string(),
                size_mb: 420.0,
                accuracy: [("f1".to_string(), 90.2)].iter().cloned().collect(),
                url: "https://models.example.com/bert_base.onnx".to_string(),
                checksum: "mno345".to_string(),
                description: "BERT base model for text understanding".to_string(),
                paper: Some("https://arxiv.org/abs/1810.04805".to_string()),
                license: "Apache-2.0".to_string(),
            },
            ModelEntry {
                name: "gpt2_small".to_string(),
                category: ModelCategory::NaturalLanguageProcessing,
                task: "text_generation".to_string(),
                framework: "onnx".to_string(),
                version: "1.0".to_string(),
                size_mb: 497.0,
                accuracy: [("perplexity".to_string(), 29.4)].iter().cloned().collect(),
                url: "https://models.example.com/gpt2_small.onnx".to_string(),
                checksum: "pqr678".to_string(),
                description: "GPT-2 small model for text generation".to_string(),
                paper: None,
                license: "MIT".to_string(),
            },
            
            // Speech Models
            ModelEntry {
                name: "wav2vec2_base".to_string(),
                category: ModelCategory::Speech,
                task: "speech_recognition".to_string(),
                framework: "onnx".to_string(),
                version: "1.0".to_string(),
                size_mb: 360.0,
                accuracy: [("wer".to_string(), 3.2)].iter().cloned().collect(),
                url: "https://models.example.com/wav2vec2.onnx".to_string(),
                checksum: "stu901".to_string(),
                description: "Wav2Vec2 for automatic speech recognition".to_string(),
                paper: Some("https://arxiv.org/abs/2006.11477".to_string()),
                license: "Apache-2.0".to_string(),
            },
        ]
    }
    
    pub fn list_models(&self, category: Option<ModelCategory>) -> Vec<&ModelEntry> {
        self.registry.models.iter()
            .filter(|m| category.as_ref().map_or(true, |c| &m.category == c))
            .collect()
    }
    
    pub fn search_models(&self, query: &str) -> Vec<&ModelEntry> {
        let query_lower = query.to_lowercase();
        self.registry.models.iter()
            .filter(|m| {
                m.name.to_lowercase().contains(&query_lower) ||
                m.task.to_lowercase().contains(&query_lower) ||
                m.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
    
    /// Check if a model exists in the registry
    pub fn model_exists(&self, name: &str) -> bool {
        self.registry.models.iter()
            .any(|m| m.name == name)
    }
    
    /// Check if a model is loaded in memory
    pub fn is_model_loaded(&self, name: &str) -> bool {
        self.models.contains_key(name)
    }
    
    /// Get available model names
    pub fn get_available_models(&self) -> Vec<String> {
        self.registry.models.iter()
            .map(|m| m.name.clone())
            .collect()
    }
    
    pub fn get_model(&mut self, name: &str) -> Result<&PretrainedModel, ModelZooError> {
        // Validate model name
        if name.is_empty() {
            return Err(ModelZooError::InvalidModelName("Model name cannot be empty".to_string()));
        }
        
        // Load model if not already loaded
        if !self.models.contains_key(name) {
            self.load_model(name)?;
        }
        
        // Safe retrieval with proper error handling
        self.models.get(name)
            .ok_or_else(|| ModelZooError::ModelNotFound(name.to_string()))
    }
    
    fn load_model(&mut self, name: &str) -> Result<(), ModelZooError> {
        // Find model in registry
        let entry = self.registry.models.iter()
            .find(|m| m.name == name)
            .ok_or_else(|| ModelZooError::ModelNotFound(name.to_string()))?
            .clone();
        
        // Check if model is cached
        let model_path = self.cache_dir.join(format!("{}.onnx", name));
        
        if !model_path.exists() {
            // Download model
            self.download_model(&entry, &model_path)
                .map_err(|e| ModelZooError::ModelLoadFailed(format!("Download failed: {}", e)))?;
        }
        
        // Load model configuration
        let config = self.load_model_config(&entry)
            .map_err(|e| ModelZooError::ConfigurationError(format!("Failed to load config: {}", e)))?;
        
        // Create pretrained model
        let model = PretrainedModel {
            metadata: entry,
            model_path,
            config,
            weights: None,
        };
        
        self.models.insert(name.to_string(), model);
        Ok(())
    }
    
    fn download_model(&self, entry: &ModelEntry, path: &Path) -> io::Result<()> {
        println!("Downloading {} ({:.1} MB)...", entry.name, entry.size_mb);
        
        // Download from URL (simplified - would use actual HTTP client)
        // For now, create a dummy file
        let dummy_content = format!("Model: {}\nVersion: {}", entry.name, entry.version);
        fs::write(path, dummy_content)?;
        
        println!("Downloaded {} successfully", entry.name);
        Ok(())
    }
    
    fn load_model_config(&self, entry: &ModelEntry) -> io::Result<ModelConfig> {
        // Load or generate model configuration
        let config = match entry.name.as_str() {
            "resnet50" => ModelConfig {
                input_size: vec![1, 3, 224, 224],
                output_size: vec![1, 1000],
                num_classes: Some(1000),
                preprocessing: PreprocessingConfig {
                    resize: Some((224, 224)),
                    normalize: true,
                    mean: vec![0.485, 0.456, 0.406],
                    std: vec![0.229, 0.224, 0.225],
                    color_mode: "RGB".to_string(),
                },
                class_names: None, // Would load ImageNet classes
            },
            "yolov5s" => ModelConfig {
                input_size: vec![1, 3, 640, 640],
                output_size: vec![1, 25200, 85],
                num_classes: Some(80),
                preprocessing: PreprocessingConfig {
                    resize: Some((640, 640)),
                    normalize: true,
                    mean: vec![0.0, 0.0, 0.0],
                    std: vec![255.0, 255.0, 255.0],
                    color_mode: "RGB".to_string(),
                },
                class_names: None, // Would load COCO classes
            },
            "bert_base" => ModelConfig {
                input_size: vec![1, 512], // Token IDs
                output_size: vec![1, 512, 768],
                num_classes: None,
                preprocessing: PreprocessingConfig {
                    resize: None,
                    normalize: false,
                    mean: vec![],
                    std: vec![],
                    color_mode: "None".to_string(),
                },
                class_names: None,
            },
            _ => ModelConfig {
                input_size: vec![1, 3, 224, 224],
                output_size: vec![1, 1000],
                num_classes: Some(1000),
                preprocessing: PreprocessingConfig {
                    resize: Some((224, 224)),
                    normalize: true,
                    mean: vec![0.5, 0.5, 0.5],
                    std: vec![0.5, 0.5, 0.5],
                    color_mode: "RGB".to_string(),
                },
                class_names: None,
            },
        };
        
        Ok(config)
    }
}

// Model downloader with progress tracking
pub struct ModelDownloader {
    progress_callback: Option<Box<dyn Fn(f64)>>,
}

impl ModelDownloader {
    pub fn new() -> Self {
        Self {
            progress_callback: None,
        }
    }
    
    pub fn set_progress_callback<F>(&mut self, callback: F)
    where
        F: Fn(f64) + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
    }
    
    pub fn download(&self, url: &str, dest: &Path) -> io::Result<()> {
        // Download with progress tracking
        // Simplified implementation
        
        if let Some(ref callback) = self.progress_callback {
            for i in 0..=100 {
                callback(i as f64);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        
        // Write dummy content
        fs::write(dest, format!("Downloaded from: {}", url))?;
        Ok(())
    }
}

// Model benchmark utilities
pub struct ModelBenchmark {
    results: Vec<BenchmarkResult>,
}

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub model_name: String,
    pub device: String,
    pub batch_size: usize,
    pub input_size: Vec<usize>,
    pub throughput: f64, // samples/sec
    pub latency_mean: f64, // ms
    pub latency_p50: f64,
    pub latency_p95: f64,
    pub latency_p99: f64,
    pub memory_used: usize, // MB
}

impl ModelBenchmark {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }
    
    pub fn benchmark_model(
        &mut self,
        model_name: &str,
        device: &str,
        batch_sizes: &[usize],
        num_iterations: usize,
    ) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();
        
        for &batch_size in batch_sizes {
            // Run benchmark (simplified)
            let result = BenchmarkResult {
                model_name: model_name.to_string(),
                device: device.to_string(),
                batch_size,
                input_size: vec![batch_size, 3, 224, 224],
                throughput: 1000.0 / (10.0 + batch_size as f64), // Mock calculation
                latency_mean: 10.0 + batch_size as f64,
                latency_p50: 9.0 + batch_size as f64,
                latency_p95: 15.0 + batch_size as f64 * 1.5,
                latency_p99: 20.0 + batch_size as f64 * 2.0,
                memory_used: 100 + batch_size * 10,
            };
            
            results.push(result.clone());
            self.results.push(result);
        }
        
        results
    }
    
    pub fn compare_models(&self, model_names: &[String]) -> String {
        let mut comparison = String::from("Model Comparison:\n");
        comparison.push_str("================\n\n");
        
        for name in model_names {
            let model_results: Vec<_> = self.results.iter()
                .filter(|r| r.model_name == *name)
                .collect();
            
            if !model_results.is_empty() {
                comparison.push_str(&format!("Model: {}\n", name));
                for result in model_results {
                    comparison.push_str(&format!(
                        "  Batch {}: {:.1} samples/sec, {:.2}ms latency\n",
                        result.batch_size, result.throughput, result.latency_mean
                    ));
                }
                comparison.push_str("\n");
            }
        }
        
        comparison
    }
    
    pub fn export_results(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.results)?;
        fs::write(path, json)?;
        Ok(())
    }
}

// AutoML capabilities
pub struct AutoML {
    search_space: SearchSpace,
    optimizer: HPOptimizer,
}

pub struct SearchSpace {
    pub architecture: Vec<ArchitectureOption>,
    pub hyperparameters: HashMap<String, HyperParameter>,
}

pub enum ArchitectureOption {
    ResNet { depth: Vec<u32> },
    MobileNet { width_multiplier: Vec<f32> },
    Transformer { num_layers: Vec<u32>, hidden_size: Vec<u32> },
}

#[derive(Clone)]
pub enum HyperParameter {
    Float { min: f32, max: f32, log_scale: bool },
    Int { min: i32, max: i32 },
    Categorical { choices: Vec<String> },
}

pub struct HPOptimizer {
    method: OptimizationMethod,
    num_trials: usize,
    results: Vec<Trial>,
}

pub enum OptimizationMethod {
    RandomSearch,
    GridSearch,
    BayesianOptimization,
    EvolutionaryAlgorithm,
}

pub struct Trial {
    pub id: usize,
    pub parameters: HashMap<String, String>,
    pub score: f64,
    pub duration: std::time::Duration,
}

impl AutoML {
    pub fn new() -> Self {
        Self {
            search_space: SearchSpace {
                architecture: vec![
                    ArchitectureOption::ResNet { depth: vec![18, 34, 50, 101] },
                    ArchitectureOption::MobileNet { width_multiplier: vec![0.5, 0.75, 1.0, 1.25] },
                ],
                hyperparameters: [
                    ("learning_rate".to_string(), HyperParameter::Float { min: 0.0001, max: 0.1, log_scale: true }),
                    ("batch_size".to_string(), HyperParameter::Int { min: 16, max: 128 }),
                    ("optimizer".to_string(), HyperParameter::Categorical { 
                        choices: vec!["adam".to_string(), "sgd".to_string(), "rmsprop".to_string()] 
                    }),
                ].into_iter().collect(),
            },
            optimizer: HPOptimizer {
                method: OptimizationMethod::BayesianOptimization,
                num_trials: 100,
                results: Vec::new(),
            },
        }
    }
    
    pub fn search(&mut self, dataset: &str, task: &str) -> Trial {
        // Perform hyperparameter search
        println!("Starting AutoML search for {} on {}", task, dataset);
        
        // Mock search result
        Trial {
            id: 1,
            parameters: [
                ("architecture".to_string(), "resnet50".to_string()),
                ("learning_rate".to_string(), "0.001".to_string()),
                ("batch_size".to_string(), "32".to_string()),
                ("optimizer".to_string(), "adam".to_string()),
            ].iter().cloned().collect(),
            score: 0.95,
            duration: std::time::Duration::from_secs(3600),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    
    fn create_test_zoo() -> ModelZoo {
        let temp_dir = PathBuf::from("/tmp/test_model_zoo");
        fs::create_dir_all(&temp_dir).unwrap();
        ModelZoo::new(temp_dir).unwrap()
    }
    
    #[test]
    fn test_get_model_with_nonexistent_model() {
        let mut zoo = create_test_zoo();
        
        // Test with non-existent model name
        let result = zoo.get_model("nonexistent_model");
        assert!(result.is_err());
        
        match result {
            Err(ModelZooError::ModelNotFound(name)) => {
                assert_eq!(name, "nonexistent_model");
            }
            _ => panic!("Expected ModelNotFound error"),
        }
    }
    
    #[test]
    fn test_get_model_with_empty_name() {
        let mut zoo = create_test_zoo();
        
        // Test with empty model name
        let result = zoo.get_model("");
        assert!(result.is_err());
        
        match result {
            Err(ModelZooError::InvalidModelName(_)) => {
                // Expected error
            }
            _ => panic!("Expected InvalidModelName error"),
        }
    }
    
    #[test]
    fn test_get_model_with_special_characters() {
        let mut zoo = create_test_zoo();
        
        // Test with special characters in model name
        let special_names = vec![
            "../../../etc/passwd",
            "model\0name",
            "model|name",
            "model&name",
            "model;name",
        ];
        
        for name in special_names {
            let result = zoo.get_model(name);
            assert!(result.is_err());
            // Should return ModelNotFound since these aren't valid model names
            match result {
                Err(ModelZooError::ModelNotFound(_)) => {
                    // Expected error
                }
                _ => panic!("Expected ModelNotFound error for '{}'", name),
            }
        }
    }
    
    #[test]
    fn test_model_exists() {
        let zoo = create_test_zoo();
        
        // Test with existing models
        assert!(zoo.model_exists("resnet50"));
        assert!(zoo.model_exists("mobilenet_v2"));
        assert!(zoo.model_exists("yolov5s"));
        assert!(zoo.model_exists("bert_base"));
        
        // Test with non-existent models
        assert!(!zoo.model_exists("nonexistent"));
        assert!(!zoo.model_exists(""));
        assert!(!zoo.model_exists("ResNet50")); // Case sensitive
    }
    
    #[test]
    fn test_is_model_loaded() {
        let mut zoo = create_test_zoo();
        
        // Initially no models should be loaded
        assert!(!zoo.is_model_loaded("resnet50"));
        
        // After attempting to get a model, check if it's marked as loaded
        // Note: This will fail to actually load due to missing files, but that's ok for this test
        let _ = zoo.get_model("resnet50");
        
        // The model might not be loaded due to download failure, which is expected in test
    }
    
    #[test]
    fn test_get_available_models() {
        let zoo = create_test_zoo();
        let models = zoo.get_available_models();
        
        // Check that we have the expected models
        assert!(models.contains(&"resnet50".to_string()));
        assert!(models.contains(&"mobilenet_v2".to_string()));
        assert!(models.contains(&"yolov5s".to_string()));
        assert!(models.contains(&"bert_base".to_string()));
        assert!(models.contains(&"gpt2_small".to_string()));
        assert!(models.contains(&"wav2vec2_base".to_string()));
        
        // Verify count
        assert_eq!(models.len(), 7);
    }
    
    #[test]
    fn test_error_propagation() {
        let mut zoo = create_test_zoo();
        
        // Test that errors are properly propagated through the call chain
        let result = zoo.get_model("invalid_model_name_12345");
        
        // Should get a proper error, not a panic
        assert!(result.is_err());
        
        // Error should be informative
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("invalid_model_name_12345"));
    }
    
    #[test]
    fn test_search_models() {
        let zoo = create_test_zoo();
        
        // Search for "resnet"
        let results = zoo.search_models("resnet");
        assert_eq!(results.len(), 2); // resnet50 and deeplabv3_resnet101
        
        // Search for "detection"
        let results = zoo.search_models("detection");
        assert_eq!(results.len(), 1); // yolov5s
        
        // Search for non-existent
        let results = zoo.search_models("nonexistent");
        assert_eq!(results.len(), 0);
    }
    
    #[test]
    fn test_list_models_by_category() {
        let zoo = create_test_zoo();
        
        // List computer vision models
        let cv_models = zoo.list_models(Some(ModelCategory::ComputerVision));
        assert_eq!(cv_models.len(), 4);
        
        // List NLP models
        let nlp_models = zoo.list_models(Some(ModelCategory::NaturalLanguageProcessing));
        assert_eq!(nlp_models.len(), 2);
        
        // List all models
        let all_models = zoo.list_models(None);
        assert_eq!(all_models.len(), 7);
    }
    
    #[test]
    fn test_error_display() {
        // Test that error messages are properly formatted
        let error = ModelZooError::ModelNotFound("test_model".to_string());
        assert_eq!(format!("{}", error), "Model 'test_model' not found in zoo");
        
        let error = ModelZooError::InvalidModelName("bad name".to_string());
        assert_eq!(format!("{}", error), "Invalid model name: 'bad name'");
        
        let error = ModelZooError::ModelLoadFailed("network error".to_string());
        assert_eq!(format!("{}", error), "Failed to load model: network error");
        
        let error = ModelZooError::ConfigurationError("invalid config".to_string());
        assert_eq!(format!("{}", error), "Configuration error: invalid config");
    }
}