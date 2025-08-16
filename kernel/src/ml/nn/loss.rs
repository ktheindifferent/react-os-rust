// Loss functions for neural network training
use crate::ml::tensor::{Tensor, DType};
use alloc::vec::Vec;

// Base trait for loss functions
pub trait Loss {
    fn forward(&self, predictions: &Tensor, targets: &Tensor) -> Tensor;
    fn name(&self) -> &str;
}

// Mean Squared Error loss
pub struct MSELoss {
    reduction: Reduction,
}

#[derive(Clone, Copy)]
pub enum Reduction {
    None,
    Mean,
    Sum,
}

impl MSELoss {
    pub fn new(reduction: Reduction) -> Self {
        Self { reduction }
    }
}

impl Loss for MSELoss {
    fn forward(&self, predictions: &Tensor, targets: &Tensor) -> Tensor {
        assert_eq!(predictions.shape(), targets.shape(), "Shape mismatch");
        
        // Compute squared differences
        let diff = predictions.sub(targets);
        let squared = diff.mul(&diff);
        
        // Apply reduction
        match self.reduction {
            Reduction::None => squared,
            Reduction::Mean => {
                let numel = squared.numel() as f32;
                let sum = compute_sum(&squared);
                sum.div(&Tensor::new(vec![numel], vec![1]))
            },
            Reduction::Sum => compute_sum(&squared),
        }
    }
    
    fn name(&self) -> &str {
        "MSELoss"
    }
}

// Cross Entropy loss
pub struct CrossEntropyLoss {
    reduction: Reduction,
    label_smoothing: f32,
}

impl CrossEntropyLoss {
    pub fn new(reduction: Reduction, label_smoothing: f32) -> Self {
        assert!(label_smoothing >= 0.0 && label_smoothing <= 1.0);
        Self {
            reduction,
            label_smoothing,
        }
    }
}

impl Loss for CrossEntropyLoss {
    fn forward(&self, predictions: &Tensor, targets: &Tensor) -> Tensor {
        // predictions shape: [batch, num_classes]
        // targets shape: [batch] (class indices)
        
        let batch_size = predictions.shape()[0];
        let num_classes = predictions.shape()[1];
        
        // Apply softmax to get probabilities
        let probs = predictions.softmax(-1);
        
        // Apply label smoothing if needed
        let probs = if self.label_smoothing > 0.0 {
            let smooth_val = self.label_smoothing / num_classes as f32;
            let confidence = 1.0 - self.label_smoothing;
            
            // Smooth probabilities
            probs.mul(&Tensor::new(vec![confidence], vec![1]))
                .add(&Tensor::new(vec![smooth_val], vec![1]))
        } else {
            probs
        };
        
        // Compute negative log likelihood
        let mut losses = Vec::with_capacity(batch_size);
        
        let probs_data = probs.as_slice::<f32>();
        let targets_data = targets.as_slice::<f32>(); // Assuming targets are float indices
        
        for b in 0..batch_size {
            let target_idx = targets_data[b] as usize;
            let prob = probs_data[b * num_classes + target_idx];
            losses.push(-(prob.ln()));
        }
        
        let loss_tensor = Tensor::new(losses, vec![batch_size]);
        
        // Apply reduction
        match self.reduction {
            Reduction::None => loss_tensor,
            Reduction::Mean => {
                let sum = compute_sum(&loss_tensor);
                sum.div(&Tensor::new(vec![batch_size as f32], vec![1]))
            },
            Reduction::Sum => compute_sum(&loss_tensor),
        }
    }
    
    fn name(&self) -> &str {
        "CrossEntropyLoss"
    }
}

// Binary Cross Entropy loss
pub struct BCELoss {
    reduction: Reduction,
}

impl BCELoss {
    pub fn new(reduction: Reduction) -> Self {
        Self { reduction }
    }
}

