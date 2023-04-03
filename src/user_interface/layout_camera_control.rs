use super::camera::{Camera, LookMode, LookTargetType};
use egui::{RichText, Ui};

// toggle target on object select
// show distance from target

pub fn camera_control_layout(ui: &mut Ui, camera: &mut Camera) {
    // reset button
    ui.horizontal(|ui| {
        let reset_res = ui.button("Reset");
        if reset_res.clicked() {
            camera.reset();
        }
    });

    // camera status
    match camera.look_mode() {
        LookMode::Direction() => {
            ui.label("Look mode: Direction");
        }

        LookMode::Target(target_type) => {
            ui.label("Look mode: Target");
            match target_type {
                LookTargetType::Position(position) => {
                    ui.label(format!(
                        "Target position: [{}, {}, {}]",
                        position.x, position.y, position.z
                    ));
                }
                LookTargetType::Object(object_ref) => {
                    if let Some(object) = object_ref.upgrade() {
                        let name = object.borrow().name().clone();
                        let origin = object.borrow().origin();

                        ui.label(format!(
                            "Target object:\n- name: {}\n- position: [{}, {}, {}]",
                            name, origin.x, origin.y, origin.z,
                        ));
                    } else {
                        let no_object_text = RichText::new("Target object dropped!").italics();
                        ui.label(no_object_text);
                    }
                }
                LookTargetType::Primitive(primitive_ref) => {
                    if let Some(primitive) = primitive_ref.upgrade() {
                        let type_name = primitive.type_name().clone();
                        let center = primitive.transform().center;

                        ui.label(format!(
                            "Target primtive:\n- type: {}\n- position: [{}, {}, {}]",
                            type_name, center.x, center.y, center.z,
                        ));
                    } else {
                        let no_primitive_text =
                            RichText::new("Target primitive dropped!").italics();
                        ui.label(no_primitive_text);
                    }
                }
            }
        }
    }
}
