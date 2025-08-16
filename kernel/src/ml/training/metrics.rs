// Training metrics and monitoring
use alloc::{vec::Vec, collections::BTreeMap, string::String};
use crate::ml::tensor::Tensor;

// Base trait for metrics
pub trait Metric {
    fn update(&mut self, predictions: &Tensor, targets: &Tensor);
    fn compute(&self) -> f32;
    fn reset(&mut self);
    fn name(&self) -> &str;
}

// Accuracy metric
pub struct Accuracy {
    correct: usize,
    total: usize,
}

impl Accuracy {
    pub fn new() -> Self {
        Self { correct: 0, total: 0 }
    }
}

impl Metric for Accuracy {
    fn update(&mut self, predictions: &Tensor, targets: &Tensor) {
        let pred_data = predictions.as_slice::<f32>();
        let target_data = targets.as_slice::<f32>();
        
        for i in 0..predictions.shape()[0] {
            let pred_class = argmax(&pred_data[i * predictions.shape()[1]..(i + 1) * predictions.shape()[1]]);
            let target_class = target_data[i] as usize;
            
            if pred_class == target_class {
                self.correct += 1;
            }
            self.total += 1;
        }
    }
    
    fn compute(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.correct as f32 / self.total as f32
        }
    }
    
    fn reset(&mut self) {
        self.correct = 0;
        self.total = 0;
    }
    
    fn name(&self) -> &str {
        "accuracy"
    }
}

// Precision metric
pub struct Precision {
    true_positives: Vec<usize>,
    false_positives: Vec<usize>,
    num_classes: usize,
}

impl Precision {
    pub fn new(num_classes: usize) -> Self {
        Self {
            true_positives: vec![0; num_classes],
            false_positives: vec![0; num_classes],
            num_classes,
        }
    }
}

impl Metric for Precision {
    fn update(&mut self, predictions: &Tensor, targets: &Tensor) {
        let pred_data = predictions.as_slice::<f32>();
        let target_data = targets.as_slice::<f32>();
        
        for i in 0..predictions.shape()[0] {
            let pred_class = argmax(&pred_data[i * self.num_classes..(i + 1) * self.num_classes]);
            let target_class = target_data[i] as usize;
            
            if pred_class == target_class {
                self.true_positives[pred_class] += 1;
            } else {
                self.false_positives[pred_class] += 1;
            }
        }
    }
    
    fn compute(&self) -> f32 {
        let mut sum = 0.0;
        let mut count = 0;
        
        for i in 0..self.num_classes {
            let tp = self.true_positives[i] as f32;
            let fp = self.false_positives[i] as f32;
            
            if tp + fp > 0.0 {
                sum += tp / (tp + fp);
                count += 1;
            }
        }
        
        if count > 0 {
            sum / count as f32
        } else {
            0.0
        }
    }
    
    fn reset(&mut self) {
        self.true_positives.fill(0);
        self.false_positives.fill(0);
    }
    
    fn name(&self) -> &str {
        "precision"
    }
}

// Recall metric
pub struct Recall {
    true_positives: Vec<usize>,
    false_negatives: Vec<usize>,
    num_classes: usize,
}

impl Recall {
    pub fn new(num_classes: usize) -> Self {
        Self {
            true_positives: vec![0; num_classes],
            false_negatives: vec![0; num_classes],
            num_classes,
        }
    }
}

impl Metric for Recall {
    fn update(&mut self, predictions: &Tensor, targets: &Tensor) {
        let pred_data = predictions.as_slice::<f32>();
        let target_data = targets.as_slice::<f32>();
        
        for i in 0..predictions.shape()[0] {
            let pred_class = argmax(&pred_data[i * self.num_classes..(i + 1) * self.num_classes]);
            let target_class = target_data[i] as usize;
            
            if pred_class == target_class {
                self.true_positives[target_class] += 1;
            } else {
                self.false_negatives[target_class] += 1;
            }
        }
    }
    
    fn compute(&self) -> f32 {
        let mut sum = 0.0;
        let mut count = 0;
        
        for i in 0..self.num_classes {
            let tp = self.true_positives[i] as f32;
            let fn_val = self.false_negatives[i] as f32;
            
            if tp + fn_val > 0.0 {
                sum += tp / (tp + fn_val);
                count += 1;
            }
        }
        
        if count > 0 {
            sum / count as f32
        } else {
            0.0
        }
    }
    
