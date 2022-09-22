use super::render_manager::{RenderManagerError, RenderManagerUnrecoverable};
use crate::shaders::shader_interfaces::{self, PrimitivesStorageBuffer};
use glam::Vec3;
use std::sync::Arc;
use vulkano::{
    buffer::{cpu_pool::CpuBufferPoolChunk, BufferUsage, CpuBufferPool},
    device::Device,
    memory::pool::StdMemoryPool,
    DeviceSize,
};

// todo tests...

const DATA_SIZE: DeviceSize = 4;
const MAX_DATA_COUNT: DeviceSize = 1024;

mod primitve_codes {
    pub const NULL: u32 = 0xFFFFFFFF;
    pub const SPHERE: u32 = 0x7FFFFFFF;
}

pub struct Primitives {
    encoded_data: PrimitivesStorageBuffer,
    buffer_pool: CpuBufferPool<u32>,
}
// Public functions
impl Primitives {
    pub fn new(device: Arc<Device>) -> Result<Self, RenderManagerError> {
        let encoded_data = PrimitivesStorageBuffer::default();
        let buffer_pool = CpuBufferPool::new(device.clone(), BufferUsage::storage_buffer());
        buffer_pool
            .reserve(DATA_SIZE * MAX_DATA_COUNT)
            .to_renderer_err("unable to reserve primitives buffer")?;
        Ok(Self {
            encoded_data,
            buffer_pool,
        })
    }

    pub fn buffer(
        &mut self,
    ) -> Result<Arc<CpuBufferPoolChunk<u32, Arc<StdMemoryPool>>>, RenderManagerError> {
        // todo should be able to update buffer wihtout updating descriptor set?
        // todo better way of handling this case...
        assert!(self.encoded_data.primitive_data.len() < u32::MAX as usize);
        self.encoded_data.data_len = self.encoded_data.primitive_data.len() as u32;
        self.buffer_pool
            .chunk(self.encoded_data.combined_data())
            .to_renderer_err("unable to create primitives subbuffer")
    }

    pub fn add_sphere(&mut self, position: Vec3, radius: f32) {
        let sphere_data: [u32; shader_interfaces::PRIMITIVE_UNIT_LEN] = [
            primitve_codes::SPHERE,
            position.x.to_bits(),
            position.y.to_bits(),
            position.z.to_bits(),
            radius.to_bits(),
            // padding
            primitve_codes::NULL,
            primitve_codes::NULL,
            primitve_codes::NULL,
        ];
        self.encoded_data
            .primitive_data
            .extend_from_slice(&sphere_data);
    }
}
