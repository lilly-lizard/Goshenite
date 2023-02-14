use crate::device::Device;
use anyhow::Context;
use ash::vk;
use std::sync::Arc;
use vk_mem::AllocatorCreateInfo;

pub const ALLOCATION_CALLBACK_NONE: Option<&ash::vk::AllocationCallbacks> = None;

// Memory Allocator

pub struct MemoryAllocator {
    inner: vk_mem::Allocator,

    // dependencies
    device: Arc<Device>,
}

impl MemoryAllocator {
    pub fn new(device: Arc<Device>) -> anyhow::Result<Self> {
        let allocator_info = AllocatorCreateInfo::new(
            device.instance().inner(),
            device.inner(),
            device.physical_device().handle(),
        );
        let inner = vk_mem::Allocator::new(allocator_info).context("creating vma allocator")?;

        Ok(Self { inner, device })
    }

    // Getters

    pub fn inner(&self) -> &vk_mem::Allocator {
        &self.inner
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

// Helper Functions

pub fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    memory_prop.memory_types[..memory_prop.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1 << index) & memory_req.memory_type_bits != 0
                && memory_type.property_flags & flags == flags
        })
        .map(|(index, _memory_type)| index as _)
}
