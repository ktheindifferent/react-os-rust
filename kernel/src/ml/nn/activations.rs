// Activation functions for neural networks
use crate::ml::tensor::Tensor;
use super::{Module, Parameters};

// ReLU activation
pub struct ReLU;

impl Module for ReLU {
    fn forward(&mut self, input: &Tensor) -> Tensor {
        input.relu()
    }
    
    fn parameters(&self) -> Parameters {
        Parameters::new()
    }
    
    fn train(&mut self, _mode: bool) {}
    
    fn name(&self) -> &str {
        "ReLU"
    }
}

// Sigmoid activation
pub struct Sigmoid;

impl Module for Sigmoid {
    fn forward(&mut self, input: &Tensor) -> Tensor {
        input.sigmoid()
    }
    
    fn parameters(&self) -> Parameters {
        Parameters::new()
    }
    
    fn train(&mut self, _mode: bool) {}
    
    fn name(&self) -> &str {
        "Sigmoid"
    }
}

// Tanh activation
pub struct Tanh;

impl Module for Tanh {
    fn forward(&mut self, input: &Tensor) -> Tensor {
        input.tanh()
    }
    
    fn parameters(&self) -> Parameters {
        Parameters::new()
    }
    
    fn train(&mut self, _mode: bool) {}
    
    fn name(&self) -> &str {
        "Tanh"
    }
}

// Softmax activation
pub struct Softmax {
    dim: isize,
}

impl Softmax {
    pub fn new(dim: isize) -> Self {
        Self { dim }
    }
}

impl Module for Softmax {
    fn forward(&mut self, input: &Tensor) -> Tensor {
        input.softmax(self.dim)
    }
    
    fn parameters(&self) -> Parameters {
        Parameters::new()
    }
    
    fn train(&mut self, _mode: bool) {}
    
    fn name(&self) -> &str {
        "Softmax"
    }
}

// GELU activation
pub struct GELU;

impl Module for GELU {
    fn forward(&mut self, input: &Tensor) -> Tensor {
        // GELU(x) = 0.5 * x * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))
        use core::f32::consts::PI;
        
        let sqrt_2_over_pi = (2.0 / PI).sqrt();
        let c = 0.044715;
        
        // Compute x^3
        let x_cubed = input.mul(input).mul(input);
        
        // Compute inner expression
        let inner = input.add(&x_cubed.mul(&Tensor::new(vec![c], vec![1])));
        let inner = inner.mul(&Tensor::new(vec![sqrt_2_over_pi], vec![1]));
        
        // Apply tanh and complete GELU
        let tanh_part = inner.tanh();
        let one_plus_tanh = tanh_part.add(&Tensor::ones(&[1], input.dtype()));
        
        input.mul(&one_plus_tanh).mul(&Tensor::new(vec![0.5], vec![1]))
    }
    
    fn parameters(&self) -> Parameters {
        Parameters::new()
    }
    
    fn train(&mut self, _mode: bool) {}
    
    fn name(&self) -> &str {
        "GELU"
    }
}

// Leaky ReLU activation
pub struct LeakyReLU {
    negative_slope: f32,
}

impl LeakyReLU {
    pub fn new(negative_slope: f32) -> Self {
        Self { negative_slope }
    }
}

impl Module for LeakyReLU {
    fn forward(&mut self, input: &Tensor) -> Tensor {
        // LeakyReLU(x) = max(0, x) + negative_slope * min(0, x)
        let positive = input.relu();
        let negative = input.mul(&Tensor::new(vec![self.negative_slope], vec![1]));
        
        // Combine positive and negative parts
        let zero = Tensor::zeros(&[1], input.dtype());
        let mask = input.relu().div(input.add(&Tensor::new(vec![1e-10], vec![1]))); // Creates 0/1 mask
        
        positive.mul(&mask).add(&negative.mul(&mask.mul(&Tensor::new(vec![-1.0], vec![1])).add(&Tensor::ones(&[1], input.dtype()))))
    }
    
    fn parameters(&self) -> Parameters {
        Parameters::new()
    }
    
    fn train(&mut self, _mode: bool) {}
    
    fn name(&self) -> &str {
        "LeakyReLU"
    }
}

// ELU activation
pub struct ELU {
    alpha: f32,
}

impl ELU {
    pub fn new(alpha: f32) -> Self {
        Self { alpha }
    }
}

impl Module for ELU {
    fn forward(&mut self, input: &Tensor) -> Tensor {
        // ELU(x) = max(0, x) + min(0, alpha * (exp(x) - 1))
        let positive = input.relu();
        
        // Compute negative part: alpha * (exp(x) - 1) for x < 0
        let exp_x = input.unary_op(|x| x.exp());
        let exp_minus_one = exp_x.sub(&Tensor::ones(&[1], input.dtype()));
        let negative = exp_minus_one.mul(&Tensor::new(vec![self.alpha], vec![1]));
        
        // Combine based on sign
        let zero = Tensor::zeros(&[1], input.dtype());
        let mask = input.relu().div(input.add(&Tensor::new(vec![1e-10], vec![1])));
        
        positive.mul(&mask).add(&negative.mul(&mask.mul(&Tensor::new(vec![-1.0], vec![1])).add(&Tensor::ones(&[1], input.dtype()))))
    }
    
    fn parameters(&self) -> Parameters {
        Parameters::new()
    }
    
    fn train(&mut self, _mode: bool) {}
    
    fn name(&self) -> &str {
        "ELU"
    }
}

// Swish/SiLU activation
pub struct Swish;

impl Module for Swish {
    fn forward(&mut self, input: &Tensor) -> Tensor {
        // Swish(x) = x * sigmoid(x)
        input.mul(&input.sigmoid())
    }
    
    fn parameters(&self) -> Parameters {
        Parameters::new()
    }
    
    fn train(&mut self, _mode: bool) {}
    
    fn name(&self) -> &str {
        "Swish"
    }
}

// Mish activation
pub struct Mish;

impl Module for Mish {
    fn forward(&mut self, input: &Tensor) -> Tensor {
        // Mish(x) = x * tanh(softplus(x))
        // softplus(x) = ln(1 + exp(x))
        
        let exp_x = input.unary_op(|x| x.exp());
        let softplus = exp_x.add(&Tensor::ones(&[1], input.dtype())).unary_op(|x| x.ln());
        
        input.mul(&softplus.tanh())
    }
    
    fn parameters(&self) -> Parameters {
        Parameters::new()
    }
    
    fn train(&mut self, _mode: bool) {}
    
    fn name(&self) -> &str {
        "Mish"
    }
}