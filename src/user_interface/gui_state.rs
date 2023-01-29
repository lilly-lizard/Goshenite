use crate::engine::object::object::{ObjectRef, PrimitiveOpId};
use egui_dnd::DragDropUi;
use std::rc::Weak;

/// Amount to increment when modifying values via dragging
pub const DRAG_INC: f64 = 0.02;

/// State persisting between frames
#[derive(Clone)]
pub struct GuiState {
    pub selected_object: Option<Weak<ObjectRef>>,
    /// Selected primitive op index in the object editor
    pub selected_primitive_op_id: Option<PrimitiveOpId>,
    /// Stores the drag and drop state of the primitive op list for the selected object
    pub primtive_op_list: Option<DragDropUi>,
}
impl GuiState {
    #[inline]
    pub fn deselect_object(&mut self) {
        self.selected_object = None;
        self.selected_primitive_op_id = None;
        self.primtive_op_list = None;
    }
}
impl Default for GuiState {
    fn default() -> Self {
        Self {
            selected_object: None,
            selected_primitive_op_id: None,
            primtive_op_list: None,
        }
    }
}
