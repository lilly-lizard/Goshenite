use super::{
    object::{Object, ObjectId},
    objects_delta::{ObjectDeltaOperation, ObjectsDelta},
};
use crate::helper::{more_errors::CollectionError, unique_id_gen::UniqueIdGen};
use glam::Vec3;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::collections::BTreeMap;

pub const DEFAULT_ORIGIN: Vec3 = Vec3::ZERO;

/// Should only be one per engine instance.
pub struct ObjectCollection {
    unique_id_gen: UniqueIdGen,
    objects: BTreeMap<ObjectId, Object>,
    objects_delta_accumulation: ObjectsDelta,
}

impl ObjectCollection {
    pub fn new() -> Self {
        Self {
            unique_id_gen: UniqueIdGen::new(),
            objects: Default::default(),
            objects_delta_accumulation: Default::default(),
        }
    }

    pub fn new_object(&mut self, name: String, origin: Vec3) -> (ObjectId, &mut Object) {
        let new_raw_id = self
            .unique_id_gen
            .new_id()
            .expect("todo should probably handle this somehow...");
        let object_id = ObjectId(new_raw_id);

        self.new_object_internal(object_id, name, origin)
    }

    pub fn new_object_default(&mut self) -> (ObjectId, &mut Object) {
        let new_raw_id = self
            .unique_id_gen
            .new_id()
            .expect("todo should probably handle this somehow...");
        let object_id = ObjectId(new_raw_id);

        let name = format!("New Object {}", object_id.raw_id());
        let origin = DEFAULT_ORIGIN;

        self.new_object_internal(object_id, name, origin)
    }

    pub fn remove_object(&mut self, object_id: ObjectId) -> Result<Object, CollectionError> {
        let removed_object_option = self.objects.remove(&object_id);

        if let Some(removed_object) = removed_object_option {
            // tell object id generator it can reuse the old object id now
            if let Err(e) = self.unique_id_gen.recycle_id(object_id.raw_id()) {
                warn!("{}", e); // todo should probably handle this somehow...
            }

            // record changed data
            self.insert_object_delta(object_id, ObjectDeltaOperation::Remove);

            return Ok(removed_object);
        } else {
            return Err(CollectionError::InvalidId {
                raw_id: object_id.raw_id(),
            });
        }
    }

    /// Call this whenever an object is modified via [`get_object_mut`] so that the updated data
    /// can be sent to the GPU.
    pub fn mark_object_for_data_update(
        &mut self,
        object_id: ObjectId,
    ) -> Result<(), CollectionError> {
        let object_duplicate = if let Some(updated_object) = self.objects.get(&object_id) {
            updated_object.duplicate()
        } else {
            return Err(CollectionError::InvalidId {
                raw_id: object_id.raw_id(),
            });
        };

        self.insert_object_delta(object_id, ObjectDeltaOperation::Update(object_duplicate));

        Ok(())
    }

    /// Returns a description of the changes to objects since last call to this function.
    pub fn get_and_clear_objects_delta(&mut self) -> ObjectsDelta {
        std::mem::take(&mut self.objects_delta_accumulation)
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

// private functions

impl ObjectCollection {
    /// Use this instead of directly inserting to perform some operation specific checks
    fn insert_object_delta(
        &mut self,
        object_id: ObjectId,
        object_delta_operation: ObjectDeltaOperation,
    ) {
        if let ObjectDeltaOperation::Update(new_object_duplicate) = object_delta_operation.clone() {
            if let Some(ObjectDeltaOperation::Add(_old_object_duplicate)) =
                self.objects_delta_accumulation.get(&object_id)
            {
                // object was previously queued for add, so we still need to treat this as an add, not an update
                self.objects_delta_accumulation
                    .insert(object_id, ObjectDeltaOperation::Add(new_object_duplicate));
                return;
            }
        }

        self.objects_delta_accumulation
            .insert(object_id, object_delta_operation);
    }

    fn new_object_internal(
        &mut self,
        object_id: ObjectId,
        name: String,
        origin: Vec3,
    ) -> (ObjectId, &mut Object) {
        let object = Object::new(object_id, name, origin);
        let object_duplicate = object.duplicate();
        self.objects.insert(object_id, object);

        // record changed data
        self.insert_object_delta(object_id, ObjectDeltaOperation::Add(object_duplicate));

        let object_ref = self
            .objects
            .get_mut(&object_id)
            .expect("literally just inserted this");
        (object_id, object_ref)
    }
}
