use super::EngineController;
use crate::{
    engine::{
        commands::{Command, CommandWithSource, TargetPrimitiveOp, ValidationCommand},
        object::{
            object::{Object, ObjectId},
            operation::Operation,
            primitive_op::{PrimitiveOp, PrimitiveOpId},
        },
        primitives::{primitive::Primitive, primitive_transform::PrimitiveTransform},
        save_states::{load_objects, load_state_camera, save_all_objects, save_state_camera},
    },
    helper::{
        list::choose_closest_valid_index, more_errors::CollectionError,
        unique_id_gen::UniqueIdError,
    },
    renderer::config_renderer::RenderOptions,
    user_interface::gui::Gui,
};
use glam::Vec3;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

// ~~ Commands ~~

impl EngineController {
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
            // ~~ Renderer ~~
            Command::SetRenderOptions(new_render_options) => {
                self.set_render_options(new_render_options);
            }

            // ~~ Save states ~~
            Command::SaveStateCamera => self.save_state_camera_via_command(command),
            Command::LoadStateCamera => self.load_state_camera_via_command(command),
            Command::SaveAllObjects => self.save_all_objects_via_command(command),
            Command::LoadObjects => self.load_objects_via_command(command),

            // ~~ Camera ~~
            Command::SetCameraLockOnPos(target_pos) => {
                self.camera.set_lock_on_target_pos(target_pos)
            }
            Command::SetCameraLockOnObject(object_id) => {
                self.set_camera_lock_on_object_via_command(object_id, command)
            }
            Command::UnsetCameraLockOn => self.camera.unset_lock_on_target(),
            Command::ResetCamera => self.camera.reset(),

