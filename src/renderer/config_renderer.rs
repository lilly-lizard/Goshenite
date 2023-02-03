use vulkano::format::Format;

pub const VULKAN_VER_MAJ: u32 = 1;
pub const VULKAN_VER_MIN: u32 = 2;
/// If true, the renderer will attempt to enable khronos valication layer. If VK_LAYER_KHRONOS_validation
/// is installed on the system, a debug callback will be created to log layer messages.
pub const ENABLE_VULKAN_VALIDATION: bool = cfg!(debug_assertions); // pending https://github.com/KhronosGroup/Vulkan-ValidationLayers/issues/4891

pub const FRAMES_IN_FLIGHT: usize = 2;

/// Function name of the entry point for shaders
pub const SHADER_ENTRY_POINT: &str = "main";

/// G-buffer formats. Note that the combined bit total of these should be under 128bits to fit in tile storage on many tile-based architectures.
pub const G_BUFFER_FORMAT_NORMAL: Format = Format::R8G8B8A8_UNORM;
pub const G_BUFFER_FORMAT_PRIMITIVE_ID: Format = Format::R32_UINT;
