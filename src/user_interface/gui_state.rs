use crate::{
    engine::{
        object::{
            object::{ObjectCell, ObjectId, PrimitiveOp, PrimitiveOpId},
            object_collection::ObjectCollection,
            operation::Operation,
        },
        primitives::{primitive, primitive_ref_types::PrimitiveRefType},
    },
    helper::list::choose_closest_valid_index,
};
use egui_dnd::DragDropUi;
use glam::Vec3;
use std::rc::{Rc, Weak};

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
    selected_object: Option<Weak<ObjectCell>>,
    selected_primitive_op_id: Option<PrimitiveOpId>,
    /// Stotes state of the op field in the 'New Primitive Op' editor
    op_field: Operation,
    /// Stotes state of the primitive fields in the 'New Primitive Op' editor
    primitive_fields: PrimitiveEditorState,
    /// Stores the drag and drop state of the primitive op list of the selected object
    primtive_op_list: DragDropUi,
}

// Setters
impl GuiState {
    pub fn set_selected_object(&mut self, selected_object: Weak<ObjectCell>) {
        self.selected_object = Some(selected_object);
    }

    pub fn set_selected_primitive_op_id(&mut self, selected_primitive_op_id: PrimitiveOpId) {
        self.selected_primitive_op_id = Some(selected_primitive_op_id);
    }

    pub fn set_primitive_op_list(&mut self, primitive_op_list: DragDropUi) {
        self.primtive_op_list = primitive_op_list;
    }

    pub fn deselect_object(&mut self) {
        self.selected_object = None;
        self.primtive_op_list = Default::default();
        self.deselect_primitive_op();
    }

    pub fn deselect_primitive_op(&mut self) {
        self.selected_primitive_op_id = None;
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

    /// Selects an object in `self` from `object_collection` which has the closest id to
    /// `target_object_id`. If `object_collection` is empty, deselects object in `self`.
    pub fn select_object_closest_index(
        &mut self,
        object_collection: &ObjectCollection,
        target_object_id: ObjectId,
    ) {
        if let Some(select_object) =
            choose_object_closest_index(object_collection, target_object_id)
        {
            self.set_selected_object(Rc::downgrade(&select_object));
        } else {
            self.deselect_object();
        }
    }
}

// Getters
impl GuiState {
    pub fn selected_object(&self) -> Option<Weak<ObjectCell>> {
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

    pub fn primtive_op_list(&self) -> &DragDropUi {
        &self.primtive_op_list
    }

    pub fn primtive_op_list_mut(&mut self) -> &mut DragDropUi {
        &mut self.primtive_op_list
    }
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            selected_object: None,
            selected_primitive_op_id: None,
            op_field: Operation::NOP,
            primitive_fields: Default::default(),
            primtive_op_list: Default::default(),
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
            center: Vec3::ZERO,
            radius: primitive::default_radius(),
            dimensions: primitive::default_dimensions(),
        }
    }
}

/// Returns an `Rc` to an object from `object_collection` which has the closest id to
/// the object `target_object_id`.
pub fn choose_object_closest_index(
    object_collection: &ObjectCollection,
    target_object_id: ObjectId,
) -> Option<Rc<ObjectCell>> {
    let mut select_object: Option<Rc<ObjectCell>> = None;
    for (&current_id, current_object) in object_collection.objects().iter() {
        select_object = Some(current_object.clone());
        if current_id >= target_object_id {
            break;
        }
    }
    return select_object;
}
