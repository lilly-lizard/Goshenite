use super::render_manager::{create_shader_module, RenderManagerError, RenderManagerUnrecoverable};
use crate::config;
use crate::primitives::primitives::PrimitiveCollection;
use crate::shaders::shader_interfaces::{
    CameraPushConstant, ComputeSpecConstant, PrimitiveData, PrimitiveDataUnit,
};
use std::sync::Arc;
use vulkano::{
    buffer::{cpu_pool::CpuBufferPoolChunk, BufferUsage, CpuBufferPool},
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{physical::PhysicalDevice, Device},
    image::{view::ImageView, StorageImage},
    memory::pool::StdMemoryPool,
    pipeline::{ComputePipeline, Pipeline, PipelineBindPoint},
    DeviceSize,
};

/// Describes descriptor set indices
pub mod descriptor {
    pub const SET_IMAGE: usize = 0;
    pub const SET_PRIMITVES: usize = 1;

    pub const BINDING_IMAGE: u32 = 0;
    pub const BINDING_PRIMITVES: u32 = 0;
}

/// todo bruh?
const MAX_DATA_COUNT: DeviceSize = 1024;

pub struct ScenePass {
    // todo remove pubs?
    pub primitive_buffer_pool: CpuBufferPool<PrimitiveDataUnit>,
    pub work_group_size: [u32; 2],
    pub work_group_count: [u32; 3],
    pub pipeline: Arc<ComputePipeline>,
    pub desc_set_render_image: Arc<PersistentDescriptorSet>,
    pub desc_set_primitives: Arc<PersistentDescriptorSet>,
}
impl ScenePass {
    pub fn new(
        device: Arc<Device>,
        primitives: &PrimitiveCollection,
        render_image_size: [u32; 2],
        render_image: Arc<ImageView<StorageImage>>,
    ) -> Result<Self, RenderManagerError> {
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
            device.physical_device(),
            render_image_size,
            work_group_size,
        )?;

        let primitive_buffer_pool =
            CpuBufferPool::new(device.clone(), BufferUsage::storage_buffer());
        primitive_buffer_pool
            .reserve(MAX_DATA_COUNT)
            .to_renderer_err("unable to reserve primitives buffer")?;

        let pipeline = Self::create_pipeline(device.clone(), work_group_size)?;
        let desc_set_render_image =
            Self::create_desc_set_render_image(pipeline.clone(), render_image)?;
        let desc_set_primitives =
            Self::create_desc_set_primitives(pipeline.clone(), primitives, &primitive_buffer_pool)?;

        Ok(Self {
            pipeline,
            desc_set_render_image,
            desc_set_primitives,
            work_group_size,
            work_group_count,
            primitive_buffer_pool,
        })
    }

    /// Calculate required work group count for a given render resolution,
    /// and checks that the work group count is within the physical device limits
    pub fn calc_work_group_count(
        physical_device: PhysicalDevice,
        resolution: [u32; 2],
        work_group_size: [u32; 2],
    ) -> Result<[u32; 3], RenderManagerError> {
        let mut group_count_x = resolution[0] / work_group_size[0];
        if (resolution[0] % work_group_size[0]) != 0 {
            group_count_x += 1;
        }
        let mut group_count_y = resolution[1] / work_group_size[1];
        if (resolution[1] % work_group_size[1]) != 0 {
            group_count_y += 1;
        }
        // check that work group count is within physical device limits
        // todo this can be handled more elegently by doing multiple dispatches...
        if group_count_x > physical_device.properties().max_compute_work_group_count[0]
            || group_count_y > physical_device.properties().max_compute_work_group_count[1]
        {
            return Err(RenderManagerError::Unrecoverable(
            "compute shader work group count exceeds physical device limits. TODO this can be handled more elegently by doing multiple dispatches...".to_string(),
        ));
        }
        Ok([group_count_x, group_count_y, 1])
    }

    fn create_pipeline(
        device: Arc<Device>,
        work_group_size: [u32; 2],
    ) -> Result<Arc<ComputePipeline>, RenderManagerError> {
        let render_shader =
            create_shader_module(device.clone(), "assets/shader_binaries/scene.comp.spv")?;

        let compute_spec_constant = ComputeSpecConstant {
            local_size_x: work_group_size[0],
            local_size_y: work_group_size[1],
        };
        ComputePipeline::new(
            device.clone(),
            render_shader
                .entry_point("main")
                .to_renderer_err("no main in scene.comp")?,
            &compute_spec_constant,
            None,
            |_| {},
        )
        .to_renderer_err("failed to create render compute pipeline")
    }

    pub fn create_desc_set_render_image(
        scene_pipeline: Arc<ComputePipeline>,
        render_image: Arc<ImageView<StorageImage>>,
    ) -> Result<Arc<PersistentDescriptorSet>, RenderManagerError> {
        PersistentDescriptorSet::new(
            scene_pipeline
                .layout()
                .set_layouts()
                .get(descriptor::SET_IMAGE)
                .unwrap()
                .to_owned(),
            [WriteDescriptorSet::image_view(
                descriptor::BINDING_IMAGE,
                render_image,
            )],
        )
        .to_renderer_err("unable to create render compute shader descriptor set")
    }

    pub fn update_frame(
        &mut self,
        primitives: &PrimitiveCollection,
    ) -> Result<(), RenderManagerError> {
        self.desc_set_primitives = ScenePass::create_desc_set_primitives(
            self.pipeline.clone(),
            primitives,
            &self.primitive_buffer_pool,
        )?;
        Ok(())
    }

    pub fn record_commands(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        camera_push_constant: CameraPushConstant,
    ) -> Result<(), RenderManagerError> {
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
            .dispatch(self.work_group_count)
            .to_renderer_err("failed to dispatch compute shader")?;
        Ok(())
    }
}
impl ScenePass {
    fn new_primitives_buffer(
        primitives: &PrimitiveCollection,
        buffer_pool: &CpuBufferPool<PrimitiveDataUnit>,
    ) -> Result<Arc<CpuBufferPoolChunk<PrimitiveDataUnit, Arc<StdMemoryPool>>>, RenderManagerError>
    {
        // todo should be able to update buffer wihtout updating descriptor set?
        buffer_pool
            .chunk(PrimitiveData::combined_data(primitives))
            .to_renderer_err("unable to create primitives subbuffer")
    }

    /// todo shouldn't have to recreate buffer for each update...
    fn create_desc_set_primitives(
        scene_pipeline: Arc<ComputePipeline>,
        primitives: &PrimitiveCollection,
        primitive_buffer_pool: &CpuBufferPool<PrimitiveDataUnit>,
    ) -> Result<Arc<PersistentDescriptorSet>, RenderManagerError> {
        PersistentDescriptorSet::new(
            scene_pipeline
                .layout()
                .set_layouts()
                .get(descriptor::SET_PRIMITVES)
                .to_renderer_err("no compute shader primitives descriptor set layout")?
                .to_owned(),
            [WriteDescriptorSet::buffer(
                descriptor::BINDING_PRIMITVES,
                Self::new_primitives_buffer(primitives, primitive_buffer_pool)?,
            )],
        )
        .to_renderer_err("unable to create render compute shader descriptor set")
    }
}
