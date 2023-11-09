use crate::{
    engine::{
        object::{
            object::ObjectId,
            operation::Operation,
            primitive_op::{PrimitiveOp, PrimitiveOpId},
        },
        primitives::primitive::Primitive,
    },
    helper::list::choose_closest_valid_index,
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
    pub primtive_op_list: DragDropUi,
}

// Setters
impl GuiState {
    pub fn set_selected_primitive_op(&mut self, selected_primitive_op: &PrimitiveOp) {
        self.primitive_fields = selected_primitive_op.primitive;
        self.op_field = selected_primitive_op.op;
    }

    pub fn set_primitive_op_list(&mut self, primitive_op_list: DragDropUi) {
        self.primtive_op_list = primitive_op_list;
    }

    pub fn deselect_object(&mut self) {
        self.primtive_op_list = Default::default();
    }

    pub fn reset_primitive_op_fields(&mut self) {
        self.op_field = Default::default();
        self.primitive_fields = Default::default();
    }

    /// Selects a primitive op in `self` from `primitive_ops` which has the closest index to
    /// `target_prim_op_index`. If `primitive_ops` is empty, deselects primitive op in `self`.
    pub fn select_primitive_op_closest_index(
        &mut self,
        primitive_ops: &Vec<PrimitiveOp>,
        target_prim_op_index: usize,
    ) {
        if let Some(select_index) = choose_closest_valid_index(primitive_ops, target_prim_op_index)
        {
            let select_primitive_op_id = primitive_ops[select_index].id();
            self.set_selected_primitive_op_id(select_primitive_op_id)
        } else {
            self.deselect_primitive_op();
        }
    }
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            op_field: Operation::NOP,
            primitive_fields: Default::default(),
            primtive_op_list: Default::default(),
        }
    }
}
