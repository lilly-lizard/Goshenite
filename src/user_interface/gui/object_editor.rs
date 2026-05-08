use super::Gui;
use crate::{
    engine::{
        commands::{Command, ValidationCommand},
        object::{
            object::{Object, ObjectId},
            object_collection::ObjectCollection,
            primitive_op::{PrimitiveOp, PrimitiveOpIndex},
        },
        primitives::primitive::{EncodablePrimitive, Primitive},
    },
    user_interface::{
        config_ui,
        editable_fields::{
            blend_editor_ui, color_specular_editor_ui, cube_editor_ui, op_drop_down,
            primitive_transform_editor_ui, sphere_editor_ui, uber_primitive_editor_ui,
        },
        gui::EditState,
        gui_state::{GuiState, DRAG_INC},
    },
};
use egui::{ComboBox, DragValue, RichText, TextStyle};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::mem::discriminant;

impl Gui {
    pub(super) fn draw_object_editor_window(
        &mut self,
        object_collection: &ObjectCollection,
        selected_object_id: Option<ObjectId>,
        selected_primitive_op_index: Option<PrimitiveOpIndex>,
    ) -> Vec<Command> {
        let mut commands = Vec::<Command>::new();

        let add_contents = |ui: &mut egui::Ui| {
            commands = layout_object_editor(
                ui,
                &mut self.gui_state,
                object_collection,
                selected_object_id,
                selected_primitive_op_index,
            );
        };
        egui::Window::new("Object Editor")
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(&self.egui_context, add_contents);

        commands
    }
}

fn layout_object_editor(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
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
        gui_state,
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
        Some(o) => o,
        None => {
            // invalid object id
            debug!("selected object {} dropped", some_selected_object_id);
            commands.push(ValidationCommand::SelectedObject().into());

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

    let original_center = object.center;
    let mut new_center = original_center;

    ui.horizontal(|ui| {
        ui.label("Center:");
        ui.add(DragValue::new(&mut new_center.x).speed(DRAG_INC))
            .changed();
        ui.add(DragValue::new(&mut new_center.y).speed(DRAG_INC));
        ui.add(DragValue::new(&mut new_center.z).speed(DRAG_INC));
    });

    if original_center != new_center {
        commands.push(Command::SetObjectCenter {
            object_id: object_id,
            center: new_center,
        });
    }
}

fn primitive_op_editor(
    ui: &mut egui::Ui,
    commands: &mut Vec<Command>,
    gui_state: &mut GuiState,
    selected_object: &Object,
    selected_object_id: ObjectId,
    selected_primitive_op_index: Option<PrimitiveOpIndex>,
) {
    if let Some(selected_primitive_op_index) = selected_primitive_op_index {
        existing_primitive_op_editor(
            ui,
            commands,
            gui_state,
            selected_object,
            selected_object_id,
            selected_primitive_op_index,
        );
    } else {
        new_primitive_op_editor(ui, commands, gui_state, selected_object_id);
    }
}

fn existing_primitive_op_editor(
    ui: &mut egui::Ui,
    commands: &mut Vec<Command>,
    gui_state: &mut GuiState,
    selected_object: &Object,
    selected_object_id: ObjectId,
    selected_primitive_op_index: PrimitiveOpIndex,
) {
    let mut primitive_op_edit_state = EditState::NoChange;

    let selected_object_id = selected_object_id;
    let Some(selected_primitive_op) = selected_object
        .primitive_ops
        .get(selected_primitive_op_index)
    else {
        // invalid primitive op index
        debug!("invalid primitive op index");
        debug!("  object id = {}", selected_object_id);
        debug!("  primitive op index = {}", selected_primitive_op_index);
        new_primitive_op_editor(ui, commands, gui_state, selected_object_id);
        return;
    };

    gui_state.set_primitive_op_edit_state(selected_primitive_op);

    ui.separator();

    ui.label(format!("Primitive op {}:", selected_primitive_op_index));

    // primitive type/op selection

    ui.horizontal(|ui_h| {
        // op drop down menu
        let possible_updated_op = op_drop_down(ui_h, gui_state.op_edit, selected_object_id);
        if let Some(updated_op) = possible_updated_op {
            // user edited the op via drop-down menu
            gui_state.op_edit = updated_op;
            primitive_op_edit_state = EditState::Modified;
        }

        // primitive type drop down menu
        let primitive_type_changed = primitive_type_drop_down(ui_h, gui_state, selected_object_id);
        primitive_op_edit_state = primitive_op_edit_state.combine(primitive_type_changed);
    });

    // primitive editor

    let primitive_edit_state = primitive_editor_ui(ui, gui_state);
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
        EditState::Modified => {
            // update the primitive op data with what we've been using
            commands.push(Command::UpdatePrimitiveOp {
                object_id: selected_object_id,
                primitive_op_index: selected_primitive_op_index,
                new_primitive_op: gui_state.get_primitive_op_from_editor_fields(),
            });
        }
        EditState::NoChange => (),
    }
}

