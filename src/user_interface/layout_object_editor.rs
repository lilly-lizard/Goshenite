use super::{
    config_ui,
    gui_state::{GuiState, DRAG_INC},
};
use crate::{
    config,
    engine::{
        object::{
            object::{Object, ObjectId, PrimitiveOp, PrimitiveOpId},
            objects_delta::ObjectsDelta,
            operation::Operation,
        },
        primitives::{
            cube::Cube, primitive::PrimitiveCell, primitive_ref_types::PrimitiveRefType,
            primitive_references::PrimitiveReferences, sphere::Sphere,
        },
    },
};
use egui::{ComboBox, DragValue, RichText, TextStyle};
use egui_dnd::{DragDropResponse, DragableItem};
use glam::{Quat, Vec3};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::rc::Rc;

pub fn object_editor(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    objects_delta: &mut ObjectsDelta,
    primitive_references: &mut PrimitiveReferences,
) {
    // selected object name
    let no_object_text = RichText::new("No object selected...").italics();
    let selected_object_weak = match gui_state.selected_object() {
        Some(o) => o.clone(),
        None => {
            ui.label(no_object_text);
            return;
        }
    };
    let selected_object_ref = match selected_object_weak.upgrade() {
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
        ui.text_edit_singleline(selected_object_ref.borrow_mut().name_mut());
    });

    object_properties_editor(ui, objects_delta, &mut selected_object_ref.borrow_mut());

    primitive_op_editor(
        ui,
        gui_state,
        objects_delta,
        &mut selected_object_ref.borrow_mut(),
        primitive_references,
    );

    primitive_op_list(
        ui,
        gui_state,
        objects_delta,
        &mut selected_object_ref.borrow_mut(),
    );
}

pub fn object_properties_editor(
    ui: &mut egui::Ui,
    objects_delta: &mut ObjectsDelta,
    object: &mut Object,
) {
    ui.separator();

    let original_origin = object.origin();
    let mut origin_mut = original_origin;
    ui.horizontal(|ui| {
        ui.label("Origin:");
        ui.add(DragValue::new(&mut origin_mut.x).speed(DRAG_INC));
        ui.add(DragValue::new(&mut origin_mut.y).speed(DRAG_INC));
        ui.add(DragValue::new(&mut origin_mut.z).speed(DRAG_INC));
    });

    if original_origin != origin_mut {
        object.set_origin(origin_mut);
        objects_delta.update.insert(object.id());
    }
}

pub fn primitive_op_editor(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    objects_delta: &mut ObjectsDelta,
    selected_object: &mut Object,
    primitive_references: &mut PrimitiveReferences,
) {
    if let Some(selected_prim_op_id) = gui_state.selected_primitive_op_id() {
        existing_primitive_op_editor(
            ui,
            gui_state,
            objects_delta,
            selected_object,
            primitive_references,
            selected_prim_op_id,
        );
    } else {
        new_primitive_op_editor(
            ui,
            gui_state,
            objects_delta,
            selected_object,
            primitive_references,
        );
    };
}

