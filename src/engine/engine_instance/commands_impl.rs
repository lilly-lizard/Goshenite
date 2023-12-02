use super::EngineInstance;
use crate::{
    engine::{
        commands::{Command, CommandWithSource, ValidationCommand},
        object::{
            object::{Object, ObjectId},
            operation::Operation,
            primitive_op::{PrimitiveOp, PrimitiveOpId},
        },
        primitives::{primitive::Primitive, primitive_transform::PrimitiveTransform},
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
            self.execute_command(command);
        }
    }

    pub(super) fn execute_command(&mut self, command: Command) {
        match command {
            // camera
            Command::SetCameraLockOnPos(target_pos) => {
                self.camera.set_lock_on_target_pos(target_pos)
            }
            Command::SetCameraLockOnObject(object_id) => {
                self.set_camera_lock_on_object_via_command(object_id, command)
            }
            Command::UnsetCameraLockOn => self.camera.unset_lock_on_target(),
            Command::ResetCamera => self.camera.reset(),

            // object
            Command::SelectObject(object_id) => {
                self.select_object_via_command(object_id, command);
            }
            Command::DeselectObject() => self.deselect_object(),
            Command::RemoveObject(object_id) => self.remove_object_via_command(object_id, command),
            Command::RemoveSelectedObject() => self.remove_selected_object_via_command(command),
            Command::CreateAndSelectNewDefaultObject() => {
                self.create_and_select_new_default_object_via_command(command)
            }
            Command::SetObjectOrigin { object_id, origin } => {
                self.set_object_origin_via_command(object_id, origin, command)
            }
            Command::SetObjectName {
                object_id,
                ref new_name,
            } => self.set_object_name_via_command(object_id, new_name.clone(), command),

            // primtive op - selection
            Command::SelectPrimitiveOpId(object_id, primitive_op_id) => {
                self.select_primitive_op_id_via_command(object_id, primitive_op_id, command)
            }
            Command::SelectPrimitiveOpIndex(object_id, primitive_op_index) => {
                self.select_primitive_op_index_via_command(object_id, primitive_op_index, command)
            }
            Command::DeselectPrimtiveOp() => self.deselect_primitive_op(),

            // primitive op - remove
            Command::RemoveSelectedPrimitiveOp() => {
                self.remove_selected_primitive_op_via_command(command);
            }
            Command::RemovePrimitiveOpId(object_id, primitive_op_id) => {
                self.remove_primitive_op_id_via_command(object_id, primitive_op_id, command)
            }
            Command::RemovePrimitiveOpIndex(object_id, primitive_op_index) => {
                self.remove_primitive_op_index_via_command(object_id, primitive_op_index, command);
            }

            // primitive op - push
            Command::PushOp {
                object_id,
                operation,
                primitive,
            } => _ = self.push_op_via_command(object_id, operation, primitive, command),
            Command::PushOpAndSelect {
                object_id,
                operation,
                primitive,
            } => self.push_op_and_select_via_command(object_id, operation, primitive, command),

            // primitive op - modify
            Command::SetPrimitiveOp {
                object_id,
                primitive_op_id,
                new_primitive,
                new_transform,
                new_operation,
            } => self.set_primitive_op_via_command(
                object_id,
                primitive_op_id,
                Some(new_primitive),
                Some(new_transform),
                Some(new_operation),
                command,
            ),
            Command::SetPrimitive {
                object_id,
                primitive_op_id,
                new_primitive,
            } => self.set_primitive_op_via_command(
                object_id,
                primitive_op_id,
                Some(new_primitive),
                None,
                None,
                command,
            ),
            Command::SetPrimitiveTransform {
                object_id,
                primitive_op_id,
                new_transform,
            } => self.set_primitive_op_via_command(
                object_id,
                primitive_op_id,
                None,
                Some(new_transform),
                None,
                command,
            ),
            Command::SetOperation {
                object_id,
                primitive_op_id,
                new_operation,
            } => self.set_primitive_op_via_command(
                object_id,
                primitive_op_id,
                None,
                None,
                Some(new_operation),
                command,
            ),
            Command::ShiftPrimitiveOps {
                object_id,
                source_index,
                target_index,
            } => {
                self.shift_primitive_ops_via_command(
                    object_id,
                    source_index,
                    target_index,
                    command,
                );
            }

            Command::Validate(v_command) => self.execute_validation_command(v_command),
        }
    }

    // ~~ Camera ~~

    fn set_camera_lock_on_object_via_command(&mut self, object_id: ObjectId, command: Command) {
        let object = match self.object_collection.get_object(object_id) {
            Some(object) => object,
            None => {
                command_failed_warn(command, "invalid object id");
                return;
            }
        };

        self.camera
            .set_lock_on_target_object(object_id, object.origin);
    }

    // ~~ Object ~~

    pub(super) fn deselect_object(&mut self) {
        if self.selected_object_id.is_some() {
            self.gui.selected_object_changed();
        }
        self.selected_object_id = None;
        self.selected_primitive_op_id = None;
    }

    fn select_object_via_command(&mut self, object_id: ObjectId, command: Command) {
        if let Some(object) = self.object_collection.get_object(object_id) {
            self.select_object_unchecked(object.id(), object.origin);
        } else {
            command_failed_warn(command, "invalid object id");
        }
    }

    /// Doesn't check validity of `object_id`. Ideally we'd pass a reference to the object here
    /// to account for this, but the borrow checker doesn't like that...
    pub(super) fn select_object_unchecked(&mut self, object_id: ObjectId, object_origin: Vec3) {
        let mut selected_object_changed = true;
        if let Some(previously_selected_object_id) = self.selected_object_id {
            if previously_selected_object_id == object_id {
                selected_object_changed = false;
            }
        }

        self.selected_object_id = Some(object_id);
        self.camera
            .set_lock_on_target_object(object_id, object_origin);

        if selected_object_changed {
            // if a different object is already selected, deselect the primitive op because it will
            // no longer be valid
            self.deselect_primitive_op();
            self.gui.selected_object_changed();
        }
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

    fn create_and_select_new_default_object_via_command(&mut self, command: Command) {
        let new_object_res = self.object_collection.new_object_default();

        let (new_object_id, new_object) = match new_object_res {
            Ok(object_and_id) => object_and_id,
            Err(e) => {
                error!(
                    "the engine has run out of unique ids to assign to new objects.\
                    this case is not yet handled by goshenite!\
                    please report this as a bug..."
                );
                error!("command {:?} critially failed with error {}", command, e);
                return;
            }
        };

        let new_object_origin = new_object.origin;
        self.select_object_unchecked(new_object_id, new_object_origin);

        let _ = self
            .object_collection
            .mark_object_for_data_update(new_object_id);
    }

    fn set_object_origin_via_command(
        &mut self,
        object_id: ObjectId,
        origin: Vec3,
        command: Command,
    ) {
        let object = match self.object_collection.get_object_mut(object_id) {
            Some(object) => object,
            None => {
                command_failed_warn(command, "invalid object id");
                return;
            }
        };

        object.origin = origin;
        let _ = self
            .object_collection
            .mark_object_for_data_update(object_id);
    }

    fn set_object_name_via_command(
        &mut self,
        object_id: ObjectId,
        new_name: String,
        command: Command,
    ) {
        let object = match self.object_collection.get_object_mut(object_id) {
            Some(object) => object,
            None => {
                command_failed_warn(command, "invalid object id");
                return;
            }
        };

        object.name = new_name;
    }

    // ~~ Primitive Op ~~

    fn select_primitive_op_id_via_command(
        &mut self,
        object_id: ObjectId,
        primitive_op_id: PrimitiveOpId,
        command: Command,
    ) {
        let object = if let Some(object) = self.object_collection.get_object(object_id) {
            object
        } else {
            command_failed_warn(command, "invalid object id");
            return;
        };

        let primitive_op = if let Some(primitive_op) = object.get_primitive_op(primitive_op_id) {
            primitive_op.clone()
        } else {
            command_failed_warn(command, "invalid primitive op id");
            return;
        };

        Self::select_primitive_op_without_self(
            &mut self.selected_primitive_op_id,
            &mut self.gui,
            primitive_op.clone(),
        );
        self.select_object_unchecked(object_id, object.origin);
    }

    pub(super) fn select_object_and_primitive_op_id(
        &mut self,
        object_id: ObjectId,
        primitive_op_id: PrimitiveOpId,
    ) {
        let object = if let Some(object) = self.object_collection.get_object(object_id) {
            object
        } else {
            warn!(
                "attempted to select object id {} that doesn't exist in object collection",
                object_id
            );
            return;
        };

        let primitive_op = if let Some(primitive_op) = object.get_primitive_op(primitive_op_id) {
            primitive_op.clone()
        } else {
            warn!(
                "attempted to select primitive op id {} that doesn't exist in object {}",
                primitive_op_id, object_id
            );
            return;
        };

        Self::select_primitive_op_without_self(
            &mut self.selected_primitive_op_id,
            &mut self.gui,
            primitive_op.clone(),
        );
        self.select_object_unchecked(object_id, object.origin);
    }

    fn select_primitive_op_index_via_command(
        &mut self,
        object_id: ObjectId,
        primitive_op_index: usize,
        command: Command,
    ) {
        let object = if let Some(object) = self.object_collection.get_object(object_id) {
            object
        } else {
            command_failed_warn(command, "invalid object id");
            return;
        };

        let primitive_op = if let Some(primitive_op) = object.primitive_ops.get(primitive_op_index)
        {
            primitive_op.clone()
        } else {
            command_failed_warn(command, "invalid primitive op index");
            return;
        };

        self.select_object_unchecked(object_id, object.origin);
        self.select_primitive_op_unchecked(primitive_op);
    }

    pub(super) fn select_object_and_primitive_op_index(
        &mut self,
        object_id: ObjectId,
        primitive_op_index: usize,
    ) {
        let object = if let Some(object) = self.object_collection.get_object(object_id) {
            object
        } else {
            warn!(
                "attempted to select object id {} that doesn't exist in object collection",
                object_id
            );
            return;
        };

        let primitive_op = if let Some(primitive_op) = object.primitive_ops.get(primitive_op_index)
        {
            primitive_op.clone()
        } else {
            warn!(
                "attempted to select primitive op index {} that doesn't exist in object {}",
                primitive_op_index, object_id
            );
            return;
        };

        self.select_object_unchecked(object_id, object.origin);
        self.select_primitive_op_unchecked(primitive_op);
    }

    #[inline]
    pub(super) fn select_primitive_op_unchecked(&mut self, primitive_op: PrimitiveOp) {
        Self::select_primitive_op_without_self(
            &mut self.selected_primitive_op_id,
            &mut self.gui,
            primitive_op,
        );
    }

    /// Because E0499 is too fucking conservative with `self`.
    pub fn select_primitive_op_without_self(
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

    pub(super) fn deselect_primitive_op(&mut self) {
        Self::deselect_primitive_op_without_self(&mut self.selected_primitive_op_id);
    }

    #[inline]
    /// Because E0499 is too fucking conservative with `self`.
    pub fn deselect_primitive_op_without_self(
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
        let object = if let Some(object) = self.object_collection.get_object_mut(object_id) {
            object
        } else {
            command_failed_warn(command, "invalid object id");
            return;
        };

        let remove_res = object.remove_primitive_op_id(primitive_op_id);
        let removed_index = match remove_res {
            Ok(removed_index) => removed_index,
            Err(_e) => {
                command_failed_warn(command, "invalid primitive op id");
                return;
            }
        };

        // this primitive op may have been currently selected, in which case we may have
        // to select the primitive op next to it.
        Self::check_and_select_closest_primitive_op(
            &mut self.selected_primitive_op_id,
            &mut self.gui,
            primitive_op_id,
            removed_index,
            object,
        );

        let _ = self
            .object_collection
            .mark_object_for_data_update(object_id);
    }

    fn remove_primitive_op_index_via_command(
        &mut self,
        object_id: ObjectId,
        primitive_op_index: usize,
        command: Command,
    ) {
        let object = if let Some(object) = self.object_collection.get_object_mut(object_id) {
            object
        } else {
            command_failed_warn(command, "invalid object id");
            return;
        };

        let remove_res = object.remove_primitive_op_index(primitive_op_index);
        let removed_id = match remove_res {
            Ok(removed_id) => removed_id,
            Err(_e) => {
                command_failed_warn(command, "invalid primitive op id");
                return;
            }
        };

        // this primitive op may have been currently selected, in which case we may have
        // to select the primitive op next to it.
        Self::check_and_select_closest_primitive_op(
            &mut self.selected_primitive_op_id,
            &mut self.gui,
            removed_id,
            primitive_op_index,
            object,
        );

        let _ = self
            .object_collection
            .mark_object_for_data_update(object_id);
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

    fn push_op_and_select_via_command(
        &mut self,
        object_id: ObjectId,
        operation: Operation,
        primitive: Primitive,
        command: Command,
    ) {
        let push_op_res = self.push_op_via_command(object_id, operation, primitive, command);

        let new_primitive_op_id = match push_op_res {
            Some(id) => id,
            None => return,
        };
        self.select_object_and_primitive_op_id(object_id, new_primitive_op_id);
    }

    /// Returns the primitive op id
    fn push_op_via_command(
        &mut self,
        object_id: ObjectId,
        operation: Operation,
        primitive: Primitive,
        command: Command,
    ) -> Option<PrimitiveOpId> {
        let object = if let Some(some_object) = self.object_collection.get_object_mut(object_id) {
            some_object
        } else {
            command_failed_warn(command, "invalid object id");
            return None;
        };

        let push_op_res = object.push_op(operation, primitive);

        let new_primitive_op_id = match push_op_res {
            Err(e) => {
                let error_msg = e.to_string();
                command_failed_warn(command, &error_msg);
                return None;
            }
            Ok(id) => id,
        };

        let _ = self
            .object_collection
            .mark_object_for_data_update(object_id);

        Some(new_primitive_op_id)
    }

    fn set_primitive_op_via_command(
        &mut self,
        object_id: ObjectId,
        primitive_op_id: PrimitiveOpId,
        new_primitive: Option<Primitive>,
        new_transform: Option<PrimitiveTransform>,
        new_operation: Option<Operation>,
        command: Command,
    ) {
        let object_get_res = self.object_collection.get_object_mut(object_id);
        let object = match object_get_res {
            Some(object) => object,
            None => {
                command_failed_warn(command, "invalid object id");
                return;
            }
        };

        let set_primitive_op_res =
            object.set_primitive_op(primitive_op_id, new_primitive, new_transform, new_operation);

        if let Err(e) = set_primitive_op_res {
            let error_msg = e.to_string();
            command_failed_warn(command, &error_msg);
            return;
        }

        let _ = self
            .object_collection
            .mark_object_for_data_update(object_id);
    }

    fn shift_primitive_ops_via_command(
        &mut self,
        object_id: ObjectId,
        source_index: usize,
        target_index: usize,
        command: Command,
    ) {
        let object = if let Some(some_object) = self.object_collection.get_object_mut(object_id) {
            some_object
        } else {
            command_failed_warn(command, "invalid object id");
            return;
        };

        let shift_res = object.shift_primitive_ops(source_index, target_index);

        if let Err(e) = shift_res {
            let error_msg = e.to_string();
            command_failed_warn(command, &error_msg);
        }

        let _ = self
            .object_collection
            .mark_object_for_data_update(object_id);
    }

    // ~~ Internal ~~

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
}

fn command_failed_warn(command: Command, failed_because: &str) {
    warn!("command {:?} failed due to {}", command, failed_because);
}
