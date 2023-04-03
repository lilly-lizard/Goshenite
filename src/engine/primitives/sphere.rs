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
pub struct Sphere {
    id: PrimitiveId,
    pub transform: PrimitiveTransform,
    pub radius: f32,
}

impl Sphere {
    pub const fn new(id: PrimitiveId, center: Vec3, radius: f32) -> Self {
        let transform = PrimitiveTransform { center };
        Self {
            id,
            transform,
            radius,
        }
    }
}

impl Primitive for Sphere {
    fn id(&self) -> PrimitiveId {
        self.id
    }

    fn type_code(&self) -> PrimitiveOpBufferUnit {
        primitive_type_codes::SPHERE
    }

    fn type_name(&self) -> &'static str {
        primitive_names::SPHERE
    }

    fn encoded_props(&self) -> PrimitivePropsSlice {
        [
            self.radius.to_bits(),
            // padding
            0,
            0,
            0,
            0,
            0,
        ]
    }

    fn transform(&self) -> &PrimitiveTransform {
        &self.transform
    }

    fn aabb(&self) -> Aabb {
        Aabb::new(self.transform, Vec3::splat(2. * self.radius))
    }
}
