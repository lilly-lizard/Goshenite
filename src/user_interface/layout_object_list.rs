use super::{camera::Camera, gui_state::GuiState};
use crate::engine::object::object_collection::ObjectCollection;
use egui::{RichText, TextStyle};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub fn object_list_layout(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    object_collection: &mut ObjectCollection,
    camera: &mut Camera,
) {
    ui.horizontal(|ui_h| {
        // add object button
        let add_response = ui_h.button("Add object");
        if add_response.clicked() {
            // create new object
            let (new_object_id, _) = object_collection.new_object_default();
            let new_object = object_collection
                .get_object(new_object_id)
                .expect("literally just created this");
            // tell the rest of the engine there's been a change to the object collection
            object_collection.mark_object_for_data_update(new_object_id);

            // select new object in the object editor
            gui_state.set_selected_object_id(new_object_id);
            // set lock on target to selected object
            camera.set_lock_on_target_from_object(new_object);
            // deselect primitive op (previous one would have been for different object)
            gui_state.deselect_primitive_op();
        }

        // delete object button
        if let Some(selected_object_id) = gui_state.selected_object_id() {
            if let Some(selected_object) = object_collection.get_object(selected_object_id) {
                let delete_clicked = ui_h
                    .button(format!("Delete: \"{}\"", selected_object.name))
                    .clicked();

                if delete_clicked {
                    object_collection.remove_object(selected_object_id);

                    // select closest object in list
                    gui_state.select_object_closest_index(&object_collection, selected_object_id);
                }
            } else {
                debug!("selected object dropped. deselecting object...");
                gui_state.deselect_object();
            }
        }
    });

    // object list
    for (current_id, current_object) in object_collection.objects().iter() {
        let label_text =
            RichText::new(format!("{} - {}", current_id.raw_id(), current_object.name))
                .text_style(TextStyle::Monospace);

        let is_selected = if let Some(selected_obeject_id) = gui_state.selected_object_id() {
            if let Some(selected_object) = object_collection.get_object(selected_obeject_id) {
                selected_object.id() == current_object.id()
            } else {
                debug!("selected object dropped. deselecting object...");
                gui_state.deselect_object();
                false
            }
        } else {
            false
        };

        let object_label_res = ui.selectable_label(is_selected, label_text);
        if object_label_res.clicked() {
            // select object in the object editor
            gui_state.set_selected_object_id(*current_id);
            // set lock on target to selected object
            camera.set_lock_on_target_from_object(current_object);
            // deselect primitive op (previous one would have been for different object)
            gui_state.deselect_primitive_op();
        }
    }
}
