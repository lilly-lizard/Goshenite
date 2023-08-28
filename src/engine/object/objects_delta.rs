use super::object::{ObjectDuplicate, ObjectId};
use ahash::HashMap;

/// Describes modifications to the [`ObjectCollection`].
pub type ObjectsDelta = HashMap<ObjectId, ObjectDeltaOperation>;

#[derive(Clone)]
pub enum ObjectDeltaOperation {
    /// New object is being added
    Add(ObjectDuplicate),
    /// Object has had data changed
    Update(ObjectDuplicate),
    /// Object is being deleted
    Remove,
}
