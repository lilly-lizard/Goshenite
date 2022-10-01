use super::common::{
    create_shader_module, CreateDescriptorSetError, CreatePipelineError, CreateShaderError,
};
use crate::config;
use crate::helper::from_err_impl::from_err_impl;
use crate::primitives::primitives::PrimitiveCollection;
use crate::shaders::shader_interfaces::{
    CameraPushConstant, ComputeSpecConstant, PrimitiveData, PrimitiveDataError, PrimitiveDataUnit,
    SHADER_ENTRY_POINT,
};
use std::fmt;
use std::sync::Arc;
use vulkano::command_buffer::PipelineExecutionError;
use vulkano::memory::pool::StandardMemoryPool;
use vulkano::memory::DeviceMemoryError;
use vulkano::pipeline::compute::ComputePipelineCreationError; // todo error propogation testing (see return below)
use vulkano::{
    buffer::{cpu_pool::CpuBufferPoolChunk, BufferUsage, CpuBufferPool},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{physical::PhysicalDevice, Device},
    image::{view::ImageView, StorageImage},
    pipeline::{ComputePipeline, Pipeline, PipelineBindPoint},
    DeviceSize,
};

const COMP_SHADER_PATH: &str = "assets/shader_binaries/scene.comp.spv";

/// Describes descriptor set indices
mod descriptor {
    pub const SET_IMAGE: usize = 0;
    pub const SET_PRIMITVES: usize = 1;

    pub const BINDING_IMAGE: u32 = 0;
    pub const BINDING_PRIMITVES: u32 = 0;
}

/// The initial primitive buffer pool allocation
const RESERVED_PRIMITIVE_BUFFER_POOL: DeviceSize = 8 * 4 * 1024;

/// Defines functionality for the scene render compute shader pass
pub struct ScenePass {
    device: Arc<Device>,

    work_group_size: [u32; 2],
    work_group_count: [u32; 3],
    primitive_buffer_pool: CpuBufferPool<PrimitiveDataUnit>,

    pipeline: Arc<ComputePipeline>,
    desc_set_render_image: Arc<PersistentDescriptorSet>,
    desc_set_primitives: Arc<PersistentDescriptorSet>,
}
// Public functions
impl ScenePass {
    pub fn new(
        device: Arc<Device>,
        primitives: &PrimitiveCollection,
        render_image_size: [u32; 2],
        render_image: Arc<ImageView<StorageImage>>,
    ) -> Result<Self, ScenePassError> {
        let physical_device = device.physical_device();

        // calculate work group size and count for scene compute shader
        let work_group_size = [
            std::cmp::min(
                config::DEFAULT_WORK_GROUP_SIZE[0],
                physical_device.properties().max_compute_work_group_size[0],
            ),
            std::cmp::min(
                config::DEFAULT_WORK_GROUP_SIZE[1],
                physical_device.properties().max_compute_work_group_size[1],
            ),
        ];
        let work_group_count = Self::calc_work_group_count(
            device.physical_device().clone(),
            render_image_size,
            work_group_size,
        )?;

        // init primitive buffer pool
        let primitive_buffer_pool = CpuBufferPool::new(
            device.clone(),
            BufferUsage {
                storage_buffer: true,
                ..BufferUsage::empty()
            },
        );
        primitive_buffer_pool.reserve(RESERVED_PRIMITIVE_BUFFER_POOL)?;

        // init compute pipeline and descriptor sets
        let pipeline = Self::create_pipeline(device.clone(), work_group_size)?;
        let desc_set_render_image =
            Self::create_desc_set_render_image(pipeline.clone(), render_image)?;
        let primitive_buffer = Self::create_primitives_buffer(primitives, &primitive_buffer_pool)?;
        let desc_set_primitives =
            Self::create_desc_set_primitives(pipeline.clone(), primitive_buffer)?;

        Ok(Self {
            device,
            pipeline,
            desc_set_render_image,
            desc_set_primitives,
            work_group_size,
            work_group_count,
            primitive_buffer_pool,
        })
    }

    /// Update the primitives storage buffer.
    ///
    /// todo shoul be optimized to not create a new buffer each time...
    pub fn update_primitives(
        &mut self,
        primitives: &PrimitiveCollection,
    ) -> Result<(), ScenePassError> {
        let primitive_buffer =
            Self::create_primitives_buffer(primitives, &self.primitive_buffer_pool)?;
        self.desc_set_primitives =
            Self::create_desc_set_primitives(self.pipeline.clone(), primitive_buffer)?;
        Ok(())
    }

    /// Updates render target data e.g. when it has been resized
    pub fn update_render_target(
        &mut self,
        resolution: [u32; 2],
        render_image: Arc<ImageView<StorageImage>>,
    ) -> Result<(), ScenePassError> {
        self.work_group_count = Self::calc_work_group_count(
            self.device.physical_device().clone(),
            resolution,
            self.work_group_size,
        )?;
        self.desc_set_render_image =
            ScenePass::create_desc_set_render_image(self.pipeline.clone(), render_image.clone())?;
        Ok(())
    }

