// Automatic differentiation and backpropagation engine
use alloc::{vec::Vec, boxed::Box, rc::Rc, collections::BTreeMap};
use core::cell::RefCell;

use crate::ml::tensor::{Tensor, DType};

// Gradient function type
type GradFn = Box<dyn Fn(&Tensor, &[Rc<RefCell<Variable>>]) -> Vec<Tensor>>;

// Variable wrapper for automatic differentiation
pub struct Variable {
    data: Tensor,
    grad: Option<Tensor>,
    grad_fn: Option<GradFn>,
    inputs: Vec<Rc<RefCell<Variable>>>,
    requires_grad: bool,
    is_leaf: bool,
}

impl Variable {
    // Create new variable from tensor
    pub fn new(tensor: Tensor, requires_grad: bool) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            data: tensor,
            grad: None,
            grad_fn: None,
            inputs: Vec::new(),
            requires_grad,
            is_leaf: true,
        }))
    }
    
    // Get data tensor
    pub fn data(&self) -> &Tensor {
        &self.data
    }
    
    // Get gradient
    pub fn grad(&self) -> Option<&Tensor> {
        self.grad.as_ref()
    }
    
    // Zero gradient
    pub fn zero_grad(&mut self) {
        self.grad = None;
    }
    
    // Backward pass
    pub fn backward(&mut self, grad_output: Option<Tensor>) {
        if !self.requires_grad {
            return;
        }
        
        // Initialize gradient if not provided
        let grad = grad_output.unwrap_or_else(|| {
            Tensor::ones(self.data.shape(), self.data.dtype())
        });
        
        // Accumulate gradient
        if let Some(ref mut existing_grad) = self.grad {
            *existing_grad = existing_grad.add(&grad);
        } else {
            self.grad = Some(grad.clone());
        }
        
        // Propagate gradients if not a leaf
        if !self.is_leaf {
            if let Some(ref grad_fn) = self.grad_fn {
                let input_grads = grad_fn(&grad, &self.inputs);
                
                for (input, input_grad) in self.inputs.iter().zip(input_grads.iter()) {
                    input.borrow_mut().backward(Some(input_grad.clone()));
                }
            }
        }
    }
}

// Computation graph for automatic differentiation
pub struct ComputationGraph {
    nodes: Vec<Rc<RefCell<Variable>>>,
}

impl ComputationGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
        }
    }
    
    pub fn add_node(&mut self, node: Rc<RefCell<Variable>>) {
        self.nodes.push(node);
    }
    
    pub fn clear(&mut self) {
        self.nodes.clear();
    }
}

// Automatic differentiation operations
pub mod ops {
    use super::*;
    
    // Addition operation
    pub fn add(a: Rc<RefCell<Variable>>, b: Rc<RefCell<Variable>>) -> Rc<RefCell<Variable>> {
        let a_data = a.borrow().data().clone();
        let b_data = b.borrow().data().clone();
        
        let output_data = a_data.add(&b_data);
        let requires_grad = a.borrow().requires_grad || b.borrow().requires_grad;
        
        let output = Rc::new(RefCell::new(Variable {
            data: output_data,
            grad: None,
            grad_fn: if requires_grad {
                Some(Box::new(move |grad_output: &Tensor, _inputs: &[Rc<RefCell<Variable>>]| {
                    vec![grad_output.clone(), grad_output.clone()]
                }))
            } else {
                None
            },
            inputs: vec![a.clone(), b.clone()],
            requires_grad,
            is_leaf: false,
        }));
        
        output
    }
    
    // Subtraction operation
    pub fn sub(a: Rc<RefCell<Variable>>, b: Rc<RefCell<Variable>>) -> Rc<RefCell<Variable>> {
        let a_data = a.borrow().data().clone();
        let b_data = b.borrow().data().clone();
        
        let output_data = a_data.sub(&b_data);
        let requires_grad = a.borrow().requires_grad || b.borrow().requires_grad;
        
        let output = Rc::new(RefCell::new(Variable {
            data: output_data,
            grad: None,
            grad_fn: if requires_grad {
                Some(Box::new(move |grad_output: &Tensor, _inputs: &[Rc<RefCell<Variable>>]| {
                    let neg_one = Tensor::new(vec![-1.0], vec![1]);
                    vec![grad_output.clone(), grad_output.mul(&neg_one)]
                }))
            } else {
                None
            },
            inputs: vec![a.clone(), b.clone()],
            requires_grad,
            is_leaf: false,
        }));
        
        output
    }
    
