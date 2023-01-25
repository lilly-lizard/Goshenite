use super::{primitive::Primitive, primitive_ref_types::primitive_names};
use crate::{
    helper::unique_id_gen::UniqueId,
    renderer::shader_interfaces::object_buffer::{primitive_codes, PrimitiveDataSlice},
};
use glam::Vec3;

#[derive(Debug, Clone, PartialEq)]
pub struct Sphere {
    id: UniqueId,
    pub center: Vec3,
    pub radius: f32,
}
impl Sphere {
    pub const fn new(id: UniqueId, center: Vec3, radius: f32) -> Self {
        Self { id, center, radius }
    }
    pub const fn new_default(id: UniqueId) -> Self {
        Self {
            id,
            center: Vec3::ZERO,
            radius: 0.,
        }
    }
}
impl Primitive for Sphere {
    fn id(&self) -> UniqueId {
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
}
