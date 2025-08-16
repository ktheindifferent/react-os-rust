// Tensor operations and N-dimensional array support
use alloc::{vec::Vec, boxed::Box, string::String};
use core::{ops::{Add, Sub, Mul, Div, Index, IndexMut}, fmt, slice};

// Tensor data types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DType {
    Float32,
    Float64,
    Int32,
    Int64,
    UInt8,
    Bool,
}

impl DType {
    pub fn size(&self) -> usize {
        match self {
            DType::Float32 | DType::Int32 => 4,
            DType::Float64 | DType::Int64 => 8,
            DType::UInt8 | DType::Bool => 1,
        }
    }
}

// Tensor storage backend
#[derive(Debug, Clone)]
pub enum Storage {
    Cpu(Vec<u8>),
    Cuda(CudaBuffer),
    OpenCL(OpenCLBuffer),
    Vulkan(VulkanBuffer),
}

// GPU buffer wrappers
#[derive(Debug, Clone)]
pub struct CudaBuffer {
    ptr: usize,
    size: usize,
    device_id: u32,
}

#[derive(Debug, Clone)]
pub struct OpenCLBuffer {
    ptr: usize,
    size: usize,
    context: usize,
}

#[derive(Debug, Clone)]
pub struct VulkanBuffer {
    buffer: usize,
    memory: usize,
    size: usize,
}

// N-dimensional tensor
#[derive(Clone)]
pub struct Tensor {
    data: Storage,
    shape: Vec<usize>,
    strides: Vec<usize>,
    dtype: DType,
    requires_grad: bool,
    grad: Option<Box<Tensor>>,
}

impl Tensor {
    // Create new tensor from data
    pub fn new(data: Vec<f32>, shape: Vec<usize>) -> Self {
        let numel: usize = shape.iter().product();
        assert_eq!(data.len(), numel, "Data size doesn't match shape");
        
        let strides = Self::compute_strides(&shape);
        let bytes = unsafe {
            slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4).to_vec()
        };
        
