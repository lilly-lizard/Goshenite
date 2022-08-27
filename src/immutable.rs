use std::ops::Deref;

/// A container for values that can only be deref'd immutably.
/// Shout out to danthedaniel https://stackoverflow.com/a/62948428/5256085
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