    // Multiplication operation
    pub fn mul(a: Rc<RefCell<Variable>>, b: Rc<RefCell<Variable>>) -> Rc<RefCell<Variable>> {
        let a_data = a.borrow().data().clone();
        let b_data = b.borrow().data().clone();
        
        let output_data = a_data.mul(&b_data);
        let requires_grad = a.borrow().requires_grad || b.borrow().requires_grad;
        
        let a_data_clone = a_data.clone();
        let b_data_clone = b_data.clone();
        
        let output = Rc::new(RefCell::new(Variable {
            data: output_data,
            grad: None,
            grad_fn: if requires_grad {
                Some(Box::new(move |grad_output: &Tensor, _inputs: &[Rc<RefCell<Variable>>]| {
                    vec![
                        grad_output.mul(&b_data_clone),
                        grad_output.mul(&a_data_clone),
                    ]
                }))
            } else {
                None
            },
            inputs: vec![a.clone(), b.clone()],
            requires_grad,
            is_leaf: false,
        }));
        
        output
    }
    
    // Division operation
    pub fn div(a: Rc<RefCell<Variable>>, b: Rc<RefCell<Variable>>) -> Rc<RefCell<Variable>> {
        let a_data = a.borrow().data().clone();
        let b_data = b.borrow().data().clone();
        
        let output_data = a_data.div(&b_data);
        let requires_grad = a.borrow().requires_grad || b.borrow().requires_grad;
        
        let b_data_clone = b_data.clone();
        let b_data_clone2 = b_data.clone();
        let output_data_clone = output_data.clone();
        
        let output = Rc::new(RefCell::new(Variable {
            data: output_data,
            grad: None,
            grad_fn: if requires_grad {
                Some(Box::new(move |grad_output: &Tensor, _inputs: &[Rc<RefCell<Variable>>]| {
                    let one = Tensor::ones(&[1], grad_output.dtype());
                    let neg_one = Tensor::new(vec![-1.0], vec![1]);
                    
                    vec![
                        grad_output.div(&b_data_clone),
                        grad_output.mul(&output_data_clone).div(&b_data_clone2).mul(&neg_one),
                    ]
                }))
            } else {
                None
            },
            inputs: vec![a.clone(), b.clone()],
            requires_grad,
            is_leaf: false,
        }));
        
        output
    }
    
    // Matrix multiplication operation
    pub fn matmul(a: Rc<RefCell<Variable>>, b: Rc<RefCell<Variable>>) -> Rc<RefCell<Variable>> {
        let a_data = a.borrow().data().clone();
        let b_data = b.borrow().data().clone();
        
        let output_data = a_data.matmul(&b_data);
        let requires_grad = a.borrow().requires_grad || b.borrow().requires_grad;
        
        let a_data_clone = a_data.clone();
        let b_data_clone = b_data.clone();
        
        let output = Rc::new(RefCell::new(Variable {
            data: output_data,
            grad: None,
            grad_fn: if requires_grad {
                Some(Box::new(move |grad_output: &Tensor, _inputs: &[Rc<RefCell<Variable>>]| {
                    vec![
                        grad_output.matmul(&b_data_clone.transpose()),
                        a_data_clone.transpose().matmul(grad_output),
                    ]
                }))
            } else {
                None
            },
            inputs: vec![a.clone(), b.clone()],
            requires_grad,
            is_leaf: false,
        }));
        
        output
    }
    
