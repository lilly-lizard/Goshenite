use super::{
    config_ui,
    gui_state::{GuiState, DRAG_INC},
};
use crate::{
    config,
    engine::{
        object::{
            object::{Object, ObjectId},
            object_collection::ObjectCollection,
            operation::Operation,
            primitive_op::{PrimitiveOp, PrimitiveOpId},
        },
        primitives::{
            cube::Cube,
            primitive::{
                primitive_names::{self, default_primitive_from_type_name},
                EncodablePrimitive, Primitive,
            },
            sphere::Sphere,
        },
    },
};
use egui::{ComboBox, DragValue, RichText, TextStyle};
use egui_dnd::{DragDropResponse, DragableItem};
use glam::Vec3;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub fn object_editor_layout(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    object_collection: &mut ObjectCollection,
) {
    // selected object name
    let no_object_text = RichText::new("No object selected...").italics();
    let selected_object_id = match gui_state.selected_object_id() {
        Some(o) => o.clone(),
        None => {
            ui.label(no_object_text);
            return;
        }
    };
    let selected_object = match object_collection.get_object_mut(selected_object_id) {
        Some(o) => o,
        None => {
            debug!("selected object dropped. deselecting object...");
            gui_state.deselect_object();
            ui.label(no_object_text);
            return;
        }
    };
    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.text_edit_singleline(&mut selected_object.name);
    });

    let mut object_updated = false;

    object_updated |= object_properties_editor(ui, selected_object);

    object_updated |= primitive_op_editor(ui, gui_state, selected_object);

    object_updated |= primitive_op_list(ui, gui_state, selected_object);

    if object_updated {
        object_collection.mark_object_for_data_update(selected_object_id);
    }
}

/// Returns true if the object was updated.
pub fn object_properties_editor(ui: &mut egui::Ui, object: &mut Object) -> bool {
    let mut object_updated = false;

    ui.separator();

    let original_origin = object.origin;
    let mut origin_mut = original_origin;

    ui.horizontal(|ui| {
        ui.label("Origin:");
        ui.add(DragValue::new(&mut origin_mut.x).speed(DRAG_INC));
        ui.add(DragValue::new(&mut origin_mut.y).speed(DRAG_INC));
        ui.add(DragValue::new(&mut origin_mut.z).speed(DRAG_INC));
    });

    if original_origin != origin_mut {
        object.origin = origin_mut;
        object_updated = true;
    }

    object_updated
}

/// Returns true if the object was updated.
pub fn primitive_op_editor(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    selected_object: &mut Object,
) -> bool {
    if let Some(selected_prim_op_id) = gui_state.selected_primitive_op_id() {
        return existing_primitive_op_editor(ui, gui_state, selected_object, selected_prim_op_id);
    } else {
        return new_primitive_op_editor(ui, gui_state, selected_object);
    }
}

/// Draw the primitive op list. each list element can be dragged/dropped elsewhere in the list,
/// or selected with a button for editing. Returns true if the object was updated.
pub fn primitive_op_list(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    selected_object: &mut Object,
) -> bool {
    let mut object_updated = false;

    ui.separator();

    // new primitive op button
    let new_op_response = ui.selectable_label(
        gui_state.selected_primitive_op_id().is_none(),
        "New primitive op",
    );
    if new_op_response.clicked() {
        gui_state.deselect_primitive_op();
    }

    let selected_prim_op = match gui_state.selected_primitive_op_id() {
        Some(selected_prim_op_id) => {
            match selected_object.get_primitive_op(selected_prim_op_id) {
                Some((selected_prim_op, _index)) => Some(selected_prim_op),
                None => {
                    // selected_prim_op_id not in selected_obejct! invalid id so we set to none
                    gui_state.deselect_primitive_op();
                    None
                }
            }
        }
        None => None,
    };

    // draw each item in the primitive op list
    let mut prim_op_list_drag_state = gui_state.primtive_op_list().clone();
    let drag_drop_response = prim_op_list_drag_state.list_ui::<PrimitiveOp>(
        ui,
        selected_object.primitive_ops.iter(),
        // function to draw a single primitive op entry in the list
        |ui, drag_handle, index, primitive_op| {
            let draggable_text =
                RichText::new(format!("{}", index)).text_style(TextStyle::Monospace);

            // label text
            let primitive_op_text = RichText::new(format!(
                "{} {}",
                primitive_op.op.name(),
                primitive_op.primitive.type_name()
            ))
            .text_style(TextStyle::Monospace);

            // check if this primitive op is selected
            let is_selected = match selected_prim_op {
                Some(some_selected_prim_op) => some_selected_prim_op.id() == primitive_op.id(),
                None => false,
            };

            // draw ui for this primitive op
            ui.horizontal(|ui_h| {
                // anything inside the handle can be used to drag the item
                drag_handle.ui(ui_h, primitive_op, |handle_ui| {
                    handle_ui.label(draggable_text);
                });

                // label to select this primitive op
                let prim_op_res = ui_h.selectable_label(is_selected, primitive_op_text);

                // if clicked, select it
                if prim_op_res.clicked() {
                    gui_state.set_selected_primitive_op_id(primitive_op.id());
                }
            });
        },
    );
    gui_state.set_primitive_op_list(prim_op_list_drag_state);

    // if an item has been dropped after being dragged, re-arrange the primtive ops list
    if let DragDropResponse::Completed(drag_indices) = drag_drop_response {
        let shift_res =
            selected_object.shift_primitive_ops(drag_indices.source, drag_indices.target);
        if let Err(e) = shift_res {
            error!(
                "bug when trying to re-arrange primitive op list of object {}: {}",
                selected_object.id().raw_id(),
                e
            );
        }

        object_updated = true;
    }

    object_updated
}

