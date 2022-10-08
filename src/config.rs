use glam::Vec3;
use log::LevelFilter;

use crate::helper::angle::Angle;

pub const ENGINE_NAME: &str = "Goshenite";

/// Default log level filter
pub const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Info;

/// If true, enables spammy debug log messages that happen every frame
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
/// Allow converting to xyz coordinates
impl From<WorldSpaceUp> for Vec3 {
    #[inline]
    fn from(w: WorldSpaceUp) -> Vec3 {
        match w {
            WorldSpaceUp::Y => Vec3::Y,
            WorldSpaceUp::Z => Vec3::Z,
        }
    }
}
impl WorldSpaceUp {
    #[inline]
    pub fn to_vec3(self) -> Vec3 {
        self.into()
    }
}

/// Field of view
pub const FIELD_OF_VIEW: Angle = Angle::from_radians(std::f64::consts::FRAC_PI_4);
/// Sensitivity for changing the view direction with the cursor = angle / pixels
pub const LOOK_SENSITIVITY: Angle = Angle::from_radians(0.001);
pub const ARC_BALL_SENSITIVITY: Angle = Angle::from_radians(0.005);

// renderer settings
pub const VULKAN_VER_MAJ: u32 = 1;
pub const VULKAN_VER_MIN: u32 = 2;
/// If true, the renderer will attempt to enable valication layers
pub const ENABLE_VULKAN_VALIDATION: bool = cfg!(debug_assertions);

/// You'll notice in src/shaders there's .cxx versions of the glsl shaders. These are equivilent
/// source files using C++ language features thanks to the [circle](https://www.circle-lang.org/)
/// compiler. This flag tells parts of the renderer to use circle compiled versions of shaders.
///
/// For more info on using circle for shader compilation see https://github.com/seanbaxter/shaders.
pub const USE_CIRCLE_SHADERS: bool = true;
