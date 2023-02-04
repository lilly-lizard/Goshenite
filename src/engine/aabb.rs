use crate::renderer::shader_interfaces::vertex_inputs::BoundingBoxVertex;
use glam::Vec3;

use super::object::object::ObjectId;

pub const AABB_VERTEX_COUNT: usize = 36;

/// Axis aligned bounding box
#[derive(Clone, PartialEq)]
pub struct Aabb {
    pub xp_yp_zp: Vec3,
    pub xn_yn_zn: Vec3,

    pub xn_yp_zp: Vec3,
    pub xp_yn_zp: Vec3,
    pub xp_yp_zn: Vec3,

    pub xn_yn_zp: Vec3,
    pub xp_yn_zn: Vec3,
    pub xn_yp_zn: Vec3,
}

impl Aabb {
    pub fn new(center: Vec3, dimensions: Vec3) -> Self {
        Self {
            xp_yp_zp: center + dimensions,
            xn_yn_zn: center - dimensions,

            xn_yp_zp: center + dimensions * Vec3::new(-1., 1., 1.),
            xp_yn_zp: center + dimensions * Vec3::new(1., -1., 1.),
            xp_yp_zn: center + dimensions * Vec3::new(1., 1., -1.),

            xn_yn_zp: center + dimensions * Vec3::new(-1., -1., 1.),
            xp_yn_zn: center + dimensions * Vec3::new(1., -1., -1.),
            xn_yp_zn: center + dimensions * Vec3::new(-1., 1., -1.),
        }
    }

    pub fn new_zero() -> Self {
        Self {
            xp_yp_zp: Vec3::ZERO,
            xn_yn_zn: Vec3::ZERO,

            xn_yp_zp: Vec3::ZERO,
            xp_yn_zp: Vec3::ZERO,
            xp_yp_zn: Vec3::ZERO,

            xn_yn_zp: Vec3::ZERO,
            xp_yn_zn: Vec3::ZERO,
            xn_yp_zn: Vec3::ZERO,
        }
    }

    pub fn union(&mut self, aabb: Aabb) {
        self.xp_yp_zp = outer_point(self.xp_yp_zp, aabb.xp_yp_zp, Vec3::ONE);
        self.xn_yn_zn = outer_point(self.xn_yn_zn, aabb.xn_yn_zn, Vec3::NEG_ONE);

        self.xn_yp_zp = outer_point(self.xn_yp_zp, aabb.xn_yp_zp, Vec3::new(-1., 1., 1.));
        self.xp_yn_zp = outer_point(self.xp_yn_zp, aabb.xp_yn_zp, Vec3::new(1., -1., 1.));
        self.xp_yp_zn = outer_point(self.xp_yp_zn, aabb.xp_yp_zn, Vec3::new(1., 1., -1.));

        self.xn_yn_zp = outer_point(self.xn_yn_zp, aabb.xn_yn_zp, Vec3::new(-1., -1., 1.));
        self.xp_yn_zn = outer_point(self.xp_yn_zn, aabb.xp_yn_zn, Vec3::new(1., -1., -1.));
        self.xn_yp_zn = outer_point(self.xn_yp_zn, aabb.xn_yp_zn, Vec3::new(-1., 1., -1.));
    }

    pub fn offset(&mut self, offset: Vec3) {
        self.xp_yp_zp += offset;
        self.xn_yn_zn += offset;

        self.xn_yp_zp += offset;
        self.xp_yn_zp += offset;
        self.xp_yp_zn += offset;

        self.xn_yn_zp += offset;
        self.xp_yn_zn += offset;
        self.xn_yp_zn += offset;
    }

