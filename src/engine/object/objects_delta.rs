use super::object::{Object, ObjectId};
use ahash::HashMap;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

/// Describes modifications to the [`ObjectCollection`].
/// This is a hash map instead of a vec because multiple delta operations can be merged into a single
/// one so that the renderer doesn't have to do any unnecessary gpu buffer uploads.
pub type ObjectsDelta = HashMap<ObjectId, ObjectDeltaOperation>;

#[derive(Clone)]
pub enum ObjectDeltaOperation {
    /// New object is being added
    Add(Object),
    /// Object has had data changed
    Update(Object),
    /// Object is being deleted
    Remove,
}

/// Use this instead of directly inserting to perform conflict checks.
///
/// Note: the reason multiple deltas for the same object are merged is so that the renderer doesn't
/// have to do any unnecessary gpu buffer uploads.
pub fn push_object_delta(
    objects_delta: &mut ObjectsDelta,
    object_id: ObjectId,
    new_object_delta: ObjectDeltaOperation,
) {
    // need to check for conflicts if there is an existing delta pending for this object id
    if objects_delta.get(&object_id).is_some() {
        merge_object_delta_operations(objects_delta, object_id, new_object_delta);
        return;
    }

    // otherwise just insert the delta
    objects_delta.insert(object_id, new_object_delta);
}

/// Merges object delta operations when there is already an existing one are for the same object id
///
/// ### Table for handling delta for same object id:
/// ```
///                       existing
///             |  add  | update | remove
///     --------+-------+--------+--------
///       add   | skip  |  bug   | update
///     --------+-------+--------+--------
/// new  update |  add  |   ow   | skip
///     --------+-------+--------+--------
///      remove | cancel|   ow   | skip
/// ```
/// * `cancel` = cancel each other out
/// * `ow` = overwrite
pub fn merge_object_delta_operations(
    objects_delta: &mut ObjectsDelta,
    object_id: ObjectId,
    new_object_delta: ObjectDeltaOperation,
) {
    let Some(existing_delta) = objects_delta.get(&object_id) else {
        // no merging to be done here
        return;
    };

    match existing_delta {
        // existing delta == add
        ObjectDeltaOperation::Add(_old_object) => {
            match &new_object_delta {
                // add already queued
                ObjectDeltaOperation::Add(_new_object) => (),
                ObjectDeltaOperation::Update(new_object) => {
                    // object is still queued for add, so should still treat this as an add
                    objects_delta.insert(object_id, ObjectDeltaOperation::Add(new_object.clone()));
                    ()
                }
                ObjectDeltaOperation::Remove => {
                    // if an object id is added and then removed it's the same as nothing happening
                    objects_delta.remove(&object_id);
                    ()
                }
            }
        }
        // existing delta == update
        ObjectDeltaOperation::Update(_old_object) => {
            match &new_object_delta {
                ObjectDeltaOperation::Add(_new_object) =>
                    warn!("push_object_delta: attempted to insert add operation on an object id that already has an update queued??? please report as a bug..."),
                // old update is overwritten
                ObjectDeltaOperation::Update(_new_object) => {
                    objects_delta
                        .insert(object_id, new_object_delta);
                    ()
                }
                // old update is overwritten
                ObjectDeltaOperation::Remove => {
                    objects_delta
                        .insert(object_id, new_object_delta);
                    ()
                }
            }
        }
        // existing delta == remove
        ObjectDeltaOperation::Remove => {
            match &new_object_delta {
                ObjectDeltaOperation::Add(new_object) => {
                    // an object id being removed and then added is the same as it being replaced
                    objects_delta
                        .insert(object_id, ObjectDeltaOperation::Update(new_object.clone()));
                    ()
                }
                // shouldn't update an object id that is queued to be removed
                ObjectDeltaOperation::Update(_new_object) => (),
                // remove already queued
                ObjectDeltaOperation::Remove => (),
            }
        }
    }
}