fn existing_primitive_op_editor(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    objects_delta: &mut ObjectsDelta,
    selected_object: &mut Object,
    primitive_references: &mut PrimitiveReferences,
    selected_prim_op_id: PrimitiveOpId,
) {
    let object_id = selected_object.id();
    let (selected_prim_op_index, selected_prim_op) =
        match selected_object.get_primitive_op_mut(selected_prim_op_id) {
            Some((index, prim_op)) => (index, prim_op),
            None => {
                // selected_prim_op_id not in selected_obejct -> invalid id
                gui_state.deselect_primitive_op();

                new_primitive_op_editor(
                    ui,
                    gui_state,
                    objects_delta,
                    selected_object,
                    primitive_references,
                );
                return;
            }
        };

    ui.separator();

    ui.label(format!("Primitive op {}:", selected_prim_op_index));

    let mut primitive_id = selected_prim_op.primitive.borrow().id();
    let old_primitive_type_name = selected_prim_op.primitive.borrow().type_name();
    let mut primitive_type: PrimitiveRefType = old_primitive_type_name.into();

    ui.horizontal(|ui_h| {
        // op drop down menu
        op_drop_down(ui_h, objects_delta, object_id, &mut selected_prim_op.op);

        // primitive type drop down menu
        let mut new_primitive_type = primitive_type;
        ComboBox::from_id_source(format!("primitive type drop down {:?}", object_id))
            .selected_text(old_primitive_type_name)
            .show_ui(ui_h, |ui_p| {
                for (p_type, p_name) in PrimitiveRefType::variant_names() {
                    ui_p.selectable_value(&mut new_primitive_type, p_type, p_name);
                }
            });

        if primitive_type != new_primitive_type {
            // replace old primitive according to new type
            selected_prim_op.primitive =
                primitive_references.create_primitive_default(new_primitive_type);
            objects_delta.update.insert(object_id);

            // update local vars for primitive editor
            primitive_type = new_primitive_type;
            primitive_id = selected_prim_op.primitive.borrow().id();
        }
    });

    // primitive editor
    match primitive_type {
        PrimitiveRefType::Sphere => {
            let sphere_ref = primitive_references.get_sphere(primitive_id).expect(
                "primitive collection doesn't contain primitive id from object op. this is a bug!",
            );
            let mut sphere = sphere_ref.borrow_mut();
            let sphere_original = sphere.clone();

            sphere_struct_ui(ui, &mut sphere);

            if *sphere != sphere_original {
                // object buffer needs updating
                objects_delta.update.insert(object_id);
            }
        }
        PrimitiveRefType::Cube => {
            let cube_ref = primitive_references.get_cube(primitive_id).expect(
                "primitive collection doesn't contain primitive id from object op. this is a bug!",
            );
            let mut cube = cube_ref.borrow_mut();
            let cube_original = cube.clone();

            cube_struct_ui(ui, &mut cube);

            if *cube != cube_original {
                // object buffer needs updating
                objects_delta.update.insert(object_id);
            }
        }
        _ => (),
    }

    // Delete button
    let delete_clicked = ui.button("Delete").clicked();
    if delete_clicked {
        // remove primitive op
        let remove_res = selected_object.remove_primitive_op_index(selected_prim_op_index);
        if let Err(_) = remove_res {
            // invalid index! what's going on??
            warn!(
                "invalid index {} when attempting to remove primitive op from object {:?}",
                selected_prim_op_index, object_id
            );
        } else {
            // successful removal -> mark object for update
            objects_delta.update.insert(object_id);
        }

        // now select a different primitive op
        gui_state.select_primitive_op_closest_index(
            selected_object.primitive_ops(),
            selected_prim_op_index,
        );
    }
}