    // ReLU activation
    pub fn relu(input: Rc<RefCell<Variable>>) -> Rc<RefCell<Variable>> {
        let input_data = input.borrow().data().clone();
        let output_data = input_data.relu();
        let requires_grad = input.borrow().requires_grad;
        
        let mask = input_data.relu().div(&input_data.add(&Tensor::new(vec![1e-10], vec![1])));
        
        let output = Rc::new(RefCell::new(Variable {
            data: output_data,
            grad: None,
            grad_fn: if requires_grad {
                Some(Box::new(move |grad_output: &Tensor, _inputs: &[Rc<RefCell<Variable>>]| {
                    vec![grad_output.mul(&mask)]
                }))
            } else {
                None
            },
            inputs: vec![input.clone()],
            requires_grad,
            is_leaf: false,
        }));
        
        output
    }
    
    // Sigmoid activation
    pub fn sigmoid(input: Rc<RefCell<Variable>>) -> Rc<RefCell<Variable>> {
        let input_data = input.borrow().data().clone();
        let output_data = input_data.sigmoid();
        let requires_grad = input.borrow().requires_grad;
        
        let output_data_clone = output_data.clone();
        let one = Tensor::ones(&[1], output_data.dtype());
        
        let output = Rc::new(RefCell::new(Variable {
            data: output_data,
            grad: None,
            grad_fn: if requires_grad {
                Some(Box::new(move |grad_output: &Tensor, _inputs: &[Rc<RefCell<Variable>>]| {
                    let sigmoid_grad = output_data_clone.mul(&one.sub(&output_data_clone));
                    vec![grad_output.mul(&sigmoid_grad)]
                }))
            } else {
                None
            },
            inputs: vec![input.clone()],
            requires_grad,
            is_leaf: false,
        }));
        
        output
    }
    
    // Tanh activation
    pub fn tanh(input: Rc<RefCell<Variable>>) -> Rc<RefCell<Variable>> {
        let input_data = input.borrow().data().clone();
        let output_data = input_data.tanh();
        let requires_grad = input.borrow().requires_grad;
        
        let output_data_clone = output_data.clone();
        let one = Tensor::ones(&[1], output_data.dtype());
        
        let output = Rc::new(RefCell::new(Variable {
            data: output_data,
            grad: None,
            grad_fn: if requires_grad {
                Some(Box::new(move |grad_output: &Tensor, _inputs: &[Rc<RefCell<Variable>>]| {
                    let tanh_grad = one.sub(&output_data_clone.mul(&output_data_clone));
                    vec![grad_output.mul(&tanh_grad)]
                }))
            } else {
                None
            },
            inputs: vec![input.clone()],
            requires_grad,
            is_leaf: false,
        }));
        
        output
    }
    
    // Softmax activation
    pub fn softmax(input: Rc<RefCell<Variable>>, axis: isize) -> Rc<RefCell<Variable>> {
        let input_data = input.borrow().data().clone();
        let output_data = input_data.softmax(axis);
        let requires_grad = input.borrow().requires_grad;
        
        let output_data_clone = output_data.clone();
        
        let output = Rc::new(RefCell::new(Variable {
            data: output_data,
            grad: None,
            grad_fn: if requires_grad {
                Some(Box::new(move |grad_output: &Tensor, _inputs: &[Rc<RefCell<Variable>>]| {
                    // Jacobian of softmax is complex, simplified here
                    let sum_grad = grad_output.mul(&output_data_clone);
                    let sum = compute_sum_along_axis(&sum_grad, axis);
                    let grad = grad_output.sub(&sum).mul(&output_data_clone);
                    vec![grad]
                }))
            } else {
                None
            },
            inputs: vec![input.clone()],
            requires_grad,
            is_leaf: false,
        }));
        
        output
    }
    
