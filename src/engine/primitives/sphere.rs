use super::{
    primitive::{Primitive, PrimitiveId},
    primitive_ref_types::primitive_names,
};
use crate::{
    engine::aabb::Aabb,
    renderer::shader_interfaces::object_buffer::{primitive_codes, PrimitiveDataSlice},
};
use glam::Vec3;

#[derive(Debug, Clone, PartialEq)]
pub struct Sphere {
    id: PrimitiveId,
    pub center: Vec3,
    pub radius: f32,
}

impl Sphere {
    pub const fn new(id: PrimitiveId, center: Vec3, radius: f32) -> Self {
        Self { id, center, radius }
    }
}

impl Primitive for Sphere {
    fn id(&self) -> PrimitiveId {
        self.id
    }

    fn encode(&self, parent_origin: Vec3) -> PrimitiveDataSlice {
        let world_center = self.center + parent_origin;
        [
            primitive_codes::SPHERE,
            world_center.x.to_bits(),
            world_center.y.to_bits(),
            world_center.z.to_bits(),
            self.radius.to_bits(),
            // padding
            primitive_codes::NULL,
            primitive_codes::NULL,
        ]
    }

    fn center(&self) -> Vec3 {
        self.center
    }

    fn type_name(&self) -> &'static str {
        primitive_names::SPHERE
    }

    fn aabb(&self) -> Aabb {
        Aabb::new(self.center, Vec3::splat(self.radius))
    }
}
