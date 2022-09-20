use crate::{config, shaders::shader_interfaces};

use super::render_manager::{create_shader_module, RenderManagerError, RenderManagerUnrecoverable};
use std::sync::Arc;
use vulkano::{
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{physical::PhysicalDevice, Device},
    image::{view::ImageView, StorageImage},
    pipeline::{ComputePipeline, Pipeline},
};

pub struct ScenePass {
    pub pipeline: Arc<ComputePipeline>,
    pub desc_set: Arc<PersistentDescriptorSet>,
    pub work_group_size: [u32; 2],
    pub work_group_count: [u32; 3],
}
impl ScenePass {
    pub fn new(
        device: Arc<Device>,
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

        let pipeline = Self::create_pipeline(device.clone(), work_group_size)?;
        let desc_set = Self::create_desc_set(pipeline.clone(), render_image)?;

        Ok(Self {
            pipeline,
            desc_set,
            work_group_size,
            work_group_count,
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

        let compute_spec_constant = shader_interfaces::ComputeSpecConstant {
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

    pub fn create_desc_set(
        scene_pipeline: Arc<ComputePipeline>,
        render_image: Arc<ImageView<StorageImage>>,
    ) -> Result<Arc<PersistentDescriptorSet>, RenderManagerError> {
        PersistentDescriptorSet::new(
            scene_pipeline
                .layout()
                .set_layouts()
                .get(shader_interfaces::descriptor::SET_RENDER_COMP)
                .unwrap()
                .to_owned(),
            [WriteDescriptorSet::image_view(
                shader_interfaces::descriptor::BINDING_IMAGE,
                render_image,
            )],
        )
        .to_renderer_err("unable to create render compute shader descriptor set")
    }
}
