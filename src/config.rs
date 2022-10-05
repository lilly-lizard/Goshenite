use glam::Vec3;

use crate::helper::angle::Radians;

pub const ENGINE_NAME: &str = "Goshenite";

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

/// Field of view in radians
pub const FIELD_OF_VIEW: Radians = Radians::new(std::f32::consts::FRAC_PI_4);
/// Sensitivity for changing the view direction with the cursor = radians / pixels
pub const LOOK_SENSITIVITY: f64 = 0.001;

// renderer settings
pub const VULKAN_VER_MAJ: u32 = 1;
pub const VULKAN_VER_MIN: u32 = 2;
pub const DEFAULT_WORK_GROUP_SIZE: [u32; 2] = [16, 16];
/// If true, the renderer will attempt to enable valication layers
pub const ENABLE_VULKAN_VALIDATION: bool = true;
