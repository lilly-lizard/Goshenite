use crate::helper::angle::Angle;
use glam::DVec3;
use log::LevelFilter;

pub const ENGINE_NAME: &str = "Goshenite";

/// Environment variables that can be used to configure the engine
#[allow(non_snake_case)]
pub mod ENV {
    /// Set to a float number to override the scale factor
    pub const SCALE_FACTOR: &str = "GOSH_SCALE_FACTOR";
}

/// Log level filter. Logs of levels lower than this will not be displayed.
pub const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Debug;

/// Wherever the app window starts maximized
pub const START_MAXIMIZED: bool = false;
/// Default window size if `START_MAXIMIZED` is false
pub const DEFAULT_WINDOW_SIZE: [u32; 2] = [1000, 700];

/// Describes which direction is up in the world space coordinate system, set to Z by default
pub const WORLD_SPACE_UP: WorldSpaceUp = WorldSpaceUp::Z;
/// Describes which direction is up in the world space coordinate system.
/// This engine uses right hand coordinates, so when set to Z, X will be forward and Y will be left.
#[derive(Clone, Copy)]
pub enum WorldSpaceUp {
    Y,
    Z,
}
impl From<WorldSpaceUp> for DVec3 {
    #[inline]
    fn from(w: WorldSpaceUp) -> DVec3 {
        match w {
            WorldSpaceUp::Y => DVec3::Y,
            WorldSpaceUp::Z => DVec3::Z,
        }
    }
}
impl WorldSpaceUp {
    #[inline]
    pub fn as_dvec3(self) -> DVec3 {
        self.into()
    }
}

/// Field of view
pub const FIELD_OF_VIEW: Angle = Angle::from_radians(std::f64::consts::FRAC_PI_4);
pub const CAMERA_NEAR_PLANE: f64 = 0.1;
pub const CAMERA_FAR_PLANE: f64 = 100.;
/// Should be ~= `CAMERA_FAR_PLANE`. Pevents view matrix from getting too crazy (too big triggers a glam_assert when calculating inverse(proj * view))
pub const CAMERA_MAX_TARGET_DISTANCE: f64 = 100_000.;
/// Minumum distance between the camera position and the camera target. Ensures valid results for view matrix etc
pub const CAMERA_MIN_TARGET_DISTANCE: f64 = 0.001;

/// Sensitivity rotating the camera in [`ViewMode::Direction`](crate::camera::ViewMode::Direction) = angle / pixels
pub const LOOK_SENSITIVITY: Angle = Angle::from_radians(0.001);
/// Sensitivity rotating the camer in [`ViewMode::Target`](crate::camera::ViewMode::Target) = angle / pixels
pub const ARC_BALL_SENSITIVITY: Angle = Angle::from_radians(0.005);
/// Scrolling sensitivity
pub const SCROLL_SENSITIVITY: f64 = 0.5;

pub const MAX_SPHERE_RADIUS: u32 = 100;
