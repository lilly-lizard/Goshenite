/// Shout out to cgmath for the idea https://github.com/rustgd/cgmath
use std::f64::consts::TAU;

/// Represents a f64 angle in radians or degrees
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub enum Angle {
    Radians(f64),
    Degrees(f64),
}

impl Angle {
    #[inline]
    pub const fn from_radians(radians: f64) -> Self {
        Self::Radians(radians)
    }
    #[inline]
    pub const fn from_degrees(degrees: f64) -> Self {
        Self::Degrees(degrees)
    }

    #[inline]
    pub fn radians(&self) -> f64 {
        match *self {
            Self::Radians(r) => r,
            Self::Degrees(d) => degrees_to_radians(d),
        }
    }
    #[inline]
    pub fn degrees(&self) -> f64 {
        match *self {
            Self::Radians(r) => radians_to_degrees(r),
            Self::Degrees(d) => d,
        }
    }
}

impl Default for Angle {
    fn default() -> Self {
        Self::Radians(Default::default())
    }
}

#[inline]
fn degrees_to_radians(degrees: f64) -> f64 {
    degrees * TAU / 360.
}

#[inline]
fn radians_to_degrees(radians: f64) -> f64 {
    radians * 360. / TAU
}