/// Returns wherever the object has been edited
fn new_primitive_op_editor(
    ui: &mut egui::Ui,
    commands: &mut Vec<Command>,
    gui_state: &mut GuiState,
    selected_object_id: ObjectId,
) {
    ui.separator();
    ui.label("New primitive");

    ui.horizontal(|ui_h| {
        // op drop down menu
        let possible_updated_op = op_drop_down(ui_h, gui_state.op_edit, selected_object_id);
        if let Some(updated_op) = possible_updated_op {
            // user edited the op via drop-down menu
            gui_state.op_edit = updated_op;
        }

        // primitive type drop down menu
        primitive_type_drop_down(ui_h, gui_state, selected_object_id);
    });

    // primitive editor

    primitive_editor_ui(ui, gui_state);

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
                primitive_op: gui_state.get_primitive_op_from_editor_fields(),
            });
        } else {
            commands.push(Command::PushPrimitiveOp {
                object_id: selected_object_id,
                primitive_op: gui_state.get_primitive_op_from_editor_fields(),
            });
        }
    }
    if clicked_reset {
        gui_state.reset_primitive_op_fields();
    }
}

/// Returns true if the primitive type was changed. If this happens, gui_state.primitive_edit_state
/// gets set to the default of the chosen type.
fn primitive_type_drop_down(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    selected_object_id: ObjectId,
) -> EditState {
    let selected_primitive_type_name: &str = gui_state.primitive_edit.type_name();
    let mut type_has_changed = EditState::NoChange;

    ComboBox::from_id_salt(format!("primitive type drop down {:?}", selected_object_id))
        .width(0_f32)
        .selected_text(selected_primitive_type_name)
        .show_ui(ui, |ui_p| {
            for (variant_default_primitive, variant_type_name) in Primitive::variants_with_names() {
                // drop-down option for each primitive type
                let this_is_selected = discriminant(&gui_state.primitive_edit)
                    == discriminant(&variant_default_primitive);
                let label_clicked = ui_p
                    .selectable_label(this_is_selected, variant_type_name)
                    .clicked();

                if label_clicked & !this_is_selected {
                    // new primitive type was selected
                    type_has_changed = EditState::Modified;
                    gui_state.primitive_edit = variant_default_primitive;
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

fn primitive_editor_ui(ui: &mut egui::Ui, gui_state: &mut GuiState) -> EditState {
    let primitive_edit_state = match &mut gui_state.primitive_edit {
        Primitive::Sphere(p) => sphere_editor_ui(ui, p),
        Primitive::Cube(p) => cube_editor_ui(ui, p),
        Primitive::UberPrimitive(p) => uber_primitive_editor_ui(ui, p),
    };
    let transform_edit_state = primitive_transform_editor_ui(ui, &mut gui_state.transform_edit);
    let blend_edit_state = blend_editor_ui(ui, &mut gui_state.blend_edit);
    let color_specular_edit_state =
        color_specular_editor_ui(ui, &mut gui_state.albedo_edit, &mut gui_state.specular_edit);

    transform_edit_state
        .combine(primitive_edit_state)
        .combine(blend_edit_state)
        .combine(color_specular_edit_state)
}