    /// Records rendering commands to a command buffer
    pub fn record_commands<L>(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<L>,
        camera_push_constant: CameraPushConstant,
    ) -> Result<(), PipelineExecutionError> {
        let mut desc_sets: Vec<Arc<PersistentDescriptorSet>> = Vec::default();
        desc_sets.insert(descriptor::SET_IMAGE, self.desc_set_render_image.clone());
        desc_sets.insert(descriptor::SET_PRIMITVES, self.desc_set_primitives.clone());
        command_buffer
            .bind_pipeline_compute(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Compute,
                self.pipeline.layout().clone(),
                0,
                desc_sets,
            )
            .push_constants(self.pipeline.layout().clone(), 0, camera_push_constant)
            .dispatch(self.work_group_count)?;
        Ok(())
    }
}
// Private functions
impl ScenePass {
    /// Calculate required work group count for a given render resolution,
    /// and checks that the work group count is within the physical device limits
    fn calc_work_group_count(
        physical_device: Arc<PhysicalDevice>,
        resolution: [u32; 2],
        work_group_size: [u32; 2],
    ) -> Result<[u32; 3], ScenePassError> {
        let mut group_count_x = resolution[0] / work_group_size[0];
        if (resolution[0] % work_group_size[0]) != 0 {
            group_count_x += 1;
        }
        let mut group_count_y = resolution[1] / work_group_size[1];
        if (resolution[1] % work_group_size[1]) != 0 {
            group_count_y += 1;
        }
        // check that work group count is within physical device limits
        let group_count_limit: [u32; 2] = [
            physical_device.properties().max_compute_work_group_count[0],
            physical_device.properties().max_compute_work_group_count[1],
        ];
        if group_count_x > group_count_limit[0] || group_count_y > group_count_limit[1] {
            return Err(ScenePassError::UnsupportedWorkGroupCount {
                group_count: [group_count_x, group_count_y],
                group_count_limit,
            });
        }
        return Ok([group_count_x, group_count_y, 1]);
    }

    fn create_pipeline(
        device: Arc<Device>,
        work_group_size: [u32; 2],
    ) -> Result<Arc<ComputePipeline>, CreatePipelineError> {
        //return Err(ComputePipelineCreationError::IncompatibleSpecializationConstants.into());
        let comp_module = create_shader_module(device.clone(), COMP_SHADER_PATH)?;
        let comp_shader = comp_module.entry_point(SHADER_ENTRY_POINT).ok_or(
            CreateShaderError::MissingEntryPoint(COMP_SHADER_PATH.to_string()),
        )?;

        let compute_spec_constant = ComputeSpecConstant {
            local_size_x: work_group_size[0],
            local_size_y: work_group_size[1],
        };
        Ok(ComputePipeline::new(
            device.clone(),
            comp_shader,
            &compute_spec_constant,
            None,
            |_| {},
        )?)
    }

    fn create_desc_set_render_image(
        scene_pipeline: Arc<ComputePipeline>,
        render_image: Arc<ImageView<StorageImage>>,
    ) -> Result<Arc<PersistentDescriptorSet>, CreateDescriptorSetError> {
        Ok(PersistentDescriptorSet::new(
            scene_pipeline
                .layout()
                .set_layouts()
                .get(descriptor::SET_IMAGE)
                .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
                    index: descriptor::SET_IMAGE,
                })?
                .to_owned(),
            [WriteDescriptorSet::image_view(
                descriptor::BINDING_IMAGE,
                render_image,
            )],
        )?)
    }

    fn create_desc_set_primitives(
        scene_pipeline: Arc<ComputePipeline>,
        primitive_buffer: Arc<CpuBufferPoolChunk<PrimitiveDataUnit, Arc<StandardMemoryPool>>>,
    ) -> Result<Arc<PersistentDescriptorSet>, CreateDescriptorSetError> {
        Ok(PersistentDescriptorSet::new(
            scene_pipeline
                .layout()
                .set_layouts()
                .get(descriptor::SET_PRIMITVES)
                .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
                    index: descriptor::SET_PRIMITVES,
                })?
                .to_owned(),
            [WriteDescriptorSet::buffer(
                descriptor::BINDING_PRIMITVES,
                primitive_buffer,
            )],
        )?)
    }

    fn create_primitives_buffer(
        primitives: &PrimitiveCollection,
        buffer_pool: &CpuBufferPool<PrimitiveDataUnit>,
    ) -> Result<Arc<CpuBufferPoolChunk<PrimitiveDataUnit, Arc<StandardMemoryPool>>>, ScenePassError>
    {
        // todo should be able to update buffer wihtout recreating?
        Ok(buffer_pool.from_iter(PrimitiveData::combined_data(primitives)?)?)
    }
}

// ~~~ Errors ~~~

// todo duplicated stuff with other passes?
#[derive(Debug)]
pub enum ScenePassError {
    /// The calculated compute shader work group count exceeds physical device limits.
    /// todo this could be handled more elegently by doing multiple dispatches?
    UnsupportedWorkGroupCount {
        group_count: [u32; 2],
        group_count_limit: [u32; 2],
    },
    /// Failed to allocate device memory for vulkan object
    DeviceMemoryError(DeviceMemoryError),
    /// Failed to combine primitive data
    PrimitiveDataError(PrimitiveDataError),
    /// Errors encountered when creating a pipeline
    CreatePipelineError(CreatePipelineError),
    /// Errors encountered when creating a descriptor set
    CreateDescriptorSetError(CreateDescriptorSetError),
}
impl fmt::Display for ScenePassError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ScenePassError::UnsupportedWorkGroupCount {
                group_count,
                group_count_limit,
            } => write!(
                f,
                "compute shader work group count {:?} exceeds driver limits {:?}",
                group_count, group_count_limit
            ),
            ScenePassError::DeviceMemoryError(e) => {
                write!(f, "failed to allocate primitive buffer: {}", e)
            }
            ScenePassError::PrimitiveDataError(e) => e.fmt(f),
            ScenePassError::CreatePipelineError(e) => e.fmt(f),
            ScenePassError::CreateDescriptorSetError(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for ScenePassError {}
from_err_impl!(ScenePassError, DeviceMemoryError);
from_err_impl!(ScenePassError, PrimitiveDataError);
from_err_impl!(ScenePassError, CreatePipelineError);
from_err_impl!(ScenePassError, CreateDescriptorSetError);
