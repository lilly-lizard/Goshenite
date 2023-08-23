use super::object::{ObjectDuplicate, ObjectId};
use ahash::HashSet;

/// Describes modifications to the [`ObjectCollection`].
pub struct ObjectsDelta {
    /// IDs of new or updated objects. Note that hashing is performed on the id of the object.
    pub update: HashSet<ObjectDuplicate>,
    /// IDs of deleted objects
    pub remove: HashSet<ObjectId>,
}

impl Default for ObjectsDelta {
    fn default() -> Self {
        Self {
            update: Default::default(),
            remove: Default::default(),
        }
    }
}
