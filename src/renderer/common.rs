use crate::shaders::shader_interfaces::SHADER_ENTRY_POINT;
use std::{fmt, sync::Arc};
use vulkano::{
    device::Device,
    shader::{ShaderCreationError, ShaderModule},
};

/// Creates a Vulkan shader module given a spirv path (relative to crate root)
pub fn create_shader_module(
    device: Arc<Device>,
    spirv_path: &str,
) -> Result<Arc<ShaderModule>, CreateShaderError> {
    // read spirv bytes
    let bytes = match std::fs::read(spirv_path) {
        Ok(x) => x,
        Err(e) => {
            return Err(CreateShaderError::IOError {
                e,
                path: spirv_path.to_string(),
            })
        }
    };
    // create shader module
    // todo conv to &[u32] and use from_words (guarentees 4 byte multiple)
    match unsafe { ShaderModule::from_bytes(device.clone(), bytes.as_slice()) } {
        Ok(x) => Ok(x),
        Err(e) => {
            return Err(CreateShaderError::ShaderCreationError {
                e,
                path: spirv_path.to_owned(),
            })
        }
    }
}

// ~~~ Errors ~~~

/// Errors encountered when preparing shader
#[derive(Debug)]
pub enum CreateShaderError {
    /// Shader SPIR-V read failed. The string should contain the shader file path.
    IOError { e: std::io::Error, path: String },
    /// Shader module creation failed. The string should contain the shader file path.
    ShaderCreationError {
        e: ShaderCreationError,
        path: String,
    },
    /// Shader is missing entry point `main`. String should contain shader path
    MissingEntryPoint(String),
}
impl fmt::Display for CreateShaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::IOError { e, path } => write!(f, "Failed to read shader file {}: {}", path, e),
            Self::ShaderCreationError { e, path } => {
                write!(f, "Failed to create shader module from {}: {}", path, e)
            }
            Self::MissingEntryPoint(path) => {
                write!(
                    f,
                    "shader {} is missing entry point `{}`",
                    path, SHADER_ENTRY_POINT
                )
            }
        }
    }
}
impl std::error::Error for CreateShaderError {}

/// Errors encountered when creating a descriptor set
#[derive(Debug)]
pub enum CreateDescriptorSetError {
    /// Descriptor set index not found in the pipeline layout
    InvalidDescriptorSetIndex { index: usize },
}
impl std::error::Error for CreateDescriptorSetError {}
impl fmt::Display for CreateDescriptorSetError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidDescriptorSetIndex { index } => {
                write!(
                    f,
                    "descriptor set index {} not found in pipeline layout",
                    index
                )
            }
        }
    }
}
