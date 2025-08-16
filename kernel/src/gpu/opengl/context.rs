// OpenGL Context Management
use alloc::vec::Vec;
use alloc::boxed::Box;
use spin::Mutex;
use super::{GlVersion, GlState, GLuint};

// OpenGL Context
pub struct GlContext {
    pub id: u64,
    pub version: GlVersion,
    pub state: GlState,
    pub shared_context: Option<u64>,
    pub is_current: bool,
}

impl GlContext {
    pub fn new(id: u64, version: GlVersion) -> Self {
        Self {
            id,
            version,
            state: GlState::new(version),
            shared_context: None,
            is_current: false,
        }
    }
    
    pub fn make_current(&mut self) {
        self.is_current = true;
    }
    
    pub fn release(&mut self) {
        self.is_current = false;
    }
    
    pub fn share_with(&mut self, other_context_id: u64) {
        self.shared_context = Some(other_context_id);
    }
}

// Context Manager
pub struct ContextManager {
    contexts: Vec<Box<GlContext>>,
    current_context: Option<usize>,
    next_context_id: u64,
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            contexts: Vec::new(),
            current_context: None,
            next_context_id: 1,
        }
    }
    
    pub fn create_context(&mut self, version: GlVersion) -> u64 {
        let id = self.next_context_id;
        self.next_context_id += 1;
        
        let context = Box::new(GlContext::new(id, version));
        self.contexts.push(context);
        
        id
    }
    
    pub fn make_current(&mut self, context_id: u64) -> Result<(), &'static str> {
        // Release current context
        if let Some(current_idx) = self.current_context {
            self.contexts[current_idx].release();
        }
        
        // Find and activate new context
        for (idx, context) in self.contexts.iter_mut().enumerate() {
            if context.id == context_id {
                context.make_current();
                self.current_context = Some(idx);
                return Ok(());
            }
        }
        
        Err("Context not found")
    }
    
    pub fn get_current_context(&self) -> Option<&GlContext> {
        self.current_context.and_then(|idx| self.contexts.get(idx).map(|c| c.as_ref()))
    }
    
    pub fn get_current_context_mut(&mut self) -> Option<&mut GlContext> {
        self.current_context.and_then(move |idx| self.contexts.get_mut(idx).map(|c| c.as_mut()))
    }
    
    pub fn destroy_context(&mut self, context_id: u64) -> Result<(), &'static str> {
        let position = self.contexts.iter().position(|c| c.id == context_id);
        
        if let Some(idx) = position {
            if Some(idx) == self.current_context {
                self.current_context = None;
            }
            self.contexts.remove(idx);
            Ok(())
        } else {
            Err("Context not found")
        }
    }
}

// Global Context Manager
lazy_static::lazy_static! {
    pub static ref CONTEXT_MANAGER: Mutex<ContextManager> = Mutex::new(ContextManager::new());
}