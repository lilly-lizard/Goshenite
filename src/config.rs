use crate::helper::angle::Angle;
use glam::DVec3;
use log::LevelFilter;
use vulkano::format::Format;

pub const ENGINE_NAME: &str = "Goshenite";

/// Log level filter. Logs of levels lower than this will not be displayed.
pub const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Info;

/// Wherever the app window starts maximized
pub const START_MAXIMIZED: bool = false;
/// Default window size if `START_MAXIMIZED` is false
pub const DEFAULT_WINDOW_SIZE: [u32; 2] = [1000, 700];

/// Describes which direction is up in the world space coordinate system, set to Z by default
pub const WORLD_SPACE_UP: WorldSpaceUp = WorldSpaceUp::Z;
/// Describes which direction is up in the world space coordinate system
#[derive(Clone, Copy)]
pub enum WorldSpaceUp {
    Y,
    Z,
}
impl From<WorldSpaceUp> for DVec3 {
    #[inline]
    fn from(w: WorldSpaceUp) -> DVec3 {
        match w {
            WorldSpaceUp::Y => DVec3::Y,
            WorldSpaceUp::Z => DVec3::Z,
        }
    }
}
impl WorldSpaceUp {
    #[inline]
    pub fn to_dvec3(self) -> DVec3 {
        self.into()
    }
}

/// Field of view
pub const FIELD_OF_VIEW: Angle = Angle::from_radians(std::f64::consts::FRAC_PI_4);
pub const CAMERA_NEAR_PLANE: f64 = 0.01;
pub const CAMERA_FAR_PLANE: f64 = 100_000.;
/// Should be ~= `CAMERA_FAR_PLANE`. Pevents view matrix from getting too crazy (too big triggers a glam_assert when calculating inverse(proj * view))
pub const CAMERA_MAX_TARGET_DISTANCE: f64 = 100_000.;
/// Minumum distance between the camera position and the camera target. Ensures valid results for view matrix etc
pub const CAMERA_MIN_TARGET_DISTANCE: f64 = 0.001;

/// Sensitivity rotating the camera in [`ViewMode::Direction`](crate::camera::ViewMode::Direction) = angle / pixels
pub const LOOK_SENSITIVITY: Angle = Angle::from_radians(0.001);
/// Sensitivity rotating the camer in [`ViewMode::Target`](crate::camera::ViewMode::Target) = angle / pixels
pub const ARC_BALL_SENSITIVITY: Angle = Angle::from_radians(0.005);
/// Scrolling sensitivity
pub const SCROLL_SENSITIVITY: f64 = 0.5;

// Renderer Settings

pub const VULKAN_VER_MAJ: u32 = 1;
pub const VULKAN_VER_MIN: u32 = 2;
/// If true, the renderer will attempt to enable khronos valication layer. If VK_LAYER_KHRONOS_validation
/// is installed on the system, a debug callback will be created to log layer messages.
pub const ENABLE_VULKAN_VALIDATION: bool = cfg!(debug_assertions); // pending https://github.com/KhronosGroup/Vulkan-ValidationLayers/issues/4891

/// Function name of the entry point for shaders
pub const SHADER_ENTRY_POINT: &str = "main";

/// G-buffer formats. Note that the combined bit total of these should be under 128bits to fit in tile storage on many tile-based architectures.
pub const G_BUFFER_FORMAT_NORMAL: Format = Format::R8G8B8A8_UNORM;
pub const G_BUFFER_FORMAT_PRIMITIVE_ID: Format = Format::R32_UINT;
