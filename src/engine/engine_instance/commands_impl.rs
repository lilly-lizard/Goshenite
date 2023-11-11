use super::EngineInstance;
use crate::{
    engine::{
        commands::{Command, CommandWithSource, ValidationCommand},
        object::{
            object::{Object, ObjectId},
            primitive_op::{PrimitiveOp, PrimitiveOpId},
        },
    },
    helper::list::choose_closest_valid_index,
    user_interface::gui::Gui,
};
use glam::Vec3;
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
                Command::SelectPrimitiveOpId(object_id, primitive_op_id) => {
                    self.select_primitive_op_id_via_command(object_id, primitive_op_id, command)
                }
                Command::SelectPrimitiveOpIndex(object_id, primitive_op_index) => self
                    .select_primitive_op_index_via_command(object_id, primitive_op_index, command),
                Command::DeselectPrimtiveOp() => self.deselect_primitive_op(),
                Command::RemoveSelectedPrimitiveOp() => {
                    self.remove_selected_primitive_op_via_command(command);
                }
                Command::RemovePrimitiveOpId(object_id, primitive_op_id) => {
                    self.remove_primitive_op_id_via_command(object_id, primitive_op_id, command);
                }
                Command::RemovePrimitiveOpIndex(object_id, primitive_op_index) => {
                    self.remove_primitive_op_index_via_command(
                        object_id,
                        primitive_op_index,
                        command,
                    );
                }
                Command::RemovePrimitiveOpIdFromSelectedObject(primitive_op_id) => {
                    self.remove_primitive_op_id_from_selected_object_via_command(
                        primitive_op_id,
                        command,
                    );
                }
                Command::RemovePrimitiveOpIndexFromSelectedObject(primitive_op_index) => {
                    self.remove_primitive_op_index_from_selected_object_via_command(
                        primitive_op_index,
                        command,
                    );
                }

                Command::Validate(v_command) => self.execute_validation_command(v_command),
            }
        }
    }

    fn execute_validation_command(&mut self, v_command: ValidationCommand) {
        match v_command {
            ValidationCommand::SelectedObject() => self.validate_selected_object(),
        }
    }

    pub(super) fn validate_selected_object(&mut self) {
        if let Some(some_selected_object_id) = self.selected_object_id {
            let object_exists = self
                .object_collection
                .get_object(some_selected_object_id)
                .is_some();
            if !object_exists {
                self.selected_object_id = None;
            }
        }
    }

    pub(super) fn deselect_object(&mut self) {
        if self.selected_object_id.is_some() {
            self.gui.selected_object_changed();
        }
        self.selected_object_id = None;
        self.selected_primitive_op_id = None;
    }

    fn select_object_via_command(&mut self, object_id: ObjectId, command: Command) {
        if let Some(object) = self.object_collection.get_object(object_id) {
            self.select_object(object.id(), object.origin);
        } else {
            command_failed_warn(command, "invalid object id");
        }
    }

    /// Doesn't check validity of `object_id`. Ideally we'd pass a reference to the object here
    /// to account for this, but the borrow checker doesn't like that...
    pub(super) fn select_object(&mut self, object_id: ObjectId, object_origin: Vec3) {
        let mut selected_object_changed = true;
        if let Some(previously_selected_object_id) = self.selected_object_id {
            if previously_selected_object_id == object_id {
                // note: written this way to consider the case where it was None before
                selected_object_changed = false;
            }
        }

        self.selected_object_id = Some(object_id);
        self.camera.set_lock_on_target(object_origin.as_dvec3());

        if selected_object_changed {
            // if a different object is already selected, deselect the primitive op because it will
            // no longer be valid
            self.deselect_primitive_op();
            self.gui.selected_object_changed();
        }
    }

    pub(super) fn select_object_and_primitive_op(
        &mut self,
        object_id: ObjectId,
        primitive_op_index: usize,
    ) {
        let (object_origin, primitive_op) =
            if let Some(object) = self.object_collection.get_object(object_id) {
                let primitive_op =
                    if let Some(primitive_op) = object.primitive_ops.get(primitive_op_index) {
                        primitive_op.clone()
                    } else {
                        warn!(
                        "attempted to select primitive op index {} that doesn't exist in object {}",
                        primitive_op_index, object_id
                    );
                        return;
                    };

                (object.origin, primitive_op)
            } else {
                warn!(
                    "attempted to select object id {} that doesn't exist in object collection",
                    object_id
                );
                return;
            };

        self.select_object(object_id, object_origin);
        self.select_primitive_op(primitive_op);
    }

    #[inline]
    pub(super) fn select_primitive_op(&mut self, primitive_op: PrimitiveOp) {
        Self::select_primitive_op_without_self(
            &mut self.selected_primitive_op_id,
            &mut self.gui,
            primitive_op,
        );
    }

    /// Because E0499 is too fucking conservative with `self`.
    pub(super) fn select_primitive_op_without_self(
        selected_primitive_op_id: &mut Option<PrimitiveOpId>,
        gui: &mut Gui,
        primitive_op: PrimitiveOp,
    ) {
        if let Some(selected_primitive_op_id) = *selected_primitive_op_id {
            if selected_primitive_op_id == primitive_op.id() {
                // don't want to unnecessarily reset the saved gui state
                return;
            }
        }
        gui.primitive_op_selected(&primitive_op);
        *selected_primitive_op_id = Some(primitive_op.id());
    }

    fn remove_object_via_command(&mut self, object_id: ObjectId, command: Command) {
        let res = self.object_collection.remove_object(object_id);
        if let Err(_e) = res {
            command_failed_warn(command, "invalid object id");
        }

        if let Some(previously_selected_object_id) = self.selected_object_id {
            if previously_selected_object_id == object_id {
                self.deselect_object();
            }
        }
    }

    fn remove_selected_object_via_command(&mut self, command: Command) {
        if let Some(selected_object_id) = self.selected_object_id {
            let res = self.object_collection.remove_object(selected_object_id);
            if let Err(_e) = res {
                command_failed_warn(command, "selected object id invalid");
            }
            self.deselect_object();
        } else {
            command_failed_warn(command, "no selected object");
        }
    }

    fn select_primitive_op_id_via_command(
        &mut self,
        object_id: ObjectId,
        primitive_op_id: PrimitiveOpId,
        command: Command,
    ) {
        if let Some(selected_object) = self.object_collection.get_object(object_id) {
            if let Some((primitive_op, _index)) = selected_object.get_primitive_op(primitive_op_id)
            {
                self.select_primitive_op(primitive_op.clone());
            } else {
                command_failed_warn(command, "invalid primitive op id");
            }
        } else {
            command_failed_warn(command, "invalid object id");
        }
    }

    fn select_primitive_op_index_via_command(
        &mut self,
        object_id: ObjectId,
        primitive_op_index: usize,
        command: Command,
    ) {
        if let Some(selected_object) = self.object_collection.get_object(object_id) {
            if let Some(primitive_op) = selected_object.primitive_ops.get(primitive_op_index) {
                self.select_primitive_op(primitive_op.clone());
            } else {
                command_failed_warn(command, "invalid primitive op index");
            }
        } else {
            command_failed_warn(command, "invalid object id");
        }
    }

    pub(super) fn deselect_primitive_op(&mut self) {
        Self::deselect_primitive_op_without_self(&mut self.selected_primitive_op_id);
    }

    #[inline]
    /// Because E0499 is too fucking conservative with `self`.
    pub(super) fn deselect_primitive_op_without_self(
        selected_primitive_op_id: &mut Option<PrimitiveOpId>,
    ) {
        *selected_primitive_op_id = None;
    }

    fn remove_primitive_op_id_via_command(
        &mut self,
        object_id: ObjectId,
        primitive_op_id: PrimitiveOpId,
        command: Command,
    ) {
        if let Some(object) = self.object_collection.get_object_mut(object_id) {
            let remove_res = object.remove_primitive_op_id(primitive_op_id);
            match remove_res {
                // this primitive op may have been currently selected, in which case we may have
                // to select the primitive op next to it.
                Ok(removed_index) => Self::check_and_select_closest_primitive_op(
                    &mut self.selected_primitive_op_id,
                    &mut self.gui,
                    primitive_op_id,
                    removed_index,
                    object,
                ),
                Err(_e) => command_failed_warn(command, "invalid primitive op id"),
            }
        } else {
            command_failed_warn(command, "invalid object id");
        }
    }

    fn remove_primitive_op_id_from_selected_object_via_command(
        &mut self,
        primitive_op_id: PrimitiveOpId,
        command: Command,
    ) {
        if let Some(selected_object_id) = self.selected_object_id {
            self.remove_primitive_op_id_via_command(selected_object_id, primitive_op_id, command);
        } else {
            command_failed_warn(command, "no selected object");
        }
    }

    fn remove_primitive_op_index_via_command(
        &mut self,
        object_id: ObjectId,
        primitive_op_index: usize,
        command: Command,
    ) {
        if let Some(object) = self.object_collection.get_object_mut(object_id) {
            let remove_res = object.remove_primitive_op_index(primitive_op_index);
            match remove_res {
                // this primitive op may have been currently selected, in which case we may have
                // to select the primitive op next to it.
                Ok(removed_id) => Self::check_and_select_closest_primitive_op(
                    &mut self.selected_primitive_op_id,
                    &mut self.gui,
                    removed_id,
                    primitive_op_index,
                    object,
                ),
                Err(_e) => command_failed_warn(command, "invalid primitive op id"),
            }
        } else {
            command_failed_warn(command, "invalid object id");
        }
    }

    fn remove_primitive_op_index_from_selected_object_via_command(
        &mut self,
        primitive_op_index: usize,
        command: Command,
    ) {
        if let Some(selected_object_id) = self.selected_object_id {
            self.remove_primitive_op_index_via_command(
                selected_object_id,
                primitive_op_index,
                command,
            );
        } else {
            command_failed_warn(command, "no selected object");
        }
    }

    /// If a removed primitive op is currently selected, select a different primitive op with the
    /// closest index to the removed primitive op.
    pub(super) fn check_and_select_closest_primitive_op(
        selected_primitive_op_id: &mut Option<PrimitiveOpId>,
        gui: &mut Gui,
        removed_primitive_op_id: PrimitiveOpId,
        removed_primitive_op_index: usize,
        selected_object: &Object,
    ) {
        if let Some(some_selected_primitive_op_id) = *selected_primitive_op_id {
            if some_selected_primitive_op_id == removed_primitive_op_id {
                Self::select_primitive_op_with_closest_index(
                    selected_primitive_op_id,
                    gui,
                    &selected_object.primitive_ops,
                    removed_primitive_op_index,
                );
            }
        }
    }

    /// Selects a primitive op in `self` from `primitive_ops` which has the closest index to
    /// `target_prim_op_index`. If `primitive_ops` is empty, deselects primitive op in `self`.
    pub(super) fn select_primitive_op_with_closest_index(
        selected_primitive_op_id: &mut Option<PrimitiveOpId>,
        gui: &mut Gui,
        primitive_ops: &Vec<PrimitiveOp>,
        target_prim_op_index: usize,
    ) {
        if let Some(select_index) =
            choose_closest_valid_index(primitive_ops.len(), target_prim_op_index)
        {
            let primitive_op = primitive_ops[select_index].clone();
            Self::select_primitive_op_without_self(selected_primitive_op_id, gui, primitive_op);
        } else {
            Self::deselect_primitive_op_without_self(selected_primitive_op_id);
        }
    }

    fn remove_selected_primitive_op_via_command(&mut self, command: Command) {
        if let Some(selected_primitive_op_id) = self.selected_primitive_op_id {
            self.remove_primitive_op_id_from_selected_object_via_command(
                selected_primitive_op_id,
                command,
            );
        } else {
            command_failed_warn(command, "no selected primitive op");
        }
    }
}

fn command_failed_warn(command: Command, failed_because: &'static str) {
    warn!("command {:?} failed due to {}", command, failed_because);
}
