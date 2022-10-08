use crate::helper::angle::Angle;
use glam::DVec3;
use log::LevelFilter;

pub const ENGINE_NAME: &str = "Goshenite";

/// Default log level filter
pub const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Info;

/// If true, enables spammy debug log messages that happen every frame. Will only show if max log level
/// is set to `LevelFilter::Debug`.
pub const PER_FRAME_DEBUG_LOGS: bool = false;

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
pub const SCROLL_SENSITIVITY: f64 = 1.0;

// Renderer Settings

pub const VULKAN_VER_MAJ: u32 = 1;
pub const VULKAN_VER_MIN: u32 = 2;
/// If true, the renderer will attempt to enable khronos valication layer. If VK_LAYER_KHRONOS_validation
/// is installed on the system, a debug callback will be created to log layer messages.
pub const ENABLE_VULKAN_VALIDATION: bool = cfg!(debug_assertions);

/// You'll notice in src/shaders there's .cxx versions of the glsl shaders. These are equivilent
/// source files using C++ language features thanks to the [circle](https://www.circle-lang.org/)
/// compiler. This flag tells parts of the renderer to use circle compiled versions of shaders.
///
/// For more info on using circle for shader compilation see https://github.com/seanbaxter/shaders.
pub const USE_CIRCLE_SHADERS: bool = true;
