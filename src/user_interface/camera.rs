use super::{
    config_ui::{self, MOUSE_ZOOM_FACTOR, PAN_FACTOR},
    controls_camera::CameraControlMappings,
    keyboard_modifiers::KeyboardModifierStates,
};
use crate::{
    config,
    engine::{
        engine_controller::EngineController,
        object::{
            object::ObjectId, object_collection::ObjectCollection, primitive_op::PrimitiveOpIndex,
        },
        settings::{setting_ui_enum, SettingDataType},
    },
    helper::angle::Angle,
    user_interface::{
        config_ui::{CAMERA_DEFAULT_ARCBALL_TARGET_DEPTH, DEFAULT_SCROLL_ZOOM_SENSITIVITY},
        controls_camera::CameraAction,
        mouse_button::MouseButton,
    },
};
use glam::{DMat3, DVec2, DVec3, Mat4, Vec3, Vec4};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum LookMode {
    /// Arcball around an invisible point in the center of the screen, `depth` far in front of the camera
    ArcballHovering,
    /// Look in a given direction and rotate from a fixed camera position
    PoV,
    SelectedObject,
    SelectedPrimitiveOp,
}

impl LookMode {
    const VARIANTS: [LookMode; 4] = [
        LookMode::ArcballHovering,
        LookMode::PoV,
        LookMode::SelectedObject,
        LookMode::SelectedPrimitiveOp,
    ];
}
impl Default for LookMode {
    fn default() -> Self {
        Self::ArcballHovering
    }
}
impl SettingDataType for LookMode {
    fn setting_name(&self) -> &str {
        "Camera Look Mode"
    }

    fn value_display_name(&self) -> &str {
        match self {
            Self::ArcballHovering => "Arcball Hovering",
            Self::PoV => "POV",
            Self::SelectedObject => "Selected Object",
            Self::SelectedPrimitiveOp => "Selected Primitive Op",
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, updated: &mut bool) {
        setting_ui_enum(ui, self, &Self::VARIANTS, updated);
    }

    fn process_update(&self, engine: &mut EngineController) {
        engine.camera.set_look_mode(*self);
    }
}

/// Describes the orientation and properties of a camera that can be used for perspective rendering
///
/// Note: there is currently a restriction that the camera direction cannot be exactly vertical in order for normals to be calculated relative to the vertical axis.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct Camera {
    position: DVec3,
    direction: DVec3,
    look_mode: LookMode,
    fov: Angle,
    aspect_ratio: f32,
    near_plane: f64,
    far_plane: f64,
    /// Note: only used for `LookMode::ArcballHovering`
    arcball_target_depth: f64,
    /// Note: only used for `LookMode::SelectedObject` and `LookMode::SelectedPrimitiveOp`
    last_known_origin: Vec3,
}

impl Default for Camera {
    fn default() -> Self {
        let position = config_ui::CAMERA_DEFAULT_POSITION;
        let target_pos = config_ui::CAMERA_DEFAULT_TARGET;
        Self {
            position,
            direction: target_pos - position,
            look_mode: LookMode::default(),
            fov: config_ui::CAMERA_DEFAULT_FOV,
            aspect_ratio: 1_f32,
            near_plane: config_ui::CAMERA_NEAR_PLANE,
            far_plane: config_ui::CAMERA_FAR_PLANE,
            arcball_target_depth: CAMERA_DEFAULT_ARCBALL_TARGET_DEPTH,
            last_known_origin: Default::default(),
        }
    }
}

// Public functions

impl Camera {
    pub fn new(resolution: [f32; 2]) -> anyhow::Result<Self> {
        Ok(Camera {
            aspect_ratio: calc_aspect_ratio(resolution),
            ..Default::default()
        })
    }

    pub fn update_camera_objects(
        &mut self,
        object_collection: &ObjectCollection,
        selected_object_id: Option<ObjectId>,
        selected_primitive_op_index: Option<PrimitiveOpIndex>,
    ) {
        match self.look_mode() {
            LookMode::SelectedObject => {
                let Some(selected_object_id) = selected_object_id else {
                    self.unset_lock_on_target();
                    return;
                };
                if let Ok(object) = object_collection.get_object(selected_object_id) {
                    self.last_known_origin = object.center;
                    // avoid vertical alignment
                    self.check_for_and_recover_from_vertical_orientation_alignment();
                } else {
                    // object dropped
                    self.unset_lock_on_target();
                }
            }
            LookMode::SelectedPrimitiveOp => {
                let Some(selected_object_id) = selected_object_id else {
                    self.unset_lock_on_target();
                    return;
                };
                let Some(selected_primitive_op_index) = selected_primitive_op_index else {
                    self.unset_lock_on_target();
                    return;
                };
                if let Ok((object, primitive_op)) = object_collection
                    .get_object_and_primitive_op(selected_object_id, selected_primitive_op_index)
                {
                    self.last_known_origin = object.center + primitive_op.center();
                    // avoid vertical alignment
                    self.check_for_and_recover_from_vertical_orientation_alignment();
                } else {
                    // object dropped or primitive op deleted
                    self.unset_lock_on_target();
                }
            }
            _ => (),
        }
    }

