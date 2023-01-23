use super::object::{new_object_ref, Object, ObjectRef};
use crate::{
    engine::primitives::primitive::PrimitiveRef,
    helper::unique_id_gen::{UniqueId, UniqueIdGen},
};
use glam::Vec3;
use std::{collections::BTreeMap, rc::Rc};

pub struct ObjectCollection {
    unique_id_gen: UniqueIdGen,
    objects: BTreeMap<UniqueId, Rc<ObjectRef>>,
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
        let id = self.unique_id_gen.new_id();
        let object = new_object_ref(Object::new(id, name, origin, base_primitive));
        self.objects.insert(id, object.clone());
        object
    }

    pub fn objects(&self) -> &BTreeMap<UniqueId, Rc<ObjectRef>> {
        &self.objects
    }

    pub fn get(&self, id: UniqueId) -> Option<&Rc<ObjectRef>> {
        self.objects.get(&id)
    }
}
