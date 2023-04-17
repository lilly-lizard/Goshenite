use glam::DVec3;

use crate::helper::angle::Angle;

/// If set to true, after clicking "Add primitive op" the newly created primitive op will be selected
pub const SELECT_PRIMITIVE_OP_AFTER_ADD: bool = false;

/// Enables option to show egui debug overlay
pub const EGUI_TRACE: bool = false;

/// Limits how close camera vertical direction can get to world space up
pub const VERTICAL_ANGLE_CLAMP: Angle = Angle::Degrees(1.);

pub const CAMERA_DEFAULT_FOV: Angle = Angle::from_radians(std::f64::consts::FRAC_PI_4);
pub const CAMERA_DEFAULT_POSITION: DVec3 = DVec3::splat(3.);
pub const CAMERA_DEFAULT_TARGET: DVec3 = DVec3::ZERO;
pub const CAMERA_NEAR_PLANE: f64 = 0.01;
pub const CAMERA_FAR_PLANE: f64 = 1000.;
/// Should be ~= `CAMERA_FAR_PLANE`. Too big causes broken view-proj matrix _note: glam assert catches this in debug_.
pub const CAMERA_MAX_TARGET_DISTANCE: f64 = 10_000.;
/// Minumum distance between the camera position and the camera target. Ensures valid results for view matrix etc
pub const CAMERA_MIN_TARGET_DISTANCE: f64 = 0.001;

/// Sensitivity rotating the camera in [`ViewMode::Direction`](crate::camera::ViewMode::Direction) = angle / pixels
pub const LOOK_SENSITIVITY: Angle = Angle::from_radians(0.001);
/// Sensitivity rotating the camer in [`ViewMode::Target`](crate::camera::ViewMode::Target) = angle / pixels
pub const ARC_BALL_SENSITIVITY: Angle = Angle::from_radians(0.005);
/// Scrolling sensitivity
pub const SCROLL_SENSITIVITY: f64 = 0.5;
