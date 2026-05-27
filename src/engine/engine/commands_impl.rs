use super::Engine;
use crate::{
    engine::{
        commands::{Command, ValidationCommand},
        object::{
            object::{Object, ObjectId},
            operation::Operation,
            primitive_op::{PrimitiveOp, PrimitiveOpIndex},
        },
        primitives::{primitive::Primitive, primitive_transform::PrimitiveTransform},
        save_states::{load_objects, load_state_camera, save_all_objects, save_state_camera},
    },
    helper::{
        list::choose_closest_valid_index, more_errors::CollectionError,
        unique_id_gen::UniqueIdError,
    },
};
use glam::Vec3;
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
            Command::SaveStateCamera => self.save_state_camera(command),
            Command::LoadStateCamera => self.load_state_camera(command),
            Command::SaveScene => self.save_all_objects(command),
            Command::LoadScene => self.load_objects(command),

            // ~~ Object ~~
            Command::SelectObject(object_id) => {
                self.select_object(object_id, Some(command));
            }
            Command::DeselectObject() => self.deselect_object(),
            Command::RemoveObject(object_id) => self.remove_object(object_id, Some(command)),
            Command::RemoveSelectedObject() => self.remove_selected_object(Some(command)),
            Command::CreateAndSelectNewDefaultObject() => {
                self.create_and_select_new_default_object(Some(command))
            }
            Command::SetObjectCenter { object_id, center } => {
                self.set_object_center(object_id, center, Some(command))
            }
            Command::SetObjectName {
                object_id,
                ref new_name,
            } => self.set_object_name(object_id, new_name.clone(), Some(command)),

            // ~~ Primtive Op: Selection ~~
            Command::SelectPrimitiveOp(object_id, primitive_op_index) => {
                self.select_primitive_op(object_id, primitive_op_index, Some(command))
            }
            Command::DeselectPrimtiveOp() => self.deselect_primitive_op(),

            // ~~ Primitive Op: Remove ~~
            Command::RemovePrimitiveOp(object_id, primitive_op_index) => {
                self.remove_primitive_op(object_id, primitive_op_index, Some(command))
            }
            Command::RemoveSelectedPrimitiveOp() => {
                self.remove_selected_primitive_op(Some(command))
            }

            // ~~ Primitive Op: Push ~~
            Command::PushPrimitiveOp {
                object_id,
                primitive_op,
            } => _ = self.push_op(object_id, primitive_op, Some(command)),
            Command::PushPrimitiveOpAndSelect {
                object_id,
                primitive_op,
            } => self.push_op_and_select(object_id, primitive_op, Some(command)),

            // ~~ Primitive Op: Modify ~~
            Command::UpdatePrimitiveOp {
                object_id,
                primitive_op_index,
                new_primitive_op,
            } => self.update_primitive_op(
                object_id,
                primitive_op_index,
                new_primitive_op,
                Some(command),
            ),
            Command::UpdatePrimitive {
                object_id,
                primitive_op_index,
                new_primitive,
            } => self.update_primitive_op_fields(
                object_id,
                primitive_op_index,
                Some(new_primitive),
                None,
                None,
                None,
                None,
                None,
                Some(command),
            ),
            Command::UpdatePrimitiveTransform {
                object_id,
                primitive_op_index,
                new_transform,
            } => self.update_primitive_op_fields(
                object_id,
                primitive_op_index,
                None,
                Some(new_transform),
                None,
                None,
                None,
                None,
                Some(command),
            ),
            Command::UpdateOperation {
                object_id,
                primitive_op_index,
                new_operation,
            } => self.update_primitive_op_fields(
                object_id,
                primitive_op_index,
                None,
                None,
                Some(new_operation),
                None,
                None,
                None,
                Some(command),
            ),
            Command::UpdateBlend {
                object_id,
                primitive_op_index,
                new_blend,
            } => self.update_primitive_op_fields(
                object_id,
                primitive_op_index,
                None,
                None,
                None,
                Some(new_blend),
                None,
                None,
                Some(command),
            ),
            Command::UpdateAlbedo {
                object_id,
                primitive_op_index,
                new_albedo,
            } => self.update_primitive_op_fields(
                object_id,
                primitive_op_index,
                None,
                None,
                None,
                None,
                Some(new_albedo),
                None,
                Some(command),
            ),
            Command::UpdateSpecular {
                object_id,
                primitive_op_index,
                new_specular,
            } => self.update_primitive_op_fields(
                object_id,
                primitive_op_index,
                None,
                None,
                None,
                None,
                None,
                Some(new_specular),
                Some(command),
            ),
            Command::ReOrderPrimitiveOp {
                object_id,
                original_index,
                target_index,
            } => {
                self.re_order_primitive_op(object_id, original_index, target_index, command);
            }

            Command::Validate(v_command) => self.execute_validation_command(v_command),
        }
    }
}

