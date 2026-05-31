use crate::{
    engine::{
        commands::Command,
        object::{
            object::{Object, ObjectId},
            object_collection::ObjectCollection,
            primitive_op::{PrimitiveOp, PrimitiveOpIndex},
        },
        primitives::{
            primitive::{EncodablePrimitive, Primitive},
            transform::ObjectInstances,
        },
    },
    user_interface::{
        config_ui,
        editable_fields::{
            blend_editor_ui, color_specular_editor_ui, cube_editor_ui, op_drop_down,
            primitive_transform_editor_ui, sphere_editor_ui, uber_primitive_editor_ui,
        },
        gui_state::{DataUpdateState, ValueState, DRAG_INC},
    },
};
use egui::{ComboBox, DragValue, RichText, TextStyle};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::mem::discriminant;

pub fn layout_object_editor(
    ui: &mut egui::Ui,
    value_state: &mut ValueState,
    object_collection: &ObjectCollection,
    selected_object_id: Option<ObjectId>,
    selected_primitive_op_index: Option<PrimitiveOpIndex>,
) -> Vec<Command> {
    let mut commands = Vec::<Command>::new();

    // selected object name
    let (selected_object, some_selected_object_id) = match label_and_get_selected_object(
        ui,
        &mut commands,
        object_collection,
        selected_object_id,
    ) {
        Some(value) => value,
        None => return commands,
    };

    object_properties_editor(ui, &mut commands, selected_object, some_selected_object_id);

    primitive_op_editor(
        ui,
        &mut commands,
        value_state,
        selected_object,
        some_selected_object_id,
        selected_primitive_op_index,
    );

    primitive_op_list(
        ui,
        &mut commands,
        selected_object,
        some_selected_object_id,
        selected_primitive_op_index,
    );

    commands
}

fn label_and_get_selected_object<'a>(
    ui: &mut egui::Ui,
    commands: &mut Vec<Command>,
    object_collection: &'a ObjectCollection,
    selected_object_id: Option<ObjectId>,
) -> Option<(&'a Object, ObjectId)> {
    let no_object_text = RichText::new("No object selected...").italics();

    let some_selected_object_id = match selected_object_id {
        Some(id) => id,
        None => {
            ui.label(no_object_text);
            return None;
        }
    };

    let selected_object = match object_collection.get_object(some_selected_object_id) {
        Ok(o) => o,
        Err(_e) => {
            // invalid object id
            debug!("selected object {} dropped", some_selected_object_id);
            commands.push(Command::ValidateSelectedObject);

            ui.label(no_object_text);
            return None;
        }
    };

    let mut new_name = selected_object.name.clone();
    ui.horizontal(|ui_h| {
        ui_h.label("Name:");
        ui_h.text_edit_singleline(&mut new_name);
    });
    if new_name != selected_object.name {
        commands.push(Command::SetObjectName {
            object_id: some_selected_object_id,
            new_name,
        });
    }

    Some((selected_object, some_selected_object_id))
}

fn object_properties_editor(
    ui: &mut egui::Ui,
    commands: &mut Vec<Command>,
    object: &Object,
    object_id: ObjectId,
) {
    ui.separator();

    // object center
    let mut new_center = object.center;
    ui.horizontal(|ui| {
        ui.label("Center:");
        ui.add(DragValue::new(&mut new_center.x).speed(DRAG_INC))
            .changed();
        ui.add(DragValue::new(&mut new_center.y).speed(DRAG_INC));
        ui.add(DragValue::new(&mut new_center.z).speed(DRAG_INC));
    });
    if object.center != new_center {
        commands.push(Command::SetObjectCenter {
            object_id: object_id,
            center: new_center,
        });
    }

    // object instances
    let mut new_instances = object.instances.clone();
    egui::ComboBox::from_label("Instances")
        .selected_text(new_instances.display_name())
        .show_ui(ui, |ui| {
            for variant in ObjectInstances::VARIANTS {
                let name = variant.display_name();
                ui.selectable_value(&mut new_instances, variant, name);
            }
        });
    if object.instances != new_instances {
        commands.push(Command::SetObjectInstances {
            object_id,
            new_instances,
        });
    }
}

fn primitive_op_editor(
    ui: &mut egui::Ui,
    commands: &mut Vec<Command>,
    value_state: &mut ValueState,
    selected_object: &Object,
    selected_object_id: ObjectId,
    selected_primitive_op_index: Option<PrimitiveOpIndex>,
) {
    if let Some(selected_primitive_op_index) = selected_primitive_op_index {
        existing_primitive_op_editor(
            ui,
            commands,
            value_state,
            selected_object,
            selected_object_id,
            selected_primitive_op_index,
        );
    } else {
        new_primitive_op_editor(ui, commands, value_state, selected_object_id);
    }
}

