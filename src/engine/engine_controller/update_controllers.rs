use crate::{
    engine::{engine_controller::Engine, object::object::ObjectId},
    helper::more_errors::CollectionError,
    renderer::element_id_reader::ElementAtPoint,
    user_interface::{
        config_ui::KEY_BINDING_COMMAND_PALETTE, cursor::MouseButtonEvent, gizmo::GizmoElement,
        mouse_button::MouseButton, view_modes::ViewMode,
    },
};
use glam::DVec2;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

impl Engine {
    pub(super) fn update_selection_gizmo(&mut self) -> anyhow::Result<()> {
        let Some(selected_object_id) = self.selected_object_id else {
            return Ok(());
        };

        let Ok(selected_object) = self.object_collection.get_object(selected_object_id) else {
            self.deselect_object();
            return Ok(());
        };

        let center = match self.view_mode {
            ViewMode::SceneEditor => selected_object.center,
            ViewMode::ObjectEditor => {
                let Some(selected_primitive_op_index) = self.selected_primitive_op_index else {
                    return Ok(());
                };
                let Some(selected_primitive_op) = selected_object
                    .primitive_ops
                    .get(selected_primitive_op_index)
                else {
                    self.deselect_primitive_op();
                    return Ok(());
                };
                selected_object.center + selected_primitive_op.center()
            }
        };

        self.controllers.renderer.update_gizmo_center(center)?;
        Ok(())
    }

    pub(super) fn process_keyboard_input(&mut self, key_event: KeyEvent, captured_by_gui: bool) {
        // update modifiers whenever focus is in window
        self.keyboard_modifier_states.set(key_event.clone());

        // todo clean up the ordering of this... move keyboard_modifiers up? think it through...
        if captured_by_gui {
            return;
        }

        let PhysicalKey::Code(key_code) = key_event.physical_key else {
            return;
        };

        match key_code {
            KEY_BINDING_COMMAND_PALETTE => {
                if let ElementState::Released = key_event.state {
                    self.controllers.gui.toggle_command_palette_visability();
                }
            }
            KeyCode::Escape => {
                if let ElementState::Released = key_event.state {
                    self.controllers.gui.hide_command_palette();
                }
            }
            _ => (),
        }
    }

    pub(super) fn update_window_inner_size(
        &mut self,
        new_inner_size: winit::dpi::PhysicalSize<u32>,
    ) {
        self.controllers
            .camera
            .set_aspect_ratio(new_inner_size.into());
        self.controllers.renderer.set_window_just_resized_flag();
    }

    pub(super) fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
        self.controllers.gui.set_scale_factor(scale_factor as f32);
        self.controllers
            .renderer
            .set_scale_factor(scale_factor as f32);
    }

    pub(super) fn process_cursor_event(
        &mut self,
        cursor_event: MouseButtonEvent,
    ) -> anyhow::Result<()> {
        let Some(cursor_screen_coordinates_dvec2) = self.controllers.cursor.position() else {
            return Ok(());
        };
        let cursor_screen_coordinates = cursor_screen_coordinates_dvec2.as_vec2().to_array();

        let element_at_point = self
            .controllers
            .renderer
            .get_element_at_screen_coordinate(cursor_screen_coordinates)?;

        let scroll_delta = self.controllers.cursor.get_and_clear_scroll_delta();
        self.controllers
            .camera
            .update_scroll(&self.settings.camera, scroll_delta);

        if let MouseButtonEvent::Dragging { .. } = cursor_event {
            if self.dragging_source_element.is_none() {
                // just started dragging
                self.dragging_source_element = element_at_point;
            }
        } else {
            self.dragging_source_element = None; // not dragging
        }

        match cursor_event {
            MouseButtonEvent::ReleaseInPlace(button) => match button {
                MouseButton::Left => match element_at_point {
                    Some(ElementAtPoint::Background) => self.background_clicked(),
                    Some(ElementAtPoint::Object {
                        object_id,
                        primitive_op_index,
                    }) => self.select_primitive_op(object_id, primitive_op_index, None),
                    Some(ElementAtPoint::BlendArea { object_id }) => {
                        self.select_object(object_id, None)
                    }
                    _ => (),
                },
                _ => (),
            },
            MouseButtonEvent::Dragging { button, delta } => match self.dragging_source_element {
                Some(ElementAtPoint::Gizmo(gizmo_element)) => {
                    self.gizmo_dragged(gizmo_element, button, delta)
                }
                _ => self.controllers.camera.update_cursor_dragging(
                    &self.settings.camera,
                    delta,
                    button,
                    self.keyboard_modifier_states,
                    self.camera_control_mappings,
                ),
            },
            MouseButtonEvent::None => match element_at_point {
                Some(ElementAtPoint::Gizmo(gizmo_type)) => self.hovered_gizmo = Some(gizmo_type),
                _ => self.hovered_gizmo = None,
            },
        }

        Ok(())
    }

    pub(super) fn gizmo_dragged(
        &mut self,
        gizmo_element: GizmoElement,
        button: MouseButton,
        delta: DVec2,
    ) {
        let Some(selected_object_id) = self.selected_object_id else {
            warn!("gizmo dragged but no object selected. how???");
            return;
        };

        if button == MouseButton::Left {
            let res = gizmo_element.process_dragged(
                delta,
                selected_object_id,
                &mut self.object_collection,
                &self.controllers.camera,
                &self.settings.camera,
            );
            if let Err(CollectionError::InvalidId { .. }) = res {
                self.deselect_object();
            }
        }
    }

    pub(super) fn background_clicked(&mut self) {
        self.deselect_object();
        self.settings.camera.unset_lock_on_target();
    }

    pub(super) fn is_object_id_selected(&self, compare_object_id: ObjectId) -> bool {
        if let Some(some_selected_object_id) = self.selected_object_id {
            some_selected_object_id == compare_object_id
        } else {
            false
        }
    }
}
