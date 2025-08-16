// Example: Image Classification with ResNet
use std::path::Path;
use std::collections::HashMap;

// Import ML framework components
use kernel::ml::{
    initialize, create_model, Tensor, DType,
    nn::{layers::*, activations::*, loss::*, Sequential, Model, SGD},
    training::{TrainingConfig, DataLoader},
    inference::{InferenceEngine, InferenceConfig, Precision, OptimizationLevel},
    accelerator::DeviceType,
    models::{ModelFormat, ModelConverter},
};

use crate::models::zoo::{ModelZoo, ModelBenchmark};
use crate::serving::{ModelServer, ServerConfig, PredictRequest, TensorData};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("AI/ML Framework Example: Image Classification");
    println!("=============================================\n");
    
    // Initialize ML framework
    let mut framework = initialize()?;
    println!("✓ ML Framework initialized");
    
    // Check available accelerators
    let device_count = framework.get_device_count();
    println!("✓ Found {} accelerator device(s)", device_count);
    
    if device_count > 0 {
        framework.set_device(0)?;
        let device_info = framework.get_device_info(0).unwrap();
        println!("  Using device: {} ({:?})", device_info.name, device_info.device_type);
        println!("  Memory: {:.1} GB", device_info.memory.total as f64 / (1024.0 * 1024.0 * 1024.0));
    }
    
    // Example 1: Build and train a custom CNN model
    println!("\n1. Building Custom CNN Model");
    println!("-----------------------------");
    
    let mut model = create_model()
        // First convolutional block
        .add(Conv2d::new(3, 64, 3, 1, 1, true))
        .add(BatchNorm2d::new(64, 1e-5, 0.1))
        .add(ReLU)
        .add(Conv2d::new(64, 64, 3, 1, 1, true))
        .add(BatchNorm2d::new(64, 1e-5, 0.1))
        .add(ReLU)
        // Pooling
        .add(MaxPool2d::new(2, 2))
        // Second convolutional block
        .add(Conv2d::new(64, 128, 3, 1, 1, true))
        .add(BatchNorm2d::new(128, 1e-5, 0.1))
        .add(ReLU)
        .add(Conv2d::new(128, 128, 3, 1, 1, true))
        .add(BatchNorm2d::new(128, 1e-5, 0.1))
        .add(ReLU)
        // Global average pooling (simplified as flatten)
        .add(Flatten)
        // Classifier
        .add(Linear::new(128 * 56 * 56, 256, true))
        .add(ReLU)
        .add(Dropout::new(0.5))
        .add(Linear::new(256, 10, true));
    
    println!("✓ Model architecture created");
    
    // Create dummy training data
    let train_images = Tensor::randn(&[100, 3, 224, 224], DType::Float32);
    let train_labels = Tensor::zeros(&[100], DType::Float32);
    
    // Training configuration
    let train_config = TrainingConfig {
        epochs: 10,
        batch_size: 32,
        learning_rate: 0.001,
        weight_decay: 0.0001,
        gradient_clip: Some(1.0),
        mixed_precision: false,
        device: DeviceType::CUDA,
        ..Default::default()
    };
    
    println!("✓ Training configuration set");
    println!("  Epochs: {}", train_config.epochs);
    println!("  Batch size: {}", train_config.batch_size);
    println!("  Learning rate: {}", train_config.learning_rate);
    
    // Example 2: Load pre-trained model from Model Zoo
    println!("\n2. Loading Pre-trained Model");
    println!("-----------------------------");
    
    let mut model_zoo = ModelZoo::new(Path::new("/tmp/model_cache").to_path_buf())?;
    
    // List available models
    let cv_models = model_zoo.list_models(Some(ModelCategory::ComputerVision));
    println!("Available computer vision models:");
    for model in cv_models {
        println!("  - {}: {} ({:.1} MB)", model.name, model.description, model.size_mb);
    }
    
    // Load ResNet-50
    let resnet50 = model_zoo.get_model("resnet50")?;
    println!("✓ Loaded ResNet-50 model");
    println!("  Input shape: {:?}", resnet50.config.input_size);
    println!("  Output shape: {:?}", resnet50.config.output_size);
    
    // Example 3: Model optimization for inference
    println!("\n3. Optimizing Model for Inference");
    println!("----------------------------------");
    
    let inference_config = InferenceConfig {
        batch_size: 1,
        num_threads: 4,
        device: DeviceType::CUDA,
        precision: Precision::FP16,
        enable_profiling: true,
        enable_caching: true,
        optimization_level: OptimizationLevel::Maximum,
        ..Default::default()
    };
    
    let mut inference_engine = InferenceEngine::new(
        Box::new(model),
        inference_config
    )?;
    
    println!("✓ Inference engine created with optimizations:");
    println!("  - Operator fusion");
    println!("  - Memory layout optimization");
    println!("  - FP16 precision");
    println!("  - Kernel auto-tuning");
    
    // Benchmark inference
    println!("\n4. Benchmarking Model Performance");
    println!("---------------------------------");
    
    let mut benchmark = ModelBenchmark::new();
    let results = benchmark.benchmark_model(
        "resnet50",
        "CUDA",
        &[1, 4, 8, 16, 32],
        100
    );
    
    println!("Benchmark Results:");
    for result in results {
        println!("  Batch {}: {:.1} samples/sec, {:.2}ms latency (P50: {:.2}ms, P99: {:.2}ms)",
            result.batch_size,
            result.throughput,
            result.latency_mean,
            result.latency_p50,
            result.latency_p99
        );
    }
    
    // Example 5: Model serving
    println!("\n5. Starting Model Server");
    println!("------------------------");
    
    let server_config = ServerConfig {
        host: "0.0.0.0".to_string(),
        port: 8080,
        num_workers: 4,
        enable_metrics: true,
        enable_model_versioning: true,
        ..Default::default()
    };
    
    let server = ModelServer::new(server_config);
    
    // Register model
    server.register_model(
        "resnet50".to_string(),
        "v1".to_string(),
        Box::new(DummyModel)
    )?;
    
    println!("✓ Model server configured");
    println!("  Endpoint: http://0.0.0.0:8080/predict");
    println!("  Models: resnet50:v1");
    
    // Example prediction request
    let mut request = PredictRequest {
        id: "req_001".to_string(),
        model_name: "resnet50".to_string(),
        model_version: Some("v1".to_string()),
        inputs: HashMap::new(),
        parameters: None,
    };
    
    // Add input tensor
    request.inputs.insert(
        "image".to_string(),
        TensorData {
            shape: vec![1, 3, 224, 224],
            dtype: "float32".to_string(),
            data: TensorValues::Float32(vec![0.0; 3 * 224 * 224]),
        }
    );
    
    println!("\n✓ Example prediction request created");
    println!("  Request ID: {}", request.id);
    println!("  Model: {}:{}", request.model_name, request.model_version.as_ref().unwrap());
    
    // Example 6: Model conversion
    println!("\n6. Model Format Conversion");
    println!("-------------------------");
    
    // Convert between formats
    ModelConverter::convert(
        ModelFormat::ONNX,
        ModelFormat::TensorRT,
        "models/resnet50.onnx",
        "models/resnet50.trt"
    )?;
    
    println!("✓ Converted ResNet-50 from ONNX to TensorRT");
    
    // Example 7: AutoML
    println!("\n7. AutoML Hyperparameter Search");
    println!("-------------------------------");
    
    let mut automl = AutoML::new();
    let best_config = automl.search("imagenet", "classification");
    
    println!("✓ AutoML search completed");
    println!("  Best architecture: {}", best_config.parameters.get("architecture").unwrap());
    println!("  Best learning rate: {}", best_config.parameters.get("learning_rate").unwrap());
    println!("  Best batch size: {}", best_config.parameters.get("batch_size").unwrap());
    println!("  Validation accuracy: {:.2}%", best_config.score * 100.0);
    
    println!("\n✓ All examples completed successfully!");
    
    Ok(())
}