    pub fn update_cursor_dragging(
        &mut self,
        drag_delta: DVec2,
        drag_button: MouseButton,
        keyboard_modifier_states: KeyboardModifierStates,
        camera_control_mappings: CameraControlMappings,
    ) {
        if camera_control_mappings.mapping_active(
            CameraAction::Look,
            drag_button,
            keyboard_modifier_states,
        ) {
            self.rotate_from_cursor_delta(drag_delta);
        }
        if camera_control_mappings.mapping_active(
            CameraAction::Pan,
            drag_button,
            keyboard_modifier_states,
        ) {
            self.pan_from_cursor_delta(drag_delta);
        }
        if camera_control_mappings.mapping_active(
            CameraAction::Zoom,
            drag_button,
            keyboard_modifier_states,
        ) {
            self.zoom_from_cursor_delta(drag_delta);
        }
    }

    pub fn update_scroll(&mut self, scroll_delta: DVec2) {
        self.zoom_from_scroll(scroll_delta.y, DEFAULT_SCROLL_ZOOM_SENSITIVITY);
    }

    /// Resets the following properties to their defaults:
    /// - position
    /// - direction (and normal)
    /// - look_mode
    /// - fov
    /// - near/far plane limits
    pub fn reset(&mut self) {
        self.position = config_ui::CAMERA_DEFAULT_POSITION;
        self.look_mode = LookMode::default();
        self.fov = config_ui::CAMERA_DEFAULT_FOV;
        self.near_plane = config_ui::CAMERA_NEAR_PLANE;
        self.far_plane = config_ui::CAMERA_FAR_PLANE;
    }

    // Setters

    pub fn set_aspect_ratio(&mut self, resolution: [f32; 2]) {
        self.aspect_ratio = calc_aspect_ratio(resolution);
    }

    pub fn set_look_mode(&mut self, look_mode: LookMode) {
        self.look_mode = look_mode;
        // avoid vertical alignment
        self.check_for_and_recover_from_vertical_orientation_alignment();
    }

    pub fn set_arcball_target_depth(&mut self, new_depth: f64) {
        self.arcball_target_depth = new_depth;
    }

    pub fn deselect_object(&mut self) {
        match self.look_mode {
            LookMode::SelectedObject => self.unset_lock_on_target(),
            LookMode::SelectedPrimitiveOp => self.unset_lock_on_target(),
            _ => (),
        }
    }

    pub fn deselect_primitive_op(&mut self) {
        if let LookMode::SelectedPrimitiveOp = self.look_mode {
            self.unset_lock_on_target();
        }
    }

    pub fn unset_lock_on_target(&mut self) {
        self.look_mode = LookMode::PoV;
    }

    // Getters

    pub fn view_matrix(&self) -> Mat4 {
        let target_pos = self.target_pos();

        Mat4::look_at_rh(
            self.position.as_vec3(),
            target_pos.as_vec3(),
            config::WORLD_SPACE_UP.as_vec3(),
        )
    }

    // https://vincent-p.github.io/posts/vulkan_perspective_matrix/#deriving-the-depth-projection
    /// right handed, reverse z, vulkan coordinates
    #[rustfmt::skip]
    pub fn projection_matrix(&self) -> Mat4 {
        let (w, h, a, b) = self.projection_matrix_components();
        Mat4::from_cols(
            Vec4::new(w , 0., 0., 0.),
            Vec4::new(0., h , 0., 0.),
            Vec4::new(0., 0., a ,-1.),
            Vec4::new(0., 0., b , 0.),
        )
    }

    // https://vincent-p.github.io/posts/vulkan_perspective_matrix/#deriving-the-depth-projection
    /// right handed, reverse z, vulkan coordinates
    #[rustfmt::skip]
    pub fn projection_matrix_inverse(&self) -> Mat4 {
        let (w, h, a, b) = self.projection_matrix_components();
        Mat4::from_cols(
            Vec4::new(1./w,  0., 0.,  0.),
            Vec4::new(  0.,1./h, 0.,  0.),
            Vec4::new(  0.,  0., 0.,1./b),
            Vec4::new(  0.,  0.,-1., a/b),
        )
    }