fn existing_primitive_op_editor(
    ui: &mut egui::Ui,
    commands: &mut Vec<Command>,
    value_state: &mut ValueState,
    selected_object: &Object,
    selected_object_id: ObjectId,
    selected_primitive_op_index: PrimitiveOpIndex,
) {
    let mut primitive_op_edit_state = DataUpdateState::NoChange;

    let selected_object_id = selected_object_id;
    let Some(selected_primitive_op) = selected_object
        .primitive_ops
        .get(selected_primitive_op_index)
    else {
        // invalid primitive op index
        debug!("invalid primitive op index");
        debug!("  object id = {}", selected_object_id);
        debug!("  primitive op index = {}", selected_primitive_op_index);
        new_primitive_op_editor(ui, commands, value_state, selected_object_id);
        return;
    };

    value_state.set_primitive_op_edit_state(selected_primitive_op);

    ui.separator();

    ui.label(format!("Primitive op {}:", selected_primitive_op_index));

    // primitive type/op selection

    ui.horizontal(|ui_h| {
        // op drop down menu
        let possible_updated_op = op_drop_down(ui_h, value_state.op, selected_object_id);
        if let Some(updated_op) = possible_updated_op {
            // user edited the op via drop-down menu
            value_state.op = updated_op;
            primitive_op_edit_state = DataUpdateState::Modified;
        }

        // primitive type drop down menu
        let primitive_type_changed =
            primitive_type_drop_down(ui_h, value_state, selected_object_id);
        primitive_op_edit_state = primitive_op_edit_state.combine(primitive_type_changed);
    });

    // primitive editor

    let primitive_edit_state = primitive_editor_ui(ui, value_state);
    primitive_op_edit_state = primitive_op_edit_state.combine(primitive_edit_state);

    // delete button

    let delete_clicked = ui.button("Delete").clicked();
    if delete_clicked {
        commands.push(Command::RemovePrimitiveOp(
            selected_object_id,
            selected_primitive_op_index,
        ));
        return;
    }

    match primitive_op_edit_state {
        DataUpdateState::Modified => {
            // update the primitive op data with what we've been using
            commands.push(Command::UpdatePrimitiveOp {
                object_id: selected_object_id,
                primitive_op_index: selected_primitive_op_index,
                new_primitive_op: value_state.get_primitive_op_from_editor_fields(),
            });
        }
        DataUpdateState::NoChange => (),
    }
}

/// Returns wherever the object has been edited
fn new_primitive_op_editor(
    ui: &mut egui::Ui,
    commands: &mut Vec<Command>,
    value_state: &mut ValueState,
    selected_object_id: ObjectId,
) {
    ui.separator();
    ui.label("New primitive");

    ui.horizontal(|ui_h| {
        // op drop down menu
        let possible_updated_op = op_drop_down(ui_h, value_state.op, selected_object_id);
        if let Some(updated_op) = possible_updated_op {
            // user edited the op via drop-down menu
            value_state.op = updated_op;
        }

        // primitive type drop down menu
        primitive_type_drop_down(ui_h, value_state, selected_object_id);
    });

    // primitive editor

    primitive_editor_ui(ui, value_state);

    // Add and Reset buttons

    let mut clicked_add = false;
    let mut clicked_reset = false;
    ui.horizontal(|ui_h| {
        clicked_add = ui_h.button("Add").clicked();
        clicked_reset = ui_h.button("Reset").clicked();
    });
    if clicked_add {
        if config_ui::SELECT_PRIMITIVE_OP_AFTER_ADD {
            commands.push(Command::PushPrimitiveOpAndSelect {
                object_id: selected_object_id,
                primitive_op: value_state.get_primitive_op_from_editor_fields(),
            });
        } else {
            commands.push(Command::PushPrimitiveOp {
                object_id: selected_object_id,
                primitive_op: value_state.get_primitive_op_from_editor_fields(),
            });
        }
    }
    if clicked_reset {
        value_state.reset_primitive_op_fields();
    }
}

/// Returns true if the primitive type was changed. If this happens, value_state.primitive_edit_state
/// gets set to the default of the chosen type.
fn primitive_type_drop_down(
    ui: &mut egui::Ui,
    value_state: &mut ValueState,
    selected_object_id: ObjectId,
) -> DataUpdateState {
    let selected_primitive_type_name: &str = value_state.primitive.type_name();
    let mut type_has_changed = DataUpdateState::NoChange;

    ComboBox::from_id_salt(format!("primitive type drop down {:?}", selected_object_id))
        .width(0_f32)
        .selected_text(selected_primitive_type_name)
        .show_ui(ui, |ui_p| {
            for (variant_default_primitive, variant_type_name) in Primitive::variants_with_names() {
                // drop-down option for each primitive type
                let this_is_selected = discriminant(&value_state.primitive)
                    == discriminant(&variant_default_primitive);
                let label_clicked = ui_p
                    .selectable_label(this_is_selected, variant_type_name)
                    .clicked();

                if label_clicked & !this_is_selected {
                    // new primitive type was selected
                    type_has_changed = DataUpdateState::Modified;
                    value_state.primitive = variant_default_primitive;
                }
            }
        });

    type_has_changed
}

