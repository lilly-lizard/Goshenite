use super::{primitive::EncodablePrimitive, primitive_transform::PrimitiveTransform};
use crate::{
    engine::{
        aabb::Aabb,
        config_engine::{primitive_names, DEFAULT_DIMENSIONS},
    },
    renderer::shader_interfaces::primitive_op_buffer::PrimitivePropsSlice,
};
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Cube {
    pub dimensions: Vec3,
}

impl Cube {
    pub const fn new(dimensions: Vec3) -> Self {
        Self { dimensions }
    }

    pub const DEFAULT: Cube = Cube {
        dimensions: DEFAULT_DIMENSIONS,
    };
}

impl Default for Cube {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl EncodablePrimitive for Cube {
    fn type_name(&self) -> &'static str {
        primitive_names::CUBE
    }

    fn encoded_props(&self) -> PrimitivePropsSlice {
        let width = self.dimensions.x / 2.0;
        let depth = self.dimensions.y / 2.0;
        let height = self.dimensions.z / 2.0;
        let thickness = 0.5_f32;
        let corner_radius = Vec2::new(-1.0, 0.0);
        [
            width.to_bits(),
            depth.to_bits(),
            height.to_bits(),
            thickness.to_bits(),
            corner_radius.x.to_bits(),
            corner_radius.y.to_bits(),
        ]
    }

    fn aabb(&self, primitive_transform: PrimitiveTransform) -> Aabb {
        // todo calculate only when props/transform changed? cache result?

        let half_dimensions = self.dimensions / 2_f32;
        let four_corners = vec![
            half_dimensions,
            Vec3 {
                x: -half_dimensions.x,
                ..half_dimensions
            },
            Vec3 {
                y: -half_dimensions.y,
                ..half_dimensions
            },
            Vec3 {
                z: -half_dimensions.z,
                ..half_dimensions
            },
        ];

        let rotation = primitive_transform.total_rotation();
        let rotated_four_corners_abs = four_corners
            .into_iter()
            .map(|corner| rotation.mul_vec3(corner).abs())
            .collect::<Vec<_>>();

        let mut aabb_dimensions = Vec3::ZERO;
        for rotated_corner in rotated_four_corners_abs {
            aabb_dimensions.x = aabb_dimensions.x.max(rotated_corner.x);
            aabb_dimensions.y = aabb_dimensions.y.max(rotated_corner.y);
            aabb_dimensions.z = aabb_dimensions.z.max(rotated_corner.z);
        }

        Aabb::new(primitive_transform.center, aabb_dimensions * 2_f32)
    }
}
