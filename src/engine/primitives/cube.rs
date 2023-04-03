use super::{
    primitive::{Primitive, PrimitiveId},
    primitive_ref_types::primitive_names,
    primitive_transform::PrimitiveTransform,
};
use crate::{
    engine::aabb::Aabb,
    renderer::shader_interfaces::primitive_op_buffer::{
        primitive_type_codes, PrimitiveOpBufferUnit, PrimitivePropsSlice,
    },
};
use glam::Vec3;

#[derive(Debug, Clone, PartialEq)]
pub struct Cube {
    id: PrimitiveId,
    pub transform: PrimitiveTransform,
    pub dimensions: Vec3,
}

impl Cube {
    pub const fn new(id: PrimitiveId, center: Vec3, dimensions: Vec3) -> Self {
        let transform = PrimitiveTransform { center };
        Self {
            id,
            transform,
            dimensions,
        }
    }
}

impl Primitive for Cube {
    fn id(&self) -> PrimitiveId {
        self.id
    }

    fn type_code(&self) -> PrimitiveOpBufferUnit {
        primitive_type_codes::CUBE
    }

    fn type_name(&self) -> &'static str {
        primitive_names::CUBE
    }

    fn encoded_props(&self) -> PrimitivePropsSlice {
        [
            self.dimensions.x.to_bits(),
            self.dimensions.y.to_bits(),
            self.dimensions.z.to_bits(),
            // padding
            0,
            0,
            0,
        ]
    }

    fn transform(&self) -> &PrimitiveTransform {
        &self.transform
    }

    fn aabb(&self) -> Aabb {
        Aabb::new(self.transform, self.dimensions)
    }
}
