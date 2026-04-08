use super::operation::Operation;
use crate::engine::primitives::{primitive::Primitive, primitive_transform::PrimitiveTransform};
use glam::Vec3;
use serde::{Deserialize, Serialize};

// PRIMITIVE OP

pub type PrimitiveOpIndex = usize;

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PrimitiveOp {
    pub primitive: Primitive,
    pub transform: PrimitiveTransform,
    pub op: Operation,
    /// Amount of blending between this primitive op and the previous ops in world-space units.
    pub blend: f32,
    pub albedo: Vec3,
    pub specular: f32,
}

impl PrimitiveOp {
    pub fn new(
        primitive: Primitive,
        transform: PrimitiveTransform,
        op: Operation,
        blend: f32,
        albedo: Vec3,
        specular: f32,
    ) -> Self {
        Self {
            primitive,
            transform,
            op,
            blend,
            albedo,
            specular,
        }
    }
}
