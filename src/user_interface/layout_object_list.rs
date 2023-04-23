use super::{camera::Camera, gui_state::GuiState};
use crate::engine::object::{object_collection::ObjectCollection, objects_delta::ObjectsDelta};
use egui::{RichText, TextStyle};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::rc::Rc;

pub fn object_list_layout(
    ui: &mut egui::Ui,
    gui_state: &mut GuiState,
    objects_delta: &mut ObjectsDelta,
    object_collection: &mut ObjectCollection,
    camera: &mut Camera,
) {
    ui.horizontal(|ui_h| {
        // add object button
        let add_response = ui_h.button("Add object");
        if add_response.clicked() {
            // create new object
            let new_object_ref = object_collection.new_empty_object();
            // tell the rest of the engine there's been a change to the object collection
            objects_delta.update.insert(new_object_ref.borrow().id());

            // select new object in the object editor
            let selected_object_ref = Rc::downgrade(&new_object_ref);
            gui_state.set_selected_object(selected_object_ref.clone());
            // set lock on target to selected object
            camera.set_lock_on_target_from_object(&new_object_ref.borrow());
            // deselect primitive op (previous one would have been for different object)
            gui_state.deselect_primitive_op();
        }

        // delete object button
        if let Some(selected_obeject_ref) = gui_state.selected_object() {
            if let Some(selected_object) = selected_obeject_ref.upgrade() {
                let delete_clicked = ui_h
                    .button(format!("Delete: \"{}\"", selected_object.borrow().name()))
                    .clicked();

                if delete_clicked {
                    let selected_object_id = selected_object.borrow().id();
                    let _ = object_collection.remove_object(selected_object_id);

                    // tell the rest of the engine there's been a change to the object collection
                    objects_delta.remove.insert(selected_object.borrow().id());

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
        let label_text = RichText::new(format!(
            "{} - {}",
            current_id.raw_id(),
            current_object.borrow().name()
        ))
        .text_style(TextStyle::Monospace);

        let is_selected = if let Some(selected_obeject_ref) = gui_state.selected_object() {
            if let Some(selected_object) = selected_obeject_ref.upgrade() {
                selected_object.borrow().id() == current_object.borrow().id()
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
            let selected_object_ref = Rc::downgrade(&current_object);
            gui_state.set_selected_object(selected_object_ref.clone());
            // set lock on target to selected object
            camera.set_lock_on_target_from_object(&current_object.borrow());
            // deselect primitive op (previous one would have been for different object)
            gui_state.deselect_primitive_op();
        }
    }
}
