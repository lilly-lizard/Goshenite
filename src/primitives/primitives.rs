use super::cube::Cube;
use super::sphere::Sphere;
use crate::{
    helper::from_enum_impl::from_enum_impl,
    shaders::shader_interfaces::{primitive_codes, PrimitiveDataSlice, PRIMITIVE_LEN},
};
use glam::Vec3;
use std::fmt;

/// Required functions for a usable primitive.
pub trait PrimitiveTrait {
    /// Returns the primitive data encoded as a [`PrimitiveDataSlice`].
    ///
    /// _Note: must match the decode process in `scene.comp`_
    fn encode(&self) -> PrimitiveDataSlice;
    /// Returns the spacial center of the primitive.
    fn center(&self) -> Vec3;
}

/// Enum of all the supported primitive types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Primitive {
    Null,
    Sphere(Sphere),
    Cube(Cube),
}
impl PrimitiveTrait for Primitive {
    fn encode(&self) -> PrimitiveDataSlice {
        match self {
            Primitive::Null => [primitive_codes::NULL; PRIMITIVE_LEN],
            Primitive::Sphere(s) => s.encode(),
            Primitive::Cube(c) => c.encode(),
        }
    }
    fn center(&self) -> Vec3 {
        match self {
            Primitive::Null => Default::default(),
            Primitive::Sphere(s) => s.center(),
            Primitive::Cube(c) => c.center(),
        }
    }
}
impl Default for Primitive {
    fn default() -> Self {
        Self::Null
    }
}
impl Primitive {
    /// Returns the name of the enum primitive type
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "Null",
            Self::Sphere(_) => "Sphere",
            Self::Cube(_) => "Cube",
        }
    }
}
from_enum_impl!(Primitive, Sphere);
from_enum_impl!(Primitive, Cube);

/// Collection of [`Primitive`]s. Also contains encoded data to upload to the gpu.
#[derive(Default, Debug, Clone)]
pub struct PrimitiveCollection {
    /// Encoded primitive data.
    data: Vec<PrimitiveDataSlice>,
    primitives: Vec<Primitive>,
}
impl PrimitiveCollection {
    /// Returns vector containing encoded data for all the primitives in the collection.
    pub fn encoded_data(&self) -> &Vec<PrimitiveDataSlice> {
        &self.data
    }

    /// Appends a new primitive to the primitive collection.
    pub fn add_primitive(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
        self.data.push(primitive.encode());
    }

    /// Returns a reference to the primitives collection.
    pub fn primitives(&self) -> &Vec<Primitive> {
        &self.primitives
    }

    /// Updates an existing primitive in collection at `index`.
    pub fn update_primitive(
        &mut self,
        index: usize,
        new_primitive: Primitive,
    ) -> Result<(), PrimitiveCollectionError> {
        if let Some(s_ref) = self.primitives.get_mut(index) {
            let data_ref = self.data.get_mut(index).expect("todo");
            let encoded = new_primitive.encode();
            *data_ref = encoded;
            *s_ref = new_primitive;
            Ok(())
        } else {
            Err(PrimitiveCollectionError::InvalidPrimitiveIndex {
                index,
                primitive_count: self.primitives.len(),
            })
        }
    }
}

#[derive(Debug)]
pub enum PrimitiveCollectionError {
    /// Attempted to access primitive with out of bounds index.
    InvalidPrimitiveIndex {
        index: usize,
        primitive_count: usize,
    },
}
impl fmt::Display for PrimitiveCollectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PrimitiveCollectionError::InvalidPrimitiveIndex { index, primitive_count } =>
                write!(f, "attempted to access primitive without out of bounds index. index = {}, primitive count = {}", index, primitive_count)
        }
    }
}
