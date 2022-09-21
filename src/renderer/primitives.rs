use super::render_manager::{RenderManagerError, RenderManagerUnrecoverable};
use glam::Vec3;
use std::sync::Arc;
use vulkano::{
    buffer::{cpu_pool::CpuBufferPoolChunk, BufferUsage, CpuBufferPool},
    device::Device,
    memory::pool::StdMemoryPool,
    DeviceSize,
};

const DATA_SIZE: DeviceSize = 4;
const MAX_DATA_COUNT: DeviceSize = 1024;

mod primitve_codes {
    pub const NULL: u32 = 0xFFFFFFFF;
    pub const SPHERE: u32 = 0x7FFFFFFF;
}

pub struct Primitives {
    data: Vec<u32>,
    buffer_pool: CpuBufferPool<u32>,
    buffer: Arc<CpuBufferPoolChunk<u32, Arc<StdMemoryPool>>>,
}
// Public functions
impl Primitives {
    pub fn new(device: Arc<Device>) -> Result<Self, RenderManagerError> {
        let data = vec![primitve_codes::NULL];
        let buffer_pool = CpuBufferPool::new(device.clone(), BufferUsage::storage_buffer());
        buffer_pool
            .reserve(DATA_SIZE * MAX_DATA_COUNT)
            .to_renderer_err("unable to reserve primitives buffer")?;
        let buffer = Self::update_buffer_access(&data, &buffer_pool)?;
        Ok(Self {
            data,
            buffer_pool,
            buffer,
        })
    }

    pub fn buffer(&self) -> Arc<CpuBufferPoolChunk<u32, Arc<StdMemoryPool>>> {
        self.buffer = Self::update_buffer_access(&self.data, &self.buffer_pool)?;
        self.buffer.clone()
    }

    pub fn add_sphere(&mut self, position: Vec3, radius: f32) {
        self.data.push(primitve_codes::SPHERE);
        self.data.push(position.x.to_bits());
        self.data.push(position.y.to_bits());
        self.data.push(position.z.to_bits());
        self.data.push(radius.to_bits());
    }
}
// Private functions
impl Primitives {
    pub fn update_buffer_access(
        data: &Vec<u32>,
        buffer_pool: &CpuBufferPool<u32>,
    ) -> Result<Arc<CpuBufferPoolChunk<u32, Arc<StdMemoryPool>>>, RenderManagerError> {
        // todo return err instead of assert
        assert!(data.len() < u32::MAX as usize);
        let mut combined_data = data.clone();
        combined_data.insert(0, data.len() as u32);
        buffer_pool
            .chunk(combined_data)
            .to_renderer_err("unable to create primitives subbuffer")
    }
}