    fn reset(&mut self) {
        self.true_positives.fill(0);
        self.false_negatives.fill(0);
    }
    
    fn name(&self) -> &str {
        "recall"
    }
}

// F1 Score
pub struct F1Score {
    precision: Precision,
    recall: Recall,
}

impl F1Score {
    pub fn new(num_classes: usize) -> Self {
        Self {
            precision: Precision::new(num_classes),
            recall: Recall::new(num_classes),
        }
    }
}

impl Metric for F1Score {
    fn update(&mut self, predictions: &Tensor, targets: &Tensor) {
        self.precision.update(predictions, targets);
        self.recall.update(predictions, targets);
    }
    
    fn compute(&self) -> f32 {
        let p = self.precision.compute();
        let r = self.recall.compute();
        
        if p + r > 0.0 {
            2.0 * p * r / (p + r)
        } else {
            0.0
        }
    }
    
    fn reset(&mut self) {
        self.precision.reset();
        self.recall.reset();
    }
    
    fn name(&self) -> &str {
        "f1_score"
    }
}

// Mean Absolute Error
pub struct MAE {
    sum_error: f32,
    count: usize,
}

impl MAE {
    pub fn new() -> Self {
        Self {
            sum_error: 0.0,
            count: 0,
        }
    }
}

impl Metric for MAE {
    fn update(&mut self, predictions: &Tensor, targets: &Tensor) {
        let pred_data = predictions.as_slice::<f32>();
        let target_data = targets.as_slice::<f32>();
        
        for i in 0..predictions.numel() {
            self.sum_error += (pred_data[i] - target_data[i]).abs();
            self.count += 1;
        }
    }
    
    fn compute(&self) -> f32 {
        if self.count == 0 {
            0.0
        } else {
            self.sum_error / self.count as f32
        }
    }
    
    fn reset(&mut self) {
        self.sum_error = 0.0;
        self.count = 0;
    }
    
    fn name(&self) -> &str {
        "mae"
    }
}

// Confusion Matrix
pub struct ConfusionMatrix {
    matrix: Vec<Vec<usize>>,
    num_classes: usize,
}

impl ConfusionMatrix {
    pub fn new(num_classes: usize) -> Self {
        Self {
            matrix: vec![vec![0; num_classes]; num_classes],
            num_classes,
        }
    }
    
    pub fn update(&mut self, predictions: &Tensor, targets: &Tensor) {
        let pred_data = predictions.as_slice::<f32>();
        let target_data = targets.as_slice::<f32>();
        
        for i in 0..predictions.shape()[0] {
            let pred_class = argmax(&pred_data[i * self.num_classes..(i + 1) * self.num_classes]);
            let target_class = target_data[i] as usize;
            
            self.matrix[target_class][pred_class] += 1;
        }
    }
    
    pub fn get_matrix(&self) -> &Vec<Vec<usize>> {
        &self.matrix
    }
    
    pub fn reset(&mut self) {
        for row in &mut self.matrix {
            row.fill(0);
        }
    }
}

// Metric collection
pub struct MetricCollection {
    metrics: BTreeMap<String, Box<dyn Metric>>,
}

impl MetricCollection {
    pub fn new() -> Self {
        Self {
            metrics: BTreeMap::new(),
        }
    }
    
    pub fn add_metric(&mut self, name: String, metric: Box<dyn Metric>) {
        self.metrics.insert(name, metric);
    }
    
    pub fn update(&mut self, predictions: &Tensor, targets: &Tensor) {
        for metric in self.metrics.values_mut() {
            metric.update(predictions, targets);
        }
    }
    
    pub fn compute(&self) -> BTreeMap<String, f32> {
        self.metrics.iter()
            .map(|(name, metric)| (name.clone(), metric.compute()))
            .collect()
    }
    
    pub fn reset(&mut self) {
        for metric in self.metrics.values_mut() {
            metric.reset();
        }
    }
}

// Helper function to find argmax
fn argmax(slice: &[f32]) -> usize {
    slice.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}