fn new_primitive_op_editor(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    objects_delta: &mut ObjectsDelta,
    selected_object: &mut Object,
    primitive_references: &mut PrimitiveReferences,
) {
    ui.separator();
    let object_id = selected_object.id();

    ui.label("New primitive");

    ui.horizontal(|ui_h| {
        // op drop down menu
        op_drop_down(ui_h, objects_delta, object_id, gui_state.op_field_mut());

        // primitive type drop down menu
        let primitive_type_name: &str = gui_state.primitive_fields().p_type.into();
        ComboBox::from_id_source(format!("primitive type drop down {:?}", object_id))
            .selected_text(primitive_type_name)
            .show_ui(ui_h, |ui_p| {
                for (p_type, p_name) in PrimitiveRefType::variant_names() {
                    ui_p.selectable_value(
                        &mut gui_state.primitive_fields_mut().p_type,
                        p_type,
                        p_name,
                    );
                }
            });
    });

    // primitive editor
    match gui_state.primitive_fields().p_type {
        PrimitiveRefType::Sphere => {
            let primitive_fields = gui_state.primitive_fields_mut();
            sphere_editor_ui(
                ui,
                &mut primitive_fields.center,
                &mut primitive_fields.radius,
            );
        }
        PrimitiveRefType::Cube => {
            let primitive_fields = gui_state.primitive_fields_mut();
            cube_editor_ui(
                ui,
                &mut primitive_fields.center,
                &mut primitive_fields.dimensions,
            );
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
        // create primitive
        let new_primitive: Rc<PrimitiveCell> = match gui_state.primitive_fields().p_type {
            PrimitiveRefType::Sphere => primitive_references.create_sphere(
                gui_state.primitive_fields().center,
                Quat::IDENTITY,
                gui_state.primitive_fields().radius,
            ),
            PrimitiveRefType::Cube => primitive_references.create_cube(
                gui_state.primitive_fields().center,
                Quat::IDENTITY,
                gui_state.primitive_fields().dimensions,
            ),
            _ => primitive_references.create_primitive_default(gui_state.primitive_fields().p_type),
        };

        // append primitive op to selected object and mark for updating
        let p_op_id = selected_object.push_op(gui_state.op_field(), new_primitive);
        objects_delta.update.insert(object_id);

        if config_ui::SELECT_PRIMITIVE_OP_AFTER_ADD {
            gui_state.set_selected_primitive_op_id(p_op_id);
        }
    }
    if clicked_reset {
        gui_state.reset_primitive_op_fields();
    }
}

fn op_drop_down(
    ui: &mut egui::Ui,
    objects_delta: &mut ObjectsDelta,
    object_id: ObjectId,
    selected_op: &mut Operation,
) {
    let mut new_op = selected_op.clone();
    ComboBox::from_id_source(format!("op drop down {:?}", object_id))
        .selected_text(selected_op.name())
        .show_ui(ui, |ui_op| {
            for (op, op_name) in Operation::variant_names() {
                ui_op.selectable_value(&mut new_op, op, op_name);
            }
        });
    if *selected_op != new_op {
        // update op
        *selected_op = new_op;
        objects_delta.update.insert(object_id);
    }
}

/// Same as `sphere_editor_ui` but takes a `Sphere` as arg
pub fn sphere_struct_ui(ui: &mut egui::Ui, sphere: &mut Sphere) {
    sphere_editor_ui(ui, &mut sphere.transform.center, &mut sphere.radius);
}

pub fn sphere_editor_ui(ui: &mut egui::Ui, center: &mut Vec3, radius: &mut f32) {
    ui.horizontal(|ui| {
        ui.label("Center:");
        ui.add(DragValue::new(&mut center.x).speed(DRAG_INC));
        ui.add(DragValue::new(&mut center.y).speed(DRAG_INC));
        ui.add(DragValue::new(&mut center.z).speed(DRAG_INC));
    });
    ui.horizontal(|ui| {
        ui.label("Radius:");
        ui.add(
            DragValue::new(radius)
                .speed(DRAG_INC)
                .clamp_range(0..=config::MAX_SPHERE_RADIUS),
        );
    });
}

/// Same as `cube_editor_ui` but takes a `Cube` as arg
pub fn cube_struct_ui(ui: &mut egui::Ui, cube: &mut Cube) {
    cube_editor_ui(ui, &mut cube.transform.center, &mut cube.dimensions);
}

pub fn cube_editor_ui(ui: &mut egui::Ui, center: &mut Vec3, dimensions: &mut Vec3) {
    ui.horizontal(|ui| {
        ui.label("Center:");
        ui.add(DragValue::new(&mut center.x).speed(DRAG_INC));
        ui.add(DragValue::new(&mut center.y).speed(DRAG_INC));
        ui.add(DragValue::new(&mut center.z).speed(DRAG_INC));
    });
    ui.horizontal(|ui| {
        ui.label("Dimensions:");
        ui.add(DragValue::new(&mut dimensions.x).speed(DRAG_INC));
        ui.add(DragValue::new(&mut dimensions.y).speed(DRAG_INC));
        ui.add(DragValue::new(&mut dimensions.z).speed(DRAG_INC));
    });
}

impl DragableItem for PrimitiveOp {
    fn drag_id(&self) -> egui::Id {
        egui::Id::new(format!("p-op-drag{}", self.id()))
    }
}

/// Draw the primitive op list. each list element can be dragged/dropped elsewhere in the list,
/// or selected with a button for editing.
pub fn primitive_op_list(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    objects_delta: &mut ObjectsDelta,
    selected_object: &mut Object,
) {
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
                Some(selected_prim_op) => Some(selected_prim_op),
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
        selected_object.primitive_ops().iter(),
        // function to draw a single primitive op entry in the list
        |ui, drag_handle, index, primitive_op| {
            let draggable_text =
                RichText::new(format!("{}", index)).text_style(TextStyle::Monospace);

            // label text
            let primitive_op_text = RichText::new(format!(
                "{} {}",
                primitive_op.op.name(),
                primitive_op.primitive.borrow().type_name()
            ))
            .text_style(TextStyle::Monospace);

            // check if this primitive op is selected
            let is_selected = match selected_prim_op {
                Some((_i, selected_prim_op)) => selected_prim_op.id() == primitive_op.id(),
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

        objects_delta.update.insert(selected_object.id());
    }
}
