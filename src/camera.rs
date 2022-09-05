use crate::config;
use glam::{Mat4, Vec3};
use std::f32::consts::PI;

const NEAR_PLANE: f32 = 0.01;
const FAR_PLANE: f32 = 100.;

#[derive(Copy, Clone)]
pub struct Camera {
    /// Camera position in world space
    pub position: Vec3,
    /// Camera target in world space
    pub target: Vec3,
    /// Field of View in radians
    pub fov: f32,
    /// Aspect ratio
    pub aspect_ratio: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Camera {
            position: glam::Vec3::new(-5., 0., 0.),
            target: glam::Vec3::splat(0.),
            fov: 0.5 * PI,
            aspect_ratio: 1.,
        }
    }
}
impl Camera {
    pub fn new(resolution: [i32; 2]) -> Self {
        Camera {
            aspect_ratio: Self::calc_aspect_ratio(resolution),
            ..Self::default()
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, config::WORLD_SPACE_UP)
    }

    pub fn proj_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect_ratio, NEAR_PLANE, FAR_PLANE)
    }
}

// Setters
impl Camera {
    pub fn set_aspect_ratio(&mut self, resolution: [i32; 2]) {
        self.aspect_ratio = Self::calc_aspect_ratio(resolution);
    }

    fn calc_aspect_ratio(resolution: [i32; 2]) -> f32 {
        resolution[0] as f32 / resolution[1] as f32
    }
}
