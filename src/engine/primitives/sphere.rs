use super::{
    primitive::{default_radius, Primitive, PrimitiveId},
    primitive_ref_types::primitive_names,
    primitive_transform::PrimitiveTransform,
};
use crate::{
    engine::aabb::Aabb,
    renderer::shader_interfaces::primitive_op_buffer::{
        primitive_type_codes, PrimitiveOpBufferUnit, PrimitivePropsSlice,
    },
};
use glam::{Quat, Vec3};

#[derive(Debug, Clone, PartialEq)]
pub struct Sphere {
    id: PrimitiveId,
    pub transform: PrimitiveTransform,
    pub radius: f32,
}

impl Sphere {
    pub const fn new_default(id: PrimitiveId) -> Self {
        Self {
            id,
            transform: PrimitiveTransform::new_default(),
            radius: default_radius(),
        }
    }

    pub const fn new(id: PrimitiveId, center: Vec3, rotation: Quat, radius: f32) -> Self {
        let transform = PrimitiveTransform { center, rotation };
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
        // todo calculate only when props/transform changed!
        Aabb::new(self.transform, Vec3::splat(2. * self.radius))
    }
}