    // https://vincent-p.github.io/posts/vulkan_perspective_matrix/#deriving-the-depth-projection
    // note that glam::DMat4::perspective_rh renders everything upside down
    /// Right handed, reverse z, vulkan coordinates.
    /// Returns `(w, h, a, b)`
    fn projection_matrix_components(&self) -> (f32, f32, f32, f32) {
        let near = self.near_plane as f32;
        let far = self.far_plane as f32;

        let fov_vertical = self.fov.radians() as f32;
        let focal_length = 1. / (fov_vertical * 0.5).tan();

        let w = focal_length / self.aspect_ratio;
        let h = -focal_length;

        let a = near / (far - near);
        let b = far * a;
        (w, h, a, b)
    }

    #[inline]
    pub fn position(&self) -> DVec3 {
        self.position
    }
    #[inline]
    pub fn arcball_target_depth(&self) -> f64 {
        self.arcball_target_depth
    }
    #[inline]
    pub fn direction(&self) -> DVec3 {
        self.direction
    }
    #[inline]
    pub fn look_mode(&self) -> LookMode {
        self.look_mode
    }
    #[inline]
    pub fn near_plane(&self) -> f64 {
        self.near_plane
    }
    #[inline]
    pub fn far_plane(&self) -> f64 {
        self.far_plane
    }
}

// Private functions

impl Camera {
    /// Changes the viewing direction based on the pixel amount the cursor has moved
    fn rotate_from_cursor_delta(&mut self, delta_cursor_position: DVec2) {
        let delta_angle = self.delta_cursor_to_angle(delta_cursor_position.into());

        // orientation shouldn't be vertical
        let normal = match self.normal_with_vertical_check() {
            Ok(normal) => normal,
            Err(CameraError::VerticalCameraDirection) => {
                self.recover_from_vertical_orientation_alignment();
                self.normal()
            }
        };

        self.rotate_from_angle_delta(normal, delta_angle);
    }

    fn pan_from_cursor_delta(&mut self, delta_cursor_position: DVec2) {
        let view_horizontal = self.normal().normalize();
        let view_vertical = self.direction.cross(view_horizontal).normalize();
        let delta_pan = delta_cursor_position * PAN_FACTOR;
        let delta_position = delta_pan.x * view_horizontal + delta_pan.y * view_vertical;
        self.position += delta_position;
    }

    fn zoom_from_scroll(&mut self, scroll_delta: f64, scroll_zoom_sensitivity: f64) {
        self.zoom(scroll_delta * scroll_zoom_sensitivity)
    }

    fn zoom_from_cursor_delta(&mut self, delta_cursor_position: DVec2) {
        self.zoom(-delta_cursor_position.y * MOUSE_ZOOM_FACTOR)
    }

    fn zoom(&mut self, zoom_delta: f64) {
        match self.look_mode {
            LookMode::ArcballHovering => {
                let new_position = self.position + zoom_delta * self.direction;
                self.set_position(new_position);
            }
            LookMode::PoV => {
                let new_position = self.position + zoom_delta * self.direction;
                self.set_position(new_position);
            }
            LookMode::SelectedObject => {
                self.scroll_zoom_target(zoom_delta, self.last_known_origin.as_dvec3());
            }
            LookMode::SelectedPrimitiveOp => {
                self.scroll_zoom_target(zoom_delta, self.last_known_origin.as_dvec3());
            }
        }
    }

    fn target_pos(&self) -> DVec3 {
        match self.look_mode {
            LookMode::ArcballHovering => self.position + self.direction * self.arcball_target_depth,
            LookMode::PoV => self.position + self.direction,
            LookMode::SelectedObject => self.last_known_origin.as_dvec3(),
            LookMode::SelectedPrimitiveOp => self.last_known_origin.as_dvec3(),
        }
    }

    /// Not normalized. May return 0 if the look orientation is aligned with the verical axis!
    fn normal(&self) -> DVec3 {
        let up = config::WORLD_SPACE_UP.as_dvec3();
        up.cross(self.direction)
    }

    /// Same as [`Self::normal`] but will return [`CameraError::VerticalCameraDirection`] if the
    /// look direction is aligned with the vertical axis.
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
        let recovery_delta_v =
            clamp_vertical_angle_delta(config::WORLD_SPACE_UP.as_dvec3(), Angle::ZERO);
        let normal = DVec3::X;

