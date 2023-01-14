use super::{cube::Cube, sphere::Sphere};
use std::rc::Weak;

pub struct PrimitiveCollection {
    pub spheres: Vec<Weak<Sphere>>,
    pub cubes: Vec<Weak<Cube>>,
}
//https://old.reddit.com/r/rust/comments/f8vfcj/idiomatic_way_of_automatically_cleaning_up_weak/
