use super::{cube::Cube, sphere::Sphere};
use std::{cell::RefCell, rc::Rc};

/// Implimentations of [`Primtive`] supported by [`PrimitiveReferences`] return one of these values
/// when calling [`Primtive::type_name`]
pub mod primitive_names {
    pub const SPHERE: &'static str = "Sphere";
    pub const CUBE: &'static str = "Cube";
}

pub enum PrimitiveRefType {
    Unknown,
    Sphere,
    Cube,
}
impl PrimitiveRefType {
    /// Pass in name from [`Primtive::type_name`]
    pub fn from_name(name: &str) -> Self {
        match name {
            primitive_names::SPHERE => Self::Sphere,
            primitive_names::CUBE => Self::Cube,
            _ => Self::Unknown,
        }
    }
}
impl Default for PrimitiveRefType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Use functions `borrow` and `borrow_mut` to access the `Sphere`.
pub type SphereRef = RefCell<Sphere>;
#[inline]
pub fn new_sphere_ref(inner: Sphere) -> Rc<SphereRef> {
    Rc::new(RefCell::new(inner))
}

/// Use functions `borrow` and `borrow_mut` to access the `Cube`.
pub type CubeRef = RefCell<Cube>;
#[inline]
pub fn new_cube_ref(inner: Cube) -> Rc<CubeRef> {
    Rc::new(RefCell::new(inner))
}