        self.rotate_from_angle_delta(normal, [Angle::ZERO, recovery_delta_v]);
    }

    fn rotate_from_angle_delta(&mut self, normal: DVec3, delta_angle: [Angle; 2]) {
        match self.look_mode {
            LookMode::ArcballHovering => {
                let target_pos = self.position + self.direction * self.arcball_target_depth;
                let new_position = arcball(
                    self.position,
                    target_pos,
                    normal,
                    delta_angle[0],
                    delta_angle[1],
                );
                self.set_position(new_position);
                self.direction = target_pos - self.position;
            }

            // no lock-on target so maintain position adjust looking direction
            LookMode::PoV => {
                self.direction =
                    rotate_fixed_pos(self.direction, normal, delta_angle[0], delta_angle[1]);
            }

            // lock on target stays the same but camera position rotates around it
            LookMode::SelectedObject => {
                let new_position = arcball(
                    self.position,
                    self.last_known_origin.as_dvec3(),
                    normal,
                    delta_angle[0],
                    delta_angle[1],
                );
                self.set_position(new_position);
                self.direction = self.last_known_origin.as_dvec3() - self.position;
            }
            LookMode::SelectedPrimitiveOp => {
                let new_position = arcball(
                    self.position,
                    self.last_known_origin.as_dvec3(),
                    normal,
                    delta_angle[0],
                    delta_angle[1],
                );
                self.set_position(new_position);
                self.direction = self.last_known_origin.as_dvec3() - self.position;
            }
        }
        self.direction = self.direction.normalize();
    }

    fn delta_cursor_to_angle(&self, delta_cursor_position: [f64; 2]) -> [Angle; 2] {
        delta_cursor_position.map(|delta| match self.look_mode {
            LookMode::ArcballHovering => {
                Angle::from_radians(delta * config_ui::ARC_BALL_FACTOR.radians())
            }

            LookMode::PoV => Angle::from_radians(delta * config_ui::LOOK_FACTOR.radians()),
            LookMode::SelectedObject => {
                Angle::from_radians(delta * config_ui::ARC_BALL_FACTOR.radians())
            }
            LookMode::SelectedPrimitiveOp => {
                Angle::from_radians(delta * config_ui::ARC_BALL_FACTOR.radians())
            }
        })
    }

    /// Sets the camera position if `new_pos` doesn't contain NaN or +-inf
    fn set_position(&mut self, new_pos: DVec3) {
        if new_pos.is_finite() {
            self.position = new_pos;
        }
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

/// Returns the new direction after camera rotating around a fixed position.
fn rotate_fixed_pos(
    current_look_direction: DVec3,
    normal: DVec3,
    delta_h: Angle,
    delta_v: Angle,
) -> DVec3 {
    let delta_v_clamped = clamp_vertical_angle_delta(current_look_direction, delta_v.invert());
    let normalized_normal = normal.normalize();

    let rotation_matrix = DMat3::from_axis_angle(normalized_normal, delta_v_clamped.radians())
        * DMat3::from_rotation_z(delta_h.radians());

    let new_direciton = rotation_matrix * current_look_direction;
    new_direciton
}

/// Returns the new position after camera rotation around a target position.
fn arcball(
    camera_pos: DVec3,
    target_pos: DVec3,
    normal: DVec3,
    delta_h: Angle,
    delta_v: Angle,
) -> DVec3 {
    let look_direction = target_pos - camera_pos;
    let delta_v_clamped = clamp_vertical_angle_delta(look_direction, delta_v);
    let delta_h_inverted = -delta_h.radians();

    // lock on target stays the same but camera position rotates around it
    let normal = normal.normalize();
    let rotation_matrix = DMat3::from_axis_angle(normal, delta_v_clamped.radians())
        * DMat3::from_rotation_z(delta_h_inverted);

    let new_position = rotation_matrix * (camera_pos - target_pos) + target_pos;
    new_position
}

/// Adjusts a requested vertical angle delta so that the camera look direction is within
/// [`config_ui::VERTICAL_ANGLE_CLAMP`] away from the vertical axis after the returned vertical
/// angle delta is applied.
/// Will prevent the look direction from crossing over world space up and doing a disorienting flip.
fn clamp_vertical_angle_delta(look_direction: DVec3, delta_v: Angle) -> Angle {
    let current_v_radians = config::WORLD_SPACE_UP
        .as_dvec3()
        .angle_between(look_direction);
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

// Errors

#[derive(Debug)]
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
