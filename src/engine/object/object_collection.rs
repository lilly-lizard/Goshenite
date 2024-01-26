use super::{
    object::{Object, ObjectId},
    objects_delta::{push_object_delta, ObjectDeltaOperation, ObjectsDelta},
    operation::Operation,
    primitive_op::PrimitiveOpId,
};
use crate::{
    engine::{
        config_engine::DEFAULT_ORIGIN,
        primitives::{primitive::Primitive, primitive_transform::PrimitiveTransform},
    },
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

    pub fn push_object(&mut self, new_object: Object) -> Result<ObjectId, UniqueIdError> {
        let new_object_id = self.unique_id_gen.new_id()?;
        self.objects.insert(new_object_id, new_object);
        self.mark_object_for_gpu_update(new_object_id)
            .expect("new object just created");
        Ok(new_object_id)
    }

    pub fn push_objects(
        &mut self,
        new_objects: impl IntoIterator<Item = Object>,
    ) -> Result<Vec<ObjectId>, UniqueIdError> {
        let mut new_object_ids: Vec<ObjectId> = Vec::new();
        for new_object in new_objects {
            let new_object_id = self.push_object(new_object)?;
            new_object_ids.push(new_object_id);
        }
        Ok(new_object_ids)
    }

    pub fn set_object(
        &mut self,
        object_id: ObjectId,
        new_object: Object,
    ) -> Result<(), CollectionError> {
        let object_mut_ref = self.get_object_mut(object_id)?;
        *object_mut_ref = new_object;
        self.mark_object_for_gpu_update(object_id)
    }

    pub fn set_object_name(
        &mut self,
        object_id: ObjectId,
        new_name: String,
    ) -> Result<(), CollectionError> {
        let object_mut_ref = self.get_object_mut(object_id)?;
        object_mut_ref.name = new_name;
        // don't need to mark for update becuase the name isn't sent to gpu
        Ok(())
    }

    pub fn set_object_origin(
        &mut self,
        object_id: ObjectId,
        new_origin: Vec3,
    ) -> Result<(), CollectionError> {
        let object_mut_ref = self.get_object_mut(object_id)?;
        object_mut_ref.origin = new_origin;
        self.mark_object_for_gpu_update(object_id)
    }

    pub fn push_op_to_object(
        &mut self,
        object_id: ObjectId,
        primitive: Primitive,
        transform: PrimitiveTransform,
        op: Operation,
        blend: f32,
        albedo: Vec3,
        specular: f32,
    ) -> Result<PrimitiveOpId, CollectionError> {
        let object_mut_ref = self.get_object_mut(object_id)?;
        let primitive_op_id =
            object_mut_ref.push_primitive_op(primitive, transform, op, blend, albedo, specular)?;
        _ = self.mark_object_for_gpu_update(object_id);
        Ok(primitive_op_id)
    }

    pub fn set_primitive_op_id_in_object(
        &mut self,
        object_id: ObjectId,
        primitive_op_id: PrimitiveOpId,
        new_primitive: Option<Primitive>,
        new_transform: Option<PrimitiveTransform>,
        new_operation: Option<Operation>,
        new_blend: Option<f32>,
        new_albedo: Option<Vec3>,
        new_specular: Option<f32>,
    ) -> Result<(), CollectionError> {
        let object_mut_ref = self.get_object_mut(object_id)?;
        object_mut_ref.set_primitive_op_id(
            primitive_op_id,
            new_primitive,
            new_transform,
            new_operation,
            new_blend,
            new_albedo,
            new_specular,
        )?;
        self.mark_object_for_gpu_update(object_id)
    }

    pub fn set_primitive_op_index_in_object(
        &mut self,
        object_id: ObjectId,
        primitive_op_index: usize,
        new_primitive: Option<Primitive>,
        new_transform: Option<PrimitiveTransform>,
        new_operation: Option<Operation>,
        new_blend: Option<f32>,
        new_albedo: Option<Vec3>,
        new_specular: Option<f32>,
    ) -> Result<(), CollectionError> {
        let object_mut_ref = self.get_object_mut(object_id)?;
        object_mut_ref.set_primitive_op_index(
            primitive_op_index,
            new_primitive,
            new_transform,
            new_operation,
            new_blend,
            new_albedo,
            new_specular,
        )?;
        self.mark_object_for_gpu_update(object_id)
    }

    pub fn shift_primitive_ops_in_object(
        &mut self,
        object_id: ObjectId,
        source_index: usize,
        target_index: usize,
    ) -> Result<(), CollectionError> {
        let object_mut_ref = self.get_object_mut(object_id)?;
        object_mut_ref.shift_primitive_ops(source_index, target_index)?;
        self.mark_object_for_gpu_update(object_id)
    }

    pub fn remove_primitive_op_id_from_object(
        &mut self,
        object_id: ObjectId,
        remove_primitive_op_id: PrimitiveOpId,
    ) -> Result<usize, CollectionError> {
        let object_mut_ref = self.get_object_mut(object_id)?;
        let index = object_mut_ref.remove_primitive_op_id(remove_primitive_op_id)?;
        _ = self.mark_object_for_gpu_update(object_id);
        Ok(index)
    }

    pub fn remove_primitive_op_index_from_object(
        &mut self,
        object_id: ObjectId,
        remove_primitive_op_index: usize,
    ) -> Result<PrimitiveOpId, CollectionError> {
        let object_mut_ref = self.get_object_mut(object_id)?;
        let id = object_mut_ref.remove_primitive_op_index(remove_primitive_op_index)?;
        _ = self.mark_object_for_gpu_update(object_id);
        Ok(id)
    }

    pub fn remove_object(&mut self, object_id: ObjectId) -> Result<Object, CollectionError> {
        let removed_object_option = self.objects.remove(&object_id);

        if let Some(removed_object) = removed_object_option {
            // tell object id generator it can reuse the old object id now
            if let Err(e) = self.unique_id_gen.recycle_id(object_id) {
                info!("{}", e);
            }

            // record changed data to update the gpu
            self.push_object_delta(object_id, ObjectDeltaOperation::Remove);

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

    /// Marks all objects for gpu update, regardless of wherever they've been modified since the
    /// last upload. Useful for debugging.
    pub fn force_gpu_update(&mut self) {
        for (object_id, object) in self.objects.clone() {
            self.push_object_delta(object_id, ObjectDeltaOperation::Update(object));
        }
    }
}

// ~~ Private Functions ~~

impl ObjectCollection {
    /// Call this whenever an object is modified via [`get_object_mut`] so that the updated data
    /// can be sent to the GPU.
    fn mark_object_for_gpu_update(&mut self, object_id: ObjectId) -> Result<(), CollectionError> {
        let Some(updated_object) = self.objects.get(&object_id) else {
            return Err(CollectionError::InvalidId {
                raw_id: object_id.raw_id(),
            });
        };
        self.push_object_delta(
            object_id,
            ObjectDeltaOperation::Update(updated_object.clone()),
        );
        Ok(())
    }

    /// Use this instead of directly inserting to perform conflict checks.
    ///
    /// Note: the reason multiple deltas for the same object are merged is so that the renderer doesn't
    /// have to do any unnecessary gpu buffer uploads.
    #[inline]
    fn push_object_delta(&mut self, object_id: ObjectId, new_object_delta: ObjectDeltaOperation) {
        push_object_delta(
            &mut self.objects_delta_accumulation,
            object_id,
            new_object_delta,
        )
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
        self.push_object_delta(object_id, ObjectDeltaOperation::Add(object.clone()));

        (object_id, object)
    }

    /// Don't want this to be public because any updates to objects should be followed by a call to
    /// `mark_object_for_gpu_update` which is hard to maintain and thus should be the
    /// responsibility of `ObjectCollection`.
    fn get_object_mut(&mut self, object_id: ObjectId) -> Result<&mut Object, CollectionError> {
        self.objects
            .get_mut(&object_id)
            .ok_or(CollectionError::InvalidId {
                raw_id: object_id.raw_id(),
            })
    }
}
