use super::object::{Object, ObjectId};
use crate::helper::{more_errors::CollectionError, unique_id_gen::UniqueIdGen};
use glam::Vec3;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::collections::BTreeMap;

/// Should only be one per engine instance.
pub struct ObjectCollection {
    unique_id_gen: UniqueIdGen,
    objects: BTreeMap<ObjectId, Object>,
}

impl ObjectCollection {
    pub fn new() -> Self {
        Self {
            unique_id_gen: UniqueIdGen::new(),
            objects: Default::default(),
        }
    }

    pub fn new_object(&mut self, name: String, origin: Vec3) -> (ObjectId, &mut Object) {
        let new_raw_id = self
            .unique_id_gen
            .new_id()
            .expect("todo should probably handle this somehow...");
        let object_id = ObjectId(new_raw_id);

        let object = Object::new(object_id, name, origin);
        self.objects.insert(object_id, object);

        let object_ref = self
            .objects
            .get_mut(&object_id)
            .expect("literally just inserted this");
        (object_id, object_ref)
    }

    pub fn new_object_default(&mut self) -> ObjectId {
        let new_raw_id = self
            .unique_id_gen
            .new_id()
            .expect("todo should probably handle this somehow...");
        let object_id = ObjectId(new_raw_id);

        let object = Object::new(
            object_id,
            format!("New Object {}", object_id.raw_id()),
            Vec3::ZERO,
        );

        self.objects.insert(object_id, object);
        object_id
    }

    pub fn remove_object(&mut self, object_id: ObjectId) -> Result<Object, CollectionError> {
        let removed_object_option = self.objects.remove(&object_id);

        if let Some(removed_object) = removed_object_option {
            // tell object id generator it can reuse the old object id now
            if let Err(e) = self.unique_id_gen.recycle_id(object_id.raw_id()) {
                warn!("{}", e); // todo should probably handle this somehow...
            }

            return Ok(removed_object);
        } else {
            return Err(CollectionError::InvalidId {
                raw_id: object_id.raw_id(),
            });
        }
    }

    pub fn objects(&self) -> &BTreeMap<ObjectId, Object> {
        &self.objects
    }

    pub fn get_object(&self, object_id: ObjectId) -> Option<&Object> {
        self.objects.get(&object_id)
    }

    pub fn get_object_mut(&mut self, object_id: ObjectId) -> Option<&mut Object> {
        self.objects.get_mut(&object_id)
    }
}
