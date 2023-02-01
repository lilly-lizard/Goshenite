use glam::Vec3;

/// Axis aligned bounding box
#[derive(Clone, PartialEq)]
pub struct Aabb {
    pub px_py_pz: Vec3,
    pub nx_ny_nz: Vec3,

    pub nx_py_pz: Vec3,
    pub px_ny_pz: Vec3,
    pub px_py_nz: Vec3,

    pub nx_ny_pz: Vec3,
    pub px_ny_nz: Vec3,
    pub nx_py_nz: Vec3,
}

impl Aabb {
    pub fn new(center: Vec3, dimensions: Vec3) -> Self {
        Self {
            px_py_pz: center + dimensions,
            nx_ny_nz: center - dimensions,

            nx_py_pz: center + dimensions * Vec3::new(-1., 1., 1.),
            px_ny_pz: center + dimensions * Vec3::new(1., -1., 1.),
            px_py_nz: center + dimensions * Vec3::new(1., 1., -1.),

            nx_ny_pz: center + dimensions * Vec3::new(-1., -1., 1.),
            px_ny_nz: center + dimensions * Vec3::new(1., -1., -1.),
            nx_py_nz: center + dimensions * Vec3::new(-1., 1., -1.),
        }
    }

    pub fn new_zero() -> Self {
        Self {
            px_py_pz: Vec3::ZERO,
            nx_ny_nz: Vec3::ZERO,

            nx_py_pz: Vec3::ZERO,
            px_ny_pz: Vec3::ZERO,
            px_py_nz: Vec3::ZERO,

            nx_ny_pz: Vec3::ZERO,
            px_ny_nz: Vec3::ZERO,
            nx_py_nz: Vec3::ZERO,
        }
    }

    pub fn union(&mut self, aabb: Aabb) {
        self.px_py_pz = outer_point(self.px_py_pz, aabb.px_py_pz, Vec3::ONE);
        self.nx_ny_nz = outer_point(self.nx_ny_nz, aabb.nx_ny_nz, Vec3::NEG_ONE);

        self.nx_py_pz = outer_point(self.nx_py_pz, aabb.nx_py_pz, Vec3::new(-1., 1., 1.));
        self.px_ny_pz = outer_point(self.px_ny_pz, aabb.px_ny_pz, Vec3::new(1., -1., 1.));
        self.px_py_nz = outer_point(self.px_py_nz, aabb.px_py_nz, Vec3::new(1., 1., -1.));

        self.nx_ny_pz = outer_point(self.nx_ny_pz, aabb.nx_ny_pz, Vec3::new(-1., -1., 1.));
        self.px_ny_nz = outer_point(self.px_ny_nz, aabb.px_ny_nz, Vec3::new(1., -1., -1.));
        self.nx_py_nz = outer_point(self.nx_py_nz, aabb.nx_py_nz, Vec3::new(-1., 1., -1.));
    }
}

fn outer_point(point1: Vec3, point2: Vec3, orientation: Vec3) -> Vec3 {
    (point1 * orientation).max(point2 * orientation) * orientation
}

/*
pub fn union(aabb1: Aabb, aabb2: Aabb) -> Self {
    Self {
        px_py_pz: outer_point(aabb1.px_py_pz, aabb2.px_py_pz, Vec3::ONE),
        nx_ny_nz: outer_point(aabb1.nx_ny_nz, aabb2.nx_ny_nz, Vec3::NEG_ONE),

        nx_py_pz: outer_point(aabb1.nx_py_pz, aabb2.nx_py_pz, Vec3::new(-1., 1., 1.)),
        px_ny_pz: outer_point(aabb1.px_ny_pz, aabb2.px_ny_pz, Vec3::new(1., -1., 1.)),
        px_py_nz: outer_point(aabb1.px_py_nz, aabb2.px_py_nz, Vec3::new(1., 1., -1.)),

        nx_ny_pz: outer_point(aabb1.nx_ny_pz, aabb2.nx_ny_pz, Vec3::new(-1., -1., 1.)),
        px_ny_nz: outer_point(aabb1.px_ny_nz, aabb2.px_ny_nz, Vec3::new(1., -1., -1.)),
        nx_py_nz: outer_point(aabb1.nx_py_nz, aabb2.nx_py_nz, Vec3::new(-1., 1., -1.)),
    }
}
*/
