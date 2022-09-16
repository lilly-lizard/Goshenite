/// Shout out to cgmath for the idea https://github.com/rustgd/cgmath
use std::ops::Deref;

/// Represents an angle in f32 radians
#[derive(Copy, Clone, PartialEq, PartialOrd, Default, Debug)]
pub struct Radians {
    pub val: f32,
}

/// Represents an angle in f32 degrees
#[derive(Copy, Clone, PartialEq, PartialOrd, Default, Debug)]
pub struct Degrees {
    pub val: f32,
}

// From impls
impl From<f32> for Radians {
    #[inline]
    fn from(val: f32) -> Radians {
        Radians { val }
    }
}
impl From<f32> for Degrees {
    #[inline]
    fn from(val: f32) -> Degrees {
        Degrees { val }
    }
}
impl From<Radians> for f32 {
    #[inline]
    fn from(rad: Radians) -> f32 {
        *rad
    }
}
impl From<Degrees> for f32 {
    #[inline]
    fn from(deg: Degrees) -> f32 {
        *deg
    }
}
impl From<Radians> for Degrees {
    #[inline]
    fn from(rad: Radians) -> Degrees {
        Degrees {
            val: rad.val * 180.0 / std::f32::consts::PI, // same as f32::to_degrees()
        }
    }
}
impl From<Degrees> for Radians {
    #[inline]
    fn from(deg: Degrees) -> Radians {
        Radians {
            val: deg.val * std::f32::consts::PI / 180., // same as f32::to_radians()
        }
    }
}

// To conversions
impl Radians {
    pub fn to_degrees(self) -> Degrees {
        self.into()
    }
}
impl Degrees {
    pub fn to_radians(self) -> Radians {
        self.into()
    }
}

// Deref impls
impl Deref for Radians {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.val
    }
}
impl Deref for Degrees {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.val
    }
}
