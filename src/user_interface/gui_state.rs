use crate::engine::{
    object::{
        object::{ObjectRef, PrimitiveOpId},
        operation::Operation,
    },
    primitives::{primitive, primitive_ref_types::PrimitiveRefType},
};
use egui_dnd::DragDropUi;
use glam::Vec3;
use std::rc::Weak;

/// Amount to increment when modifying values via dragging
pub const DRAG_INC: f64 = 0.02;

/// State persisting between frames
pub struct GuiState {
    pub selected_object: Option<Weak<ObjectRef>>,
    pub selected_primitive_op_id: Option<PrimitiveOpId>,
    /// Stotes state for the 'new primitive op' editor
    pub new_op: Operation,
    /// Stotes state for the 'new primitive op' editor
    pub new_primitive: PrimtiveEditorState,
    /// Stores the drag and drop state of the primitive op list for the selected object
    pub primtive_op_list: Option<DragDropUi>,
}

impl GuiState {
    pub fn deselect_object(&mut self) {
        self.selected_object = None;
        self.primtive_op_list = None;
        self.deselect_primitive_op();
    }

    pub fn deselect_primitive_op(&mut self) {
        self.selected_primitive_op_id = None;
    }

    pub fn reset_primitive_op_fields(&mut self) {
        self.new_op = Default::default();
        self.new_primitive = Default::default();
    }
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            selected_object: None,
            selected_primitive_op_id: None,
            new_op: Operation::NOP,
            new_primitive: Default::default(),
            primtive_op_list: None,
        }
    }
}

pub struct PrimtiveEditorState {
    pub p_type: PrimitiveRefType,
    pub center: Vec3,
    pub radius: f32,
    pub dimensions: Vec3,
}

impl Default for PrimtiveEditorState {
    fn default() -> Self {
        Self {
            p_type: Default::default(),
            center: primitive::default_center(),
            radius: primitive::default_radius(),
            dimensions: primitive::default_dimensions(),
        }
    }
}
