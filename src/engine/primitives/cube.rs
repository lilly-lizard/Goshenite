use super::{primitive::Primitive, primitive_ref_types::primitive_names};
use crate::{
    helper::unique_id_gen::UniqueId,
    renderer::shader_interfaces::object_buffer::{primitive_codes, PrimitiveDataSlice},
};
use glam::Vec3;

#[derive(Debug, Clone, PartialEq)]
pub struct Cube {
    id: UniqueId,
    pub center: Vec3,
    pub dimensions: Vec3,
}
impl Cube {
    pub const fn new(id: UniqueId, center: Vec3, dimensions: Vec3) -> Self {
        Self {
            id,
            center,
            dimensions,
        }
    }
}
impl Primitive for Cube {
    fn id(&self) -> UniqueId {
        self.id
    }
    fn encode(&self, parent_origin: Vec3) -> PrimitiveDataSlice {
        let world_center = self.center + parent_origin;
        [
            primitive_codes::CUBE,
            world_center.x.to_bits(),
            world_center.y.to_bits(),
            world_center.z.to_bits(),
            self.dimensions.x.to_bits(),
            self.dimensions.y.to_bits(),
            self.dimensions.z.to_bits(),
        ]
    }
    fn center(&self) -> Vec3 {
        self.center
    }
    fn type_name(&self) -> &'static str {
        primitive_names::CUBE
    }
}