    // Mean reduction
    pub fn mean(input: Rc<RefCell<Variable>>) -> Rc<RefCell<Variable>> {
        let input_data = input.borrow().data().clone();
        let numel = input_data.numel() as f32;
        
        let sum = compute_sum(&input_data);
        let output_data = sum.div(&Tensor::new(vec![numel], vec![1]));
        let requires_grad = input.borrow().requires_grad;
        
        let input_shape = input_data.shape().to_vec();
        
        let output = Rc::new(RefCell::new(Variable {
            data: output_data,
            grad: None,
            grad_fn: if requires_grad {
                Some(Box::new(move |grad_output: &Tensor, _inputs: &[Rc<RefCell<Variable>>]| {
                    let grad = grad_output.div(&Tensor::new(vec![numel], vec![1]));
                    let expanded_grad = grad.reshape(&vec![1; input_shape.len()]);
                    vec![expanded_grad]
                }))
            } else {
                None
            },
            inputs: vec![input.clone()],
            requires_grad,
            is_leaf: false,
        }));
        
        output
    }
    
    // Sum reduction
    pub fn sum(input: Rc<RefCell<Variable>>) -> Rc<RefCell<Variable>> {
        let input_data = input.borrow().data().clone();
        let output_data = compute_sum(&input_data);
        let requires_grad = input.borrow().requires_grad;
        
        let input_shape = input_data.shape().to_vec();
        
        let output = Rc::new(RefCell::new(Variable {
            data: output_data,
            grad: None,
            grad_fn: if requires_grad {
                Some(Box::new(move |grad_output: &Tensor, _inputs: &[Rc<RefCell<Variable>>]| {
                    let expanded_grad = grad_output.reshape(&vec![1; input_shape.len()]);
                    vec![expanded_grad]
                }))
            } else {
                None
            },
            inputs: vec![input.clone()],
            requires_grad,
            is_leaf: false,
        }));
        
        output
    }
    
    // Helper functions
    fn compute_sum(tensor: &Tensor) -> Tensor {
        let data = tensor.as_slice::<f32>();
        let sum: f32 = data.iter().sum();
        Tensor::new(vec![sum], vec![1])
    }
    
    fn compute_sum_along_axis(tensor: &Tensor, axis: isize) -> Tensor {
        // Simplified sum along axis
        tensor.clone()
    }
}

// Gradient tape for recording operations
pub struct GradientTape {
    operations: Vec<(String, Vec<Rc<RefCell<Variable>>>, Rc<RefCell<Variable>>)>,
    enabled: bool,
}

impl GradientTape {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            enabled: true,
        }
    }
    
    pub fn record(&mut self, op_name: String, inputs: Vec<Rc<RefCell<Variable>>>, output: Rc<RefCell<Variable>>) {
        if self.enabled {
            self.operations.push((op_name, inputs, output));
        }
    }
    
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    pub fn reset(&mut self) {
        self.operations.clear();
    }
    
    pub fn compute_gradients(&self, loss: Rc<RefCell<Variable>>) -> BTreeMap<String, Tensor> {
        let mut gradients = BTreeMap::new();
        
        // Start backward pass from loss
        loss.borrow_mut().backward(None);
        
        // Collect gradients
        for (op_name, inputs, _output) in &self.operations {
            for (i, input) in inputs.iter().enumerate() {
                if let Some(grad) = input.borrow().grad() {
                    let key = format!("{}_{}", op_name, i);
                    gradients.insert(key, grad.clone());
                }
            }
        }
        
        gradients
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_autograd_add() {
        let a = Variable::new(Tensor::new(vec![2.0], vec![1]), true);
        let b = Variable::new(Tensor::new(vec![3.0], vec![1]), true);
        
        let c = ops::add(a.clone(), b.clone());
        
        c.borrow_mut().backward(None);
        
        assert_eq!(a.borrow().grad().unwrap().as_slice::<f32>()[0], 1.0);
        assert_eq!(b.borrow().grad().unwrap().as_slice::<f32>()[0], 1.0);
    }
    
    #[test]
    fn test_autograd_mul() {
        let a = Variable::new(Tensor::new(vec![2.0], vec![1]), true);
        let b = Variable::new(Tensor::new(vec![3.0], vec![1]), true);
        
        let c = ops::mul(a.clone(), b.clone());
        
        c.borrow_mut().backward(None);
        
        assert_eq!(a.borrow().grad().unwrap().as_slice::<f32>()[0], 3.0);
        assert_eq!(b.borrow().grad().unwrap().as_slice::<f32>()[0], 2.0);
    }
}