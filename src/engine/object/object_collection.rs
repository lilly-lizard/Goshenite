use super::object::{new_object_ref, Object, ObjectRef};
use crate::{engine::primitives::primitive::PrimitiveRef, helper::unique_id_gen::UniqueIdGen};
use glam::Vec3;
use std::rc::Rc;

pub struct ObjectCollection {
    unique_id_gen: UniqueIdGen,
    objects: Vec<Rc<ObjectRef>>,
}

impl ObjectCollection {
    pub fn new() -> Self {
        Self {
            unique_id_gen: UniqueIdGen::new(),
            objects: Vec::default(),
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
        self.objects.push(object.clone());
        object
    }

    pub fn objects(&self) -> &Vec<Rc<ObjectRef>> {
        &self.objects
    }

    pub fn get(&self, index: usize) -> Option<&Rc<ObjectRef>> {
        self.objects.get(index)
    }

    pub fn push(&mut self, object: Object) {
        self.objects.push(new_object_ref(object));
    }

    pub fn remove(&mut self, index: usize) {
        self.objects.remove(index);
    }
}
