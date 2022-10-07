use crate::{config, helper::angle::Angle};
use anyhow::ensure;
use glam::{DVec2, Mat3, Mat4, Vec3};

const NEAR_PLANE: f32 = 0.01;
const FAR_PLANE: f32 = 100.;

/// Defines where the camera is looking at. Can set to either a direction or target
#[derive(Debug, Clone, Copy)]
enum LookMode {
    /// Look in a given direction
    Direction(Vec3),
    /// Look at a target position
    Target(Vec3),
}

/// Describes the orientation and properties of a camera that can be used for perspective rendering
#[derive(Debug, Copy, Clone)]
pub struct Camera {
    /// Position in world space
    position: Vec3,
    /// Defines where the camera is looking at
    look_mode: LookMode,
    /// Cross product of the looking direction and configured world space up
    normal: Vec3,
    /// Field of View
    fov: Angle,
    /// Aspect ratio
    aspect_ratio: f32,
}
// Public functions
impl Camera {
    pub fn new(resolution: [f32; 2]) -> anyhow::Result<Self> {
        let position = Vec3::splat(3.);
        let target = Vec3::ZERO;
        let direction = target - position;
        let up = config::WORLD_SPACE_UP.to_vec3();
        ensure!(
            // ensures initial normal value won't be 0
            up != Vec3::X,
            "config::WORLD_SPACE_UP can not be set to the x axis. this is a bug!"
        );
        let normal = up.cross(direction);
        Ok(Camera {
            position,
            look_mode: LookMode::Direction(direction),
            normal: normal.normalize(),
            fov: config::FIELD_OF_VIEW,
            aspect_ratio: calc_aspect_ratio(resolution),
        })
    }

    /// Changes the viewing direction based on the pixel amount the cursor has moved
    pub fn rotate(&mut self, delta_cursor_position: DVec2) {
        let delta_angle = self.delta_cursor_to_angle(delta_cursor_position.into());
        let [horizontal, vertical] = delta_angle.map(|a| a.radians() as f32);
        let normal = self.normal.normalize();
        match self.look_mode {
            LookMode::Direction(direction) => {
                // no lock-on target so maintain position and arcball direction
                let rotation_matrix =
                    Mat3::from_axis_angle(normal, -vertical) * Mat3::from_rotation_z(horizontal);
                self.look_mode = LookMode::Direction(rotation_matrix * direction);
            }
            LookMode::Target(target) => {
                // lock on target stays the same but camera position rotates around it
                let rotation_matrix =
                    Mat3::from_axis_angle(normal, vertical) * Mat3::from_rotation_z(-horizontal);
                self.position = rotation_matrix * (self.position - target) + target;
            }
        }
        // update normal now that camera orientation has changed
        self.update_normal();
    }

    // Setters

    /// Set the aspect ratio given the screen dimensions
    pub fn set_aspect_ratio(&mut self, resolution: [f32; 2]) {
        self.aspect_ratio = calc_aspect_ratio(resolution);
    }

    pub fn set_lock_on_target(&mut self, target: Vec3) {
        self.look_mode = LookMode::Target(target);
    }

    pub fn unset_lock_on_target(&mut self) {
        if let LookMode::Target(target) = self.look_mode {
            self.look_mode = LookMode::Direction((target - self.position).normalize());
        }
    }

    // Getters

    /// Retrurns the view transform matrix
    pub fn view_matrix(&self) -> Mat4 {
        // either look at the lock-on target or in self.direction
        let target = match self.look_mode {
            LookMode::Direction(direction) => self.position + direction,
            LookMode::Target(target) => target,
        };
        Mat4::look_at_rh(self.position, target, config::WORLD_SPACE_UP.to_vec3())
    }

    /// Returns the projection transfor matrix
    pub fn proj_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.fov.radians() as f32,
            self.aspect_ratio,
            NEAR_PLANE,
            FAR_PLANE,
        )
    }

    /// Position in world space
    pub fn position(&self) -> Vec3 {
        self.position
    }
}
// Private functions
impl Camera {
    /// Sets direction and calculates normal
    fn update_normal(&mut self) {
        // only set normal if cross product won't be zero i.e. normal doesn't change if facing up
        let up = config::WORLD_SPACE_UP.to_vec3();
        let direction = match self.look_mode {
            LookMode::Direction(direction) => direction,
            LookMode::Target(target) => target - self.position,
        };
        if direction != up {
            self.normal = up.cross(direction);
        }
    }

    /// Converts cursor position vector to an angle vector. Sensitivity depends on the current look-mode
    fn delta_cursor_to_angle(&self, delta_cursor_position: [f64; 2]) -> [Angle; 2] {
        delta_cursor_position.map(|delta| match self.look_mode {
            LookMode::Direction(_) => {
                Angle::from_radians(delta * config::LOOK_SENSITIVITY.radians())
            }
            LookMode::Target(_) => {
                Angle::from_radians(delta * config::ARC_BALL_SENSITIVITY.radians())
            }
        })
    }
}

fn calc_aspect_ratio(resolution: [f32; 2]) -> f32 {
    resolution[0] / resolution[1]
}
