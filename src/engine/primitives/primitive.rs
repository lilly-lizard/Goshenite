use super::primitive_transform::PrimitiveTransform;
use crate::{
    engine::aabb::Aabb,
    helper::unique_id_gen::UniqueId,
    renderer::shader_interfaces::primitive_op_buffer::{
        PrimitiveOpBufferUnit, PrimitivePropsSlice,
    },
};
use glam::Vec3;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrimitiveId(pub UniqueId);
impl PrimitiveId {
    pub const fn raw_id(&self) -> UniqueId {
        self.0
    }
}
impl From<UniqueId> for PrimitiveId {
    fn from(id: UniqueId) -> Self {
        Self(id)
    }
}
impl std::fmt::Display for PrimitiveId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw_id())
    }
}

/// A primitive is a basic geometric building block that can be manipulated and combined
/// using [`Operation`]s in an [`Object`]
pub trait Primitive: Send + Sync {
    /// Unique id that can be passed to `PrimitiveReferences` to lookup the actual struct
    fn id(&self) -> PrimitiveId;

    /// Returns the primitive type code. See [`primitive_type_codes`].
    fn type_code(&self) -> PrimitiveOpBufferUnit;

    /// Returns the primitive type as a str
    fn type_name(&self) -> &'static str;

    /// Returns buffer compatible primitive data as a [`PrimitivePropsSlice`].
    /// `parent_origin` is the world space origin of the parent object, which should be added to
    /// the primitive center before encoding.
    ///
    /// _Note: must match the decode process in `scene_geometry.frag`_
    fn encoded_props(&self) -> PrimitivePropsSlice;

    /// Returns a reference to the primitive tranform of this instance
    fn transform(&self) -> &PrimitiveTransform;

    /// Axis aligned bounding box
    fn aabb(&self) -> Aabb;
}

#[inline]
pub const fn default_radius() -> f32 {
    0.5
}

#[inline]
pub const fn default_dimensions() -> Vec3 {
    Vec3::ONE
}
