use crate::{helper::from_err_impl::from_err_impl, shaders::shader_interfaces::SHADER_ENTRY_POINT};
use std::{fmt, sync::Arc};
use vulkano::{
    descriptor_set::DescriptorSetCreationError,
    device::Device,
    pipeline::{compute::ComputePipelineCreationError, graphics::GraphicsPipelineCreationError},
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
        Err(e) => return Err(CreateShaderError::IOError((e, spirv_path.to_string()))),
    };
    // create shader module
    // todo conv to &[u32] and use from_words (guarentees 4 byte multiple)
    match unsafe { ShaderModule::from_bytes(device.clone(), bytes.as_slice()) } {
        Ok(x) => Ok(x),
        Err(e) => {
            return Err(CreateShaderError::ShaderCreationError(
                e,
                spirv_path.to_owned(),
            ))
        }
    }
}

// ~~~ Errors ~~~

/// Errors encountered when preparing shader
#[derive(Debug)]
pub enum CreateShaderError {
    /// Shader SPIR-V read failed. The string should contain the shader file path.
    IOError((std::io::Error, String)),
    /// Shader module creation failed. The string should contain the shader file path.
    ShaderCreationError(ShaderCreationError, String),
    /// Shader is missing entry point `main`. String should contain shader path
    MissingEntryPoint(String),
}
impl fmt::Display for CreateShaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::IOError((e, path)) => write!(f, "Failed to read shader file {}: {}", path, e),
            Self::ShaderCreationError(e, path) => {
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

/// Errors encountered when creating a pipeline
#[derive(Debug)]
pub enum CreatePipelineError {
    /// Failed to create shader
    CreateShaderError(CreateShaderError),
    /// Failed to create graphics pipeline
    GraphicsPipelineCreationError(GraphicsPipelineCreationError),
    /// Failed to create compute pipeline
    ComputePipelineCreationError(ComputePipelineCreationError),
}
impl std::error::Error for CreatePipelineError {}
impl fmt::Display for CreatePipelineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::CreateShaderError(e) => e.fmt(f),
            Self::GraphicsPipelineCreationError(e) => {
                write!(f, "failed to create graphics pipeline: {}", e)
            }
            Self::ComputePipelineCreationError(e) => {
                write!(f, "failed to create compute pipeline: {}", e)
            }
        }
    }
}
from_err_impl!(CreatePipelineError, CreateShaderError);
from_err_impl!(CreatePipelineError, GraphicsPipelineCreationError);
from_err_impl!(CreatePipelineError, ComputePipelineCreationError);

/// Errors encountered when creating a descriptor set
#[derive(Debug)]
pub enum CreateDescriptorSetError {
    /// Descriptor set index not found in the pipeline layout
    InvalidDescriptorSetIndex(usize),
    /// Failed to create descriptor set
    DescriptorSetCreationError(DescriptorSetCreationError),
}
impl std::error::Error for CreateDescriptorSetError {}
impl fmt::Display for CreateDescriptorSetError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::DescriptorSetCreationError(e) => {
                write!(f, "failed to create blit pass descriptor set: {}", e)
            }
            Self::InvalidDescriptorSetIndex(i) => {
                write!(f, "descriptor set index {} not found in pipeline layout", i)
            }
        }
    }
}
from_err_impl!(CreateDescriptorSetError, DescriptorSetCreationError);
