use super::object::{Object, ObjectId};
use ahash::HashMap;

/// Describes modifications to the [`ObjectCollection`].
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
