use super::object::{Object, ObjectId};
use ahash::HashSet;

/// Describes modifications to the [`ObjectCollection`].
pub struct ObjectsDelta {
    /// IDs of new or updated objects
    pub update: HashSet<Object>,
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