/// Returns true if the object was updated.
fn existing_primitive_op_editor(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    selected_object: &mut Object,
    selected_prim_op_id: PrimitiveOpId,
) -> bool {
    let mut object_updated = false;

    let selected_object_id = selected_object.id();
    let (mut selected_op, selected_prim_op_index) =
        match selected_object.get_primitive_op(selected_prim_op_id) {
            Some((prim_op, index)) => (prim_op.op, index),
            None => {
                // selected_prim_op_id not in selected_obejct -> invalid id
                gui_state.deselect_primitive_op();

                return new_primitive_op_editor(ui, gui_state, selected_object);
            }
        };

    ui.separator();

    ui.label(format!("Primitive op {}:", selected_prim_op_index));

    // primitive type/op selection

    let mut possible_updated_op: Option<Operation> = None;
    let mut possible_updated_primitive: Option<Primitive> = None;
    ui.horizontal(|ui_h| {
        // op drop down menu
        possible_updated_op = op_drop_down(ui_h, selected_object_id, selected_op);

        // primitive type drop down menu
        let primitive_type_changed = primitive_type_drop_down(ui_h, gui_state, selected_object_id);

        if primitive_type_changed {
            // replace old primitive according to new type
            possible_updated_primitive = Some(gui_state.primitive_fields().clone());
        }
    });
    if let Some((selected_prim_op, _)) = selected_object.get_primitive_op_mut(selected_prim_op_id) {
        // user edited op and/or primitive via drop-down menus...
        if let Some(updated_op) = possible_updated_op {
            selected_prim_op.op = updated_op;
            object_updated = true;
        }
        if let Some(updated_primitive) = possible_updated_primitive {
            selected_prim_op.primitive = updated_primitive;
            object_updated = true;
        }
    }

    // primitive editor

    let primitive_edited = match gui_state.primitive_fields_mut() {
        Primitive::Sphere(p) => sphere_editor_ui(ui, p),
        Primitive::Cube(p) => cube_editor_ui(ui, p),
        _ => false,
    };
    if primitive_edited {
        // replace primitive with edited one
        if let Some((selected_prim_op, _)) =
            selected_object.get_primitive_op_mut(selected_prim_op_id)
        {
            selected_prim_op.primitive = gui_state.primitive_fields().clone();
            object_updated = true;
        }
    }

    // delete button

    let delete_clicked = ui.button("Delete").clicked();
    if delete_clicked {
        // remove primitive op
        let remove_res = selected_object.remove_primitive_op_index(selected_prim_op_index);
        if let Err(_) = remove_res {
            // invalid index! what's going on??
            warn!(
                "invalid index {} when attempting to remove primitive op from object {:?}",
                selected_prim_op_index, selected_object_id
            );
        } else {
            // successful removal -> mark object for update
            object_updated = true;
        }

        // now select a different primitive op
        gui_state.select_primitive_op_closest_index(
            &selected_object.primitive_ops,
            selected_prim_op_index,
        );
    }

    object_updated
}

/// Returns true if the object was updated.
fn new_primitive_op_editor(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    selected_object: &mut Object,
) -> bool {
    let mut object_updated = false;

    ui.separator();
    ui.label("New primitive");

    ui.horizontal(|ui_h| {
        // op drop down menu
        let _ = op_drop_down(ui_h, selected_object.id(), gui_state.op_field());

        // primitive type drop down menu
        primitive_type_drop_down(ui_h, gui_state, selected_object.id());
    });

    // primitive editor

    match gui_state.primitive_fields_mut() {
        Primitive::Sphere(p) => {
            sphere_editor_ui(ui, p);
        }
        Primitive::Cube(p) => {
            cube_editor_ui(ui, p);
        }
        _ => (),
    }

    // Add and Reset buttons

    let mut clicked_add = false;
    let mut clicked_reset = false;
    ui.horizontal(|ui_h| {
        clicked_add = ui_h.button("Add").clicked();
        clicked_reset = ui_h.button("Reset").clicked();
    });
    if clicked_add {
        // append primitive op to selected object and mark for updating
        let new_primitive = gui_state.primitive_fields().clone();
        let p_op_id = selected_object.push_op(gui_state.op_field(), new_primitive);
        object_updated = true;

        if config_ui::SELECT_PRIMITIVE_OP_AFTER_ADD {
            gui_state.set_selected_primitive_op_id(p_op_id);
        }
    }
    if clicked_reset {
        gui_state.reset_primitive_op_fields();
    }

    object_updated
}

