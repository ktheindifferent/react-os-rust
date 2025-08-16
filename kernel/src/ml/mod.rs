// Machine Learning Framework
//
// A comprehensive ML framework with neural network support, hardware acceleration,
// and model deployment capabilities.

pub mod tensor;
pub mod nn;
pub mod training;
pub mod inference;
pub mod accelerator;
pub mod models;
pub mod ops;

use alloc::{vec::Vec, string::String, boxed::Box};

// Re-export commonly used types
pub use tensor::{Tensor, DType, Storage};
pub use nn::{Module, Sequential, Model};
pub use training::{TrainingConfig, Dataset, DataLoader};
pub use inference::{InferenceEngine, InferenceConfig};
pub use accelerator::{Accelerator, AcceleratorManager, DeviceType};
pub use models::{ModelFormat, ModelLoader, ModelConverter};

// Framework version
pub const VERSION: &str = "1.0.0";

// Initialize ML framework
pub fn initialize() -> Result<MLFramework, MLError> {
    let mut framework = MLFramework::new();
    framework.initialize()?;
    Ok(framework)
}

// Main ML framework struct
pub struct MLFramework {
    accelerator_manager: AcceleratorManager,
    initialized: bool,
}

impl MLFramework {
    pub fn new() -> Self {
        Self {
            accelerator_manager: AcceleratorManager::new(),
            initialized: false,
        }
    }
    
    pub fn initialize(&mut self) -> Result<(), MLError> {
        if self.initialized {
            return Ok(());
        }
        
        // Initialize accelerators
        if let Err(e) = self.accelerator_manager.initialize() {
            log::warn!("Failed to initialize accelerators: {:?}", e);
            // Continue without acceleration
        }
        
        self.initialized = true;
        log::info!("ML Framework v{} initialized", VERSION);
        
        Ok(())
    }
    
    pub fn get_device_count(&self) -> usize {
        self.accelerator_manager.get_device_count()
    }
    
    pub fn set_device(&mut self, device_id: usize) -> Result<(), MLError> {
        self.accelerator_manager.set_device(device_id)
            .map_err(|e| MLError::AcceleratorError(format!("{:?}", e)))
    }
    
    pub fn get_device_info(&self, device_id: usize) -> Option<accelerator::DeviceInfo> {
        self.accelerator_manager.get_device_info(device_id)
    }
}

// ML framework errors
#[derive(Debug)]
pub enum MLError {
    InitializationError(String),
    AcceleratorError(String),
    ModelError(String),
    TrainingError(String),
    InferenceError(String),
}

// Convenience functions for common operations

/// Create a neural network model
pub fn create_model() -> nn::Sequential {
    nn::Sequential::new()
}

/// Load a model from file
pub fn load_model(path: &str, format: ModelFormat) -> Result<Box<dyn Module>, MLError> {
    let loader = match format {
        ModelFormat::ONNX => Box::new(models::onnx::ONNXLoader::new()),
        _ => return Err(MLError::ModelError(format!("Unsupported format: {:?}", format))),
    };
    
    loader.load(path)
        .map_err(|e| MLError::ModelError(format!("{:?}", e)))
}

/// Save a model to file
pub fn save_model(model: &dyn Module, path: &str, format: ModelFormat) -> Result<(), MLError> {
    let saver = match format {
        ModelFormat::ONNX => Box::new(models::onnx::ONNXLoader::new()),
        _ => return Err(MLError::ModelError(format!("Unsupported format: {:?}", format))),
    };
    
    saver.save(model, path)
        .map_err(|e| MLError::ModelError(format!("{:?}", e)))
}

// Example usage and tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::nn::layers::{Linear, Conv2d};
    use crate::ml::nn::activations::ReLU;
    
    #[test]
    fn test_framework_initialization() {
        let framework = initialize();
        assert!(framework.is_ok());
    }
    
    #[test]
    fn test_tensor_operations() {
        let a = Tensor::ones(&[2, 3], DType::Float32);
        let b = Tensor::ones(&[2, 3], DType::Float32);
        let c = a.add(&b);
        
        assert_eq!(c.shape(), &[2, 3]);
    }
    
    #[test]
    fn test_model_creation() {
        let model = create_model()
            .add(Linear::new(784, 128, true))
            .add(ReLU)
            .add(Linear::new(128, 10, true));
        
        let input = Tensor::randn(&[32, 784], DType::Float32);
        let output = model.forward(&input);
        
        assert_eq!(output.shape(), &[32, 10]);
    }
    
    #[test]
    fn test_convolution() {
        let conv = Conv2d::new(3, 64, 3, 1, 1, true);
        let input = Tensor::randn(&[1, 3, 224, 224], DType::Float32);
        let output = conv.forward(&input);
        
        assert_eq!(output.shape(), &[1, 64, 224, 224]);
    }
}

// Logging support
mod log {
    use alloc::format;
    
    pub fn info(msg: &str) {
        #[cfg(feature = "std")]
        println!("[INFO] {}", msg);
    }
    
    pub fn warn(msg: &str) {
        #[cfg(feature = "std")]
        println!("[WARN] {}", msg);
    }
    
    pub fn error(msg: &str) {
        #[cfg(feature = "std")]
        eprintln!("[ERROR] {}", msg);
    }
}