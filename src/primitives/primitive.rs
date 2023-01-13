use crate::shaders::primitive_buffer::PrimitiveDataSlice;
use glam::Vec3;

/// A primitive is a basic geometric building block that can be manipulated and combined
/// using [`Operation`]s
pub trait PrimitiveTrait: Default + PartialEq + Clone {
    /// Returns buffer compatible primitive data as a [`PrimitiveDataSlice`].
    ///
    /// _Note: must match the decode process in `scene.comp`_
    fn encode(&self) -> PrimitiveDataSlice;
    /// Returns the spacial center of the primitive.
    fn center(&self) -> Vec3;
    /// Returns the primitive type as a str
    fn type_name(&self) -> &'static str;
}
