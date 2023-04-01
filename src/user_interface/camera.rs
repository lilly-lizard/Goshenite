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

use super::config_ui;

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
    near_plane: f64,
    far_plane: f64,
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
            fov: config_ui::FIELD_OF_VIEW,
            aspect_ratio: calc_aspect_ratio(resolution),
            near_plane: config_ui::CAMERA_NEAR_PLANE,
            far_plane: config_ui::CAMERA_FAR_PLANE,
        })
    }

    /// Changes the viewing direction based on the pixel amount the cursor has moved
    pub fn rotate(&mut self, delta_cursor_position: DVec2) {
        let delta_angle = self.delta_cursor_to_angle(delta_cursor_position.into());

        match &self.look_mode {
            // no lock-on target so maintain position adjust looking direction
            LookMode::Direction() => {
                self.rotate_fixed_pos(delta_angle[0], delta_angle[1]);
            }

            // lock on target stays the same but camera position rotates around it
            LookMode::Target(target_type) => {
                if let Some(target_pos) =
                    self.get_target_position_or_switch_look_modes(target_type.clone())
                {
                    self.arcball(self.normal, target_pos, delta_angle[0], delta_angle[1]);
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
            self.near_plane,
            self.far_plane,
        )
    }

    pub fn position(&self) -> DVec3 {
        self.position
    }

    pub fn look_mode(&self) -> LookMode {
        self.look_mode.clone()
    }

    pub fn near_plane(&self) -> f64 {
        self.near_plane
    }

    pub fn far_plane(&self) -> f64 {
        self.far_plane
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
                Angle::from_radians(delta * config_ui::LOOK_SENSITIVITY.radians())
            }
            LookMode::Target(_) => {
                Angle::from_radians(delta * config_ui::ARC_BALL_SENSITIVITY.radians())
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

    fn rotate_fixed_pos(&mut self, delta_h: Angle, delta_v: Angle) {
        let delta_v_clamped = self.clamp_vertical_angle_delta(delta_v.invert());

        let rotation_matrix = DMat3::from_axis_angle(self.normal, delta_v_clamped.radians())
            * DMat3::from_rotation_z(delta_h.radians());
        self.direction = rotation_matrix * self.direction;
    }

    fn arcball(&mut self, normal: DVec3, target_pos: DVec3, delta_h: Angle, delta_v: Angle) {
        let delta_v_clamped = self.clamp_vertical_angle_delta(delta_v);

        // lock on target stays the same but camera position rotates around it
        let rotation_matrix = DMat3::from_axis_angle(normal, delta_v_clamped.radians())
            * DMat3::from_rotation_z(-delta_h.radians());

        self.set_position(rotation_matrix * (self.position - target_pos) + target_pos);
        self.set_direction(target_pos);
    }

    /// Limits how close camera vertical direction can get to world space up.
    /// Also prevents camera angle from crossing over world space up and doing a disorienting flip.
    fn clamp_vertical_angle_delta(&self, delta_v: Angle) -> Angle {
        let current_v_radians = config::WORLD_SPACE_UP
            .as_dvec3()
            .angle_between(self.direction);
        let final_v_radians = current_v_radians + delta_v.radians();

        let min_radians = config_ui::VERTICAL_ANGLE_CLAMP.radians();
        if final_v_radians < min_radians {
            return Angle::from_radians(min_radians - current_v_radians);
        }

        let max_radians = std::f64::consts::PI - config_ui::VERTICAL_ANGLE_CLAMP.radians();
        if final_v_radians > max_radians {
            return Angle::from_radians(max_radians - current_v_radians);
        }

        delta_v
    }

    // `scroll_delta` is number of scroll clicks
    fn scroll_zoom_target(&mut self, scroll_delta: f64, target_pos: DVec3) {
        if scroll_delta == 0. {
            return;
        }

        // vector from camera position to target
        let target_vector = target_pos - self.position;
        // how far along that vector we want to travel
        let mut travel_factor = dual_asymptote(scroll_delta);

        // clamp travel distance
        let target_vector_length = target_vector.length();
        let max_travel_factor = 1. - config_ui::CAMERA_MIN_TARGET_DISTANCE / target_vector_length;
        let min_travel_factor = 1. - config_ui::CAMERA_MAX_TARGET_DISTANCE / target_vector_length;
        if travel_factor > max_travel_factor {
            travel_factor = max_travel_factor;
        } else if travel_factor < min_travel_factor {
            travel_factor = min_travel_factor;
        }

        let new_position = self.position + target_vector * travel_factor;

        self.set_position(new_position);
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
                Some(object.borrow().origin().as_dvec3())
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

/// (2^x - 1) / (2^x + 1)
///
/// Has asymptote at y = 1 when x = +∞ and another at y = -1 when x = -∞.
/// Gradient is 1 at x = 0. Inspired by tanh but with lighter gradient falloff.
fn dual_asymptote(x: f64) -> f64 {
    (2_f64.powf(x) - 1.) / (2_f64.powf(x) + 1.)
}
