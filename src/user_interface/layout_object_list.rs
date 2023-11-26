use crate::{
    engine::{
        commands::{Command, ValidationCommand},
        object::{object::ObjectId, object_collection::ObjectCollection},
    },
    helper::unique_id_gen::UniqueIdType,
};
use egui::{RichText, TextStyle};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub fn object_list_layout(
    ui: &mut egui::Ui,
    selected_object_id: Option<ObjectId>,
    object_collection: &ObjectCollection,
) -> Vec<Command> {
    let mut commands = Vec::<Command>::new();

    ui.horizontal(|ui_h| {
        // add object button
        let add_response = ui_h.button("Add object");
        if add_response.clicked() {
            commands.push(Command::CreateAndSelectNewDefaultObject());
        }

        // delete object button
        if let Some(selected_object_id) = selected_object_id {
            if let Some(selected_object) = object_collection.get_object(selected_object_id) {
                let delete_clicked = ui_h
                    .button(format!("Delete: \"{}\"", selected_object.name))
                    .clicked();

                if delete_clicked {
                    commands.push(Command::RemoveObject(selected_object_id));
                }
            } else {
                debug!("selected object dropped. deselecting object...");
            }
        }
    });

    // object list
    for (&current_id, current_object) in object_collection.objects().iter() {
        let label_text =
            RichText::new(format!("{} - {}", current_id.raw_id(), current_object.name))
                .text_style(TextStyle::Monospace);

        let is_selected = if let Some(some_selected_object_id) = selected_object_id {
            if let Some(selected_object) = object_collection.get_object(some_selected_object_id) {
                selected_object.id() == current_object.id()
            } else {
                debug!("selected object {} dropped", some_selected_object_id);
                commands.push(ValidationCommand::SelectedObject().into());
                false
            }
        } else {
            false
        };

        let object_label_res = ui.selectable_label(is_selected, label_text);
        if object_label_res.clicked() {
            // select object in the object editor
            commands.push(Command::SelectObject(current_id));
            // set lock on target to selected object
            commands.push(Command::SetCameraLockOnObject(current_id));
        }
    }

    commands
}
