pub mod audio;
pub mod video;

use alloc::vec::Vec;
use super::MediaError;

pub trait Effect: Send + Sync {
    fn name(&self) -> &str;
    fn effect_type(&self) -> EffectType;
    fn process(&mut self, input: &[u8], output: &mut Vec<u8>) -> Result<(), MediaError>;
    fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), MediaError>;
    fn get_parameter(&self, name: &str) -> Option<f32>;
    fn reset(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    AudioFilter,
    VideoFilter,
    AudioGenerator,
    VideoGenerator,
    Transition,
}

pub struct EffectChain {
    effects: Vec<Box<dyn Effect>>,
}

impl EffectChain {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    pub fn add_effect(&mut self, effect: Box<dyn Effect>) {
        self.effects.push(effect);
    }

    pub fn remove_effect(&mut self, index: usize) -> Option<Box<dyn Effect>> {
        if index < self.effects.len() {
            Some(self.effects.remove(index))
        } else {
            None
        }
    }

    pub fn process(&mut self, input: &[u8]) -> Result<Vec<u8>, MediaError> {
        let mut data = input.to_vec();
        let mut output = Vec::new();
        
        for effect in &mut self.effects {
            output.clear();
            effect.process(&data, &mut output)?;
            data = output.clone();
        }
        
        Ok(data)
    }

    pub fn clear(&mut self) {
        self.effects.clear();
    }
}