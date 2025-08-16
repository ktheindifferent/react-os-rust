// OpenGL Shader Management
use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use super::{GLenum, GLuint, GLint};

// Shader Type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Geometry,
    TessControl,
    TessEvaluation,
    Compute,
}

impl ShaderType {
    pub fn to_glenum(&self) -> GLenum {
        match self {
            ShaderType::Vertex => super::GL_VERTEX_SHADER,
            ShaderType::Fragment => super::GL_FRAGMENT_SHADER,
            ShaderType::Geometry => super::GL_GEOMETRY_SHADER,
            ShaderType::TessControl => super::GL_TESS_CONTROL_SHADER,
            ShaderType::TessEvaluation => super::GL_TESS_EVALUATION_SHADER,
            ShaderType::Compute => super::GL_COMPUTE_SHADER,
        }
    }
}

// Shader Stage
#[derive(Debug, Clone)]
pub struct ShaderStage {
    pub id: GLuint,
    pub shader_type: ShaderType,
    pub source: String,
    pub compiled: bool,
    pub compile_log: String,
}

impl ShaderStage {
    pub fn new(id: GLuint, shader_type: ShaderType, source: String) -> Self {
        Self {
            id,
            shader_type,
            source,
            compiled: false,
            compile_log: String::new(),
        }
    }
}

// Shader Program
#[derive(Debug, Clone)]
pub struct ShaderProgram {
    pub id: GLuint,
    pub vertex_shader: Option<GLuint>,
    pub fragment_shader: Option<GLuint>,
    pub geometry_shader: Option<GLuint>,
    pub tess_control_shader: Option<GLuint>,
    pub tess_eval_shader: Option<GLuint>,
    pub compute_shader: Option<GLuint>,
    pub linked: bool,
    pub link_log: String,
    pub uniforms: BTreeMap<String, UniformInfo>,
    pub attributes: BTreeMap<String, AttributeInfo>,
}

// Uniform Information
#[derive(Debug, Clone)]
pub struct UniformInfo {
    pub location: GLint,
    pub uniform_type: UniformType,
    pub size: usize,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UniformType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Int,
    IVec2,
    IVec3,
    IVec4,
    Bool,
    BVec2,
    BVec3,
    BVec4,
    Mat2,
    Mat3,
    Mat4,
    Sampler2D,
    Sampler3D,
    SamplerCube,
}

// Attribute Information
#[derive(Debug, Clone)]
pub struct AttributeInfo {
    pub location: GLint,
    pub attribute_type: AttributeType,
    pub size: usize,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttributeType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Mat2,
    Mat3,
    Mat4,
}

impl ShaderProgram {
    pub fn new(id: GLuint) -> Self {
        Self {
            id,
            vertex_shader: None,
            fragment_shader: None,
            geometry_shader: None,
            tess_control_shader: None,
            tess_eval_shader: None,
            compute_shader: None,
            linked: false,
            link_log: String::new(),
            uniforms: BTreeMap::new(),
            attributes: BTreeMap::new(),
        }
    }
    
    pub fn attach_shader(&mut self, shader_id: GLuint, shader_type: ShaderType) {
        match shader_type {
            ShaderType::Vertex => self.vertex_shader = Some(shader_id),
            ShaderType::Fragment => self.fragment_shader = Some(shader_id),
            ShaderType::Geometry => self.geometry_shader = Some(shader_id),
            ShaderType::TessControl => self.tess_control_shader = Some(shader_id),
            ShaderType::TessEvaluation => self.tess_eval_shader = Some(shader_id),
            ShaderType::Compute => self.compute_shader = Some(shader_id),
        }
    }
}

// GLSL Compiler (simplified)
pub struct GlslCompiler {
    next_shader_id: GLuint,
    next_program_id: GLuint,
    shaders: BTreeMap<GLuint, ShaderStage>,
    programs: BTreeMap<GLuint, ShaderProgram>,
}

impl GlslCompiler {
    pub fn new() -> Self {
        Self {
            next_shader_id: 1,
            next_program_id: 1,
            shaders: BTreeMap::new(),
            programs: BTreeMap::new(),
        }
    }
    
    pub fn create_shader(&mut self, shader_type: ShaderType, source: &str) -> GLuint {
        let id = self.next_shader_id;
        self.next_shader_id += 1;
        
        let shader = ShaderStage::new(id, shader_type, source.to_string());
        self.shaders.insert(id, shader);
        
        id
    }
    
    pub fn compile_shader(&mut self, shader_id: GLuint) -> Result<(), String> {
        if let Some(shader) = self.shaders.get_mut(&shader_id) {
            // Perform basic GLSL validation
            let result = self.validate_glsl(&shader.source, shader.shader_type);
            
            match result {
                Ok(_) => {
                    shader.compiled = true;
                    shader.compile_log = String::from("Compilation successful");
                    Ok(())
                }
                Err(e) => {
                    shader.compiled = false;
                    shader.compile_log = e.clone();
                    Err(e)
                }
            }
        } else {
            Err(String::from("Shader not found"))
        }
    }
    
