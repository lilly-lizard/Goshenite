use super::{
    primitive::{primitive_names, EncodablePrimitive, DEFAULT_DIMENSIONS},
    primitive_transform::PrimitiveTransform,
};
use crate::{
    engine::aabb::Aabb, renderer::shader_interfaces::primitive_op_buffer::PrimitivePropsSlice,
};
use glam::{Quat, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cube {
    pub transform: PrimitiveTransform,
    pub dimensions: Vec3,
}

impl Cube {
    pub const fn new(center: Vec3, rotation: Quat, dimensions: Vec3) -> Self {
        let transform = PrimitiveTransform { center, rotation };
        Self {
            transform,
            dimensions,
        }
    }
}

impl Default for Cube {
    fn default() -> Self {
        Self {
            transform: PrimitiveTransform::default(),
            dimensions: DEFAULT_DIMENSIONS,
        }
    }
}

impl EncodablePrimitive for Cube {
    fn type_name(&self) -> &'static str {
        primitive_names::CUBE
    }

    fn encoded_props(&self) -> PrimitivePropsSlice {
        [
            self.dimensions.x.to_bits(),
            self.dimensions.y.to_bits(),
            self.dimensions.z.to_bits(),
            0_f32.to_bits(),
            0_f32.to_bits(),
            0_f32.to_bits(),
        ]
    }

    fn transform(&self) -> &PrimitiveTransform {
        &self.transform
    }

    fn aabb(&self) -> Aabb {
        // todo calculate only when props/transform changed!
        //todo!("dimensions need to be adjusted for rotation!");
        Aabb::new(self.transform.center, self.dimensions)
    }
}
