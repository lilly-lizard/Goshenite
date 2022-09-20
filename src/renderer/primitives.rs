use super::render_manager::{RenderManagerError, RenderManagerUnrecoverable};
use std::sync::Arc;
use vulkano::{
    buffer::{BufferUsage, CpuBufferPool},
    device::Device,
    DeviceSize,
};

const DATA_SIZE: DeviceSize = 4;
const MAX_DATA_COUNT: DeviceSize = 1024;

pub struct Primitives {
    buffer: CpuBufferPool<u32>,
}

impl Primitives {
    pub fn new(device: Arc<Device>) -> Result<Self, RenderManagerError> {
        let buffer = CpuBufferPool::new(device.clone(), BufferUsage::storage_buffer());
        buffer
            .reserve(DATA_SIZE * MAX_DATA_COUNT)
            .to_renderer_err("unable to reserve primitives buffer")?;
        Ok(Self { buffer })
    }
}
