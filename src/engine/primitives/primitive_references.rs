use crate::helper::unique_id_gen::UniqueIdGen;

use super::{cube::Cube, sphere::Sphere};
use ahash::AHashMap;
use glam::Vec3;
use std::rc::{Rc, Weak};

// todo doc
pub mod primitive_names {
    pub const SPHERE: &'static str = "Sphere";
    pub const CUBE: &'static str = "Cube";
}

pub struct PrimitiveReferences {
    unique_id_gen: UniqueIdGen,
    pub spheres: AHashMap<usize, Weak<Sphere>>,
    pub cubes: AHashMap<usize, Weak<Cube>>,
}

impl PrimitiveReferences {
    pub fn new() -> Self {
        Self {
            unique_id_gen: UniqueIdGen::new(),
            spheres: AHashMap::<usize, Weak<Sphere>>::default(),
            cubes: AHashMap::<usize, Weak<Cube>>::default(),
        }
    }

    pub fn new_sphere(&mut self, center: Vec3, radius: f32) -> Rc<Sphere> {
        let id = self.unique_id_gen.new_id();
        let sphere = Rc::new(Sphere::new(id, center, radius));
        self.spheres.insert(id, Rc::downgrade(&sphere));
        sphere
    }
    pub fn new_cube(&mut self, center: Vec3, dimensions: Vec3) -> Rc<Cube> {
        let id = self.unique_id_gen.new_id();
        let cube = Rc::new(Cube::new(id, center, dimensions));
        self.cubes.insert(id, Rc::downgrade(&cube));
        cube
    }

    pub fn get_sphere(&self, id: usize) -> Option<Rc<Sphere>> {
        get_primitive::<Sphere>(id, &self.spheres)
    }
    pub fn get_cube(&self, id: usize) -> Option<Rc<Cube>> {
        get_primitive::<Cube>(id, &self.cubes)
    }
}

fn get_primitive<T>(id: usize, collection: &AHashMap<usize, Weak<T>>) -> Option<Rc<T>> {
    if let Some(weak_ref) = collection.get(&id) {
        weak_ref.upgrade()
    } else {
        None
    }
}
