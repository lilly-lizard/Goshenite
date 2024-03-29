use crate::engine::{
    config_engine::{DEFAULT_ALBEDO, DEFAULT_SPECULAR},
    object::{operation::Operation, primitive_op::PrimitiveOp},
    primitives::{primitive::Primitive, primitive_transform::PrimitiveTransform},
};
use egui_dnd::DragDropUi;
use glam::Vec3;

/// Wherver or not different windows are open
#[derive(Clone)]
pub struct SubWindowStates {
    pub object_list: bool,
    pub object_editor: bool,
    pub camera_control: bool,
    pub command_palette: bool,
    pub debug_options: bool,
}
impl Default for SubWindowStates {
    fn default() -> Self {
        Self {
            object_list: true,
            object_editor: true,
            camera_control: false,
            command_palette: false,
            debug_options: false,
        }
    }
}

/// Amount to increment when modifying values via dragging
pub const DRAG_INC: f64 = 0.02;

/// State persisting between frames
pub struct GuiState {
    /// Stotes the state of the fields in the gui editor
    pub primitive_edit: Primitive,
    /// Stores the state of the primitive transform fields in the gui editor
    pub transform_edit: PrimitiveTransform,
    /// Stotes the state of the op field in the gui editor
    pub op_edit: Operation,
    /// Stores the state of the blend field in the gui editor
    pub blend_edit: f32,
    /// Stores the drag and drop state of the primitive op list of the selected object
    pub primitive_op_list_drag: DragDropUi,

    pub albedo_edit: Vec3,
    pub specular_edit: f32,
}

// Setters
impl GuiState {
    pub fn set_selected_primitive_op(&mut self, selected_primitive_op: &PrimitiveOp) {
        self.primitive_edit = selected_primitive_op.primitive;
        self.op_edit = selected_primitive_op.op;
        self.albedo_edit = selected_primitive_op.albedo;
        self.specular_edit = selected_primitive_op.specular;
    }

    /// Call this if no object is selected
    pub fn reset_primitive_op_list_drag_state(&mut self) {
        self.primitive_op_list_drag = Default::default();
    }

    pub fn reset_primitive_op_fields(&mut self) {
        self.op_edit = Default::default();
        self.transform_edit = Default::default();
        self.primitive_edit = Default::default();
    }

    pub fn set_primitive_op_edit_state(&mut self, primitive_op: &PrimitiveOp) {
        self.primitive_edit = primitive_op.primitive;
        self.transform_edit = primitive_op.transform;
        self.op_edit = primitive_op.op;
        self.blend_edit = primitive_op.blend;
    }
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            op_edit: Default::default(),
            blend_edit: 0.,
            transform_edit: Default::default(),
            primitive_edit: Default::default(),
            albedo_edit: DEFAULT_ALBEDO,
            specular_edit: DEFAULT_SPECULAR,
            primitive_op_list_drag: Default::default(),
        }
    }
}
