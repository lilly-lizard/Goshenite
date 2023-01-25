use super::{
    cube::Cube,
    primitive::Primitive,
    primitive_ref_types::{new_cube_ref, new_sphere_ref, CubeRef, PrimitiveRefType, SphereRef},
    sphere::Sphere,
};
use crate::helper::unique_id_gen::{UniqueId, UniqueIdGen};
use ahash::AHashMap;
use glam::Vec3;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

pub struct PrimitiveReferences {
    unique_id_gen: UniqueIdGen,
    pub spheres: AHashMap<UniqueId, Weak<SphereRef>>,
    pub cubes: AHashMap<UniqueId, Weak<CubeRef>>,
}

/// Should only be one per engine instance.
impl PrimitiveReferences {
    pub fn new() -> Self {
        Self {
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

    /// Ensures at compile time that [`PrimitiveRefType`] cases match what [`PrimitiveReferences`] supports.
    #[allow(unused)]
    pub fn get(
        &self,
        primitive_type: PrimitiveRefType,
        primitive_id: UniqueId,
    ) -> Option<Rc<RefCell<dyn Primitive>>> {
        match primitive_type {
            PrimitiveRefType::Sphere => self
                .get_sphere(primitive_id)
                .map(|x| x as Rc<RefCell<dyn Primitive>>),
            PrimitiveRefType::Cube => self
                .get_cube(primitive_id)
                .map(|x| x as Rc<RefCell<dyn Primitive>>),
            PrimitiveRefType::Unknown => None,
        }
    }
}

fn get_primitive<T>(
    primitive_id: UniqueId,
    collection: &AHashMap<UniqueId, Weak<T>>,
) -> Option<Rc<T>> {
    collection
        .get(&primitive_id)
        .map(|weak_ref| weak_ref.upgrade())
        .flatten()
}
