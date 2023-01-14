use super::{cube::Cube, sphere::Sphere};

pub struct PrimitiveCollection {
    pub spheres: Vec<Sphere>,
    pub cubes: Vec<Cube>,
}
