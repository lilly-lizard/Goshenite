/// Shorthand for writing a `From` impl to convert from a nested error type into your error enum.
///
/// Arguments:
/// 1. the name of your error enum.
/// 2. the nested error type, which should have the same name as its enum variant.
///
/// Will expand to something like this:
/// ```
/// impl From<NestedError> for ParentError {
///     fn from(e: NestedError) -> Self {
///         Self::NestedError(e)
///     }
/// }
/// ```
macro_rules! from_err_impl {
    ($en:ty, $er:ident) => {
        impl From<$er> for $en {
            fn from(e: $er) -> Self {
                Self::$er(e)
            }
        }
    };
}
pub(crate) use from_err_impl;
