use crate::helper::axis::CartesianAxis;
use log::LevelFilter;

pub const ENGINE_NAME: &str = "Goshenite";

pub const ENGINE_VERSION_MAJOR: u8 = 0;
pub const ENGINE_VERSION_MINOR: u8 = 2;
pub const ENGINE_VERSION_PATCH: u8 = 2;

pub const MAGIC_BYTE: u8 = 0b_1001;
pub const PRECURSOR_BYTE_COUNT: usize = 4;
/// Used at the beginning of binary files written by the engine
pub const PRECURSOR_BYTES: [u8; PRECURSOR_BYTE_COUNT] = [
    MAGIC_BYTE,
    ENGINE_VERSION_MAJOR,
    ENGINE_VERSION_MINOR,
    ENGINE_VERSION_PATCH,
];

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

/// Default window size if `START_MAXIMIZED` is false
pub const DEFAULT_WINDOW_SIZE: [u32; 2] = [1000, 700];

/// Describes which direction is up in the world space coordinate system, set to Z by default
pub const WORLD_SPACE_UP: CartesianAxis = CartesianAxis::Z;

pub const MAX_SPHERE_RADIUS: u32 = 100;
