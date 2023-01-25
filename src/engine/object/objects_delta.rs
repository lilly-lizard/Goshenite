use crate::helper::unique_id_gen::UniqueId;
use ahash::HashSet;

/// Describes modifications to the [`ObjectCollection`].
pub struct ObjectsDelta {
    /// IDs of new or updated objects
    pub update: HashSet<UniqueId>,
    /// IDs of deleted objects
    pub remove: HashSet<UniqueId>,
}
impl Default for ObjectsDelta {
    fn default() -> Self {
        Self {
            update: Default::default(),
            remove: Default::default(),
        }
    }
}
