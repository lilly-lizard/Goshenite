use super::object::{new_object_ref, Object, ObjectId, ObjectRef};
use crate::{engine::primitives::primitive::PrimitiveRef, helper::unique_id_gen::UniqueIdGen};
use glam::Vec3;
use std::{collections::BTreeMap, rc::Rc};

/// Should only be one per engine instance.
pub struct ObjectCollection {
    unique_id_gen: UniqueIdGen,
    objects: BTreeMap<ObjectId, Rc<ObjectRef>>,
}

impl ObjectCollection {
    pub fn new() -> Self {
        Self {
            unique_id_gen: UniqueIdGen::new(),
            objects: Default::default(),
        }
    }

    pub fn new_object(
        &mut self,
        name: String,
        origin: Vec3,
        base_primitive: Rc<PrimitiveRef>,
    ) -> Rc<ObjectRef> {
        let object_id = self.unique_id_gen.new_id();
        let object = new_object_ref(Object::new(object_id, name, origin, base_primitive));
        self.objects.insert(object_id, object.clone());
        object
    }

    pub fn objects(&self) -> &BTreeMap<ObjectId, Rc<ObjectRef>> {
        &self.objects
    }

    pub fn get(&self, object_id: ObjectId) -> Option<&Rc<ObjectRef>> {
        self.objects.get(&object_id)
    }
}
