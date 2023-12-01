use crate::helper::axis::CartesianAxis;
use log::LevelFilter;

pub const ENGINE_NAME: &str = "Goshenite";

/// Environment variables that can be used to configure the engine
#[allow(non_snake_case)]
pub mod ENV {
    /// Set to a float number to override the scale factor
    pub const SCALE_FACTOR: &str = "GOSH_SCALE_FACTOR";
}

/// Log level filter. Log messages with lower levels than this will not be displayed.
#[cfg(debug_assertions)]
pub const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Debug;
#[cfg(not(debug_assertions))]
pub const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Info;

/// Wherever the app window starts maximized
pub const START_MAXIMIZED: bool = false;
/// Default window size if `START_MAXIMIZED` is false
pub const DEFAULT_WINDOW_SIZE: [u32; 2] = [1000, 700];

/// Describes which direction is up in the world space coordinate system, set to Z by default
pub const WORLD_SPACE_UP: CartesianAxis = CartesianAxis::Z;

pub const MAX_SPHERE_RADIUS: u32 = 100;
