use crate::renderer::shaders::object_buffer::PrimitiveDataSlice;
use glam::Vec3;

/// A primitive is a basic geometric building block that can be manipulated and combined
/// using [`Operation`]s
pub trait Primitive {
    /// Unique id that can be passed to `PrimitiveReferences` to lookup the actual struct
    fn id(&self) -> usize;
    /// Returns buffer compatible primitive data as a [`PrimitiveDataSlice`].
    /// `parent_origin` is the world space origin of the parent object, which should be added to
    /// the primitive center before encoding.
    ///
    /// _Note: must match the decode process in `scene_geometry.frag`_
    fn encode(&self, parent_origin: Vec3) -> PrimitiveDataSlice;
    /// Returns the center of mass of the primitive, relative to the center of the parent object.
    fn center(&self) -> Vec3;
    /// Returns the primitive type as a str
    fn type_name(&self) -> &'static str;
}
