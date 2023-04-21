use super::config_ui;
use crate::{
    config,
    engine::{object::object::Object, primitives::primitive::Primitive},
    helper::angle::Angle,
};
use glam::{DMat3, DMat4, DVec2, DVec3, Mat4, Vec4};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Clone)]
pub enum LookMode {
    /// Look in a given direction
    Direction(DVec3),
    /// Lock on to a target position
    Target(DVec3),
}

/// Describes the orientation and properties of a camera that can be used for perspective rendering
#[derive(Clone)]
pub struct Camera {
    position: DVec3,
    look_mode: LookMode,
    fov: Angle,
    aspect_ratio: f32,
    near_plane: f64,
    far_plane: f64,
}

// Public functions

impl Camera {
    pub fn new(resolution: [f32; 2]) -> anyhow::Result<Self> {
        let position = config_ui::CAMERA_DEFAULT_POSITION;
        let target_pos = config_ui::CAMERA_DEFAULT_TARGET;
        let direction = target_pos - position;

        Ok(Camera {
            position,
            look_mode: LookMode::Direction(direction),
            fov: config_ui::CAMERA_DEFAULT_FOV,
            aspect_ratio: calc_aspect_ratio(resolution),
            near_plane: config_ui::CAMERA_NEAR_PLANE,
            far_plane: config_ui::CAMERA_FAR_PLANE,
        })
    }

    /// Changes the viewing direction based on the pixel amount the cursor has moved
    pub fn rotate(&mut self, delta_cursor_position: DVec2) {
        let delta_angle = self.delta_cursor_to_angle(delta_cursor_position.into());

        match self.look_mode {
            // no lock-on target so maintain position adjust looking direction
            LookMode::Direction(direction) => {
                self.rotate_fixed_pos(delta_angle[0], delta_angle[1]);
            }

            // lock on target stays the same but camera position rotates around it
            LookMode::Target(target_pos) => {
                // orientation shouldn't be vertical
                let normal = match self.normal_with_vertical_check() {
                    Ok(normal) => normal,
                    Err(CameraError::VerticalCameraDirection) => {
                        self.recover_from_vertical_direction_alignment();
                        self.normal()
                    }
                };

                self.arcball(normal, target_pos, delta_angle[0], delta_angle[1]);
            }
        }

        // update normal now that camera orientation has changed
        self.update_normal();
    }

