use crate::{
    config,
    engine::{object::object::ObjectRef, primitives::primitive::Primitive},
    helper::angle::Angle,
};
use anyhow::ensure;
use glam::{DMat3, DMat4, DVec2, DVec3};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::rc::Weak;

#[derive(Clone)]
pub enum LookTargetType {
    Position(DVec3),
    Object(Weak<ObjectRef>),
    Primitive(Weak<dyn Primitive>),
}

#[derive(Clone)]
pub enum LookMode {
    /// Look in a given direction
    Direction(),
    /// Lock on to a target position
    Target(LookTargetType),
}

/// Describes the orientation and properties of a camera that can be used for perspective rendering
#[derive(Clone)]
pub struct Camera {
    position: DVec3,
    look_mode: LookMode,
    direction: DVec3,
    normal: DVec3,
    fov: Angle,
    aspect_ratio: f32,
}
// Public functions
impl Camera {
    pub fn new(resolution: [f32; 2]) -> anyhow::Result<Self> {
        let position = DVec3::splat(3.);
        let target_pos = DVec3::ZERO;
        let direction = target_pos - position;
        let up = config::WORLD_SPACE_UP.as_dvec3();
        // ensures initial normal value won't be 0
        ensure!(
            up != DVec3::X,
            "config::WORLD_SPACE_UP can not be set to the x axis. this is a bug!"
        );
        let normal = up.cross(direction).normalize();
        Ok(Camera {
            position,
            look_mode: LookMode::Direction(),
            direction,
            normal,
            fov: config::FIELD_OF_VIEW,
            aspect_ratio: calc_aspect_ratio(resolution),
        })
    }

    /// Changes the viewing direction based on the pixel amount the cursor has moved
    pub fn rotate(&mut self, delta_cursor_position: DVec2) {
        let delta_angle = self.delta_cursor_to_angle(delta_cursor_position.into());
        let [horizontal, vertical] = delta_angle.map(|a| a.radians());
        match &self.look_mode {
            // no lock-on target so maintain position adjust looking direction
            LookMode::Direction() => {
                let rotation_matrix = DMat3::from_axis_angle(self.normal, -vertical)
                    * DMat3::from_rotation_z(horizontal);
                self.direction = rotation_matrix * self.direction;
            }
            // lock on target stays the same but camera position rotates around it
            LookMode::Target(target_type) => {
                if let Some(target_pos) =
                    self.get_target_position_or_switch_look_modes(target_type.clone())
                {
                    self.arcball(self.normal, target_pos, vertical, horizontal);
                }
            }
        }
        // update normal now that camera orientation has changed
        self.update_normal();
    }

    /// Move camera position forwards/backwards according to cursor scroll value
    pub fn scroll_zoom(&mut self, scroll_delta: f64) {
        match &self.look_mode {
            LookMode::Direction() => {
                self.set_position(self.position + scroll_delta * self.direction);
            }
            LookMode::Target(target_type) => {
                if let Some(target_pos) =
                    self.get_target_position_or_switch_look_modes(target_type.clone())
                {
                    self.scroll_zoom_target(scroll_delta, target_pos);
                }
            }
        }
    }

    // Setters

    pub fn set_aspect_ratio(&mut self, resolution: [f32; 2]) {
        self.aspect_ratio = calc_aspect_ratio(resolution);
    }

    pub fn set_lock_on_target(&mut self, target_pos: DVec3) {
        self.look_mode = LookMode::Target(LookTargetType::Position(target_pos));
    }

    pub fn set_lock_on_object(&mut self, object: Weak<ObjectRef>) {
        self.look_mode = LookMode::Target(LookTargetType::Object(object));
    }

    pub fn set_lock_on_primitive(&mut self, primitive: Weak<dyn Primitive>) {
        self.look_mode = LookMode::Target(LookTargetType::Primitive(primitive));
    }

    pub fn unset_lock_on_target(&mut self) {
        if let LookMode::Target(target_type) = &self.look_mode {
            if let Some(target_pos) = target_pos(target_type.clone()) {
                self.set_direction(target_pos);
            }
            self.look_mode = LookMode::Direction();
        }
    }

    // Getters

