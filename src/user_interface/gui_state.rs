use crate::engine::{
    object::{operation::Operation, primitive_op::PrimitiveOp},
    primitives::{primitive::Primitive, primitive_transform::PrimitiveTransform},
};
use egui_dnd::DragDropUi;

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
    /// Stotes the state of the op field in the gui editor
    pub op_edit_state: Operation,
    /// Stores the state of the primitive transform fields in the gui editor
    pub transform_edit_state: PrimitiveTransform,
    /// Stotes the state of the fields in the gui editor
    pub primitive_edit_state: Primitive,
    /// Stores the drag and drop state of the primitive op list of the selected object
    pub primitive_op_list_drag_state: DragDropUi,
}

// Setters
impl GuiState {
    pub fn set_selected_primitive_op(&mut self, selected_primitive_op: &PrimitiveOp) {
        self.primitive_edit_state = selected_primitive_op.primitive;
        self.op_edit_state = selected_primitive_op.op;
    }

    /// Call this if no object is selected
    pub fn reset_primitive_op_list_drag_state(&mut self) {
        self.primitive_op_list_drag_state = Default::default();
    }

    pub fn reset_primitive_op_fields(&mut self) {
        self.op_edit_state = Default::default();
        self.transform_edit_state = Default::default();
        self.primitive_edit_state = Default::default();
    }
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            op_edit_state: Default::default(),
            transform_edit_state: Default::default(),
            primitive_edit_state: Default::default(),
            primitive_op_list_drag_state: Default::default(),
        }
    }
}
