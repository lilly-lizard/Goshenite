use ash::vk;

pub const MAX_VULKAN_VER_MAJ: u32 = 1;
pub const MAX_VULKAN_VER_MIN: u32 = 3;
/// If true, the renderer will attempt to enable khronos valication layer. If VK_LAYER_KHRONOS_validation
/// is installed on the system, a debug callback will be created to log layer messages.
pub const ENABLE_VULKAN_VALIDATION: bool = cfg!(debug_assertions);

/// Function name of the entry point for shaders
pub const SHADER_ENTRY_POINT: &str = "main";

/// G-buffer formats. Note that the combined bit total of these should be under 128bits to fit in tile storage on many tile-based architectures.
pub const FORMAT_NORMAL_BUFFER: vk::Format = vk::Format::R8G8B8A8_UNORM;
pub const FORMAT_PRIMITIVE_ID_BUFFER: vk::Format = vk::Format::R32_UINT;

/// 1 second
pub const TIMEOUT_NANOSECS: u64 = 1_000_000_000;

/// Double-buffering
pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub const CPU_ACCESS_BUFFER_SIZE: vk::DeviceSize = 1024;