    pub fn view_matrix(&mut self) -> DMat4 {
        // either look at the lock-on target or in self.direction
        let target_pos = match &self.look_mode {
            LookMode::Direction() => self.position + self.direction,
            LookMode::Target(target_type) => {
                if let Some(target_pos) =
                    self.get_target_position_or_switch_look_modes(target_type.clone())
                {
                    target_pos
                } else {
                    self.position + self.direction
                }
            }
        };
        DMat4::look_at_rh(self.position, target_pos, config::WORLD_SPACE_UP.as_dvec3())
    }

    pub fn proj_matrix(&self) -> DMat4 {
        DMat4::perspective_rh(
            self.fov.radians(),
            self.aspect_ratio as f64,
            config::CAMERA_NEAR_PLANE,
            config::CAMERA_FAR_PLANE,
        )
    }

    pub fn position(&self) -> DVec3 {
        self.position
    }

    pub fn look_mode(&self) -> LookMode {
        self.look_mode.clone()
    }
}

// Private functions

impl Camera {
    fn update_normal(&mut self) {
        let direction = match &self.look_mode {
            LookMode::Direction() => self.direction,
            LookMode::Target(target_type) => {
                if let Some(target_pos) =
                    self.get_target_position_or_switch_look_modes(target_type.clone())
                {
                    target_pos - self.position
                } else {
                    self.direction
                }
            }
        };
        // only set normal if cross product won't be zero i.e. normal doesn't change if facing up
        let up = config::WORLD_SPACE_UP.as_dvec3();
        if direction != up {
            self.normal = up.cross(direction).normalize();
        }
    }

    fn delta_cursor_to_angle(&self, delta_cursor_position: [f64; 2]) -> [Angle; 2] {
        delta_cursor_position.map(|delta| match self.look_mode {
            LookMode::Direction() => {
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

    fn set_direction(&mut self, target_pos: DVec3) {
        self.direction = (target_pos - self.position).normalize()
    }

    fn arcball(
        &mut self,
        normal: DVec3,
        target_pos: DVec3,
        delta_vertical: f64,
        delta_horizontal: f64,
    ) {
        // lock on target stays the same but camera position rotates around it
        let rotation_matrix = DMat3::from_axis_angle(normal, delta_vertical)
            * DMat3::from_rotation_z(-delta_horizontal);
        self.set_position(rotation_matrix * (self.position - target_pos) + target_pos);
        self.set_direction(target_pos);
    }

    fn scroll_zoom_target(&mut self, scroll_delta: f64, target_pos: DVec3) {
        // move towards/away from target. can never quite reach target because lim(scroll_delta -> ∞) = 1
        let target_vector = self.direction;
        let travel = target_vector * (1. - 1. / (-scroll_delta).exp());
        let new_position = self.position + travel;
        let new_target_vector = target_pos - new_position;
        let new_target_dist = new_target_vector.length();
        // clamp distance to target
        if config::CAMERA_MIN_TARGET_DISTANCE < new_target_dist
            && new_target_dist < config::CAMERA_MAX_TARGET_DISTANCE
        {
            self.set_position(new_position);
        }
    }

    /// Get the target position depending on the type of `target_type`. If the target position is
    /// tied to a dropped reference, unsets lock-on mode.
    fn get_target_position_or_switch_look_modes(
        &mut self,
        target_type: LookTargetType,
    ) -> Option<DVec3> {
        if let Some(target_pos) = target_pos(target_type) {
            Some(target_pos)
        } else {
            debug!("dropped target reference -> switching to `LookMode::Direction`.");
            self.unset_lock_on_target();
            None
        }
    }
}

/// Get the target position depending on the type of `target_type`. If value pointed to by a weak
/// reference has been dropped, returns `None`.
fn target_pos(target_type: LookTargetType) -> Option<DVec3> {
    match target_type {
        LookTargetType::Position(position) => Some(position),
        LookTargetType::Object(object_ref) => {
            if let Some(object) = object_ref.upgrade() {
                Some(object.borrow().origin.as_dvec3())
            } else {
                debug!("camera target object reference no longer present...");
                None
            }
        }
        LookTargetType::Primitive(primitive_ref) => {
            if let Some(primitive) = primitive_ref.upgrade() {
                Some(primitive.center().as_dvec3())
            } else {
                debug!("camera target object reference no longer present...");
                None
            }
        }
    }
}

fn calc_aspect_ratio(resolution: [f32; 2]) -> f32 {
    resolution[0] / resolution[1]
}