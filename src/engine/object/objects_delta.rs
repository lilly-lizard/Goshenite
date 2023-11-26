use super::object::{ObjectId, ObjectSnapshot};
use ahash::HashMap;

/// Describes modifications to the [`ObjectCollection`].
pub type ObjectsDelta = HashMap<ObjectId, ObjectDeltaOperation>;

#[derive(Clone)]
pub enum ObjectDeltaOperation {
    /// New object is being added
    Add(ObjectSnapshot),
    /// Object has had data changed
    Update(ObjectSnapshot),
    /// Object is being deleted
    Remove,
}
