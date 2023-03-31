use crate::helper::angle::Angle;

pub const SELECT_PRIMITIVE_OP_AFTER_ADD: bool = true;

/// Enables option to show egui debug overlay
pub const EGUI_TRACE: bool = false;

/// Limits how close camera vertical direction can get to world space up
pub const VERTICAL_ANGLE_CLAMP: Angle = Angle::Degrees(1.);
