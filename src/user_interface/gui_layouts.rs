/// UI layout sub-functions
use super::gui_state::{GuiState, DRAG_INC};
use crate::{
    config,
    engine::{
        object::{
            object::{Object, PrimitiveOp},
            object_collection::ObjectCollection,
            objects_delta::ObjectsDelta,
        },
        primitives::{
            primitive_ref_types::PrimitiveRefType, primitive_references::PrimitiveReferences,
        },
    },
    helper::unique_id_gen::UniqueId,
};
use egui::{DragValue, RichText, TextStyle};
use egui_dnd::{DragDropItem, DragDropResponse};
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
        let label_text =
            RichText::new(format!("{} - {}", current_id, current_object.borrow().name))
                .text_style(TextStyle::Monospace);

        let is_selected = if let Some(object_ref) = &gui_state.selected_object {
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
                gui_state.selected_object = Some(Rc::downgrade(current_object));
                gui_state.selected_primitive_op_id = None;
            }
        }
    }
}

pub fn primitive_op_editor(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    objects_delta: &mut ObjectsDelta,
    selected_object: &Object,
    primitive_references: &PrimitiveReferences,
) {
    if let Some(selected_primitive_op_id) = gui_state.selected_primitive_op_id {
        let selected_primitive_op = match selected_object.get_primitive_op(selected_primitive_op_id)
        {
            Some(prim_op) => prim_op,
            None => {
                // selected_primitive_op_id not in selected_obejct! invalid id so we set to none
                gui_state.selected_primitive_op_id = None;
                return;
            }
        };

        let object_id = selected_object.id();
        let primitive_id = selected_primitive_op.prim.borrow().id();
        let primitive_type =
            PrimitiveRefType::from_name(selected_primitive_op.prim.borrow().type_name());

        ui.separator();

        match primitive_type {
            PrimitiveRefType::Sphere => {
                sphere_editor(
                    ui,
                    objects_delta,
                    object_id,
                    primitive_references,
                    primitive_id,
                );
            }
            PrimitiveRefType::Cube => {
                cube_editor(
                    ui,
                    objects_delta,
                    object_id,
                    primitive_references,
                    primitive_id,
                );
            }
            _ => {
                ui.heading(format!(
                    "Primitive Type: {}",
                    selected_primitive_op.prim.borrow().type_name()
                ));
            }
        }
    }
}

pub fn sphere_editor(
    ui: &mut egui::Ui,
    objects_delta: &mut ObjectsDelta,
    object_id: UniqueId,
    primitive_references: &PrimitiveReferences,
    primitive_id: UniqueId,
) {
    let sphere_ref = primitive_references
        .get_sphere(primitive_id)
        .expect("primitive collection doesn't contain primitive id from object op. this is a bug!");
    let mut sphere = sphere_ref.borrow_mut();
    let sphere_original = sphere.clone();

    ui.heading("Edit Sphere");
    ui.horizontal(|ui| {
        ui.label("Center:");
        ui.add(DragValue::new(&mut sphere.center.x).speed(DRAG_INC));
        ui.add(DragValue::new(&mut sphere.center.y).speed(DRAG_INC));
        ui.add(DragValue::new(&mut sphere.center.z).speed(DRAG_INC));
    });
    ui.horizontal(|ui| {
        ui.label("Radius:");
        ui.add(
            DragValue::new(&mut sphere.radius)
                .speed(DRAG_INC)
                .clamp_range(0..=config::MAX_SPHERE_RADIUS),
        );
    });

    // if updates performed on this primtive, indicate that object buffer needs updating
    if *sphere != sphere_original {
        objects_delta.update.insert(object_id);
    }
}

pub fn cube_editor(
    ui: &mut egui::Ui,
    objects_delta: &mut ObjectsDelta,
    object_id: UniqueId,
    primitive_references: &PrimitiveReferences,
    primitive_id: UniqueId,
) {
    let cube_ref = primitive_references
        .get_cube(primitive_id)
        .expect("primitive collection doesn't contain primitive id from object op. this is a bug!");
    let mut cube = cube_ref.borrow_mut();
    let cube_original = cube.clone();

    ui.heading("Edit Cube");
    ui.horizontal(|ui| {
        ui.label("Center:");
        ui.add(DragValue::new(&mut cube.center.x).speed(DRAG_INC));
        ui.add(DragValue::new(&mut cube.center.y).speed(DRAG_INC));
        ui.add(DragValue::new(&mut cube.center.z).speed(DRAG_INC));
    });
    ui.horizontal(|ui| {
        ui.label("Dimensions:");
        ui.add(DragValue::new(&mut cube.dimensions.x).speed(DRAG_INC));
        ui.add(DragValue::new(&mut cube.dimensions.y).speed(DRAG_INC));
        ui.add(DragValue::new(&mut cube.dimensions.z).speed(DRAG_INC));
    });

    // if updates performed on this primtive, indicate that object buffer needs updating
    if *cube != cube_original {
        objects_delta.update.insert(object_id);
    }
}

impl DragDropItem for PrimitiveOp {
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
    let mut list_drag_state = gui_state.primtive_op_list.clone().unwrap_or_default();
    let selected_primitive_op = match gui_state.selected_primitive_op_id {
        Some(selected_primitive_op_id) => {
            match selected_object.get_primitive_op(selected_primitive_op_id) {
                Some(selected_primitive_op) => Some(selected_primitive_op),
                None => {
                    // selected_primitive_op_id not in selected_obejct! invalid id so we set to none
                    gui_state.selected_primitive_op_id = None;
                    None
                }
            }
        }
        None => None,
    };

    ui.separator();

    // draw each item in the primitive op list
    let drag_drop_response = list_drag_state.ui::<PrimitiveOp>(
        ui,
        selected_object.primitive_ops.iter(),
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

            let is_selected = match selected_primitive_op {
                Some(selected_primitive_op) => selected_primitive_op.id() == primitive_op.id(),
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
                    gui_state.selected_primitive_op_id = Some(primitive_op.id());
                }
            });
        },
    );

    // if an item has been dropped after being dragged, re-arrange the primtive ops list
    if let DragDropResponse::Completed(drag_indices) = drag_drop_response {
        egui_dnd::utils::shift_vec(
            drag_indices.source,
            drag_indices.target,
            &mut selected_object.primitive_ops,
        );
        objects_delta.update.insert(selected_object.id());
    }

    gui_state.primtive_op_list = Some(list_drag_state);
}
