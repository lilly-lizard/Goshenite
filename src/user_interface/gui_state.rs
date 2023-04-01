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
    selected_object: Option<Weak<ObjectRef>>,
    selected_primitive_op_id: Option<PrimitiveOpId>,
    /// Stotes state of the op field in the 'New Primitive Op' editor
    op_field: Operation,
    /// Stotes state of the primitive fields in the 'New Primitive Op' editor
    primitive_fields: PrimitiveEditorState,
    /// Stores the drag and drop state of the primitive op list of the selected object
    primtive_op_list: Option<DragDropUi>,
}

// Setters
impl GuiState {
    pub fn set_selected_object(&mut self, selected_object: Weak<ObjectRef>) {
        self.selected_object = Some(selected_object);
    }

    pub fn set_selected_primitive_op_id(&mut self, selected_primitive_op_id: PrimitiveOpId) {
        self.selected_primitive_op_id = Some(selected_primitive_op_id);
    }

    pub fn set_primitive_op_list(&mut self, primitive_op_list: DragDropUi) {
        self.primtive_op_list = Some(primitive_op_list);
    }

    pub fn deselect_object(&mut self) {
        self.selected_object = None;
        self.primtive_op_list = None;
        self.deselect_primitive_op();
    }

    pub fn deselect_primitive_op(&mut self) {
        self.selected_primitive_op_id = None;
    }

    pub fn reset_primitive_op_fields(&mut self) {
        self.op_field = Default::default();
        self.primitive_fields = Default::default();
    }
}

// Getters
impl GuiState {
    pub fn selected_object(&self) -> Option<Weak<ObjectRef>> {
        self.selected_object.clone()
    }

    pub fn selected_primitive_op_id(&self) -> Option<PrimitiveOpId> {
        self.selected_primitive_op_id
    }

    pub fn op_field(&self) -> Operation {
        self.op_field
    }

    pub fn op_field_mut(&mut self) -> &mut Operation {
        &mut self.op_field
    }

    pub fn primitive_fields(&self) -> &PrimitiveEditorState {
        &self.primitive_fields
    }

    pub fn primitive_fields_mut(&mut self) -> &mut PrimitiveEditorState {
        &mut self.primitive_fields
    }

    pub fn primtive_op_list(&mut self) -> &Option<DragDropUi> {
        &self.primtive_op_list
    }
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            selected_object: None,
            selected_primitive_op_id: None,
            op_field: Operation::NOP,
            primitive_fields: Default::default(),
            primtive_op_list: None,
        }
    }
}

pub struct PrimitiveEditorState {
    pub p_type: PrimitiveRefType,
    pub center: Vec3,
    pub radius: f32,
    pub dimensions: Vec3,
}

impl Default for PrimitiveEditorState {
    fn default() -> Self {
        Self {
            p_type: Default::default(),
            center: primitive::default_center(),
            radius: primitive::default_radius(),
            dimensions: primitive::default_dimensions(),
        }
    }
}
