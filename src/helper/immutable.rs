// Shout out to danthedaniel for the code/idea https://stackoverflow.com/a/62948428/5256085
use std::ops::Deref;

/// A container for values that can only be deref'd immutably.
pub struct Immutable<T> {
    value: T,
}
impl<T> Immutable<T> {
    pub fn new(value: T) -> Self {
        Immutable { value }
    }
}
impl<T> Deref for Immutable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
