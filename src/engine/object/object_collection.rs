use super::{
    object::{Object, ObjectId},
    objects_delta::{ObjectDeltaOperation, ObjectsDelta},
};
use crate::{
    engine::config_engine::DEFAULT_ORIGIN,
    helper::{
        more_errors::CollectionError,
        unique_id_gen::{UniqueIdError, UniqueIdGen, UniqueIdType},
    },
};
use glam::Vec3;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::collections::BTreeMap;

/// Should only be one per engine instance.
pub struct ObjectCollection {
    unique_id_gen: UniqueIdGen<ObjectId>,
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

    pub fn new_object(
        &mut self,
        name: impl Into<String>,
        origin: Vec3,
    ) -> Result<(ObjectId, Object), UniqueIdError> {
        let object_id = self.unique_id_gen.new_id()?;
        Ok(self.new_object_internal(object_id, name.into(), origin))
    }

    pub fn new_object_default(&mut self) -> Result<(ObjectId, Object), UniqueIdError> {
        let object_id = self.unique_id_gen.new_id()?;
        let name = format!("New Object {}", object_id.raw_id());
        let origin = DEFAULT_ORIGIN;
        Ok(self.new_object_internal(object_id, name, origin))
    }

    pub fn insert_object(&mut self, new_object: Object) -> Result<ObjectId, UniqueIdError> {
        let new_object_id = self.unique_id_gen.new_id()?;
        self.objects.insert(new_object_id, new_object);
        self.mark_object_for_data_update(new_object_id)
            .expect("new object just created");
        Ok(new_object_id)
    }

    pub fn insert_objects(
        &mut self,
        new_objects: impl IntoIterator<Item = Object>,
    ) -> Result<Vec<ObjectId>, UniqueIdError> {
        let mut new_object_ids: Vec<ObjectId> = Vec::new();
        for new_object in new_objects {
            let new_object_id = self.insert_object(new_object)?;
            new_object_ids.push(new_object_id);
        }
        Ok(new_object_ids)
    }

    pub fn set_object(
        &mut self,
        object_id: ObjectId,
        updated_object: Object,
    ) -> Result<(), CollectionError> {
        let mut object_mut_ref = self.get_object_mut(object_id)?;
        *object_mut_ref = updated_object;
        _ = self.mark_object_for_data_update(object_id);
        Ok(())
    }

    pub fn set_object_name(
        &mut self,
        object_id: ObjectId,
        updated_name: String,
    ) -> Result<(), CollectionError> {
        let mut object_mut_ref = self.get_object_mut(object_id)?;
        object_mut_ref.name = updated_name;
        // don't need to mark for update becuase name isn't sent to gpu
        Ok(())
    }

    pub fn remove_object(&mut self, object_id: ObjectId) -> Result<Object, CollectionError> {
        let removed_object_option = self.objects.remove(&object_id);

        if let Some(removed_object) = removed_object_option {
            // tell object id generator it can reuse the old object id now
            if let Err(e) = self.unique_id_gen.recycle_id(object_id) {
                info!("{}", e);
            }

            // record changed data to update the gpu
            self.insert_object_delta(object_id, ObjectDeltaOperation::Remove);

            return Ok(removed_object);
        } else {
            return Err(CollectionError::InvalidId {
                raw_id: object_id.raw_id(),
            });
        }
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
}

// ~~ Private Functions ~~

impl ObjectCollection {
    /// Call this whenever an object is modified via [`get_object_mut`] so that the updated data
    /// can be sent to the GPU.
    fn mark_object_for_data_update(&mut self, object_id: ObjectId) -> Result<(), CollectionError> {
        let cloned_object = if let Some(updated_object) = self.objects.get(&object_id) {
            updated_object.clone()
        } else {
            return Err(CollectionError::InvalidId {
                raw_id: object_id.raw_id(),
            });
        };

        self.insert_object_delta(object_id, ObjectDeltaOperation::Update(cloned_object));
        Ok(())
    }

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
    ) -> (ObjectId, Object) {
        let object = Object::new(name, origin);
        self.objects.insert(object_id, object.clone());

        // record changed data
        self.insert_object_delta(object_id, ObjectDeltaOperation::Add(object.clone()));

        (object_id, object)
    }

    fn get_object_mut(&mut self, object_id: ObjectId) -> Result<&mut Object, CollectionError> {
        self.objects
            .get_mut(&object_id)
            .ok_or(CollectionError::InvalidId {
                raw_id: object_id.raw_id(),
            })
    }
}
