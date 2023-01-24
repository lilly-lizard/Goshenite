use super::{
    cube::Cube,
    primitive_ref_types::{new_cube_ref, new_sphere_ref, CubeRef, SphereRef},
    sphere::Sphere,
};
use crate::helper::unique_id_gen::{UniqueId, UniqueIdGen};
use ahash::AHashMap;
use glam::Vec3;
use std::{
    rc::{Rc, Weak},
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct PrimitiveReferences {
    id: usize,
    unique_id_gen: UniqueIdGen,
    pub spheres: AHashMap<UniqueId, Weak<SphereRef>>,
    pub cubes: AHashMap<UniqueId, Weak<CubeRef>>,
}

impl PrimitiveReferences {
    pub fn new() -> Self {
        Self {
            id: PRIMITIVE_REFERENCE_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            unique_id_gen: UniqueIdGen::new(),
            spheres: AHashMap::<UniqueId, Weak<SphereRef>>::default(),
            cubes: AHashMap::<UniqueId, Weak<CubeRef>>::default(),
        }
    }

    pub fn new_sphere(&mut self, center: Vec3, radius: f32) -> Rc<SphereRef> {
        let primitive_id = self.unique_id_gen.new_id();
        let sphere = new_sphere_ref(Sphere::new(primitive_id, center, radius));
        self.spheres.insert(primitive_id, Rc::downgrade(&sphere));
        sphere
    }
    pub fn new_cube(&mut self, center: Vec3, dimensions: Vec3) -> Rc<CubeRef> {
        let primitive_id = self.unique_id_gen.new_id();
        let cube = new_cube_ref(Cube::new(primitive_id, center, dimensions));
        self.cubes.insert(primitive_id, Rc::downgrade(&cube));
        cube
    }

    pub fn get_sphere(&self, primitive_id: UniqueId) -> Option<Rc<SphereRef>> {
        get_primitive::<SphereRef>(primitive_id, &self.spheres)
    }
    pub fn get_cube(&self, primitive_id: UniqueId) -> Option<Rc<CubeRef>> {
        get_primitive::<CubeRef>(primitive_id, &self.cubes)
    }
}

fn get_primitive<T>(
    primitive_id: UniqueId,
    collection: &AHashMap<UniqueId, Weak<T>>,
) -> Option<Rc<T>> {
    if let Some(weak_ref) = collection.get(&primitive_id) {
        weak_ref.upgrade()
    } else {
        None
    }
}

static PRIMITIVE_REFERENCE_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);
