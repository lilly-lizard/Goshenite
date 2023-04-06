use super::{
    cube::Cube,
    null_primitive::NullPrimitive,
    primitive::{default_dimensions, default_radius, PrimitiveCell, PrimitiveId},
    primitive_ref_types::{new_cube_ref, new_sphere_ref, CubeCell, PrimitiveRefType, SphereCell},
    sphere::Sphere,
};
use crate::helper::unique_id_gen::UniqueIdGen;
use ahash::AHashMap;
use glam::{Quat, Vec3};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

pub struct PrimitiveReferences {
    unique_id_gen: UniqueIdGen,
    null_primitive: Rc<RefCell<NullPrimitive>>,
    spheres: AHashMap<PrimitiveId, Weak<SphereCell>>,
    cubes: AHashMap<PrimitiveId, Weak<CubeCell>>,
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

    /// Removes any primitives that aren't being used anywhere.
    pub fn clean_unused_references(&mut self) {
        self.spheres
            .retain(|_sphere_id, sphere_ref| sphere_ref.strong_count() != 0);
    }

    // new primitive fns

    pub fn null_primitive(&self) -> Rc<RefCell<NullPrimitive>> {
        self.null_primitive.clone()
    }

    /// Creates a new default primitive of type `primitive_type`. If type `PrimitiveRefType::Unknown`
    /// requested, returns a `NullPrimitive`.
    pub fn create_primitive_default(
        &mut self,
        primitive_type: PrimitiveRefType,
    ) -> Rc<PrimitiveCell> {
        match primitive_type {
            PrimitiveRefType::Null => self.null_primitive(),
            PrimitiveRefType::Sphere => {
                self.create_sphere(Vec3::ZERO, Quat::IDENTITY, default_radius())
            }
            PrimitiveRefType::Cube => {
                self.create_cube(Vec3::ZERO, Quat::IDENTITY, default_dimensions())
            }
            PrimitiveRefType::Unknown => self.null_primitive(),
        }
    }

    pub fn create_sphere(&mut self, center: Vec3, rotation: Quat, radius: f32) -> Rc<SphereCell> {
        let primitive_id = PrimitiveId::from(
            self.unique_id_gen
                .new_id()
                .expect("todo should probably handle this..."),
        );

        let sphere = new_sphere_ref(Sphere::new(primitive_id, center, rotation, radius));
        self.spheres.insert(primitive_id, Rc::downgrade(&sphere));
        sphere
    }

    pub fn create_cube(&mut self, center: Vec3, rotation: Quat, dimensions: Vec3) -> Rc<CubeCell> {
        let primitive_id = PrimitiveId::from(
            self.unique_id_gen
                .new_id()
                .expect("todo should probably handle this..."),
        );

        let cube = new_cube_ref(Cube::new(primitive_id, center, rotation, dimensions));
        self.cubes.insert(primitive_id, Rc::downgrade(&cube));
        cube
    }

    // primitive access fns

    pub fn get_sphere(&self, primitive_id: PrimitiveId) -> Option<Rc<SphereCell>> {
        get_primitive::<SphereCell>(primitive_id, &self.spheres)
    }

    pub fn get_cube(&self, primitive_id: PrimitiveId) -> Option<Rc<CubeCell>> {
        get_primitive::<CubeCell>(primitive_id, &self.cubes)
    }

    /// Ensures at compile time that [`PrimitiveRefType`] cases match what [`PrimitiveReferences`] supports.
    #[allow(unused)]
    pub fn get(
        &self,
        primitive_type: PrimitiveRefType,
        primitive_id: PrimitiveId,
    ) -> Option<Rc<PrimitiveCell>> {
        match primitive_type {
            PrimitiveRefType::Null => Some(self.null_primitive() as Rc<PrimitiveCell>),
            PrimitiveRefType::Sphere => self
                .get_sphere(primitive_id)
                .map(|x| x as Rc<PrimitiveCell>),
            PrimitiveRefType::Cube => self.get_cube(primitive_id).map(|x| x as Rc<PrimitiveCell>),
            PrimitiveRefType::Unknown => None,
        }
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