impl Loss for BCELoss {
    fn forward(&self, predictions: &Tensor, targets: &Tensor) -> Tensor {
        assert_eq!(predictions.shape(), targets.shape(), "Shape mismatch");
        
        // BCE = -[y * log(p) + (1-y) * log(1-p)]
        let eps = 1e-7;
        let eps_tensor = Tensor::new(vec![eps], vec![1]);
        
        // Clamp predictions to avoid log(0)
        let pred_clamped = predictions.add(&eps_tensor);
        let one = Tensor::ones(&[1], predictions.dtype());
        let one_minus_pred = one.sub(predictions).add(&eps_tensor);
        
        // Compute loss components
        let pos_loss = targets.mul(&pred_clamped.unary_op(|x| x.ln()));
        let one_minus_targets = one.sub(targets);
        let neg_loss = one_minus_targets.mul(&one_minus_pred.unary_op(|x| x.ln()));
        
        // Combine and negate
        let loss = pos_loss.add(&neg_loss).mul(&Tensor::new(vec![-1.0], vec![1]));
        
        // Apply reduction
        match self.reduction {
            Reduction::None => loss,
            Reduction::Mean => {
                let numel = loss.numel() as f32;
                let sum = compute_sum(&loss);
                sum.div(&Tensor::new(vec![numel], vec![1]))
            },
            Reduction::Sum => compute_sum(&loss),
        }
    }
    
    fn name(&self) -> &str {
        "BCELoss"
    }
}

// Binary Cross Entropy with Logits loss
pub struct BCEWithLogitsLoss {
    reduction: Reduction,
    pos_weight: Option<Tensor>,
}

impl BCEWithLogitsLoss {
    pub fn new(reduction: Reduction, pos_weight: Option<Tensor>) -> Self {
        Self {
            reduction,
            pos_weight,
        }
    }
}

impl Loss for BCEWithLogitsLoss {
    fn forward(&self, predictions: &Tensor, targets: &Tensor) -> Tensor {
        assert_eq!(predictions.shape(), targets.shape(), "Shape mismatch");
        
        // More numerically stable version:
        // loss = max(x, 0) - x * y + log(1 + exp(-abs(x)))
        
        let zero = Tensor::zeros(&[1], predictions.dtype());
        let max_pred_zero = predictions.relu();
        let abs_pred = predictions.mul(&predictions.unary_op(|x| x.signum()));
        let exp_neg_abs = abs_pred.mul(&Tensor::new(vec![-1.0], vec![1])).unary_op(|x| x.exp());
        let log_term = exp_neg_abs.add(&Tensor::ones(&[1], predictions.dtype())).unary_op(|x| x.ln());
        
        let mut loss = max_pred_zero.sub(&predictions.mul(targets)).add(&log_term);
        
        // Apply positive weight if specified
        if let Some(ref pw) = self.pos_weight {
            let weighted_targets = targets.mul(pw);
            loss = loss.mul(&weighted_targets.add(&Tensor::ones(&[1], targets.dtype()).sub(targets)));
        }
        
        // Apply reduction
        match self.reduction {
            Reduction::None => loss,
            Reduction::Mean => {
                let numel = loss.numel() as f32;
                let sum = compute_sum(&loss);
                sum.div(&Tensor::new(vec![numel], vec![1]))
            },
            Reduction::Sum => compute_sum(&loss),
        }
    }
    
    fn name(&self) -> &str {
        "BCEWithLogitsLoss"
    }
}

// L1 loss (Mean Absolute Error)
pub struct L1Loss {
    reduction: Reduction,
}

impl L1Loss {
    pub fn new(reduction: Reduction) -> Self {
        Self { reduction }
    }
}

impl Loss for L1Loss {
    fn forward(&self, predictions: &Tensor, targets: &Tensor) -> Tensor {
        assert_eq!(predictions.shape(), targets.shape(), "Shape mismatch");
        
        // Compute absolute differences
        let diff = predictions.sub(targets);
        let abs_diff = diff.mul(&diff.unary_op(|x| x.signum()));
        
        // Apply reduction
        match self.reduction {
            Reduction::None => abs_diff,
            Reduction::Mean => {
                let numel = abs_diff.numel() as f32;
                let sum = compute_sum(&abs_diff);
                sum.div(&Tensor::new(vec![numel], vec![1]))
            },
            Reduction::Sum => compute_sum(&abs_diff),
        }
    }
    
