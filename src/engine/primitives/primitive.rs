use super::primitive_transform::PrimitiveTransform;
use crate::{
    engine::aabb::Aabb,
    helper::unique_id_gen::UniqueId,
    renderer::shader_interfaces::primitive_op_buffer::{
        PrimitiveOpBufferUnit, PrimitivePropsSlice,
    },
};
use glam::Vec3;
use std::{cell::RefCell, rc::Rc};

pub type PrimitiveId = UniqueId;

/// Use functions `borrow` and `borrow_mut` to access the `Primitive`.
pub type PrimitiveCell = RefCell<dyn Primitive>;
#[inline]
pub fn new_primitive_ref<T: Primitive + 'static>(inner: T) -> Rc<PrimitiveCell> {
    Rc::new(RefCell::new(inner))
}

/// A primitive is a basic geometric building block that can be manipulated and combined
/// using [`Operation`]s in an [`Object`]
pub trait Primitive {
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

pub fn default_center() -> Vec3 {
    Vec3::ZERO
}

pub fn default_radius() -> f32 {
    1.
}

pub fn default_dimensions() -> Vec3 {
    Vec3::ONE
}
