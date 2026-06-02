use super::Engine;
use crate::engine::{
    commands::Command,
    save_states::{load_objects, load_state_camera, save_all_objects, save_state_camera},
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

// ~~ Commands ~~

impl Engine {
    pub(super) fn execute_engine_commands(&mut self) {
        while let Some(command) = self.pending_commands.pop_front() {
            self.execute_command(command);
        }
    }

    pub(super) fn execute_command(&mut self, command: Command) {
        match command {
            // ~~ Save states ~~
            Command::SaveStateCamera => save_state_camera(&self.controllers.camera),
            Command::LoadStateCamera => load_state_camera(&mut self.controllers.camera),
            Command::SaveScene => save_all_objects(&self.object_collection),
            Command::LoadScene => load_objects(&mut self.object_collection),

            // ~~ Object ~~
            Command::SelectObject(object_id) => {
                self.select_object(object_id);
            }
            Command::DeselectObject() => self.deselect_object(),
            Command::RemoveObject(object_id) => self.remove_object(object_id),
            Command::CreateAndSelectNewDefaultObject() => {
                self.create_and_select_new_default_object()
            }
            Command::SetObjectCenter { object_id, center } => {
                self.set_object_center(object_id, center)
            }
            Command::SetObjectName {
                object_id,
                ref new_name,
            } => self.set_object_name(object_id, new_name.clone()),
            Command::SetObjectInstances {
                object_id,
                new_instances,
            } => self.set_object_instances(object_id, new_instances),

            // ~~ Primtive Op: Selection ~~
            Command::SelectPrimitiveOp(object_id, primitive_op_index) => {
                self.select_primitive_op(object_id, primitive_op_index)
            }
            Command::DeselectPrimtiveOp() => self.deselect_primitive_op(),

            // ~~ Primitive Op: Remove ~~
            Command::RemovePrimitiveOp(object_id, primitive_op_index) => {
                self.remove_primitive_op(object_id, primitive_op_index)
            }

            // ~~ Primitive Op: Push ~~
            Command::PushPrimitiveOp {
                object_id,
                primitive_op,
            } => _ = self.push_op(object_id, primitive_op),
            Command::PushPrimitiveOpAndSelect {
                object_id,
                primitive_op,
            } => self.push_op_and_select(object_id, primitive_op),

            // ~~ Primitive Op: Modify ~~
            Command::UpdatePrimitiveOp {
                object_id,
                primitive_op_index,
                new_primitive_op,
            } => self.update_primitive_op(object_id, primitive_op_index, new_primitive_op),
            Command::ReOrderPrimitiveOp {
                object_id,
                original_index,
                target_index,
            } => {
                self.re_order_primitive_op(object_id, original_index, target_index);
            }

            Command::ValidateSelectedObject => self.validate_selected_object(),
        }
    }
}
