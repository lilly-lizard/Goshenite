use super::cube::Cube;
use super::sphere::Sphere;
use crate::shaders::shader_interfaces::{primitive_codes, PrimitiveDataSlice, PRIMITIVE_LEN};

pub trait EncodablePrimitive {
    fn encode(&self) -> PrimitiveDataSlice;
}

#[derive(Debug, Clone, Copy)]
pub enum Primitive {
    Null,
    Sphere(Sphere),
    Cube(Cube),
}
impl EncodablePrimitive for Primitive {
    fn encode(&self) -> PrimitiveDataSlice {
        match self {
            Primitive::Null => [primitive_codes::NULL; PRIMITIVE_LEN],
            Primitive::Sphere(s) => s.encode(),
            Primitive::Cube(c) => c.encode(),
        }
    }
}
impl Default for Primitive {
    fn default() -> Self {
        Self::Null
    }
}
impl From<Sphere> for Primitive {
    fn from(p: Sphere) -> Self {
        Self::Sphere(p)
    }
}
impl From<Cube> for Primitive {
    fn from(p: Cube) -> Self {
        Self::Cube(p)
    }
}

#[derive(Default, Debug, Clone)]
pub struct PrimitiveCollection {
    /// Encoded primitive data.
    data: Vec<PrimitiveDataSlice>,
    primitives: Vec<Primitive>,
}
impl PrimitiveCollection {
    pub fn encoded_data(&self) -> &Vec<PrimitiveDataSlice> {
        &self.data
    }

    pub fn add_primitive(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
        self.data.push(primitive.encode());
    }

    pub fn primitives(&self) -> &Vec<Primitive> {
        &self.primitives
    }

    pub fn update_primitive(&mut self, index: usize, new_primitive: Primitive) {
        if let Some(s_ref) = self.primitives.get_mut(index) {
            let data_ref = self.data.get_mut(index).expect("todo");
            let encoded = new_primitive.encode();
            *data_ref = encoded;
            *s_ref = new_primitive;
        } else {
            todo!();
        }
    }
}
