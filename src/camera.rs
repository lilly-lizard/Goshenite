use crate::config;
use crate::helper::angle::Radians;
use glam::{Mat3, Mat4, Vec3};

const NEAR_PLANE: f32 = 0.01;
const FAR_PLANE: f32 = 100.;

/// Describes the orientation and properties of a camera that can be used for perspective rendering
#[derive(Copy, Clone)]
pub struct Camera {
    /// Position in world space
    position: Vec3,
    /// View direction in world space
    direction: Vec3,
    /// Perpindicular to direction and up = cross(up, direction)
    normal: Vec3,
    /// Field of View
    fov: Radians,
    /// Aspect ratio
    aspect_ratio: f32,
}
// Public functions
impl Camera {
    pub fn new(resolution: [i32; 2]) -> Self {
        let direction = Vec3::new(1., 0., 0.);
        let normal = config::WORLD_SPACE_UP.to_vec3().cross(direction);
        debug_assert!(
            normal != Vec3::splat(0.),
            "config::WORLD_SPACE_UP shouldn't be x axis..."
        );
        Camera {
            position: Vec3::new(-5., 0., 0.),
            direction,
            normal: normal.normalize(),
            fov: std::f32::consts::FRAC_PI_2.into(),
            aspect_ratio: Self::calc_aspect_ratio(resolution),
        }
    }

    /// Changes the viewing direction by
    pub fn rotate(&mut self, horizontal: Radians, vertical: Radians) {
        let new_direction = Mat3::from_axis_angle(self.normal, vertical.val)
            * Mat3::from_rotation_z(horizontal.val)
            * self.direction;
        self.set_direction_and_normal(new_direction);
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(
            self.position,
            self.position + self.direction,
            config::WORLD_SPACE_UP.to_vec3(),
        )
    }

    pub fn proj_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(*self.fov, self.aspect_ratio, NEAR_PLANE, FAR_PLANE)
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
    pub fn position(&self) -> Vec3 {
        self.position
    }
    /*
    /// View direction in world space
    pub fn direction(&self) -> Vec3 {
        self.direction
    }
    /// Perpindicular to direction and up = cross(up, direction)
    pub fn normal(&self) -> Vec3 {
        self.normal
    }
    /// Field of View in radians
    pub fn fov(&self) -> f32 {
        self.fov
    }
    /// Aspect ratio
    pub fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }
    */
}
// Private functions
impl Camera {
    /// Sets direction and calculates normal
    fn set_direction_and_normal(&mut self, direction: Vec3) {
        self.direction = direction;
        // only set normal if cross product won't be zero i.e. normal doesn't change if facing up
        let up = config::WORLD_SPACE_UP.to_vec3();
        if direction != up {
            self.normal = Vec3::normalize(up.cross(direction));
        }
    }
}