// Dummy model for serving example
struct DummyModel;

impl crate::serving::Model for DummyModel {
    fn predict(&self, input: &PredictRequest) -> Result<PredictResponse, ModelError> {
        Ok(PredictResponse {
            id: input.id.clone(),
            model_name: input.model_name.clone(),
            model_version: "v1".to_string(),
            outputs: HashMap::new(),
            metadata: None,
        })
    }
    
    fn batch_predict(&self, inputs: &[PredictRequest]) -> Result<Vec<PredictResponse>, ModelError> {
        inputs.iter().map(|input| self.predict(input)).collect()
    }
    
    fn get_info(&self) -> ModelInfo {
        ModelInfo {
            name: "dummy".to_string(),
            version: "v1".to_string(),
            framework: "native".to_string(),
            inputs: vec![],
            outputs: vec![],
            metadata: HashMap::new(),
        }
    }
}

// Flatten layer (simplified)
struct Flatten;

impl Module for Flatten {
    fn forward(&mut self, input: &Tensor) -> Tensor {
        let batch_size = input.shape()[0];
        let numel_per_batch = input.numel() / batch_size;
        input.reshape(&[batch_size, numel_per_batch])
    }
    
    fn parameters(&self) -> Parameters {
        Parameters::new()
    }
    
    fn train(&mut self, _mode: bool) {}
    
    fn name(&self) -> &str {
        "Flatten"
    }
}

// MaxPool2d layer (simplified)
struct MaxPool2d {
    kernel_size: usize,
    stride: usize,
}

impl MaxPool2d {
    fn new(kernel_size: usize, stride: usize) -> Self {
        Self { kernel_size, stride }
    }
}

impl Module for MaxPool2d {
    fn forward(&mut self, input: &Tensor) -> Tensor {
        input.max_pool2d(self.kernel_size, self.stride)
    }
    
    fn parameters(&self) -> Parameters {
        Parameters::new()
    }
    
    fn train(&mut self, _mode: bool) {}
    
    fn name(&self) -> &str {
        "MaxPool2d"
    }
}