    fn name(&self) -> &str {
        "L1Loss"
    }
}

// Smooth L1 loss (Huber loss)
pub struct SmoothL1Loss {
    reduction: Reduction,
    beta: f32,
}

impl SmoothL1Loss {
    pub fn new(reduction: Reduction, beta: f32) -> Self {
        Self { reduction, beta }
    }
}

impl Loss for SmoothL1Loss {
    fn forward(&self, predictions: &Tensor, targets: &Tensor) -> Tensor {
        assert_eq!(predictions.shape(), targets.shape(), "Shape mismatch");
        
        let diff = predictions.sub(targets);
        let abs_diff = diff.mul(&diff.unary_op(|x| x.signum()));
        
        // Smooth L1: 
        // if |x| < beta: 0.5 * x^2 / beta
        // else: |x| - 0.5 * beta
        
        let beta_tensor = Tensor::new(vec![self.beta], vec![1]);
        let half_beta = Tensor::new(vec![0.5 * self.beta], vec![1]);
        
        // Create mask for small differences
        let mask = abs_diff.div(&beta_tensor).unary_op(|x| if x < 1.0 { 1.0 } else { 0.0 });
        
        // Compute both branches
        let squared_loss = diff.mul(&diff).mul(&Tensor::new(vec![0.5 / self.beta], vec![1]));
        let linear_loss = abs_diff.sub(&half_beta);
        
        // Combine based on mask
        let loss = squared_loss.mul(&mask).add(&linear_loss.mul(&mask.mul(&Tensor::new(vec![-1.0], vec![1])).add(&Tensor::ones(&[1], diff.dtype()))));
        
        // Apply reduction
        match self.reduction {
            Reduction::None => loss,
            Reduction::Mean => {
                let numel = loss.numel() as f32;
                let sum = compute_sum(&loss);
                sum.div(&Tensor::new(vec![numel], vec![1]))
            },
            Reduction::Sum => compute_sum(&loss),
        }
    }
    
    fn name(&self) -> &str {
        "SmoothL1Loss"
    }
}

// Negative Log Likelihood loss
pub struct NLLLoss {
    reduction: Reduction,
    weight: Option<Tensor>,
}

impl NLLLoss {
    pub fn new(reduction: Reduction, weight: Option<Tensor>) -> Self {
        Self { reduction, weight }
    }
}

impl Loss for NLLLoss {
    fn forward(&self, predictions: &Tensor, targets: &Tensor) -> Tensor {
        // predictions shape: [batch, num_classes] (log probabilities)
        // targets shape: [batch] (class indices)
        
        let batch_size = predictions.shape()[0];
        let num_classes = predictions.shape()[1];
        
        let mut losses = Vec::with_capacity(batch_size);
        
        let pred_data = predictions.as_slice::<f32>();
        let targets_data = targets.as_slice::<f32>();
        
        for b in 0..batch_size {
            let target_idx = targets_data[b] as usize;
            let mut loss = -pred_data[b * num_classes + target_idx];
            
            // Apply class weight if specified
            if let Some(ref w) = self.weight {
                let weight_data = w.as_slice::<f32>();
                loss *= weight_data[target_idx];
            }
            
            losses.push(loss);
        }
        
        let loss_tensor = Tensor::new(losses, vec![batch_size]);
        
        // Apply reduction
        match self.reduction {
            Reduction::None => loss_tensor,
            Reduction::Mean => {
                let sum = compute_sum(&loss_tensor);
                let divisor = if self.weight.is_some() {
                    // Weight-adjusted mean
                    compute_sum(&loss_tensor.unary_op(|_| 1.0))
                } else {
                    Tensor::new(vec![batch_size as f32], vec![1])
                };
                sum.div(&divisor)
            },
            Reduction::Sum => compute_sum(&loss_tensor),
        }
    }
    
    fn name(&self) -> &str {
        "NLLLoss"
    }
}

