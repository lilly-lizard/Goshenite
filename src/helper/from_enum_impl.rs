/// Shorthand for writing a `From` impl to convert from a nested enum type.
///
/// Arguments:
/// 1. the name of your enum.
/// 2. the nested type, which should have the same name as its enum variant.
///
/// Will expand to something like this:
/// ```
/// impl From<NestedType> for ParentEnum {
///     #[inline]
///     fn from(x: NestedType) -> Self {
///         Self::NestedType(x)
///     }
/// }
/// ```
macro_rules! from_enum_impl {
    ($en:ty, $ty:ident) => {
        impl From<$ty> for $en {
            #[inline]
            fn from(x: $ty) -> Self {
                Self::$ty(x)
            }
        }
    };
}
pub(crate) use from_enum_impl;
