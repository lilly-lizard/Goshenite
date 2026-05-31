use crate::engine::{
    config_engine::{DEFAULT_ALBEDO, DEFAULT_SPECULAR},
    object::{operation::Operation, primitive_op::PrimitiveOp},
    primitives::{primitive::Primitive, transform::Transform},
};
use glam::Vec3;

/// Amount to increment when modifying values via dragging
pub const DRAG_INC: f64 = 0.02;

/// Describes how something has been edited/added/removed by a function
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataUpdateState {
    NoChange,
    Modified,
}
impl DataUpdateState {
    pub fn combine(self, other: Self) -> Self {
        self.max(other)
    }
}

/// State of editable fields persisting between frames
pub struct ValueState {
    pub primitive: Primitive,
    pub transform: Transform,
    pub op: Operation,
    pub blend: f32,
    pub albedo: Vec3,
    pub specular: f32,
}

// Setters
impl ValueState {
    pub fn set_selected_primitive_op_fields(&mut self, selected_primitive_op: &PrimitiveOp) {
        self.primitive = selected_primitive_op.primitive;
        self.op = selected_primitive_op.op;
        self.albedo = selected_primitive_op.albedo;
        self.specular = selected_primitive_op.specular;
    }

    pub fn reset_primitive_op_fields(&mut self) {
        self.op = Default::default();
        self.transform = Default::default();
        self.primitive = Default::default();
    }

    pub fn set_primitive_op_edit_state(&mut self, primitive_op: &PrimitiveOp) {
        self.primitive = primitive_op.primitive;
        self.transform = primitive_op.transform;
        self.op = primitive_op.op;
        self.blend = primitive_op.blend;
    }

    pub fn get_primitive_op_from_editor_fields(&self) -> PrimitiveOp {
        PrimitiveOp::new(
            self.primitive,
            self.transform,
            self.op,
            self.blend,
            self.albedo,
            self.specular,
        )
    }
}

impl Default for ValueState {
    fn default() -> Self {
        Self {
            op: Default::default(),
            blend: 0.,
            transform: Default::default(),
            primitive: Default::default(),
            albedo: DEFAULT_ALBEDO,
            specular: DEFAULT_SPECULAR,
        }
    }
}
