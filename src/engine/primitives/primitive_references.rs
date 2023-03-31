use super::{
    cube::Cube,
    null_primitive::NullPrimitive,
    primitive::{default_center, default_dimensions, default_radius, PrimitiveId, PrimitiveRef},
    primitive_ref_types::{new_cube_ref, new_sphere_ref, CubeRef, PrimitiveRefType, SphereRef},
    sphere::Sphere,
};
use crate::helper::unique_id_gen::UniqueIdGen;
use ahash::AHashMap;
use glam::Vec3;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

pub struct PrimitiveReferences {
    unique_id_gen: UniqueIdGen,
    null_primitive: Rc<RefCell<NullPrimitive>>,
    spheres: AHashMap<PrimitiveId, Weak<SphereRef>>,
    cubes: AHashMap<PrimitiveId, Weak<CubeRef>>,
}

/// Should only be one per engine instance.
impl PrimitiveReferences {
    pub fn new() -> Self {
        Self {
            unique_id_gen: UniqueIdGen::new(),
            null_primitive: NullPrimitive::new_ref(),
            spheres: Default::default(),
            cubes: Default::default(),
        }
    }

    pub fn null_primitive(&self) -> Rc<RefCell<NullPrimitive>> {
        self.null_primitive.clone()
    }

    /// Creates a new default primitive of type `primitive_type`. If type `PrimitiveRefType::Unknown`
    /// requested, returns a `NullPrimitive`.
    pub fn new_default(&mut self, primitive_type: PrimitiveRefType) -> Rc<PrimitiveRef> {
        match primitive_type {
            PrimitiveRefType::Null => self.null_primitive(),
            PrimitiveRefType::Sphere => self.new_sphere(default_center(), default_radius()),
            PrimitiveRefType::Cube => self.new_cube(default_center(), default_dimensions()),
            PrimitiveRefType::Unknown => self.null_primitive(),
        }
    }

    pub fn new_sphere(&mut self, center: Vec3, radius: f32) -> Rc<SphereRef> {
        let primitive_id = PrimitiveId::from(
            self.unique_id_gen
                .new_id()
                .expect("todo should probably handle this..."),
        );

        let sphere = new_sphere_ref(Sphere::new(primitive_id, center, radius));
        self.spheres.insert(primitive_id, Rc::downgrade(&sphere));
        sphere
    }
    pub fn new_cube(&mut self, center: Vec3, dimensions: Vec3) -> Rc<CubeRef> {
        let primitive_id = PrimitiveId::from(
            self.unique_id_gen
                .new_id()
                .expect("todo should probably handle this..."),
        );

        let cube = new_cube_ref(Cube::new(primitive_id, center, dimensions));
        self.cubes.insert(primitive_id, Rc::downgrade(&cube));
        cube
    }

    pub fn get_sphere(&self, primitive_id: PrimitiveId) -> Option<Rc<SphereRef>> {
        get_primitive::<SphereRef>(primitive_id, &self.spheres)
    }
    pub fn get_cube(&self, primitive_id: PrimitiveId) -> Option<Rc<CubeRef>> {
        get_primitive::<CubeRef>(primitive_id, &self.cubes)
    }

    /// Ensures at compile time that [`PrimitiveRefType`] cases match what [`PrimitiveReferences`] supports.
    #[allow(unused)]
    pub fn get(
        &self,
        primitive_type: PrimitiveRefType,
        primitive_id: PrimitiveId,
    ) -> Option<Rc<PrimitiveRef>> {
        match primitive_type {
            PrimitiveRefType::Null => Some(self.null_primitive() as Rc<PrimitiveRef>),
            PrimitiveRefType::Sphere => {
                self.get_sphere(primitive_id).map(|x| x as Rc<PrimitiveRef>)
            }
            PrimitiveRefType::Cube => self.get_cube(primitive_id).map(|x| x as Rc<PrimitiveRef>),
            PrimitiveRefType::Unknown => None,
        }
    }

    /// Removes any primitives that aren't being used anywhere.
    pub fn clean_unused_references(&mut self) {
        self.spheres
            .retain(|_sphere_id, sphere_ref| sphere_ref.strong_count() != 0);
    }
}

fn get_primitive<T>(
    primitive_id: PrimitiveId,
    collection: &AHashMap<PrimitiveId, Weak<T>>,
) -> Option<Rc<T>> {
    collection
        .get(&primitive_id)
        .map(|weak_ref| weak_ref.upgrade())
        .flatten()
}
