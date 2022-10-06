/// Shout out to cgmath for the idea https://github.com/rustgd/cgmath

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
    degrees * std::f64::consts::PI / 180.
}

#[inline]
fn radians_to_degrees(radians: f64) -> f64 {
    radians * 180.0 / std::f64::consts::PI
}

// // convert between floats and angles
// impl From<f32> for Radians {
//     #[inline]
//     fn from(val: f32) -> Radians {
//         Radians { val }
//     }
// }
// impl From<f32> for Degrees {
//     #[inline]
//     fn from(val: f32) -> Degrees {
//         Degrees { val }
//     }
// }

// // convert between radians and degrees
// impl From<Radians> for Degrees {
//     #[inline]
//     fn from(rad: Radians) -> Degrees {
//         Degrees {
//             val: rad.val * 180.0 / std::f32::consts::PI, // same as f32::to_degrees()
//         }
//     }
// }
// impl From<Degrees> for Radians {
//     #[inline]
//     fn from(deg: Degrees) -> Radians {
//         Radians {
//             val: deg.val * std::f32::consts::PI / 180., // same as f32::to_radians()
//         }
//     }
// }
// impl Radians {
//     pub fn to_degrees(self) -> Degrees {
//         self.into()
//     }
// }
// impl Degrees {
//     pub fn to_radians(self) -> Radians {
//         self.into()
//     }
// }

// // Deref impls
// impl Deref for Radians {
//     type Target = f32;
//     fn deref(&self) -> &Self::Target {
//         &self.val
//     }
// }
// impl Deref for Degrees {
//     type Target = f32;
//     fn deref(&self) -> &Self::Target {
//         &self.val
//     }
// }
