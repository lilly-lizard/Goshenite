use crate::helper::angle::Angle;

pub const SELECT_PRIMITIVE_OP_AFTER_ADD: bool = true;

/// Enables option to show egui debug overlay
pub const EGUI_TRACE: bool = false;

/// Limits how close camera vertical direction can get to world space up
pub const VERTICAL_ANGLE_CLAMP: Angle = Angle::Degrees(1.);

/// Field of view
pub const FIELD_OF_VIEW: Angle = Angle::from_radians(std::f64::consts::FRAC_PI_4);
pub const CAMERA_NEAR_PLANE: f64 = 0.01;
pub const CAMERA_FAR_PLANE: f64 = 100.;
/// Should be ~= `CAMERA_FAR_PLANE`. Pevents view matrix from getting too crazy (too big triggers a glam_assert when calculating inverse(proj * view))
pub const CAMERA_MAX_TARGET_DISTANCE: f64 = 10_000.;
/// Minumum distance between the camera position and the camera target. Ensures valid results for view matrix etc
pub const CAMERA_MIN_TARGET_DISTANCE: f64 = 0.001;

/// Sensitivity rotating the camera in [`ViewMode::Direction`](crate::camera::ViewMode::Direction) = angle / pixels
pub const LOOK_SENSITIVITY: Angle = Angle::from_radians(0.001);
/// Sensitivity rotating the camer in [`ViewMode::Target`](crate::camera::ViewMode::Target) = angle / pixels
pub const ARC_BALL_SENSITIVITY: Angle = Angle::from_radians(0.005);
/// Scrolling sensitivity
pub const SCROLL_SENSITIVITY: f64 = 0.5;
