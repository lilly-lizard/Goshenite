use crate::{config, helper::angle::Angle};
use anyhow::ensure;
use glam::{DMat3, DMat4, DVec2, DVec3};

/// Defines where the camera is looking at. Can set to either a direction or target
#[derive(Debug, Clone, Copy)]
pub enum LookMode {
    /// Look in a given direction
    Direction(DVec3),
    /// Look at a target position
    Target(DVec3),
}

/// Describes the orientation and properties of a camera that can be used for perspective rendering
#[derive(Debug, Copy, Clone)]
pub struct Camera {
    /// Position in world space
    position: DVec3,
    /// Defines where the camera is looking at
    look_mode: LookMode,
    /// Cross product of the looking direction and configured world space up
    normal: DVec3,
    /// Field of View
    fov: Angle,
    /// Aspect ratio
    aspect_ratio: f32,
}
// Public functions
impl Camera {
    pub fn new(resolution: [f32; 2]) -> anyhow::Result<Self> {
        let position = DVec3::splat(3.);
        let target = DVec3::ZERO;
        let direction = target - position;
        let up = config::WORLD_SPACE_UP.to_dvec3();
        // ensures initial normal value won't be 0
        ensure!(
            up != DVec3::X,
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
        let [horizontal, vertical] = delta_angle.map(|a| a.radians());
        let normal = self.normal.normalize();
        match self.look_mode {
            LookMode::Direction(direction) => {
                // no lock-on target so maintain position and arcball direction
                let rotation_matrix =
                    DMat3::from_axis_angle(normal, -vertical) * DMat3::from_rotation_z(horizontal);
                self.look_mode = LookMode::Direction(rotation_matrix * direction);
            }
            LookMode::Target(target) => {
                // lock on target stays the same but camera position rotates around it
                let rotation_matrix =
                    DMat3::from_axis_angle(normal, vertical) * DMat3::from_rotation_z(-horizontal);
                self.set_position(rotation_matrix * (self.position - target) + target);
            }
        }
        // update normal now that camera orientation has changed
        self.update_normal();
    }

    /// Move camera position forwards/backwards according to cursor scroll value
    pub fn scroll_zoom(&mut self, scroll_delta: f64) {
        match self.look_mode {
            LookMode::Direction(direction) => {
                // move linearly in direction vector
                self.set_position(self.position + scroll_delta * direction);
            }
            LookMode::Target(target) => {
                // move towards/away from target. can never quite reach target because lim(scroll_delta -> ∞) = 1
                let target_vector = target - self.position;
                let travel = target_vector * (1. - 1. / (-scroll_delta).exp());
                let new_position = self.position + travel;
                let new_target_vector = target - new_position;
                let new_target_dist = new_target_vector.length();
                // clamp distance to target
                if config::CAMERA_MIN_TARGET_DISTANCE < new_target_dist
                    && new_target_dist < config::CAMERA_MAX_TARGET_DISTANCE
                {
                    self.set_position(new_position);
                }
            }
        }
    }

    // Setters

    /// Set the aspect ratio given the screen dimensions
    pub fn set_aspect_ratio(&mut self, resolution: [f32; 2]) {
        self.aspect_ratio = calc_aspect_ratio(resolution);
    }

    /// Sets look mode to [`LookMode::Target`] with camera aiming at `target`
    pub fn set_lock_on_target(&mut self, target: DVec3) {
        self.look_mode = LookMode::Target(target);
    }

    /// Sets look mode to [`LookMode::Direction`]
    pub fn unset_lock_on_target(&mut self) {
        if let LookMode::Target(target) = self.look_mode {
            self.look_mode = LookMode::Direction((target - self.position).normalize());
        }
    }

    // Getters

    /// Retrurns the view transform matrix
    pub fn view_matrix(&self) -> DMat4 {
        // either look at the lock-on target or in self.direction
        let target = match self.look_mode {
            LookMode::Direction(direction) => self.position + direction,
            LookMode::Target(target) => target,
        };
        DMat4::look_at_rh(self.position, target, config::WORLD_SPACE_UP.to_dvec3())
    }

    /// Returns the projection transfor matrix
    pub fn proj_matrix(&self) -> DMat4 {
        DMat4::perspective_rh(
            self.fov.radians(),
            self.aspect_ratio as f64,
            config::CAMERA_NEAR_PLANE,
            config::CAMERA_FAR_PLANE,
        )
    }

    /// Position in world space
    pub fn position(&self) -> DVec3 {
        self.position
    }
}
// Private functions
impl Camera {
    /// Sets direction and calculates normal
    fn update_normal(&mut self) {
        // only set normal if cross product won't be zero i.e. normal doesn't change if facing up
        let up = config::WORLD_SPACE_UP.to_dvec3();
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

    /// Sets the camera position if `new_pos` doesn't contain NaN or +-inf
    fn set_position(&mut self, new_pos: DVec3) {
        if new_pos.is_finite() {
            self.position = new_pos;
        }
    }
}

fn calc_aspect_ratio(resolution: [f32; 2]) -> f32 {
    resolution[0] / resolution[1]
}
