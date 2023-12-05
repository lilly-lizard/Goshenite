/// Describes the state of a value that can be modified and committed separately e.g. via a draggable slider
#[derive(Clone, Debug)]
pub enum CommitableValue<T>
where
    T: Clone + std::fmt::Debug,
{
    /// Value has changed but changes aren't to be commited
    NotCommitted(T),
    /// The user has finished modifying this value and wants changes to be commited
    Committed(T),
    /// No change to the modifiable value
    NoChange,
}
