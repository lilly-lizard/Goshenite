use super::{
    cube::Cube,
    primitive_ref_types::{new_cube_ref, new_sphere_ref, CubeRef, SphereRef},
    sphere::Sphere,
};
use crate::helper::unique_id_gen::UniqueIdGen;
use ahash::AHashMap;
use glam::Vec3;
use std::rc::{Rc, Weak};

pub struct PrimitiveReferences {
    unique_id_gen: UniqueIdGen,
    pub spheres: AHashMap<usize, Weak<SphereRef>>,
    pub cubes: AHashMap<usize, Weak<CubeRef>>,
}

impl PrimitiveReferences {
    pub fn new() -> Self {
        Self {
            unique_id_gen: UniqueIdGen::new(),
            spheres: AHashMap::<usize, Weak<SphereRef>>::default(),
            cubes: AHashMap::<usize, Weak<CubeRef>>::default(),
        }
    }

    pub fn new_sphere(&mut self, center: Vec3, radius: f32) -> Rc<SphereRef> {
        let id = self.unique_id_gen.new_id();
        let sphere = new_sphere_ref(Sphere::new(id, center, radius));
        self.spheres.insert(id, Rc::downgrade(&sphere));
        sphere
    }
    pub fn new_cube(&mut self, center: Vec3, dimensions: Vec3) -> Rc<CubeRef> {
        let id = self.unique_id_gen.new_id();
        let cube = new_cube_ref(Cube::new(id, center, dimensions));
        self.cubes.insert(id, Rc::downgrade(&cube));
        cube
    }

    pub fn get_sphere(&self, id: usize) -> Option<Rc<SphereRef>> {
        get_primitive::<SphereRef>(id, &self.spheres)
    }
    pub fn get_cube(&self, id: usize) -> Option<Rc<CubeRef>> {
        get_primitive::<CubeRef>(id, &self.cubes)
    }
}

fn get_primitive<T>(id: usize, collection: &AHashMap<usize, Weak<T>>) -> Option<Rc<T>> {
    if let Some(weak_ref) = collection.get(&id) {
        weak_ref.upgrade()
    } else {
        None
    }
}
