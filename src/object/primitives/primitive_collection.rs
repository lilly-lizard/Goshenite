use super::{cube::Cube, sphere::Sphere};
use std::rc::Rc;

pub struct PrimitiveCollection {
    pub spheres: Vec<Rc<Sphere>>,
    pub cubes: Vec<Rc<Cube>>,
}
