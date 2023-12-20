use super::camera::{Camera, LookMode};
use crate::engine::commands::Command;
use egui::Ui;

pub fn layout_camera_control(ui: &mut Ui, camera: Camera) -> Vec<Command> {
    let mut commands = Vec::<Command>::new();

    // reset button
    ui.horizontal(|ui| {
        let reset_res = ui.button("Reset");
        if reset_res.clicked() {
            commands.push(Command::ResetCamera);
        }
    });

    // unset button
    ui.horizontal(|ui| {
        let target_mode_on = match camera.look_mode() {
            LookMode::Direction(_) => false,
            LookMode::TargetPos(_) => true,
            LookMode::TargetObject { .. } => true,
        };

        let unset_res = ui.add_enabled(target_mode_on, |ui_inner: &mut Ui| {
            ui_inner.button("Unset lock-on taget")
        });

        if unset_res.clicked() {
            commands.push(Command::UnsetCameraLockOn);
        }
    });

    // position
    let position = camera.position();
    ui.label(format!(
        "Position: [{:.2}, {:.2}, {:.2}]",
        position.x, position.y, position.z
    ));

    // look mode
    match camera.look_mode() {
        LookMode::Direction(look_direction) => {
            let look_direction_normalized = look_direction.normalize();

            ui.label("Look mode: Direction");
            ui.label(format!(
                "Look direction: [{:.2}, {:.2}, {:.2}]",
                look_direction_normalized.x,
                look_direction_normalized.y,
                look_direction_normalized.z
            ));
        }

        LookMode::TargetPos(target_pos) => {
            ui.label("Look mode: Target position");
            ui.label(format!(
                "Target position: [{:.2}, {:.2}, {:.2}]",
                target_pos.x, target_pos.y, target_pos.z
            ));
        }

        LookMode::TargetObject {
            object_id,
            last_known_origin,
        } => {
            ui.label(format!("Look mode: Target object (id = {})", object_id));
            ui.label(format!(
                "Target position: [{:.2}, {:.2}, {:.2}]",
                last_known_origin.x, last_known_origin.y, last_known_origin.z
            ));
        }
    }

    commands
}