    pub fn vertices(&self, object_id: ObjectId) -> [BoundingBoxVertex; AABB_VERTEX_COUNT] {
        [
            // positive x face 1
            BoundingBoxVertex::new(self.xp_yn_zp, object_id),
            BoundingBoxVertex::new(self.xp_yp_zn, object_id),
            BoundingBoxVertex::new(self.xp_yp_zp, object_id),
            // positive x face 2
            BoundingBoxVertex::new(self.xp_yp_zn, object_id),
            BoundingBoxVertex::new(self.xp_yn_zp, object_id),
            BoundingBoxVertex::new(self.xp_yn_zn, object_id),
            // positive y face 1
            BoundingBoxVertex::new(self.xp_yp_zn, object_id),
            BoundingBoxVertex::new(self.xn_yp_zp, object_id),
            BoundingBoxVertex::new(self.xp_yp_zp, object_id),
            // positive y face 2
            BoundingBoxVertex::new(self.xn_yp_zp, object_id),
            BoundingBoxVertex::new(self.xp_yp_zn, object_id),
            BoundingBoxVertex::new(self.xn_yp_zn, object_id),
            // positive z face 1
            BoundingBoxVertex::new(self.xn_yn_zp, object_id),
            BoundingBoxVertex::new(self.xp_yp_zp, object_id),
            BoundingBoxVertex::new(self.xn_yp_zp, object_id),
            // positive z face 2
            BoundingBoxVertex::new(self.xp_yp_zp, object_id),
            BoundingBoxVertex::new(self.xn_yn_zp, object_id),
            BoundingBoxVertex::new(self.xp_yn_zp, object_id),
            // negative x face 1
            BoundingBoxVertex::new(self.xn_yp_zn, object_id),
            BoundingBoxVertex::new(self.xn_yn_zp, object_id),
            BoundingBoxVertex::new(self.xn_yp_zp, object_id),
            // negative x face 2
            BoundingBoxVertex::new(self.xn_yn_zp, object_id),
            BoundingBoxVertex::new(self.xn_yp_zn, object_id),
            BoundingBoxVertex::new(self.xn_yn_zn, object_id),
            // negative y face 1
            BoundingBoxVertex::new(self.xn_yn_zp, object_id),
            BoundingBoxVertex::new(self.xp_yn_zn, object_id),
            BoundingBoxVertex::new(self.xp_yn_zp, object_id),
            // negative y face 2
            BoundingBoxVertex::new(self.xp_yn_zn, object_id),
            BoundingBoxVertex::new(self.xn_yn_zp, object_id),
            BoundingBoxVertex::new(self.xn_yn_zn, object_id),
            // negative z face 1
            BoundingBoxVertex::new(self.xp_yp_zn, object_id),
            BoundingBoxVertex::new(self.xn_yn_zn, object_id),
            BoundingBoxVertex::new(self.xn_yp_zn, object_id),
            // negative z face 2
            BoundingBoxVertex::new(self.xn_yn_zn, object_id),
            BoundingBoxVertex::new(self.xp_yp_zn, object_id),
            BoundingBoxVertex::new(self.xp_yn_zn, object_id),
        ]
    }
}

fn outer_point(point1: Vec3, point2: Vec3, orientation: Vec3) -> Vec3 {
    (point1 * orientation).max(point2 * orientation) * orientation
}

/*
pub fn union(aabb1: Aabb, aabb2: Aabb) -> Self {
    Self {
        xp_yp_zp: outer_point(aabb1.xp_yp_zp, aabb2.xp_yp_zp, Vec3::ONE),
        xn_yn_zn: outer_point(aabb1.xn_yn_zn, aabb2.xn_yn_zn, Vec3::NEG_ONE),

        xn_yp_zp: outer_point(aabb1.xn_yp_zp, aabb2.xn_yp_zp, Vec3::new(-1., 1., 1.)),
        xp_yn_zp: outer_point(aabb1.xp_yn_zp, aabb2.xp_yn_zp, Vec3::new(1., -1., 1.)),
        xp_yp_zn: outer_point(aabb1.xp_yp_zn, aabb2.xp_yp_zn, Vec3::new(1., 1., -1.)),

        xn_yn_zp: outer_point(aabb1.xn_yn_zp, aabb2.xn_yn_zp, Vec3::new(-1., -1., 1.)),
        xp_yn_zn: outer_point(aabb1.xp_yn_zn, aabb2.xp_yn_zn, Vec3::new(1., -1., -1.)),
        xn_yp_zn: outer_point(aabb1.xn_yp_zn, aabb2.xn_yp_zn, Vec3::new(-1., 1., -1.)),
    }
}
*/
