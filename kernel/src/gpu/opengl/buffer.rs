// OpenGL Buffer Management
use super::{GLenum, GLuint, GLsizeiptr};

pub struct Buffer {
    pub id: GLuint,
    pub target: GLenum,
    pub size: GLsizeiptr,
    pub usage: GLenum,
}

impl Buffer {
    pub fn new(id: GLuint, target: GLenum) -> Self {
        Self {
            id,
            target,
            size: 0,
            usage: super::GL_STATIC_DRAW,
        }
    }
}