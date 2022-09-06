use glam::Vec3;

pub const ENGINE_NAME: &str = "Goshenite";

pub const WORLD_SPACE_UP: Vec3 = Vec3::new(0., 0., 1.);
pub const SENSITIVITY_LOOK: f64 = 0.001; // = radians / pixels

// renderer
pub const VULKAN_VER_MAJ: u32 = 1;
pub const VULKAN_VER_MIN: u32 = 3;
pub const DEFAULT_WORK_GROUP_SIZE: [u32; 2] = [16, 16];