        Self {
            data: Storage::Cpu(bytes),
            shape,
            strides,
            dtype: DType::Float32,
            requires_grad: false,
            grad: None,
        }
    }
    
    // Create zeros tensor
    pub fn zeros(shape: &[usize], dtype: DType) -> Self {
        let numel: usize = shape.iter().product();
        let size = numel * dtype.size();
        
        Self {
            data: Storage::Cpu(vec![0u8; size]),
            shape: shape.to_vec(),
            strides: Self::compute_strides(shape),
            dtype,
            requires_grad: false,
            grad: None,
        }
    }
    
    // Create ones tensor
    pub fn ones(shape: &[usize], dtype: DType) -> Self {
        let numel: usize = shape.iter().product();
        let mut tensor = Self::zeros(shape, dtype);
        
        match dtype {
            DType::Float32 => {
                if let Storage::Cpu(ref mut data) = tensor.data {
                    let float_data = unsafe {
                        slice::from_raw_parts_mut(data.as_mut_ptr() as *mut f32, numel)
                    };
                    float_data.fill(1.0);
                }
            },
            DType::Float64 => {
                if let Storage::Cpu(ref mut data) = tensor.data {
                    let float_data = unsafe {
                        slice::from_raw_parts_mut(data.as_mut_ptr() as *mut f64, numel)
                    };
                    float_data.fill(1.0);
                }
            },
            _ => panic!("Unsupported dtype for ones"),
        }
        
        tensor
    }
    
    // Create random tensor
    pub fn randn(shape: &[usize], dtype: DType) -> Self {
        let numel: usize = shape.iter().product();
        let mut tensor = Self::zeros(shape, dtype);
        
        // Simple LCG for random numbers (Box-Muller transform for normal distribution)
        let mut seed = 12345u64;
        
        match dtype {
            DType::Float32 => {
                if let Storage::Cpu(ref mut data) = tensor.data {
                    let float_data = unsafe {
                        slice::from_raw_parts_mut(data.as_mut_ptr() as *mut f32, numel)
                    };
                    
                    for i in 0..numel/2 {
                        seed = (seed.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fffffff;
                        let u1 = (seed as f32) / 0x7fffffff as f32;
                        seed = (seed.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fffffff;
                        let u2 = (seed as f32) / 0x7fffffff as f32;
                        
                        let r = (-2.0 * u1.ln()).sqrt();
                        let theta = 2.0 * core::f32::consts::PI * u2;
                        
                        float_data[i*2] = r * theta.cos();
                        if i*2 + 1 < numel {
                            float_data[i*2 + 1] = r * theta.sin();
                        }
                    }
                }
            },
            _ => panic!("Unsupported dtype for randn"),
        }
        
        tensor
    }
    
    // Compute strides from shape
    fn compute_strides(shape: &[usize]) -> Vec<usize> {
        let mut strides = vec![1; shape.len()];
        for i in (0..shape.len() - 1).rev() {
            strides[i] = strides[i + 1] * shape[i + 1];
        }
        strides
    }
    
    // Get shape
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }
    
    // Get number of elements
    pub fn numel(&self) -> usize {
        self.shape.iter().product()
    }
    
    // Get data type
    pub fn dtype(&self) -> DType {
        self.dtype
    }
    
    // Enable gradient computation
    pub fn requires_grad_(mut self, requires_grad: bool) -> Self {
        self.requires_grad = requires_grad;
        self
    }
    
    // Reshape tensor
    pub fn reshape(&self, new_shape: &[usize]) -> Self {
        let new_numel: usize = new_shape.iter().product();
        assert_eq!(self.numel(), new_numel, "Cannot reshape: element count mismatch");
        
        Self {
            data: self.data.clone(),
            shape: new_shape.to_vec(),
            strides: Self::compute_strides(new_shape),
            dtype: self.dtype,
            requires_grad: self.requires_grad,
            grad: None,
        }
    }
    
    // Transpose tensor (2D only for now)
    pub fn transpose(&self) -> Self {
        assert_eq!(self.shape.len(), 2, "Transpose only supports 2D tensors");
        
        let new_shape = vec![self.shape[1], self.shape[0]];
        let new_strides = vec![self.strides[1], self.strides[0]];
        
        Self {
            data: self.data.clone(),
            shape: new_shape,
            strides: new_strides,
            dtype: self.dtype,
            requires_grad: self.requires_grad,
            grad: None,
        }
    }
    
    // Matrix multiplication
    pub fn matmul(&self, other: &Tensor) -> Tensor {
        assert_eq!(self.shape.len(), 2, "First tensor must be 2D");
        assert_eq!(other.shape.len(), 2, "Second tensor must be 2D");
        assert_eq!(self.shape[1], other.shape[0], "Matrix dimensions don't match");
        assert_eq!(self.dtype, other.dtype, "Data types must match");
        
        let m = self.shape[0];
        let k = self.shape[1];
        let n = other.shape[1];
        
        let mut result = Tensor::zeros(&[m, n], self.dtype);
        
        match self.dtype {
            DType::Float32 => {
                let a_data = self.as_slice::<f32>();
                let b_data = other.as_slice::<f32>();
                let c_data = result.as_mut_slice::<f32>();
                
                // Simple matrix multiplication
                for i in 0..m {
                    for j in 0..n {
                        let mut sum = 0.0f32;
                        for l in 0..k {
                            sum += a_data[i * k + l] * b_data[l * n + j];
                        }
                        c_data[i * n + j] = sum;
                    }
                }
            },
            _ => panic!("Unsupported dtype for matmul"),
        }
        
        result
    }
    
    // Convolution operation (2D)
    pub fn conv2d(&self, kernel: &Tensor, stride: usize, padding: usize) -> Tensor {
        assert_eq!(self.shape.len(), 4, "Input must be 4D (NCHW)");
        assert_eq!(kernel.shape.len(), 4, "Kernel must be 4D (OCHW)");
        
        let batch = self.shape[0];
        let in_channels = self.shape[1];
        let in_height = self.shape[2];
        let in_width = self.shape[3];
        
        let out_channels = kernel.shape[0];
        let kernel_height = kernel.shape[2];
        let kernel_width = kernel.shape[3];
        
        assert_eq!(kernel.shape[1], in_channels, "Kernel input channels mismatch");
        
        let out_height = (in_height + 2 * padding - kernel_height) / stride + 1;
        let out_width = (in_width + 2 * padding - kernel_width) / stride + 1;
        
        let mut output = Tensor::zeros(&[batch, out_channels, out_height, out_width], self.dtype);
        
        // Perform convolution
        match self.dtype {
            DType::Float32 => {
                let input_data = self.as_slice::<f32>();
                let kernel_data = kernel.as_slice::<f32>();
                let output_data = output.as_mut_slice::<f32>();
                
                for b in 0..batch {
                    for oc in 0..out_channels {
                        for oh in 0..out_height {
                            for ow in 0..out_width {
                                let mut sum = 0.0f32;
                                
                                for ic in 0..in_channels {
                                    for kh in 0..kernel_height {
                                        for kw in 0..kernel_width {
                                            let ih = oh * stride + kh;
                                            let iw = ow * stride + kw;
                                            
                                            if ih >= padding && ih < in_height + padding &&
                                               iw >= padding && iw < in_width + padding {
                                                let ih_actual = ih - padding;
                                                let iw_actual = iw - padding;
                                                
                                                let input_idx = b * in_channels * in_height * in_width +
                                                              ic * in_height * in_width +
                                                              ih_actual * in_width + iw_actual;
                                                let kernel_idx = oc * in_channels * kernel_height * kernel_width +
                                                               ic * kernel_height * kernel_width +
                                                               kh * kernel_width + kw;
                                                
                                                sum += input_data[input_idx] * kernel_data[kernel_idx];
                                            }
                                        }
                                    }
                                }
                                
                                let output_idx = b * out_channels * out_height * out_width +
                                               oc * out_height * out_width +
                                               oh * out_width + ow;
                                output_data[output_idx] = sum;
                            }
                        }
                    }
                }
            },
            _ => panic!("Unsupported dtype for conv2d"),
        }
        
        output
    }
    
    // Max pooling operation (2D)
    pub fn max_pool2d(&self, kernel_size: usize, stride: usize) -> Tensor {
        assert_eq!(self.shape.len(), 4, "Input must be 4D (NCHW)");
        
        let batch = self.shape[0];
        let channels = self.shape[1];
        let in_height = self.shape[2];
        let in_width = self.shape[3];
        
        let out_height = (in_height - kernel_size) / stride + 1;
        let out_width = (in_width - kernel_size) / stride + 1;
        
        let mut output = Tensor::zeros(&[batch, channels, out_height, out_width], self.dtype);
        
        match self.dtype {
            DType::Float32 => {
                let input_data = self.as_slice::<f32>();
                let output_data = output.as_mut_slice::<f32>();
                
                for b in 0..batch {
                    for c in 0..channels {
                        for oh in 0..out_height {
                            for ow in 0..out_width {
                                let mut max_val = f32::NEG_INFINITY;
                                
                                for kh in 0..kernel_size {
                                    for kw in 0..kernel_size {
                                        let ih = oh * stride + kh;
                                        let iw = ow * stride + kw;
                                        
                                        let input_idx = b * channels * in_height * in_width +
                                                      c * in_height * in_width +
                                                      ih * in_width + iw;
                                        
                                        max_val = max_val.max(input_data[input_idx]);
                                    }
                                }
                                
                                let output_idx = b * channels * out_height * out_width +
                                               c * out_height * out_width +
                                               oh * out_width + ow;
                                output_data[output_idx] = max_val;
                            }
                        }
                    }
                }
            },
            _ => panic!("Unsupported dtype for max_pool2d"),
        }
        
        output
    }
    
    // Get data as slice
    pub fn as_slice<T>(&self) -> &[T] {
        match &self.data {
            Storage::Cpu(data) => unsafe {
                slice::from_raw_parts(data.as_ptr() as *const T, self.numel())
            },
            _ => panic!("GPU tensors not yet supported for as_slice"),
        }
    }
    
    // Get mutable data as slice
    pub fn as_mut_slice<T>(&mut self) -> &mut [T] {
        match &mut self.data {
            Storage::Cpu(data) => unsafe {
                slice::from_raw_parts_mut(data.as_mut_ptr() as *mut T, self.numel())
            },
            _ => panic!("GPU tensors not yet supported for as_mut_slice"),
        }
    }
    
    // Element-wise operations
    pub fn add(&self, other: &Tensor) -> Tensor {
        self.binary_op(other, |a, b| a + b)
    }
    
    pub fn sub(&self, other: &Tensor) -> Tensor {
        self.binary_op(other, |a, b| a - b)
    }
    
    pub fn mul(&self, other: &Tensor) -> Tensor {
        self.binary_op(other, |a, b| a * b)
    }
    
    pub fn div(&self, other: &Tensor) -> Tensor {
        self.binary_op(other, |a, b| a / b)
    }
    
    // Generic binary operation with broadcasting
    fn binary_op<F>(&self, other: &Tensor, op: F) -> Tensor
    where
        F: Fn(f32, f32) -> f32,
    {
        assert_eq!(self.dtype, other.dtype, "Data types must match");
        
        // Check for broadcasting compatibility
        let broadcast_shape = self.broadcast_shape(&other.shape);
        let mut result = Tensor::zeros(&broadcast_shape, self.dtype);
        
        match self.dtype {
            DType::Float32 => {
                let a_data = self.as_slice::<f32>();
                let b_data = other.as_slice::<f32>();
                let c_data = result.as_mut_slice::<f32>();
                
                // Simple element-wise operation (broadcasting not fully implemented)
                if self.shape == other.shape {
                    for i in 0..self.numel() {
                        c_data[i] = op(a_data[i], b_data[i]);
                    }
                } else {
                    // Basic broadcasting for scalar
                    if other.numel() == 1 {
                        let scalar = b_data[0];
                        for i in 0..self.numel() {
                            c_data[i] = op(a_data[i], scalar);
                        }
                    } else if self.numel() == 1 {
                        let scalar = a_data[0];
                        for i in 0..other.numel() {
                            c_data[i] = op(scalar, b_data[i]);
                        }
                    } else {
                        panic!("Complex broadcasting not yet implemented");
                    }
                }
            },
            _ => panic!("Unsupported dtype for binary operation"),
        }
        
        result
    }
    
    // Compute broadcast shape
    fn broadcast_shape(&self, other: &[usize]) -> Vec<usize> {
        let max_len = self.shape.len().max(other.len());
        let mut result = vec![1; max_len];
        
        // Pad shapes with 1s on the left
        let self_padded = [vec![1; max_len - self.shape.len()], self.shape.to_vec()].concat();
        let other_padded = [vec![1; max_len - other.len()], other.to_vec()].concat();
        
        for i in 0..max_len {
            if self_padded[i] == other_padded[i] {
                result[i] = self_padded[i];
            } else if self_padded[i] == 1 {
                result[i] = other_padded[i];
            } else if other_padded[i] == 1 {
                result[i] = self_padded[i];
            } else {
                panic!("Shapes are not broadcast compatible");
            }
        }
        
        result
    }
    
    // Activation functions
    pub fn relu(&self) -> Tensor {
        self.unary_op(|x| x.max(0.0))
    }
    
    pub fn sigmoid(&self) -> Tensor {
        self.unary_op(|x| 1.0 / (1.0 + (-x).exp()))
    }
    
    pub fn tanh(&self) -> Tensor {
        self.unary_op(|x| x.tanh())
    }
    
    pub fn softmax(&self, axis: isize) -> Tensor {
        let axis = if axis < 0 {
            (self.shape.len() as isize + axis) as usize
        } else {
            axis as usize
        };
        
        // Compute exp and sum along axis
        let exp_tensor = self.unary_op(|x| x.exp());
        
        // For simplicity, implementing for last axis only
        if axis == self.shape.len() - 1 {
            let mut result = exp_tensor.clone();
            let last_dim = self.shape[axis];
            let outer_size = self.numel() / last_dim;
            
            match self.dtype {
                DType::Float32 => {
                    let data = result.as_mut_slice::<f32>();
                    
                    for i in 0..outer_size {
                        let offset = i * last_dim;
                        let mut sum = 0.0f32;
                        
                        // Compute sum
                        for j in 0..last_dim {
                            sum += data[offset + j];
                        }
                        
                        // Normalize
                        for j in 0..last_dim {
                            data[offset + j] /= sum;
                        }
                    }
                },
                _ => panic!("Unsupported dtype for softmax"),
            }
            
            result
        } else {
            panic!("Softmax only implemented for last axis");
        }
    }
    
    // Generic unary operation
    fn unary_op<F>(&self, op: F) -> Tensor
    where
        F: Fn(f32) -> f32,
    {
        let mut result = Tensor::zeros(&self.shape, self.dtype);
        
        match self.dtype {
            DType::Float32 => {
                let input_data = self.as_slice::<f32>();
                let output_data = result.as_mut_slice::<f32>();
                
                for i in 0..self.numel() {
                    output_data[i] = op(input_data[i]);
                }
            },
            _ => panic!("Unsupported dtype for unary operation"),
        }
        
        result
    }
    
    // Move tensor to GPU
    pub fn cuda(&mut self) -> Result<(), String> {
        match &self.data {
            Storage::Cpu(data) => {
                // Allocate CUDA memory and copy data
                let size = data.len();
                let cuda_buffer = CudaBuffer {
                    ptr: 0, // Would call CUDA allocation
                    size,
                    device_id: 0,
                };
                self.data = Storage::Cuda(cuda_buffer);
                Ok(())
            },
            Storage::Cuda(_) => Ok(()), // Already on CUDA
            _ => Err("Cannot move from this device to CUDA".into()),
        }
    }
    
    // Move tensor to CPU
    pub fn cpu(&mut self) -> Result<(), String> {
        match &self.data {
            Storage::Cuda(buffer) => {
                // Copy data from CUDA to CPU
                let cpu_data = vec![0u8; buffer.size];
                self.data = Storage::Cpu(cpu_data);
                Ok(())
            },
            Storage::Cpu(_) => Ok(()), // Already on CPU
            _ => Err("Cannot move from this device to CPU".into()),
        }
    }
}

// Implement display for tensors
impl fmt::Debug for Tensor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tensor(shape={:?}, dtype={:?})", self.shape, self.dtype)
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tensor_creation() {
        let t = Tensor::zeros(&[2, 3], DType::Float32);
        assert_eq!(t.shape(), &[2, 3]);
        assert_eq!(t.numel(), 6);
    }
    
    #[test]
    fn test_tensor_ops() {
        let a = Tensor::ones(&[2, 3], DType::Float32);
        let b = Tensor::ones(&[2, 3], DType::Float32);
        let c = a.add(&b);
        
        let data = c.as_slice::<f32>();
        for &val in data {
            assert_eq!(val, 2.0);
        }
    }
    
    #[test]
    fn test_matmul() {
        let a = Tensor::ones(&[2, 3], DType::Float32);
        let b = Tensor::ones(&[3, 4], DType::Float32);
        let c = a.matmul(&b);
        
        assert_eq!(c.shape(), &[2, 4]);
        let data = c.as_slice::<f32>();
        for &val in data {
            assert_eq!(val, 3.0);
        }
    }
}