use super::object::{new_object_ref, Object, ObjectId, ObjectRef};
use crate::{
    engine::primitives::{primitive::PrimitiveRef, primitive_references::PrimitiveReferences},
    helper::unique_id_gen::UniqueIdGen,
};
use glam::Vec3;
use std::{collections::BTreeMap, rc::Rc};

/// Should only be one per engine instance.
pub struct ObjectCollection {
    unique_id_gen: UniqueIdGen,
    primitive_references: PrimitiveReferences,
    objects: BTreeMap<ObjectId, Rc<ObjectRef>>,
}

impl ObjectCollection {
    pub fn new() -> Self {
        Self {
            unique_id_gen: UniqueIdGen::new(),
            primitive_references: PrimitiveReferences::new(),
            objects: Default::default(),
        }
    }

    pub fn new_object(
        &mut self,
        name: String,
        origin: Vec3,
        base_primitive: Rc<PrimitiveRef>,
    ) -> Rc<ObjectRef> {
        let object_id = ObjectId(self.unique_id_gen.new_id());
        let object = new_object_ref(Object::new(object_id, name, origin, base_primitive));
        self.objects.insert(object_id, object.clone());
        object
    }

    pub fn new_empty_object(&mut self) -> Rc<ObjectRef> {
        let object_id = ObjectId(self.unique_id_gen.new_id());
        let object = new_object_ref(Object::new(
            object_id,
            format!("New Object {}", object_id.raw_id()),
            Vec3::ZERO,
            self.primitive_references.null_primitive(),
        ));

        self.objects.insert(object_id, object.clone());
        object
    }

    pub fn remove_object(&mut self, object_id: ObjectId) -> Option<Rc<ObjectRef>> {
        let removed_obeject = self.objects.remove(&object_id);
        // will probably have some 0-ref-count weak primitive ptrs now, so lets clean those up
        self.primitive_references.clean_unused_references();
        removed_obeject
    }

    pub fn primitive_references(&self) -> &PrimitiveReferences {
        &self.primitive_references
    }

    pub fn primitive_references_mut(&mut self) -> &mut PrimitiveReferences {
        &mut self.primitive_references
    }

    pub fn objects(&self) -> &BTreeMap<ObjectId, Rc<ObjectRef>> {
        &self.objects
    }

    pub fn get(&self, object_id: ObjectId) -> Option<&Rc<ObjectRef>> {
        self.objects.get(&object_id)
    }
}
