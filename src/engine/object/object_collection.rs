use super::object::{new_object_ref, Object, ObjectRef};
use crate::{
    engine::primitives::primitive::PrimitiveRef,
    helper::unique_id_gen::{UniqueId, UniqueIdGen},
};
use glam::Vec3;
use std::{
    collections::BTreeMap,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct ObjectsDelta {
    /// Reference to object collection that these object ids are from
    pub object_collection: Rc<ObjectCollection>,
    /// New or updated objects
    pub set: Vec<UniqueId>,
    /// Deleted objects
    pub free: Vec<UniqueId>,
}

pub struct ObjectCollection {
    id: UniqueId,
    unique_id_gen: UniqueIdGen,
    objects: BTreeMap<UniqueId, Rc<ObjectRef>>,
}

impl ObjectCollection {
    pub fn new() -> Self {
        Self {
            id: OBJECT_COLLECTION_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            unique_id_gen: UniqueIdGen::new(),
            objects: Default::default(),
        }
    }

    pub fn id(&self) -> UniqueId {
        self.id
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

    pub fn objects(&self) -> &BTreeMap<UniqueId, Rc<ObjectRef>> {
        &self.objects
    }

    pub fn get(&self, object_id: UniqueId) -> Option<&Rc<ObjectRef>> {
        self.objects.get(&object_id)
    }
}

static OBJECT_COLLECTION_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);