// Cosine Embedding loss
pub struct CosineEmbeddingLoss {
    margin: f32,
    reduction: Reduction,
}

impl CosineEmbeddingLoss {
    pub fn new(margin: f32, reduction: Reduction) -> Self {
        Self { margin, reduction }
    }
}

impl Loss for CosineEmbeddingLoss {
    fn forward(&self, input1: &Tensor, input2: &Tensor) -> Tensor {
        // Compute cosine similarity
        let cos_sim = cosine_similarity(input1, input2, 1);
        
        // For similar pairs (target=1): loss = 1 - cos_sim
        // For dissimilar pairs (target=-1): loss = max(0, cos_sim - margin)
        
        // Simplified version assuming all pairs are similar
        let one = Tensor::ones(&[1], cos_sim.dtype());
        let loss = one.sub(&cos_sim);
        
        // Apply reduction
        match self.reduction {
            Reduction::None => loss,
            Reduction::Mean => {
                let numel = loss.numel() as f32;
                let sum = compute_sum(&loss);
                sum.div(&Tensor::new(vec![numel], vec![1]))
            },
            Reduction::Sum => compute_sum(&loss),
        }
    }
    
    fn name(&self) -> &str {
        "CosineEmbeddingLoss"
    }
}

// Triplet Margin loss
pub struct TripletMarginLoss {
    margin: f32,
    p: f32, // norm degree
    reduction: Reduction,
}

impl TripletMarginLoss {
    pub fn new(margin: f32, p: f32, reduction: Reduction) -> Self {
        Self { margin, p, reduction }
    }
}

impl Loss for TripletMarginLoss {
    fn forward(&self, anchor: &Tensor, positive: &Tensor) -> Tensor {
        // Note: negative sample would be third input
        // loss = max(0, d(a,p) - d(a,n) + margin)
        
        // Compute distances
        let dist_ap = pairwise_distance(anchor, positive, self.p);
        
        // For simplicity, using dist_ap as placeholder
        let margin_tensor = Tensor::new(vec![self.margin], vec![1]);
        let loss = dist_ap.add(&margin_tensor).relu();
        
        // Apply reduction
        match self.reduction {
            Reduction::None => loss,
            Reduction::Mean => {
                let numel = loss.numel() as f32;
                let sum = compute_sum(&loss);
                sum.div(&Tensor::new(vec![numel], vec![1]))
            },
            Reduction::Sum => compute_sum(&loss),
        }
    }
    
    fn name(&self) -> &str {
        "TripletMarginLoss"
    }
}

// Helper functions
fn compute_sum(tensor: &Tensor) -> Tensor {
    let data = tensor.as_slice::<f32>();
    let sum: f32 = data.iter().sum();
    Tensor::new(vec![sum], vec![1])
}

fn cosine_similarity(x1: &Tensor, x2: &Tensor, dim: usize) -> Tensor {
    // cos_sim = (x1 . x2) / (||x1|| * ||x2||)
    
    let dot_product = x1.mul(x2);
    let x1_norm = compute_norm(x1, 2.0);
    let x2_norm = compute_norm(x2, 2.0);
    
    dot_product.div(&x1_norm.mul(&x2_norm))
}

fn pairwise_distance(x1: &Tensor, x2: &Tensor, p: f32) -> Tensor {
    // Lp distance: ||x1 - x2||_p
    
    let diff = x1.sub(x2);
    compute_norm(&diff, p)
}

fn compute_norm(tensor: &Tensor, p: f32) -> Tensor {
    if p == 2.0 {
        // L2 norm
        let squared = tensor.mul(tensor);
        let sum = compute_sum(&squared);
        sum.unary_op(|x| x.sqrt())
    } else if p == 1.0 {
        // L1 norm
        let abs = tensor.mul(&tensor.unary_op(|x| x.signum()));
        compute_sum(&abs)
    } else {
        // Lp norm
        let powered = tensor.unary_op(|x| x.abs().powf(p));
        let sum = compute_sum(&powered);
        sum.unary_op(|x| x.powf(1.0 / p))
    }
}