/// Draw the primitive op list. each list element can be dragged/dropped elsewhere in the list,
/// or selected for editing.
fn primitive_op_list(
    ui: &mut egui::Ui,
    commands: &mut Vec<Command>,
    selected_object: &Object,
    selected_object_id: ObjectId,
    selected_primitive_op_index: Option<PrimitiveOpIndex>,
) {
    ui.separator();

    // new primitive op button
    let new_op_response =
        ui.selectable_label(selected_primitive_op_index.is_none(), "New primitive op");
    if new_op_response.clicked() {
        commands.push(Command::DeselectPrimtiveOp());
    }

    let frame = egui::Frame::default().inner_margin(4.0);
    let (_, dropped_payload) = ui.dnd_drop_zone::<PrimitiveOpIndex, ()>(frame, |ui| {
        for (list_index, primitive_op) in selected_object.primitive_ops.iter().enumerate() {
            primitive_op_list_item(
                ui,
                commands,
                primitive_op,
                list_index,
                selected_primitive_op_index,
                selected_object_id,
            );
        }
    });

    if let Some(dropped_payload_arc) = dropped_payload {
        let original_index = *dropped_payload_arc;
        // The user dropped onto the column, but not on any one item
        // the area this happens in is below the list
        commands.push(Command::ReOrderPrimitiveOp {
            object_id: selected_object_id,
            original_index: original_index,
            // move to end of the list
            target_index: selected_object.primitive_ops.len(),
        });
    }
}

fn primitive_op_list_item(
    ui: &mut egui::Ui,
    commands: &mut Vec<Command>,
    primitive_op: &PrimitiveOp,
    primitive_op_index: PrimitiveOpIndex,
    selected_primitive_op_index: Option<PrimitiveOpIndex>,
    selected_object_id: ObjectId,
) {
    // label text
    let label_text = RichText::new(format!(
        "{} {}",
        primitive_op.op.name(),
        primitive_op.primitive.type_name()
    ))
    .text_style(TextStyle::Monospace);

    let item_gui_id = egui::Id::new(("p-op-list-menu", primitive_op_index));

    // check if this primitive op is selected
    let is_selected = match selected_primitive_op_index {
        Some(some_selected_prim_op_index) => some_selected_prim_op_index == primitive_op_index,
        None => false,
    };

    // label to select or drag/drop this primitive op
    let dnd_response = ui
        .dnd_drag_source(item_gui_id, primitive_op_index, |ui_dnd| {
            ui_dnd.selectable_label(is_selected, label_text)
        })
        .response;

    // primitive op selected
    if dnd_response.clicked() {
        commands.push(Command::SelectPrimitiveOp(
            selected_object_id,
            primitive_op_index,
        ))
    }

    if let (Some(dragging_position), Some(dragging_payload)) = (
        ui.input(|i| i.pointer.interact_pos()),
        dnd_response.dnd_hover_payload::<PrimitiveOpIndex>(),
    ) {
        let dragging_original_index = *dragging_payload;
        let rect = dnd_response.rect;

        // preview insertion
        let stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        let drop_target_index = if dragging_original_index == primitive_op_index {
            // dragged onto itself
            ui.painter().hline(rect.x_range(), rect.center().y, stroke);
            primitive_op_index
        } else if dragging_position.y < rect.center().y {
            // above current item
            ui.painter().hline(rect.x_range(), rect.top(), stroke);
            primitive_op_index
        } else {
            // below current item
            ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
            primitive_op_index + 1
        };

        if let Some(dropped_payload) = dnd_response.dnd_release_payload::<PrimitiveOpIndex>() {
            let original_index = *dropped_payload;
            commands.push(Command::ReOrderPrimitiveOp {
                object_id: selected_object_id,
                original_index: original_index,
                target_index: drop_target_index,
            });
        }
    }
}

fn primitive_editor_ui(ui: &mut egui::Ui, value_state: &mut ValueState) -> DataUpdateState {
    let primitive_edit_state = match &mut value_state.primitive {
        Primitive::Sphere(p) => sphere_editor_ui(ui, p),
        Primitive::Cube(p) => cube_editor_ui(ui, p),
        Primitive::UberPrimitive(p) => uber_primitive_editor_ui(ui, p),
    };
    let transform_edit_state = primitive_transform_editor_ui(ui, &mut value_state.transform);
    let blend_edit_state = blend_editor_ui(ui, &mut value_state.blend);
    let color_specular_edit_state =
        color_specular_editor_ui(ui, &mut value_state.albedo, &mut value_state.specular);

    transform_edit_state
        .combine(primitive_edit_state)
        .combine(blend_edit_state)
        .combine(color_specular_edit_state)
}