    /// Move camera position forwards/backwards according to cursor scroll value
    pub fn scroll_zoom(&mut self, scroll_delta: f64) {
        match self.look_mode {
            LookMode::Direction(direction) => {
                self.set_position(self.position + scroll_delta * direction);
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

    /// Resets the following properties to their defaults:
    /// - position
    /// - direction (and normal)
    /// - look_mode
    /// - fov
    /// - near/far plane limits
    pub fn reset(&mut self) {
        self.position = config_ui::CAMERA_DEFAULT_POSITION;
        let target_pos = config_ui::CAMERA_DEFAULT_TARGET;
        let direction = target_pos - self.position;

        self.look_mode = LookMode::Direction(direction);
        self.fov = config_ui::CAMERA_DEFAULT_FOV;
        self.near_plane = config_ui::CAMERA_NEAR_PLANE;
        self.far_plane = config_ui::CAMERA_FAR_PLANE;
    }

    // Setters

    pub fn set_aspect_ratio(&mut self, resolution: [f32; 2]) {
        self.aspect_ratio = calc_aspect_ratio(resolution);
    }

    /// Changes the look mode to direction.
    pub fn set_direction(&mut self, direction: DVec3) {
        self.look_mode = LookMode::Direction(direction);

        // avoid vertical alignment
        self.check_for_and_recover_from_vertical_orientation_alignment();
    }

    pub fn set_lock_on_target(&mut self, target_pos: DVec3) {
        self.look_mode = LookMode::Target(target_pos);

        // avoid vertical alignment
        self.check_for_and_recover_from_vertical_orientation_alignment();
    }

    /// Sets the lock on target to the object origin.
    pub fn set_lock_on_target_from_object(&mut self, object: &Object) {
        let target_pos = object.origin().as_dvec3();
        self.set_lock_on_target(target_pos);
    }

    /// Sets the lock on target to the primitive center.
    pub fn set_lock_on_target_from_primitive(&mut self, primitive: &dyn Primitive) {
        let target_pos = primitive.transform().center.as_dvec3();
        self.set_lock_on_target(target_pos);
    }

    pub fn unset_lock_on_target(&mut self) {
        if let LookMode::Target(target_pos) = self.look_mode {
            let direction = self.position - target_pos;
            self.look_mode = LookMode::Direction(direction);
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        let target_pos = self.target_pos();

        Mat4::look_at_rh(
            self.position.as_vec3(),
            target_pos.as_vec3(),
            config::WORLD_SPACE_UP.as_vec3(),
        )
    }

    pub fn proj_matrix_glam(&self) -> DMat4 {
        // don't need to invert y in shaders
        // inverse(proj_view_inverse) works fine
        // view mat too?
        DMat4::perspective_rh(
            self.fov.radians(),
            self.aspect_ratio as f64,
            self.near_plane,
            self.far_plane,
        )
    }

    // https://vincent-p.github.io/posts/vulkan_perspective_matrix/#deriving-the-depth-projection
    /// right handed, reverse z, vulkan coordinates
    pub fn projection_matrices(&self) -> ProjectionMatrixReturn {
        let near = self.near_plane as f32;
        let far = self.far_plane as f32;

        let fov_vertical = self.fov.radians() as f32;
        let focal_length = 1. / (fov_vertical * 0.5).tan();

        let w = focal_length / self.aspect_ratio;
        let h = -focal_length;

        let a = near / (far - near);
        let b = far * a;

        let proj = Mat4::from_cols(
            Vec4::new(w, 0., 0., 0.),
            Vec4::new(0., h, 0., 0.),
            Vec4::new(0., 0., a, -1.),
            Vec4::new(0., 0., b, 0.),
        );

        let proj_inverse = Mat4::from_cols(
            Vec4::new(1. / w, 0., 0., 0.),
            Vec4::new(0., 1. / h, 0., 0.),
            Vec4::new(0., 0., 0., 1. / b),
            Vec4::new(0., 0., -1., a / b),
        );

        ProjectionMatrixReturn {
            proj,
            proj_inverse,
            proj_a: a,
            proj_b: b,
        }
    }

    // Getters

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
    /// Not necessarily normalized
    fn direction(&self) -> DVec3 {
        match self.look_mode {
            LookMode::Direction(direction) => direction,
            LookMode::Target(target_pos) => self.position - target_pos,
        }
    }

    /// Not necessarily normalized
    fn target_pos(&self) -> DVec3 {
        match self.look_mode {
            LookMode::Direction(direction) => self.position + direction,
            LookMode::Target(target_pos) => target_pos,
        }
    }

    /// Not normalized. May return 0 if the look orientation is aligned with the verical axis!
    fn normal(&self) -> DVec3 {
        let direction = self.direction();
        let up = config::WORLD_SPACE_UP.as_dvec3();

        up.cross(direction)
    }

    fn normal_with_vertical_check(&self) -> Result<DVec3, CameraError> {
        let normal = self.normal();

        if normal == DVec3::ZERO {
            return Err(CameraError::VerticalCameraDirection);
        }

        Ok(normal)
    }

    /// If required, adjust the camera so that it isn't looking vertically. Allows a normal to be
    /// calculated.
    fn check_for_and_recover_from_vertical_orientation_alignment(&mut self) {
        if let Err(CameraError::VerticalCameraDirection) = self.normal_with_vertical_check() {
            self.recover_from_vertical_orientation_alignment();
        }
    }

    /// Adjust the camera so that it isn't looking vertically. Allows a normal to be calculated.
    fn recover_from_vertical_orientation_alignment(&mut self) {
        match self.look_mode {
            LookMode::Direction(direction) => {
                todo!();
            }
            LookMode::Target(target_pos) => {
                todo!();
            }
        }
    }

    fn delta_cursor_to_angle(&self, delta_cursor_position: [f64; 2]) -> [Angle; 2] {
        delta_cursor_position.map(|delta| match self.look_mode {
            LookMode::Direction(_) => {
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

    fn rotate_fixed_pos(&mut self, delta_h: Angle, delta_v: Angle) {
        let delta_v_clamped = self.clamp_vertical_angle_delta(delta_v.invert());

        let rotation_matrix = DMat3::from_axis_angle(self.normal, delta_v_clamped.radians())
            * DMat3::from_rotation_z(delta_h.radians());
        self.direction = rotation_matrix * self.direction;
    }

    fn arcball(&mut self, normal: DVec3, target_pos: DVec3, delta_h: Angle, delta_v: Angle) {
        let delta_v_clamped = self.clamp_vertical_angle_delta(delta_v);

        // lock on target stays the same but camera position rotates around it
        let normal = normal.normalize();
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
}

#[inline]
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

#[derive(Clone)]
pub struct ProjectionMatrixReturn {
    pub proj: Mat4,
    pub proj_inverse: Mat4,
    pub proj_a: f32,
    pub proj_b: f32,
}

// Errors

#[derive(Clone, Debug)]
pub enum CameraError {
    /// Camera direction lines up with `WORLD_SPACE_UP` meaning that a normal vector cannot be calculated
    VerticalCameraDirection,
}
impl std::fmt::Display for CameraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VerticalCameraDirection => {
                write!(
                    f,
                    "camera direction is vertical meaning a normal vector cannot be calculated"
                )
            }
        }
    }
}
impl std::error::Error for CameraError {}
