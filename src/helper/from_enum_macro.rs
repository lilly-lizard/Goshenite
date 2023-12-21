/// Shorthand for writing a `From` impl to convert from a nested enum type.
///
/// Arguments:
/// 1. the name of your enum.
/// 2. the nested type, which should have the same name as its enum variant.
///
/// For example this:
/// ```
/// impl_from_for_enum_variant!(Enum, Variant);
/// ```
/// Will expand to this:
/// ```
/// impl From<Variant> for Enum {
///     #[inline]
///     fn from(value: Variant) -> Self {
///         Self::Variant(value)
///     }
/// }
/// ```
macro_rules! impl_from_for_enum_variant {
    ($enum:ident, $variant:ident) => {
        impl From<$variant> for $enum {
            #[inline]
            fn from(value: $variant) -> Self {
                Self::$variant(value)
            }
        }
    };
}
pub(crate) use impl_from_for_enum_variant;
