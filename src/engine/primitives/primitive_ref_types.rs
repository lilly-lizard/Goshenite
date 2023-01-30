use super::{cube::Cube, sphere::Sphere};
use std::{cell::RefCell, rc::Rc};

/// Implimentations of [`Primtive`] supported by [`PrimitiveReferences`] return one of these values
/// when calling [`Primtive::type_name`]
pub mod primitive_names {
    pub const NULL: &'static str = "Null Primitive";
    pub const SPHERE: &'static str = "Sphere";
    pub const CUBE: &'static str = "Cube";
}

static VARIANTS: &[PrimitiveRefType] = &[
    PrimitiveRefType::Null,
    PrimitiveRefType::Sphere,
    PrimitiveRefType::Cube,
    // PrimitiveRefType::Unknown -> shouldn't be shown in lists
];

/// Possible primitive variations supported by [`PrimitiveReferences`]
#[derive(PartialEq, Clone, Copy)]
pub enum PrimitiveRefType {
    Null,
    Sphere,
    Cube,
    Unknown,
}

impl PrimitiveRefType {
    pub fn variant_names() -> Vec<(Self, &'static str)> {
        VARIANTS
            .iter()
            .map(|&p| (p, p.into()))
            .collect::<Vec<(Self, &'static str)>>()
    }
}

impl From<PrimitiveRefType> for &str {
    fn from(value: PrimitiveRefType) -> Self {
        match value {
            PrimitiveRefType::Null => primitive_names::NULL,
            PrimitiveRefType::Sphere => primitive_names::SPHERE,
            PrimitiveRefType::Cube => primitive_names::CUBE,
            PrimitiveRefType::Unknown => "Unknown",
        }
    }
}
impl From<&str> for PrimitiveRefType {
    fn from(name: &str) -> Self {
        match name {
            primitive_names::NULL => PrimitiveRefType::Null,
            primitive_names::SPHERE => PrimitiveRefType::Sphere,
            primitive_names::CUBE => PrimitiveRefType::Cube,
            _ => PrimitiveRefType::Unknown,
        }
    }
}

impl Default for PrimitiveRefType {
    fn default() -> Self {
        Self::Null
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
