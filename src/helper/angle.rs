/// Shout out to cgmath for the idea https://github.com/rustgd/cgmath
use std::{cmp::Ordering, f64::consts::TAU};

/// Represents a f64 angle in radians or degrees
///
/// _Note:using an enum allows us to define const values in radians or degrees!_
#[derive(Copy, Clone, Debug)]
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

    pub fn radians(&self) -> f64 {
        match *self {
            Self::Radians(r) => r,
            Self::Degrees(d) => degrees_to_radians(d),
        }
    }

    pub fn degrees(&self) -> f64 {
        match *self {
            Self::Radians(r) => radians_to_degrees(r),
            Self::Degrees(d) => d,
        }
    }

    pub fn to_radians(&self) -> Self {
        Self::Radians(self.radians())
    }

    pub fn to_degrees(&self) -> Self {
        Self::Degrees(self.degrees())
    }

    pub fn invert(&self) -> Self {
        match self {
            Self::Radians(r) => Self::Radians(-r),
            Self::Degrees(d) => Self::Degrees(-d),
        }
    }

    pub const ZERO: Self = Self::Radians(0.);
}

impl PartialEq for Angle {
    fn eq(&self, other: &Self) -> bool {
        let (float_1, float_2) = comparable_floats(*self, *other);
        float_1 == float_2
    }
}

impl PartialOrd for Angle {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let (float_1, float_2) = comparable_floats(*self, *other);
        if float_1 < float_2 {
            Some(Ordering::Less)
        } else if float_1 > float_2 {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl Default for Angle {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Returns two floats of the same type (could be radians or degrees) corresponding to the two
/// provided angles.
fn comparable_floats(value_1: Angle, value_2: Angle) -> (f64, f64) {
    match value_1 {
        Angle::Radians(r_1) => match value_2 {
            Angle::Radians(r_2) => (r_1, r_2),
            Angle::Degrees(d_2) => (radians_to_degrees(r_1), d_2),
        },
        Angle::Degrees(d_1) => match value_2 {
            Angle::Radians(r_2) => (d_1, radians_to_degrees(r_2)),
            Angle::Degrees(d_2) => (d_1, d_2),
        },
    }
}

#[inline]
pub fn degrees_to_radians(degrees: f64) -> f64 {
    degrees * TAU / 360.
}

#[inline]
pub fn radians_to_degrees(radians: f64) -> f64 {
    radians * 360. / TAU
}

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn angle_eq_rr() {
        assert!(Angle::Radians(3.) == Angle::Radians(3.));
    }

    #[test]
    fn angle_eq_dd() {
        assert!(Angle::Degrees(3.) == Angle::Degrees(3.));
    }

    #[test]
    fn angle_eq_rd() {
        assert!(Angle::Radians(TAU) == Angle::Degrees(360.));
    }

    #[test]
    fn angle_eq_dr() {
        assert!(Angle::Degrees(360.) == Angle::Radians(TAU));
    }

    #[test]
    fn angle_neq_rd() {
        assert!(Angle::Radians(TAU) != Angle::Degrees(361.));
    }

    #[test]
    fn angle_neq_dr() {
        assert!(Angle::Degrees(359.) != Angle::Radians(TAU));
    }

    #[test]
    fn angle_ord_rr() {
        assert!(Angle::Radians(1.) < Angle::Radians(2.));
    }

    #[test]
    fn angle_ord_dd() {
        assert!(Angle::Degrees(1.) < Angle::Degrees(2.));
    }

    #[test]
    fn angle_ord_rd() {
        assert!(Angle::Radians(3.) < Angle::Degrees(180.));
    }
    #[test]
    fn angle_ord_dr() {
        assert!(Angle::Degrees(180.) < Angle::Radians(4.));
    }
}
