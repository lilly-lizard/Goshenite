use crate::engine::{
    object::{operation::Operation, primitive_op::PrimitiveOp},
    primitives::primitive::Primitive,
};
use egui_dnd::DragDropUi;

/// Wherver or not different windows are open
#[derive(Clone)]
pub struct WindowStates {
    pub object_list: bool,
    pub object_editor: bool,
    pub camera_control: bool,
}
impl Default for WindowStates {
    fn default() -> Self {
        Self {
            object_list: true,
            object_editor: true,
            camera_control: false,
        }
    }
}

/// Amount to increment when modifying values via dragging
pub const DRAG_INC: f64 = 0.02;

/// State persisting between frames
pub struct GuiState {
    /// Stotes state of the op field in the 'New Primitive Op' editor
    pub op_field: Operation,
    /// Stotes state of the fields in the 'New Primitive Op' editor
    pub primitive_fields: Primitive,
    /// Stores the drag and drop state of the primitive op list of the selected object
    pub primitive_op_list_drag_state: DragDropUi,
}

// Setters
impl GuiState {
    pub fn set_selected_primitive_op(&mut self, selected_primitive_op: &PrimitiveOp) {
        self.primitive_fields = selected_primitive_op.primitive;
        self.op_field = selected_primitive_op.op;
    }

    /// Call this if no object is selected
    pub fn reset_primitive_op_list_drag_state(&mut self) {
        self.primitive_op_list_drag_state = Default::default();
    }

    pub fn reset_primitive_op_fields(&mut self) {
        self.op_field = Default::default();
        self.primitive_fields = Default::default();
    }
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            op_field: Operation::NOP,
            primitive_fields: Default::default(),
            primitive_op_list_drag_state: Default::default(),
        }
    }
}