// ~~ Per-Command Processing ~~

impl Engine {
    // ~~ Save states ~~

    fn save_state_camera(&self, command: Command) {
        let save_state_res = save_state_camera(&self.controllers.camera);
        if let Err(e) = save_state_res {
            let failed_because = format!("error while saving camera state: {}", e);
            command_failed_warn(command, &failed_because);
        }
    }

    fn load_state_camera(&mut self, command: Command) {
        let load_state_res = load_state_camera();
        let loaded_camera = match load_state_res {
            Ok(c) => c,
            Err(e) => {
                let failed_because = format!("error while loading saved camera state: {}", e);
                command_failed_warn(command, &failed_because);
                return;
            }
        };
        self.controllers.camera = loaded_camera;
    }

    fn save_all_objects(&self, command: Command) {
        let save_state_res = save_all_objects(&self.object_collection);
        if let Err(e) = save_state_res {
            let failed_because = format!("error while saving objects: {}", e);
            command_failed_warn(command, &failed_because);
        }
    }

    fn load_objects(&mut self, command: Command) {
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

    // ~~ Objects: Selection ~~

    pub(super) fn deselect_object(&mut self) {
        self.selected_object_id = None;
        self.selected_primitive_op_index = None;
        self.gizmo_visibility.hide_all();
        self.settings.camera.object_deselected();
    }

    /// Doesn't deselect object
    pub(super) fn deselect_primitive_op(&mut self) {
        self.selected_primitive_op_index = None;
        self.gizmo_visibility.hide_all();
        self.settings.camera.primitive_op_deselected();
    }

    pub(super) fn select_primitive_op(
        &mut self,
        object_id_to_select: ObjectId,
        primitive_op_index_to_select: PrimitiveOpIndex,
        source_command: Option<Command>,
    ) {
        let Ok(object) = self.object_collection.get_object(object_id_to_select) else {
            failure_warn_invalid_object_id(object_id_to_select, source_command);
            return;
        };
        let Some(primitive_op) = object.primitive_ops.get(primitive_op_index_to_select) else {
            failure_warn_invalid_primitive_op_index(
                object_id_to_select,
                primitive_op_index_to_select,
                source_command,
            );
            return;
        };

        // check if already selected
        if let Some(selected_object_id) = self.selected_object_id {
            if let Some(selected_primitive_op_index) = self.selected_primitive_op_index {
                if selected_object_id == object_id_to_select
                    && selected_primitive_op_index == primitive_op_index_to_select
                {
                    // don't want to unnecessarily reset gui state
                    return;
                }
            }
        }

        self.selected_object_id = Some(object_id_to_select);
        self.selected_primitive_op_index = Some(primitive_op_index_to_select);

        self.gizmo_visibility.show_all();
        // note: render_manager.update_gizmo_center not called here
        self.controllers
            .gui
            .update_selected_primitive_op(&primitive_op);
        self.settings.camera.primitive_op_selected();
    }

    /// Also deselects primitive op
    pub(super) fn select_object(
        &mut self,
        object_id_to_select: ObjectId,
        source_command: Option<Command>,
    ) {
        if let Err(_e) = self.object_collection.get_object(object_id_to_select) {
            failure_warn_invalid_object_id(object_id_to_select, source_command);
            return;
        };

        self.selected_object_id = Some(object_id_to_select);
        self.selected_primitive_op_index = None;

        self.gizmo_visibility.show_all();
        // note: render_manager.update_gizmo_center not called here
        self.settings.camera.object_selected();
    }

    // ~~ Objects: Removal ~~

    pub(super) fn remove_object(
        &mut self,
        object_id_to_remove: ObjectId,
        source_command: Option<Command>,
    ) {
        let res = self.object_collection.remove_object(object_id_to_remove);
        if let Err(e) = res {
            failure_warn_collection_error(e, object_id_to_remove, source_command);
        }

        if let Some(previously_selected_object_id) = self.selected_object_id {
            if previously_selected_object_id == object_id_to_remove {
                self.deselect_object();
            }
        }
    }

    pub(super) fn remove_selected_object(&mut self, source_command: Option<Command>) {
        let Some(selected_object_id) = self.selected_object_id else {
            failure_warn_no_selected_object(source_command);
            return;
        };
        let res = self.object_collection.remove_object(selected_object_id);
        if let Err(e) = res {
            failure_warn_collection_error(e, selected_object_id, source_command);
        }
        self.deselect_object();
    }

    pub(super) fn remove_primitive_op(
        &mut self,
        object_id: ObjectId,
        primitive_op_index: PrimitiveOpIndex,
        source_command: Option<Command>,
    ) {
        let remove_res = self
            .object_collection
            .remove_primitive_op_from_object(object_id, primitive_op_index);
        if let Err(e) = remove_res {
            failure_warn_collection_error(e, object_id, source_command);
            return;
        };

        if !self.is_object_id_selected(object_id) {
            return;
        }

        // this primitive op may have been currently selected, in which case we may have
        // to select the primitive op next to it.
        let updated_object = self
            .object_collection
            .get_object(object_id)
            .expect("remove_primitive_op_from_object suceeded");
        self.check_and_select_closest_primitive_op(
            primitive_op_index,
            &updated_object.clone(), // clone is used here to avoid mixing immutable and mutable references (object and self)
            object_id,
            source_command,
        );
    }

    pub(super) fn remove_selected_primitive_op(&mut self, source_command: Option<Command>) {
        let Some(object_id) = self.selected_object_id else {
            failure_warn_no_selected_object(source_command);
            return;
        };
        let Some(primitive_op_index) = self.selected_primitive_op_index else {
            failure_warn_no_selected_primitive_op(source_command);
            return;
        };

        let remove_res = self
            .object_collection
            .remove_primitive_op_from_object(object_id, primitive_op_index);
        if let Err(e) = remove_res {
            failure_warn_collection_error(e, object_id, source_command);
            return;
        };

        let updated_object = self
            .object_collection
            .get_object(object_id)
            .expect("remove_primitive_op_from_object suceeded");
        self.select_primitive_op_with_closest_index(
            primitive_op_index,
            &updated_object.clone(), // clone is used here to avoid mixing immutable and mutable references (object and self)
            object_id,
            source_command,
        );
    }

    /// If a removed primitive op is currently selected, select a different primitive op with the
    /// closest index to the removed primitive op.
    fn check_and_select_closest_primitive_op(
        &mut self,
        removed_primitive_op_index: PrimitiveOpIndex,
        object: &Object,
        object_id: ObjectId,
        source_command: Option<Command>,
    ) {
        if let Some(some_selected_primitive_op_index) = self.selected_primitive_op_index {
            if some_selected_primitive_op_index == removed_primitive_op_index {
                self.select_primitive_op_with_closest_index(
                    removed_primitive_op_index,
                    object,
                    object_id,
                    source_command,
                );
            }
        }
    }

    /// Selects a primitive op in `self` from `primitive_ops` which has the closest index to
    /// `target_prim_op_index`. If `primitive_ops` is empty, deselects primitive op in `self`.
    fn select_primitive_op_with_closest_index(
        &mut self,
        target_prim_op_index: PrimitiveOpIndex,
        object: &Object,
        object_id: ObjectId,
        source_command: Option<Command>,
    ) {
        if let Some(select_index) =
            choose_closest_valid_index(object.primitive_ops.len(), target_prim_op_index)
        {
            self.select_primitive_op(object_id, select_index, source_command);
        } else {
            self.deselect_primitive_op();
        }
    }

    // ~~ Objects: Create New ~~

    fn create_and_select_new_default_object(&mut self, source_command: Option<Command>) {
        let new_object_res = self.object_collection.new_object_default();

        let (new_object_id, _new_object) = match new_object_res {
            Ok(object_and_id) => object_and_id,
            Err(e) => {
                failure_warn_unique_id_error(source_command, e);
                return;
            }
        };

        self.select_object(new_object_id, source_command);
    }

    fn set_object_center(
        &mut self,
        object_id: ObjectId,
        new_center: Vec3,
        source_command: Option<Command>,
    ) {
        let update_res = self
            .object_collection
            .set_object_center(object_id, new_center);
        if let Err(e) = update_res {
            failure_warn_collection_error(e, object_id, source_command);
        }
    }

    fn set_object_name(
        &mut self,
        object_id: ObjectId,
        new_name: String,
        source_command: Option<Command>,
    ) {
        let update_res = self.object_collection.set_object_name(object_id, new_name);
        if let Err(e) = update_res {
            failure_warn_collection_error(e, object_id, source_command);
        }
    }

    // ~~ Objects: Push Op ~~

    fn push_op_and_select(
        &mut self,
        object_id: ObjectId,
        primitive_op: PrimitiveOp,
        source_command: Option<Command>,
    ) {
        let Some(new_primitive_op_index) =
            self.push_op(object_id, primitive_op, source_command.clone())
        else {
            return;
        };
        self.select_primitive_op(object_id, new_primitive_op_index, source_command);
    }

    fn push_op(
        &mut self,
        object_id: ObjectId,
        new_primitive_op: PrimitiveOp,
        source_command: Option<Command>,
    ) -> Option<PrimitiveOpIndex> {
        let push_op_res = self
            .object_collection
            .push_op_to_object(object_id, new_primitive_op);
        match push_op_res {
            Ok(primitive_op_index) => Some(primitive_op_index),
            Err(e) => {
                failure_warn_collection_error(e, object_id, source_command);
                None
            }
        }
    }

    // ~~ Objects: Modify Op ~~

    fn update_primitive_op(
        &mut self,
        object_id: ObjectId,
        primitive_op_index: PrimitiveOpIndex,
        new_primitive_op: PrimitiveOp,
        source_command: Option<Command>,
    ) {
        let res = self.object_collection.update_primitive_op_in_object(
            object_id,
            primitive_op_index,
            new_primitive_op,
        );
        if let Err(e) = res {
            failure_warn_collection_error(e, object_id, source_command);
        }
    }

    fn update_primitive_op_fields(
        &mut self,
        object_id: ObjectId,
        primitive_op_index: PrimitiveOpIndex,
        new_primitive: Option<Primitive>,
        new_transform: Option<PrimitiveTransform>,
        new_operation: Option<Operation>,
        new_blend: Option<f32>,
        new_albedo: Option<Vec3>,
        new_specular: Option<f32>,
        source_command: Option<Command>,
    ) {
        // check early to ensure if `remove_primitive_op_id_from_object` or `failure_warn_invalid_primitive_op_index`
        // fails it is because of invalid primitive op id/index
        if let Err(_e) = self.object_collection.get_object(object_id) {
            failure_warn_invalid_object_id(object_id, source_command);
            return;
        };

        let res = self.object_collection.update_primitive_op_fields_in_object(
            object_id,
            primitive_op_index,
            new_primitive,
            new_transform,
            new_operation,
            new_blend,
            new_albedo,
            new_specular,
        );
        if let Err(e) = res {
            failure_warn_collection_error(e, object_id, source_command);
        }
    }

    /// Moves a primitive op to a new index in the object's rendering order
    fn re_order_primitive_op(
        &mut self,
        object_id: ObjectId,
        original_index: PrimitiveOpIndex,
        target_index: PrimitiveOpIndex,
        command: Command,
    ) {
        // check early to ensure that later failure is because of invalid primitive op indices
        if let Err(_e) = self.object_collection.get_object(object_id) {
            failure_warn_invalid_object_id(object_id, Some(command));
            return;
        };

        let shift_res = self.object_collection.shift_primitive_ops_in_object(
            object_id,
            original_index,
            target_index,
        );

        if let Err(shift_error) = shift_res {
            command_failed_warn(command, &shift_error.to_string());
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
                .is_ok();

            if !object_exists {
                self.selected_object_id = None;
            }
        }
    }
}

// ~~ Failed Command Handling ~~

fn command_failed_warn(command: Command, failed_because: &str) {
    warn!("command {:?} failed due to: {}", command, failed_because);
}

fn failure_warn_collection_error(
    e: CollectionError,
    object_id: ObjectId,
    source_command: Option<Command>,
) {
    match e {
        CollectionError::InvalidId { .. } => {
            failure_warn_invalid_object_id(object_id, source_command)
        }
        CollectionError::OutOfBounds { index, .. } => {
            failure_warn_invalid_primitive_op_index(object_id, index, source_command);
        }
        CollectionError::UniqueIdError(unique_id_error) => {
            failure_warn_unique_id_error(source_command, unique_id_error)
        }
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

fn failure_warn_invalid_primitive_op_index(
    object_id: ObjectId,
    primitive_op_index: PrimitiveOpIndex,
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
