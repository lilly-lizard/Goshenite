/// UI layout sub-functions
use super::{
    gui_state::{GuiState, DRAG_INC},
    ui_config,
};
use crate::{
    config,
    engine::{
        object::{
            object::{Object, ObjectId, PrimitiveOp},
            object_collection::ObjectCollection,
            objects_delta::ObjectsDelta,
            operation::Operation,
        },
        primitives::{
            cube::Cube, primitive::PrimitiveRef, primitive_ref_types::PrimitiveRefType,
            primitive_references::PrimitiveReferences, sphere::Sphere,
        },
    },
};
use egui::{ComboBox, DragValue, RichText, TextStyle};
use egui_dnd::{DragDropResponse, DragableItem};
use glam::Vec3;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::rc::Rc;

pub fn object_list(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    object_collection: &ObjectCollection,
) {
    let objects = object_collection.objects();
    for (current_id, current_object) in objects.iter() {
        let label_text = RichText::new(format!(
            "{} - {}",
            current_id,
            current_object.borrow().name()
        ))
        .text_style(TextStyle::Monospace);

        let is_selected = if let Some(object_ref) = gui_state.selected_object() {
            if let Some(selected_object) = object_ref.upgrade() {
                selected_object.borrow().id() == current_object.borrow().id()
            } else {
                debug!("selected object dropped. deselecting object...");
                gui_state.deselect_object();
                false
            }
        } else {
            false
        };

        if ui.selectable_label(is_selected, label_text).clicked() {
            if !is_selected {
                gui_state.set_selected_object(Rc::downgrade(current_object));
                gui_state.deselect_primitive_op();
            }
        }
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
    selected_prim_op_id: usize,
) {
    let object_id = selected_object.id();
    let (selected_prim_op_index, selected_prim_op) =
        match selected_object.get_primitive_op_mut(selected_prim_op_id) {
            Some((index, prim_op)) => (index, prim_op),
            None => {
                // selected_prim_op_id not in selected_obejct! invalid id so we deselect primitive op.
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

    ui.label(format!("Primitive Op {}:", selected_prim_op_index));

    let mut primitive_id = selected_prim_op.prim.borrow().id();
    let old_primitive_type_name = selected_prim_op.prim.borrow().type_name();
    let mut primitive_type: PrimitiveRefType = old_primitive_type_name.into();

    ui.horizontal(|ui_h| {
        // op drop down menu
        op_drop_down(ui_h, objects_delta, object_id, &mut selected_prim_op.op);

        // primitive type drop down menu
        let mut new_primitive_type = primitive_type;
        ComboBox::from_id_source(format!("primitive type drop down {}", object_id))
            .selected_text(old_primitive_type_name)
            .show_ui(ui_h, |ui_p| {
                for (p_type, p_name) in PrimitiveRefType::variant_names() {
                    ui_p.selectable_value(&mut new_primitive_type, p_type, p_name);
                }
            });

        if primitive_type != new_primitive_type {
            // replace old primitive according to new type
            selected_prim_op.prim = primitive_references.new_default(new_primitive_type);
            objects_delta.update.insert(object_id);

            // update local vars for primitive editor
            primitive_type = new_primitive_type;
            primitive_id = selected_prim_op.prim.borrow().id();
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
        let remove_res = selected_object.remove_primitive_op_index(selected_prim_op_index);
        if let Err(_) = remove_res {
            // invalid index! what's going on??
            warn!(
                "invalid index {} when attempting to remove primitive op from object {}",
                selected_prim_op_index, object_id
            );
            gui_state.deselect_primitive_op();
        }
        objects_delta.update.insert(object_id);
    }
}

fn new_primitive_op_editor(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    objects_delta: &mut ObjectsDelta,
    selected_object: &mut Object,
    primitive_references: &mut PrimitiveReferences,
) {
    let object_id = selected_object.id();

    ui.separator();

    ui.label("New Primitive");

    ui.horizontal(|ui_h| {
        // op drop down menu
        op_drop_down(ui_h, objects_delta, object_id, gui_state.op_field_mut());

        // primitive type drop down menu
        let primitive_type_name: &str = gui_state.primitive_fields().p_type.into();
        ComboBox::from_id_source(format!("primitive type drop down {}", object_id))
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
        let new_primitive: Rc<PrimitiveRef> = match gui_state.primitive_fields().p_type {
            PrimitiveRefType::Sphere => primitive_references.new_sphere(
                gui_state.primitive_fields().center,
                gui_state.primitive_fields().radius,
            ),
            PrimitiveRefType::Cube => primitive_references.new_cube(
                gui_state.primitive_fields().center,
                gui_state.primitive_fields().dimensions,
            ),
            _ => primitive_references.new_default(gui_state.primitive_fields().p_type),
        };

        // append primitive op to selected object and mark for updating
        let p_op_id = selected_object.push_op(gui_state.op_field(), new_primitive);
        objects_delta.update.insert(object_id);

        if ui_config::SELECT_PRIMITIVE_OP_AFTER_ADD {
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
    ComboBox::from_id_source(format!("op drop down {}", object_id))
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
    sphere_editor_ui(ui, &mut sphere.center, &mut sphere.radius);
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
    cube_editor_ui(ui, &mut cube.center, &mut cube.dimensions);
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
    fn id(&self) -> egui::Id {
        egui::Id::new(self.prim.borrow().id())
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
    let new_op_text = RichText::new("New Primitive Op").text_style(TextStyle::Monospace);
    let new_op_response =
        ui.selectable_label(gui_state.selected_primitive_op_id().is_none(), new_op_text);
    if new_op_response.clicked() {
        gui_state.deselect_primitive_op();
    }

    let mut list_drag_state = gui_state.primtive_op_list().clone().unwrap_or_default();
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
    let drag_drop_response = list_drag_state.ui::<PrimitiveOp>(
        ui,
        selected_object.primitive_ops().iter(),
        // function to draw a single item in the list
        |ui, handle, index, primitive_op| {
            let draggable_text =
                RichText::new(format!("{}", index)).text_style(TextStyle::Monospace);

            let button_text = RichText::new(format!(
                "{} {}",
                primitive_op.op.name(),
                primitive_op.prim.borrow().type_name()
            ))
            .text_style(TextStyle::Monospace);

            let is_selected = match selected_prim_op {
                Some((_i, selected_prim_op)) => selected_prim_op.id() == primitive_op.id(),
                None => false,
            };

            // draw ui for this primitive op
            ui.horizontal(|ui_h| {
                // anything inside the handle can be used to drag the item
                handle.ui(ui_h, primitive_op, |handle_ui| {
                    handle_ui.label(draggable_text);
                });

                // label to select this primitive op
                if ui_h.selectable_label(is_selected, button_text).clicked() {
                    gui_state.set_selected_primitive_op_id(primitive_op.id());
                }
            });
        },
    );

    // if an item has been dropped after being dragged, re-arrange the primtive ops list
    if let DragDropResponse::Completed(drag_indices) = drag_drop_response {
        selected_object.shift_primitive_ops(drag_indices.source, drag_indices.target);
        objects_delta.update.insert(selected_object.id());
    }

    gui_state.set_primitive_op_list(list_drag_state);
}