    fn validate_glsl(&self, source: &str, shader_type: ShaderType) -> Result<(), String> {
        // Basic GLSL validation
        // Check for required elements based on shader type
        
        match shader_type {
            ShaderType::Vertex => {
                if !source.contains("void main()") {
                    return Err(String::from("Vertex shader missing main() function"));
                }
                if !source.contains("gl_Position") {
                    return Err(String::from("Vertex shader must write to gl_Position"));
                }
            }
            ShaderType::Fragment => {
                if !source.contains("void main()") {
                    return Err(String::from("Fragment shader missing main() function"));
                }
            }
            ShaderType::Compute => {
                if !source.contains("void main()") {
                    return Err(String::from("Compute shader missing main() function"));
                }
                if !source.contains("layout") || !source.contains("local_size") {
                    return Err(String::from("Compute shader missing local_size layout"));
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    pub fn create_program(&mut self) -> GLuint {
        let id = self.next_program_id;
        self.next_program_id += 1;
        
        let program = ShaderProgram::new(id);
        self.programs.insert(id, program);
        
        id
    }
    
    pub fn attach_shader_to_program(&mut self, program_id: GLuint, shader_id: GLuint) {
        if let Some(shader) = self.shaders.get(&shader_id) {
            if let Some(program) = self.programs.get_mut(&program_id) {
                program.attach_shader(shader_id, shader.shader_type);
            }
        }
    }
    
    pub fn link_program(&mut self, program_id: GLuint) -> Result<(), String> {
        if let Some(program) = self.programs.get_mut(&program_id) {
            // Check that we have at least vertex and fragment shaders
            if program.vertex_shader.is_none() {
                return Err(String::from("Program missing vertex shader"));
            }
            if program.fragment_shader.is_none() && program.compute_shader.is_none() {
                return Err(String::from("Program missing fragment shader"));
            }
            
            // Extract uniforms and attributes from shaders
            self.extract_program_interface(program_id)?;
            
            program.linked = true;
            program.link_log = String::from("Linking successful");
            Ok(())
        } else {
            Err(String::from("Program not found"))
        }
    }
    
    fn extract_program_interface(&mut self, program_id: GLuint) -> Result<(), String> {
        if let Some(program) = self.programs.get_mut(&program_id) {
            // Parse shader sources to extract uniforms and attributes
            // This is a simplified version - real implementation would parse GLSL properly
            
            let mut location_counter = 0;
            
            // Extract from vertex shader
            if let Some(vs_id) = program.vertex_shader {
                if let Some(shader) = self.shaders.get(&vs_id) {
                    // Look for attribute declarations
                    for line in shader.source.lines() {
                        if line.contains("attribute") || line.contains("in ") {
                            // Parse attribute
                            if let Some(name) = Self::extract_variable_name(line) {
                                program.attributes.insert(name.clone(), AttributeInfo {
                                    location: location_counter,
                                    attribute_type: AttributeType::Vec3, // Simplified
                                    size: 1,
                                    name,
                                });
                                location_counter += 1;
                            }
                        }
                        if line.contains("uniform") {
                            // Parse uniform
                            if let Some(name) = Self::extract_variable_name(line) {
                                program.uniforms.insert(name.clone(), UniformInfo {
                                    location: location_counter,
                                    uniform_type: UniformType::Mat4, // Simplified
                                    size: 1,
                                    name,
                                });
                                location_counter += 1;
                            }
                        }
                    }
                }
            }
            
            Ok(())
        } else {
            Err(String::from("Program not found"))
        }
    }
    
    fn extract_variable_name(line: &str) -> Option<String> {
        // Simple extraction - find the variable name in a declaration
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let name = parts[parts.len() - 1].trim_end_matches(';');
            Some(name.to_string())
        } else {
            None
        }
    }
    
    pub fn delete_shader(&mut self, shader_id: GLuint) {
        self.shaders.remove(&shader_id);
    }
    
    pub fn delete_program(&mut self, program_id: GLuint) {
        self.programs.remove(&program_id);
    }
}

// Built-in shader sources
pub mod builtin_shaders {
    pub const BASIC_VERTEX_SHADER: &str = r#"
#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aColor;

out vec3 vertexColor;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

void main() {
    gl_Position = projection * view * model * vec4(aPos, 1.0);
    vertexColor = aColor;
}
"#;

    pub const BASIC_FRAGMENT_SHADER: &str = r#"
#version 330 core
in vec3 vertexColor;
out vec4 FragColor;

void main() {
    FragColor = vec4(vertexColor, 1.0);
}
"#;

    pub const TEXTURED_VERTEX_SHADER: &str = r#"
#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec2 aTexCoord;

out vec2 TexCoord;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

void main() {
    gl_Position = projection * view * model * vec4(aPos, 1.0);
    TexCoord = aTexCoord;
}
"#;

    pub const TEXTURED_FRAGMENT_SHADER: &str = r#"
#version 330 core
in vec2 TexCoord;
out vec4 FragColor;

uniform sampler2D texture1;

void main() {
    FragColor = texture(texture1, TexCoord);
}
"#;

    pub const COMPUTE_SHADER_TEMPLATE: &str = r#"
#version 430 core
layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

layout (rgba32f, binding = 0) uniform image2D imgOutput;

void main() {
    ivec2 pixelCoords = ivec2(gl_GlobalInvocationID.xy);
    vec4 pixelColor = vec4(1.0, 0.0, 0.0, 1.0);
    imageStore(imgOutput, pixelCoords, pixelColor);
}
"#;
}