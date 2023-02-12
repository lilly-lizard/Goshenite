pub mod common;
pub mod debug_callback;
pub mod device;
pub mod instance;
pub mod physical_device;
pub mod queue;
pub mod surface;
pub mod swapchain;

const ALLOCATION_CALLBACK: Option<&ash::vk::AllocationCallbacks> = None;
