use super::EngineInstance;
use crate::{
    engine::{
        commands::{Command, CommandWithSource},
        object::{
            object::{Object, ObjectId},
            primitive_op::{PrimitiveOp, PrimitiveOpId},
        },
    },
    helper::list::choose_closest_valid_index,
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

impl EngineInstance {
    pub(super) fn execute_engine_commands(&mut self) {
        while let Some(CommandWithSource {
            command,
            source: _source,
        }) = self.pending_commands.pop_front()
        {
            match command {
                // camera
                Command::SetCameraLockOn { target_pos } => {
                    self.camera.set_lock_on_target(target_pos)
                }
                Command::UnsetCameraLockOn => self.camera.unset_lock_on_target(),
                Command::ResetCamera => self.camera.reset(),

                // object
                Command::SelectObject(object_id) => {
                    self.select_object_via_command(object_id, command);
                }
                Command::DeselectObject() => self.deselect_object(),
                Command::RemoveObject(object_id) => {
                    self.remove_object_via_command(object_id, command)
                }
                Command::RemoveSelectedObject() => self.remove_selected_object_via_command(command),

                // primitive op
                Command::SelectPrimitiveOpId(primitive_op_id) => {
                    self.select_primitive_op_id_via_command(primitive_op_id, command)
                }
                Command::SelectPrimitiveOpIndex(primitive_op_index) => {
                    self.select_primitive_op_index_via_command(primitive_op_index, command)
                }
                Command::DeselectPrimtiveOp() => self.deselect_primitive_op(),
                Command::RemovePrimitiveOpId(primitive_op_id) => {
                    self.remove_primitive_op_id_via_command(primitive_op_id, command);
                }
                Command::RemovePrimitiveOpIndex(primitive_op_index) => {
                    self.remove_primitive_op_index_via_command(primitive_op_index, command);
                }
                Command::RemoveSelectedPrimitiveOp() => {
                    self.remove_selected_primitive_op_via_command(command);
                }
            }
        }
    }

    fn deselect_object(&mut self) {
        if self.selected_object_id.is_some() {
            self.gui.selected_object_changed();
        }
        self.selected_object_id = None;
        self.selected_primitive_op_id = None;
    }

    fn select_object_via_command(&mut self, object_id: ObjectId, command: Command) {
        if let Some(object) = self.object_collection.get_object(object_id) {
            self.select_object(object);
        } else {
            command_failed_warn(command, "invalid object id");
        }
    }

    fn select_object(&mut self, object: &Object) {
        let mut selected_object_changed = true;
        if let Some(previously_selected_object_id) = self.selected_object_id {
            if previously_selected_object_id == object.id() {
                // note: written this way to consider the case where it was None before
                selected_object_changed = false;
            }
        }

        self.selected_object_id = Some(object.id());
        self.camera.set_lock_on_target(object.origin.as_dvec3());

        if selected_object_changed {
            // if a different object is already selected, deselect the primitive op because it will
            // no longer be valid
            self.deselect_primitive_op();
            self.gui.selected_object_changed();
        }
    }

    fn remove_object_via_command(&mut self, object_id: ObjectId, command: Command) {
        let res = self.object_collection.remove_object(object_id);
        if let Err(e) = res {
            command_failed_warn(command, "invalid object id");
        }

        if let Some(previously_selected_object_id) = self.selected_object_id {
            if previously_selected_object_id == object_id {
                self.deselect_object();
            }
        }
    }

    pub(super) fn select_object_and_primitive_op(
        &mut self,
        object_id: ObjectId,
        primitive_op_index: usize,
    ) {
        if let Some(object) = self.object_collection.get_object(object_id) {
            self.select_object(object);

            if let Some(primitive_op) = object.primitive_ops.get(primitive_op_index) {
                self.selected_primitive_op_id = Some(primitive_op.id());
                self.gui.set_selected_primitive_op(primitive_op.id());
            } else {
                warn!("attempted to select primitive op index that doesn't exist in object");
            }
        } else {
            warn!("attempted to select object id that doesn't exist in object collection");
        }
    }

    fn remove_selected_object_via_command(&mut self, command: Command) {
        if let Some(selected_object_id) = self.selected_object_id {
            let res = self.object_collection.remove_object(selected_object_id);
            if let Err(e) = res {
                command_failed_warn(command, "selected object id invalid");
            }
        } else {
            command_failed_warn(command, "no selected object");
        }
    }

    fn select_primitive_op_id_via_command(
        &mut self,
        primitive_op_id: PrimitiveOpId,
        command: Command,
    ) {
        if let Some(selected_object_id) = self.selected_object_id {
            if let Some(selected_object) = self.object_collection.get_object(selected_object_id) {
                if let Some(_primitive_op) = selected_object.get_primitive_op(primitive_op_id) {
                    self.selected_primitive_op_id = Some(primitive_op_id);
                } else {
                    command_failed_warn(command, "invalid primitive op id");
                }
            } else {
                command_failed_warn(command, "selected object dropped");
            }
        } else {
            command_failed_warn(command, "no selected object");
        }
    }

    fn select_primitive_op_index_via_command(
        &mut self,
        primitive_op_index: usize,
        command: Command,
    ) {
        if let Some(selected_object_id) = self.selected_object_id {
            if let Some(selected_object) = self.object_collection.get_object(selected_object_id) {
                if let Some(primitive_op) = selected_object.primitive_ops.get(primitive_op_index) {
                    self.selected_primitive_op_id = Some(primitive_op.id());
                } else {
                    command_failed_warn(command, "invalid primitive op index");
                }
            } else {
                command_failed_warn(command, "selected object dropped");
            }
        } else {
            command_failed_warn(command, "no selected object");
        }
    }

    pub(super) fn deselect_primitive_op(&mut self) {
        self.selected_primitive_op_id = None;
    }

    fn remove_primitive_op_id_via_command(
        &mut self,
        primitive_op_id: PrimitiveOpId,
        command: Command,
    ) {
        if let Some(selected_object_id) = self.selected_object_id {
            if let Some(selected_object) = self.object_collection.get_object(selected_object_id) {
                let remove_res = selected_object.remove_primitive_op_id(primitive_op_id);
                match remove_res {
                    // this primitive op may have been currently selected, in which case we may have
                    // to select the primitive op next to it.
                    Ok(removed_index) => self.check_and_select_closest_primitive_op(
                        primitive_op_id,
                        removed_index,
                        selected_object,
                    ),
                    Err(_e) => command_failed_warn(command, "invalid primitive op id"),
                }
            } else {
                command_failed_warn(command, "selected object dropped");
            }
        } else {
            command_failed_warn(command, "no selected object");
        }
    }

    fn remove_primitive_op_index_via_command(
        &mut self,
        primitive_op_index: usize,
        command: Command,
    ) {
        if let Some(selected_object_id) = self.selected_object_id {
            if let Some(selected_object) = self.object_collection.get_object(selected_object_id) {
                let remove_res = selected_object.remove_primitive_op_index(primitive_op_index);
                match remove_res {
                    // this primitive op may have been currently selected, in which case we may have
                    // to select the primitive op next to it.
                    Ok(removed_id) => self.check_and_select_closest_primitive_op(
                        removed_id,
                        primitive_op_index,
                        selected_object,
                    ),
                    Err(_e) => command_failed_warn(command, "invalid primitive op id"),
                }
            } else {
                command_failed_warn(command, "selected object dropped");
            }
        } else {
            command_failed_warn(command, "no selected object");
        }
    }

    /// If a removed primitive op is currently selected, select a different primitive op with the
    /// closest index to the removed primitive op.
    fn check_and_select_closest_primitive_op(
        &mut self,
        removed_primitive_op_id: PrimitiveOpId,
        removed_primitive_op_index: usize,
        selected_object: &Object,
    ) {
        if let Some(selected_primitive_op_id) = self.selected_primitive_op_id {
            if selected_primitive_op_id == removed_primitive_op_id {
                self.select_primitive_op_with_closest_index(
                    &selected_object.primitive_ops,
                    removed_primitive_op_index,
                );
            }
        }
    }

    fn remove_selected_primitive_op_via_command(&mut self, command: Command) {
        if let Some(selected_primitive_op_id) = self.selected_primitive_op_id {
            self.remove_primitive_op_id_via_command(selected_primitive_op_id, command);
        } else {
            command_failed_warn(command, "no selected primitive op");
        }
    }

    /// Selects a primitive op in `self` from `primitive_ops` which has the closest index to
    /// `target_prim_op_index`. If `primitive_ops` is empty, deselects primitive op in `self`.
    pub fn select_primitive_op_with_closest_index(
        &mut self,
        primitive_ops: &Vec<PrimitiveOp>,
        target_prim_op_index: usize,
    ) {
        if let Some(select_index) = choose_closest_valid_index(primitive_ops, target_prim_op_index)
        {
            let select_primitive_op_id = primitive_ops[select_index].id();
            self.selected_primitive_op_id = Some(select_primitive_op_id)
        } else {
            self.deselect_primitive_op();
        }
    }
}

fn command_failed_warn(command: Command, failed_because: &'static str) {
    warn!("command {:?} failed due to {}", command, failed_because);
}
