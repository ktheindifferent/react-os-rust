// Metal Performance Shaders acceleration support (macOS/iOS)
use super::{Accelerator, DeviceType, DevicePtr, MemoryInfo, ComputeCapability};
use super::{Kernel, KernelArg, AcceleratorError};
use crate::ml::tensor::Tensor;

pub struct MetalAccelerator {
    device_id: u32,
}

impl MetalAccelerator {
    pub fn new(device_id: u32) -> Result<Self, AcceleratorError> {
        Ok(Self { device_id })
    }
}