            // ~~ Object ~~
            Command::SelectObject(object_id) => {
                self.select_object(object_id, Some(command));
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

            // ~~ Primtive Op: Selection ~~
            Command::SelectPrimitiveOp(target_primitive_op) => {
                self.select_primitive_op_and_object(target_primitive_op, Some(command))
            }
            Command::DeselectPrimtiveOp() => self.deselect_primitive_op(),

            // ~~ Primitive Op: Remove ~~
            Command::RemovePrimitiveOp(target_primitive_op) => {
                self.remove_primitive_op(target_primitive_op, Some(command))
            }

            // ~~ Primitive Op: Push ~~
            Command::PushPrimitiveOp {
                object_id,
                primitive,
                transform,
                operation,
                blend,
                albedo,
                specular,
            } => {
                _ = self.push_op_via_command(
                    object_id, primitive, transform, operation, blend, albedo, specular, command,
                )
            }
            Command::PushPrimitiveOpAndSelect {
                object_id,
                primitive,
                transform,
                operation,
                blend,
                albedo,
                specular,
            } => self.push_op_and_select_via_command(
                object_id, primitive, transform, operation, blend, albedo, specular, command,
            ),

            // ~~ Primitive Op: Modify ~~
            Command::SetPrimitiveOp {
                target_primitive_op,
                new_primitive,
                new_transform,
                new_operation,
                new_blend,
                new_albedo,
                new_specular,
            } => self.set_primitive_op(
                target_primitive_op,
                Some(new_primitive),
                Some(new_transform),
                Some(new_operation),
                Some(new_blend),
                Some(new_albedo),
                Some(new_specular),
                Some(command),
            ),
            Command::SetPrimitive {
                target_primitive_op,
                new_primitive,
            } => self.set_primitive_op(
                target_primitive_op,
                Some(new_primitive),
                None,
                None,
                None,
                None,
                None,
                Some(command),
            ),
            Command::SetPrimitiveTransform {
                target_primitive_op,
                new_transform,
            } => self.set_primitive_op(
                target_primitive_op,
                None,
                Some(new_transform),
                None,
                None,
                None,
                None,
                Some(command),
            ),
            Command::SetOperation {
                target_primitive_op,
                new_operation,
            } => self.set_primitive_op(
                target_primitive_op,
                None,
                None,
                Some(new_operation),
                None,
                None,
                None,
                Some(command),
            ),
            Command::SetBlend {
                target_primitive_op,
                new_blend,
            } => self.set_primitive_op(
                target_primitive_op,
                None,
                None,
                None,
                Some(new_blend),
                None,
                None,
                Some(command),
            ),
            Command::SetAlbedo {
                target_primitive_op,
                new_albedo,
            } => self.set_primitive_op(
                target_primitive_op,
                None,
                None,
                None,
                None,
                Some(new_albedo),
                None,
                Some(command),
            ),
            Command::SetSpecular {
                target_primitive_op,
                new_specular,
            } => self.set_primitive_op(
                target_primitive_op,
                None,
                None,
                None,
                None,
                None,
                Some(new_specular),
                Some(command),
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
}

// ~~ Per-Command Processing ~~

impl EngineController {
    // ~~ Renderer ~~

    fn set_render_options(&mut self, new_render_options: RenderOptions) {
        self.render_options = new_render_options;
    }

    // ~~ Save states ~~

    fn save_state_camera_via_command(&self, command: Command) {
        let save_state_res = save_state_camera(&self.camera);
        if let Err(e) = save_state_res {
            let failed_because = format!("error while saving camera state: {}", e);
            command_failed_warn(command, &failed_because);
        }
    }

    fn load_state_camera_via_command(&mut self, command: Command) {
        let load_state_res = load_state_camera();
        let loaded_camera = match load_state_res {
            Ok(c) => c,
            Err(e) => {
                let failed_because = format!("error while loading saved camera state: {}", e);
                command_failed_warn(command, &failed_because);
                return;
            }
        };
        self.camera = loaded_camera;
    }

    fn save_all_objects_via_command(&self, command: Command) {
        let save_state_res = save_all_objects(&self.object_collection);
        if let Err(e) = save_state_res {
            let failed_because = format!("error while saving objects: {}", e);
            command_failed_warn(command, &failed_because);
        }
    }

    fn load_objects_via_command(&mut self, command: Command) {
        let load_state_res = load_objects();
        let loaded_objects = match load_state_res {
            Ok(o) => o,
            Err(e) => {
                let failed_because = format!("error while loading saved objects: {}", e);
                command_failed_warn(command, &failed_because);
                return;
            }
        };

        let insert_objects_res = self.object_collection.push_objects(loaded_objects);
        if let Err(e) = insert_objects_res {
            let failed_because = format!("error while inserting loaded objects: {}", e);
            command_failed_warn(command, &failed_because);
        }
    }

    // ~~ Camera ~~

    fn set_camera_lock_on_object_via_command(
        &mut self,
        target_object_id: ObjectId,
        command: Command,
    ) {
        let Some(object) = self.object_collection.get_object(target_object_id) else {
            failure_warn_invalid_object_id(target_object_id, Some(command));
            return;
        };

        self.camera
            .set_lock_on_target_object(target_object_id, object.origin);
    }

    // ~~ Object ~~

    fn deselect_object(&mut self) {
        if self.selected_object_id.is_some() {
            self.gui.selected_object_changed();
        }
        self.selected_object_id = None;
        self.selected_primitive_op_id = None;
    }

    pub(super) fn select_object(
        &mut self,
        object_id_to_select: ObjectId,
        command: Option<Command>,
    ) {
        if self
            .object_collection
            .get_object(object_id_to_select)
            .is_some()
        {
            self.select_object_unchecked(object_id_to_select);
        } else {
            failure_warn_invalid_object_id(object_id_to_select, command);
        }
    }

    /// Doesn't check validity of `object_id`. Ideally we'd pass a reference to the object here
    /// to account for this, but the borrow checker doesn't like that...
    fn select_object_unchecked(&mut self, object_id_to_select: ObjectId) {
        let mut selected_object_changed = true;
        if let Some(previously_selected_object_id) = self.selected_object_id {
            if previously_selected_object_id == object_id_to_select {
                selected_object_changed = false;
            }
        }

        self.selected_object_id = Some(object_id_to_select);

        if selected_object_changed {
            // if a different object is already selected, deselect the primitive op because it will
            // no longer be valid
            self.deselect_primitive_op();
            self.gui.selected_object_changed();
        }
    }

    fn remove_object_via_command(&mut self, object_id_to_remove: ObjectId, command: Command) {
        let res = self.object_collection.remove_object(object_id_to_remove);
        if let Err(_e) = res {
            failure_warn_invalid_object_id(object_id_to_remove, Some(command));
        }

        if let Some(previously_selected_object_id) = self.selected_object_id {
            if previously_selected_object_id == object_id_to_remove {
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

        let (new_object_id, _) = match new_object_res {
            Ok(object_and_id) => object_and_id,
            Err(e) => {
                failure_warn_unique_id_error(Some(command), e);
                return;
            }
        };

        self.select_object_unchecked(new_object_id);
    }

    fn set_object_origin_via_command(
        &mut self,
        object_id: ObjectId,
        new_origin: Vec3,
        command: Command,
    ) {
        let update_res = self
            .object_collection
            .set_object_origin(object_id, new_origin);
        if let Err(_) = update_res {
            failure_warn_invalid_object_id(object_id, Some(command));
        }
    }

    fn set_object_name_via_command(
        &mut self,
        object_id: ObjectId,
        new_name: String,
        command: Command,
    ) {
        let update_res = self.object_collection.set_object_name(object_id, new_name);
        if let Err(_) = update_res {
            failure_warn_invalid_object_id(object_id, Some(command));
        }
    }

    // ~~ Primtive Op: Selection ~~

    pub(super) fn select_primitive_op_and_object(
        &mut self,
        target_primitive_op: TargetPrimitiveOp,
        source_command: Option<Command>,
    ) {
        let object_id = match target_primitive_op {
            TargetPrimitiveOp::Id(object_id, _) => object_id,
            TargetPrimitiveOp::Index(object_id, _) => object_id,
            TargetPrimitiveOp::Selected => {
                failure_warn_already_selected(source_command);
                return;
            }
        };

        let Some(object) = self.object_collection.get_object(object_id) else {
            failure_warn_invalid_object_id(object_id, source_command);
            return;
        };

        let primitive_op = match target_primitive_op {
            TargetPrimitiveOp::Id(_, primitive_op_id) => {
                let get_res = object.get_primitive_op(primitive_op_id);
                match get_res {
                    Some(primitive_op) => primitive_op.clone(),
                    None => {
                        failure_warn_invalid_primitive_op_id(
                            object_id,
                            primitive_op_id,
                            source_command,
                        );
                        return;
                    }
                }
            }
            TargetPrimitiveOp::Index(_, primitive_op_index) => {
                let get_res = object.primitive_ops.get(primitive_op_index);
                match get_res {
                    Some(primitive_op) => primitive_op.clone(),
                    None => {
                        failure_warn_invalid_primitive_op_index(
                            object_id,
                            primitive_op_index,
                            source_command,
                        );
                        return;
                    }
                }
            }
            TargetPrimitiveOp::Selected => unreachable!("returned for this case at start of fn"),
        };

        self.select_object_unchecked(object_id);
        self.select_primitive_op_unchecked(primitive_op);
    }

    #[inline]
    /// Convenience fn for `select_primitive_op_without_self``
    fn select_primitive_op_unchecked(&mut self, primitive_op_to_select: PrimitiveOp) {
        Self::select_primitive_op_without_self(
            &mut self.selected_primitive_op_id,
            &mut self.gui,
            primitive_op_to_select,
        );
    }

    /// Because E0499 is too fucking conservative with `self`.
    fn select_primitive_op_without_self(
        selected_primitive_op_id: &mut Option<PrimitiveOpId>,
        gui: &mut Gui,
        primitive_op_to_select: PrimitiveOp,
    ) {
        if let Some(selected_primitive_op_id) = *selected_primitive_op_id {
            if selected_primitive_op_id == primitive_op_to_select.id() {
                // don't want to unnecessarily reset the saved gui state
                return;
            }
        }

        gui.primitive_op_selected(&primitive_op_to_select);
        *selected_primitive_op_id = Some(primitive_op_to_select.id());
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

    // ~~ Primitive Op: Remove ~~

    fn remove_primitive_op(
        &mut self,
        target_primitive_op: TargetPrimitiveOp,
        source_command: Option<Command>,
    ) {
        let Some(object_id) =
            self.object_id_from_target_primitive_op(target_primitive_op, source_command.clone())
        else {
            failure_warn_no_selected_object(source_command);
            return;
        };

        // check early to ensure if `remove_primitive_op_id_from_object` or `failure_warn_invalid_primitive_op_index`
        // fails it is because of invalid primitive op id/index
        if let None = self.object_collection.get_object(object_id) {
            failure_warn_invalid_object_id(object_id, source_command);
            return;
        };

        let (removed_id, removed_index) = match target_primitive_op {
            TargetPrimitiveOp::Id(_, primitive_op_id) => {
                let remove_res = self
                    .object_collection
                    .remove_primitive_op_id_from_object(object_id, primitive_op_id);
                let Ok(removed_index) = remove_res else {
                    failure_warn_invalid_primitive_op_id(
                        object_id,
                        primitive_op_id,
                        source_command,
                    );
                    return;
                };
                (primitive_op_id, removed_index)
            }
            TargetPrimitiveOp::Index(_, primitive_op_index) => {
                let remove_res = self
                    .object_collection
                    .remove_primitive_op_index_from_object(object_id, primitive_op_index);
                let Ok(removed_id) = remove_res else {
                    failure_warn_invalid_primitive_op_index(
                        object_id,
                        primitive_op_index,
                        source_command,
                    );
                    return;
                };
                (removed_id, primitive_op_index)
            }
            TargetPrimitiveOp::Selected => match self.selected_primitive_op_id {
                Some(primitive_op_id) => {
                    let remove_res = self
                        .object_collection
                        .remove_primitive_op_id_from_object(object_id, primitive_op_id);
                    let Ok(removed_index) = remove_res else {
                        failure_warn_invalid_primitive_op_id(
                            object_id,
                            primitive_op_id,
                            source_command,
                        );
                        self.selected_primitive_op_id = None;
                        return;
                    };
                    (primitive_op_id, removed_index)
                }
                None => {
                    failure_warn_no_selected_primitive_op(source_command);
                    return;
                }
            },
        };

        if !self.is_object_id_selected(object_id) {
            return;
        }

        // this primitive op may have been currently selected, in which case we may have
        // to select the primitive op next to it.
        let updated_object = self
            .object_collection
            .get_object(object_id)
            .expect("checked that object id is valid at beginning of fn");
        Self::check_and_select_closest_primitive_op(
            &mut self.selected_primitive_op_id,
            &mut self.gui,
            removed_id,
            removed_index,
            updated_object,
        );
    }

    /// If a removed primitive op is currently selected, select a different primitive op with the
    /// closest index to the removed primitive op.
    fn check_and_select_closest_primitive_op(
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
    fn select_primitive_op_with_closest_index(
        selected_primitive_op_id: &mut Option<PrimitiveOpId>,
        gui: &mut Gui,
        primitive_op_list: &Vec<PrimitiveOp>,
        target_prim_op_index: usize,
    ) {
        if let Some(select_index) =
            choose_closest_valid_index(primitive_op_list.len(), target_prim_op_index)
        {
            let primitive_op = primitive_op_list[select_index].clone();
            Self::select_primitive_op_without_self(selected_primitive_op_id, gui, primitive_op);
        } else {
            Self::deselect_primitive_op_without_self(selected_primitive_op_id);
        }
    }

    // ~~ Primitive Op: Push ~~

    fn push_op_and_select_via_command(
        &mut self,
        object_id: ObjectId,
        primitive: Primitive,
        transform: PrimitiveTransform,
        operation: Operation,
        blend: f32,
        albedo: Vec3,
        specular: f32,
        command: Command,
    ) {
        let push_op_res = self.push_op_via_command(
            object_id,
            primitive,
            transform,
            operation,
            blend,
            albedo,
            specular,
            command.clone(),
        );

        let new_primitive_op_id = match push_op_res {
            Some(id) => id,
            None => return,
        };
        self.select_primitive_op_and_object(
            TargetPrimitiveOp::Id(object_id, new_primitive_op_id),
            Some(command),
        );
    }

    /// Returns the primitive op id
    fn push_op_via_command(
        &mut self,
        object_id: ObjectId,
        primitive: Primitive,
        transform: PrimitiveTransform,
        operation: Operation,
        blend: f32,
        albedo: Vec3,
        specular: f32,
        command: Command,
    ) -> Option<PrimitiveOpId> {
        let push_op_res = self.object_collection.push_op_to_object(
            object_id, primitive, transform, operation, blend, albedo, specular,
        );
        match push_op_res {
            Ok(primitive_op_id) => Some(primitive_op_id),
            Err(collection_error) => {
                match collection_error {
                    CollectionError::InvalidId { .. } => {
                        failure_warn_invalid_object_id(object_id, Some(command))
                    }
                    CollectionError::UniqueIdError(unique_id_error) => {
                        failure_warn_unique_id_error(Some(command), unique_id_error)
                    }
                    _ => (),
                }
                None
            }
        }
    }

    // ~~ Primitive Op: Modify ~~

    fn set_primitive_op(
        &mut self,
        target_primitive_op: TargetPrimitiveOp,
        new_primitive: Option<Primitive>,
        new_transform: Option<PrimitiveTransform>,
        new_operation: Option<Operation>,
        new_blend: Option<f32>,
        new_albedo: Option<Vec3>,
        new_specular: Option<f32>,
        source_command: Option<Command>,
    ) {
        let Some(object_id) =
            self.object_id_from_target_primitive_op(target_primitive_op, source_command.clone())
        else {
            failure_warn_no_selected_object(source_command);
            return;
        };

        // check early to ensure if `remove_primitive_op_id_from_object` or `failure_warn_invalid_primitive_op_index`
        // fails it is because of invalid primitive op id/index
        if let None = self.object_collection.get_object(object_id) {
            failure_warn_invalid_object_id(object_id, source_command);
            return;
        };

        match target_primitive_op {
            TargetPrimitiveOp::Id(_, primitive_op_id) => {
                let set_res = self.object_collection.set_primitive_op_id_in_object(
                    object_id,
                    primitive_op_id,
                    new_primitive,
                    new_transform,
                    new_operation,
                    new_blend,
                    new_albedo,
                    new_specular,
                );
                if let Err(_) = set_res {
                    failure_warn_invalid_primitive_op_id(
                        object_id,
                        primitive_op_id,
                        source_command,
                    );
                }
            }
            TargetPrimitiveOp::Index(_, primitive_op_index) => {
                let set_res = self.object_collection.set_primitive_op_index_in_object(
                    object_id,
                    primitive_op_index,
                    new_primitive,
                    new_transform,
                    new_operation,
                    new_blend,
                    new_albedo,
                    new_specular,
                );
                if let Err(_) = set_res {
                    failure_warn_invalid_primitive_op_index(
                        object_id,
                        primitive_op_index,
                        source_command,
                    );
                }
            }
            TargetPrimitiveOp::Selected => match self.selected_primitive_op_id {
                Some(primitive_op_id) => {
                    let set_res = self.object_collection.set_primitive_op_id_in_object(
                        object_id,
                        primitive_op_id,
                        new_primitive,
                        new_transform,
                        new_operation,
                        new_blend,
                        new_albedo,
                        new_specular,
                    );
                    if let Err(_) = set_res {
                        failure_warn_invalid_primitive_op_id(
                            object_id,
                            primitive_op_id,
                            source_command,
                        );
                        self.selected_primitive_op_id = None;
                    }
                }
                None => {
                    failure_warn_no_selected_primitive_op(source_command);
                }
            },
        }
    }

    fn shift_primitive_ops_via_command(
        &mut self,
        object_id: ObjectId,
        source_index: usize,
        target_index: usize,
        command: Command,
    ) {
        // check early to ensure that later failure is because of invalid primitive op indices
        if let None = self.object_collection.get_object(object_id) {
            failure_warn_invalid_object_id(object_id, Some(command));
            return;
        };

        let shift_res = self.object_collection.shift_primitive_ops_in_object(
            object_id,
            source_index,
            target_index,
        );
        if let Err(e) = shift_res {
            let error_msg = e.to_string();
            command_failed_warn(command, &error_msg);
        }
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

    // ~~ Misc Helper Functions ~~

    fn object_id_from_target_primitive_op(
        &mut self,
        target_primitive_op: TargetPrimitiveOp,
        source_command: Option<Command>,
    ) -> Option<ObjectId> {
        let object_id = match target_primitive_op {
            TargetPrimitiveOp::Id(object_id, _) => object_id,
            TargetPrimitiveOp::Index(object_id, _) => object_id,
            TargetPrimitiveOp::Selected => match self.selected_object_id {
                Some(object_id) => object_id,
                None => {
                    failure_warn_no_selected_object(source_command);
                    return None;
                }
            },
        };
        Some(object_id)
    }
}

// ~~ Failed Command Handling ~~

fn command_failed_warn(command: Command, failed_because: &str) {
    warn!("command {:?} failed due to: {}", command, failed_because);
}

fn command_failed_error(command: Command, failed_because: &str) {
    error!(
        "command {:?} critically failed due to: {}",
        command, failed_because
    );
}

fn failure_warn_already_selected(source_command: Option<Command>) {
    if let Some(some_command) = source_command {
        command_failed_warn(some_command, "selecting the selected primitive op is NOP");
    } else {
        warn!("selecting the selected primitive op is NOP");
    }
}

fn failure_warn_invalid_object_id(object_id: ObjectId, source_command: Option<Command>) {
    if let Some(some_command) = source_command {
        command_failed_warn(some_command, "invalid object id");
    } else {
        warn!(
            "attempted to modify object id {} that doesn't exist in object collection",
            object_id
        );
    }
}

fn failure_warn_invalid_primitive_op_id(
    object_id: ObjectId,
    primitive_op_id: PrimitiveOpId,
    source_command: Option<Command>,
) {
    if let Some(some_command) = source_command {
        command_failed_warn(some_command, "invalid primitive op id");
    } else {
        warn!(
            "attempted to modify primitive op id {} that doesn't exist in object {}",
            primitive_op_id, object_id
        );
    }
}

fn failure_warn_invalid_primitive_op_index(
    object_id: ObjectId,
    primitive_op_index: usize,
    source_command: Option<Command>,
) {
    if let Some(some_command) = source_command {
        command_failed_warn(some_command, "invalid primitive op index");
    } else {
        warn!(
            "attempted to modify primitive op index {} that doesn't exist in object {}",
            primitive_op_index, object_id
        );
    }
}

fn failure_warn_no_selected_object(source_command: Option<Command>) {
    if let Some(some_command) = source_command {
        command_failed_warn(some_command, "no object is currently selected");
    } else {
        warn!("attempting to modify selected object when no object is currently selected");
    }
}

fn failure_warn_no_selected_primitive_op(source_command: Option<Command>) {
    if let Some(some_command) = source_command {
        command_failed_warn(some_command, "no primitive op is currently selected");
    } else {
        warn!(
            "attempting to modify selected primitive op when no primitive op is currently selected"
        );
    }
}

fn failure_warn_unique_id_error(source_command: Option<Command>, unique_id_error: UniqueIdError) {
    let failed_because = format!(
        "The engine has run out of unique ids to assign to new objects.\
        This case is not yet handled by goshenite!\
        Please report this as a bug...\n
        Returned error: {}",
        unique_id_error
    );
    if let Some(some_command) = source_command {
        command_failed_warn(some_command, &failed_because);
    } else {
        warn!("{}", failed_because);
    }
}
