use glam::Vec3;

pub const ENGINE_NAME: &str = "Goshenite";

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

/// Sensitivity for changing the view direction with the cursor = radians / pixels
pub const SENSITIVITY_LOOK: f64 = 0.001;

// renderer settings
pub const VULKAN_VER_MAJ: u32 = 1;
pub const VULKAN_VER_MIN: u32 = 3;
pub const DEFAULT_WORK_GROUP_SIZE: [u32; 2] = [16, 16];