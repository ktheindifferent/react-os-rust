// OpenGL Pipeline State Management
use super::{GLenum, GLuint};

pub struct PipelineState {
    pub program: GLuint,
    pub vertex_array: GLuint,
    pub framebuffer: GLuint,
    pub depth_test: bool,
    pub blend: bool,
    pub cull_face: bool,
}

impl PipelineState {
    pub fn new() -> Self {
        Self {
            program: 0,
            vertex_array: 0,
            framebuffer: 0,
            depth_test: false,
            blend: false,
            cull_face: false,
        }
    }
}