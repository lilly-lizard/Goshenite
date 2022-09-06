use crate::config;
use glam::{Mat3, Mat4, Vec3};
use std::f32::consts::PI;

const NEAR_PLANE: f32 = 0.01;
const FAR_PLANE: f32 = 100.;

#[derive(Copy, Clone)]
pub struct Camera {
    /// Position in world space
    position: Vec3,
    /// View direction in world space
    direction: Vec3,
    /// Perpindicular to direction and up = cross(up, direction)
    normal: Vec3,
    /// Field of View in radians
    fov: f32,
    /// Aspect ratio
    aspect_ratio: f32,
}
impl Default for Camera {
    fn default() -> Self {
        let direction = Vec3::new(1., 0., 0.);
        Camera {
            position: Vec3::new(-5., 0., 0.),
            direction,
            normal: config::WORLD_SPACE_UP.cross(direction),
            fov: 0.5 * PI,
            aspect_ratio: 1.,
        }
    }
}
// Public functions
impl Camera {
    pub fn new(resolution: [i32; 2]) -> Self {
        Camera {
            aspect_ratio: Self::calc_aspect_ratio(resolution),
            ..Self::default()
        }
    }

    pub fn rotate(&mut self, radians_horizontal: f32, radians_vertical: f32) {
        let new_direction = Mat3::from_axis_angle(self.normal, radians_vertical)
            * Mat3::from_rotation_z(radians_horizontal)
            * self.direction;
        self.set_direction_and_normal(new_direction);
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(
            self.position,
            self.position + self.direction,
            config::WORLD_SPACE_UP,
        )
    }

    pub fn proj_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect_ratio, NEAR_PLANE, FAR_PLANE)
    }

    // Setters
    pub fn set_aspect_ratio(&mut self, resolution: [i32; 2]) {
        self.aspect_ratio = Self::calc_aspect_ratio(resolution);
    }
    fn calc_aspect_ratio(resolution: [i32; 2]) -> f32 {
        resolution[0] as f32 / resolution[1] as f32
    }

    // Getters
    /// Position in world space
    pub fn get_position(&self) -> Vec3 {
        self.position
    }
    /// View direction in world space
    pub fn get_direction(&self) -> Vec3 {
        self.direction
    }
    /// Perpindicular to direction and up = cross(up, direction)
    pub fn get_normal(&self) -> Vec3 {
        self.normal
    }
    /// Field of View in radians
    pub fn get_fov(&self) -> f32 {
        self.fov
    }
    /// Aspect ratio
    pub fn get_aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }
}
// Private functions
impl Camera {
    /// Sets direction and calculates normal
    fn set_direction_and_normal(&mut self, direction: Vec3) {
        self.direction = direction;
        // only set normal if cross product won't be zero i.e. normal doesn't change if facing up
        if direction != config::WORLD_SPACE_UP {
            self.normal = Vec3::normalize(config::WORLD_SPACE_UP.cross(direction));
        }
    }
}
