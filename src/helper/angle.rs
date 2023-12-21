//! Shout out to cgmath for the idea https://github.com/rustgd/cgmath

use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    f64::consts::{PI, TAU},
    ops,
};

/// Represents a f64 angle in radians or degrees
///
/// _Note:using an enum allows us to define const values in radians or degrees!_
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub enum Angle {
    Radians(f64),
    Degrees(f64),
}

impl Default for Angle {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Angle {
    pub const ZERO: Self = Self::Radians(0.);
    pub const PI: Self = Self::Radians(PI);
    pub const TAU: Self = Self::Radians(TAU);

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

    pub fn to_radians(self) -> Self {
        if let Self::Degrees(d) = self {
            Self::Radians(degrees_to_radians(d))
        } else {
            self
        }
    }

    pub fn to_degrees(self) -> Self {
        if let Self::Radians(r) = self {
            Self::Degrees(radians_to_degrees(r))
        } else {
            self
        }
    }

    pub fn invert(&self) -> Self {
        match self {
            Self::Radians(r) => Self::Radians(-r),
            Self::Degrees(d) => Self::Degrees(-d),
        }
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

// ~~ Compare Operators ~~

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

// ~~ Arithmetic Operators ~~

// metavariable designator types: https://doc.rust-lang.org/reference/macros-by-example.html#metavariables
macro_rules! angle_operator_impl {
    ($op_trait:ident, $op_fn:ident, $op:tt) => {

        impl<T> ops::$op_trait<T> for Angle
        where
            T: Into<f64>,
        {
            type Output = Self;
            fn $op_fn(self, rhs: T) -> Self::Output {
                let rhs_f: f64 = rhs.into();
                match self {
                    Self::Radians(r) => Self::Radians(r $op rhs_f),
                    Self::Degrees(d) => Self::Degrees(d $op rhs_f),
                }
            }
        }

        impl ops::$op_trait for Angle {
            type Output = Self;
            fn $op_fn(self, rhs: Self) -> Self::Output {
                match self {
                    Self::Radians(r_lhs) => {
                        let r_rhs = rhs.radians();
                        Self::Radians(r_lhs $op r_rhs)
                    }
                    Self::Degrees(d_lhs) => {
                        let d_rhs = rhs.degrees();
                        Self::Degrees(d_lhs $op d_rhs)
                    }
                }
            }
        }
    }
}

angle_operator_impl!(Add, add, +);
angle_operator_impl!(Sub, sub, -);
angle_operator_impl!(Mul, mul, *);
angle_operator_impl!(Div, div, /);

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
