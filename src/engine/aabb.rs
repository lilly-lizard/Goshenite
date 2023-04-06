use crate::renderer::shader_interfaces::vertex_inputs::BoundingBoxVertex;
use glam::Vec3;

use super::{object::object::ObjectId, primitives::primitive_transform::PrimitiveTransform};

pub const AABB_VERTEX_COUNT: usize = 36;

/// Axis aligned bounding box
#[derive(Clone, PartialEq)]
pub struct Aabb {
    /// positive corner
    pub max: Vec3,
    /// negative corner
    pub min: Vec3,
}

impl Aabb {
    /// `dimensions` is the x/y/z lengths of the box.
    pub fn new(transform: PrimitiveTransform, dimensions: Vec3) -> Self {
        let dimensions_halved = dimensions / 2.;
        let max = transform.center + dimensions_halved;
        let min = transform.center - dimensions_halved;

        Self { max, min }
    }

    pub fn new_zero() -> Self {
        Self {
            max: Vec3::ZERO,
            min: Vec3::ZERO,
        }
    }

    pub fn union(&mut self, aabb: Aabb) {
        self.max = self.max.max(aabb.max);
        self.min = self.min.min(aabb.min);
    }

    pub fn offset(&mut self, offset: Vec3) {
        self.max += offset;
        self.min += offset;
    }

    /// Counter-clockwise front face
    pub fn vertices(&self, object_id: ObjectId) -> [BoundingBoxVertex; AABB_VERTEX_COUNT] {
        // note that vertex generation happens far less often than other operations (e.g. union)
        // so its more efficient to only store min/max and then generate other corners here.

        let center = (self.max + self.min) / 2.;
        let dimensions_halved = (self.max - self.min) / 2.;

        let xp_yp_zp = self.max;
        let xn_yn_zn = self.min;

        let xn_yp_zp = center + dimensions_halved * Vec3::new(-1., 1., 1.);
        let xp_yn_zp = center + dimensions_halved * Vec3::new(1., -1., 1.);
        let xp_yp_zn = center + dimensions_halved * Vec3::new(1., 1., -1.);

        let xn_yn_zp = center + dimensions_halved * Vec3::new(-1., -1., 1.);
        let xp_yn_zn = center + dimensions_halved * Vec3::new(1., -1., -1.);
        let xn_yp_zn = center + dimensions_halved * Vec3::new(-1., 1., -1.);

        [
            // positive x face 1
            BoundingBoxVertex::new(xp_yn_zp, object_id),
            BoundingBoxVertex::new(xp_yp_zn, object_id),
            BoundingBoxVertex::new(xp_yp_zp, object_id),
            // positive x face 2
            BoundingBoxVertex::new(xp_yp_zn, object_id),
            BoundingBoxVertex::new(xp_yn_zp, object_id),
            BoundingBoxVertex::new(xp_yn_zn, object_id),
            // positive y face 1
            BoundingBoxVertex::new(xp_yp_zn, object_id),
            BoundingBoxVertex::new(xn_yp_zp, object_id),
            BoundingBoxVertex::new(xp_yp_zp, object_id),
            // positive y face 2
            BoundingBoxVertex::new(xn_yp_zp, object_id),
            BoundingBoxVertex::new(xp_yp_zn, object_id),
            BoundingBoxVertex::new(xn_yp_zn, object_id),
            // positive z face 1
            BoundingBoxVertex::new(xn_yn_zp, object_id),
            BoundingBoxVertex::new(xp_yp_zp, object_id),
            BoundingBoxVertex::new(xn_yp_zp, object_id),
            // positive z face 2
            BoundingBoxVertex::new(xp_yp_zp, object_id),
            BoundingBoxVertex::new(xn_yn_zp, object_id),
            BoundingBoxVertex::new(xp_yn_zp, object_id),
            // negative x face 1
            BoundingBoxVertex::new(xn_yp_zn, object_id),
            BoundingBoxVertex::new(xn_yn_zp, object_id),
            BoundingBoxVertex::new(xn_yp_zp, object_id),
            // negative x face 2
            BoundingBoxVertex::new(xn_yn_zp, object_id),
            BoundingBoxVertex::new(xn_yp_zn, object_id),
            BoundingBoxVertex::new(xn_yn_zn, object_id),
            // negative y face 1
            BoundingBoxVertex::new(xn_yn_zp, object_id),
            BoundingBoxVertex::new(xp_yn_zn, object_id),
            BoundingBoxVertex::new(xp_yn_zp, object_id),
            // negative y face 2
            BoundingBoxVertex::new(xp_yn_zn, object_id),
            BoundingBoxVertex::new(xn_yn_zp, object_id),
            BoundingBoxVertex::new(xn_yn_zn, object_id),
            // negative z face 1
            BoundingBoxVertex::new(xp_yp_zn, object_id),
            BoundingBoxVertex::new(xn_yn_zn, object_id),
            BoundingBoxVertex::new(xn_yp_zn, object_id),
            // negative z face 2
            BoundingBoxVertex::new(xn_yn_zn, object_id),
            BoundingBoxVertex::new(xp_yp_zn, object_id),
            BoundingBoxVertex::new(xp_yn_zn, object_id),
        ]
    }
}
