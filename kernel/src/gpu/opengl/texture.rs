// OpenGL Texture Management
use super::{GLenum, GLuint, GLint, GLsizei};

pub struct Texture {
    pub id: GLuint,
    pub target: GLenum,
    pub width: GLsizei,
    pub height: GLsizei,
    pub format: GLenum,
    pub internal_format: GLint,
}

impl Texture {
    pub fn new(id: GLuint, target: GLenum) -> Self {
        Self {
            id,
            target,
            width: 0,
            height: 0,
            format: super::GL_RGBA,
            internal_format: super::GL_RGBA8 as GLint,
        }
    }
}