/// Returns a new operation if a different one is selected
fn op_drop_down(
    ui: &mut egui::Ui,
    object_id: ObjectId,
    selected_op: Operation,
) -> Option<Operation> {
    let mut new_op = selected_op.clone();

    ComboBox::from_id_source(format!("op drop down {:?}", object_id))
        .selected_text(selected_op.name())
        .show_ui(ui, |ui_op| {
            for (op, op_name) in Operation::variant_names() {
                ui_op.selectable_value(&mut new_op, op, op_name);
            }
        });

    if selected_op != new_op {
        return Some(new_op);
    }
    None
}

/// Returns true if the primitive type was changed. If this happens, gui_state.primitive_fields
/// gets set to the default of the chosen type.
fn primitive_type_drop_down(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    selected_object_id: ObjectId,
) -> bool {
    let selected_primitive_type_name: &str = gui_state.primitive_fields().type_name();
    let mut type_has_changed = false;

    ComboBox::from_id_source(format!("primitive type drop down {:?}", selected_object_id))
        .selected_text(selected_primitive_type_name)
        .show_ui(ui, |ui_p| {
            for primitive_type_name in primitive_names::NAME_LIST {
                // drop-down option for each primitive type
                let this_is_selected = selected_primitive_type_name == primitive_type_name;
                let label_clicked = ui_p
                    .selectable_label(this_is_selected, primitive_type_name)
                    .clicked();

                if label_clicked & !this_is_selected {
                    // new primitive type was selected
                    type_has_changed = true;
                    let new_primitive = default_primitive_from_type_name(primitive_type_name);
                    gui_state.set_primitive_fields(new_primitive);
                }
            }
        });

    type_has_changed
}

/// Same as `sphere_editor_ui` but takes a `Sphere` as arg.
/// Returns true if a value was changed.
#[inline]
pub fn sphere_editor_ui(ui: &mut egui::Ui, sphere: &mut Sphere) -> bool {
    sphere_editor_ui_fields(ui, &mut sphere.transform.center, &mut sphere.radius)
}

/// Returns true if a value was changed.
pub fn sphere_editor_ui_fields(ui: &mut egui::Ui, center: &mut Vec3, radius: &mut f32) -> bool {
    let mut something_changed: bool = false;

    ui.horizontal(|ui| {
        ui.label("Center:");
        something_changed |= ui
            .add(DragValue::new(&mut center.x).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut center.y).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut center.z).speed(DRAG_INC))
            .changed();
    });

    ui.horizontal(|ui| {
        ui.label("Radius:");
        something_changed |= ui
            .add(
                DragValue::new(radius)
                    .speed(DRAG_INC)
                    .clamp_range(0..=config::MAX_SPHERE_RADIUS),
            )
            .changed();
    });

    something_changed
}

/// Same as `cube_editor_ui` but takes a `Cube` as arg.
/// Returns true if a value was changed.
#[inline]
pub fn cube_editor_ui(ui: &mut egui::Ui, cube: &mut Cube) -> bool {
    cube_editor_ui_fields(ui, &mut cube.transform.center, &mut cube.dimensions)
}

/// Returns true if a value was changed.
pub fn cube_editor_ui_fields(ui: &mut egui::Ui, center: &mut Vec3, dimensions: &mut Vec3) -> bool {
    let mut something_changed: bool = false;

    ui.horizontal(|ui| {
        ui.label("Center:");
        something_changed |= ui
            .add(DragValue::new(&mut center.x).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut center.y).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut center.z).speed(DRAG_INC))
            .changed();
    });

    ui.horizontal(|ui| {
        ui.label("Dimensions:");
        something_changed |= ui
            .add(DragValue::new(&mut dimensions.x).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut dimensions.y).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut dimensions.z).speed(DRAG_INC))
            .changed();
    });

    something_changed
}

impl DragableItem for PrimitiveOp {
    fn drag_id(&self) -> egui::Id {
        egui::Id::new(format!("p-op-drag{}", self.id()))
    }
}
