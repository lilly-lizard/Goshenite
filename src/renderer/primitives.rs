use super::render_manager::{RenderManagerError, RenderManagerUnrecoverable};
use std::sync::Arc;
use vulkano::{
    buffer::{cpu_pool::CpuBufferPoolChunk, BufferUsage, CpuBufferPool},
    device::Device,
    memory::pool::StdMemoryPool,
    DeviceSize,
};

const DATA_SIZE: DeviceSize = 4;
const MAX_DATA_COUNT: DeviceSize = 1024;

pub struct Primitives {
    buffer_pool: CpuBufferPool<u32>,
    data: Vec<u32>,
}
// Public functions
impl Primitives {
    pub fn new(device: Arc<Device>) -> Result<Self, RenderManagerError> {
        let buffer_pool = CpuBufferPool::new(device.clone(), BufferUsage::storage_buffer());
        buffer_pool
            .reserve(DATA_SIZE * MAX_DATA_COUNT)
            .to_renderer_err("unable to reserve primitives buffer")?;
        let data = vec![0u32];
        Ok(Self { data, buffer_pool })
    }

    pub fn buffer_access(
        &self,
    ) -> Result<Arc<CpuBufferPoolChunk<u32, Arc<StdMemoryPool>>>, RenderManagerError> {
        self.buffer_pool
            .chunk(self.data.clone())
            .to_renderer_err("unable to create primitives subbuffer")
    }
}
