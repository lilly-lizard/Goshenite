use crate::{
    config,
    engine::{
        commands::Command,
        object::{
            object::ObjectId, object_collection::ObjectCollection, primitive_op::PrimitiveOpIndex,
        },
        preset_models::object_testing,
        settings::{Settings, SettingsIO},
        window_thread::WindowThreadChannels,
    },
    renderer::{element_id_reader::ElementAtPoint, render_manager::RenderManager},
    user_interface::{
        camera::Camera,
        controls_camera::CameraControlMappings,
        cursor::Cursor,
        gizmo::{GizmoElement, GizmoVisibility},
        gui::Gui,
        keyboard_modifiers::KeyboardModifierStates,
        view_modes::ViewMode,
    },
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{collections::VecDeque, env, sync::Arc};
use winit::{event::WindowEvent, window::Window};

// engine_instance sub-modules (files in engine_instance directory)
mod process_commands;
mod update_controllers;
mod update_objects;

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum EngineCommand {
    Run,
    Pause,
    Quit,
}

/// So that settings and controllers can be (conveniently) mutated at the same
pub struct EngineControllers {
    pub cursor: Cursor,
    pub camera: Camera,
    pub gui: Gui,
    pub renderer: RenderManager,
}

pub struct Engine {
    window: Arc<Window>,

    // state
    view_mode: ViewMode,
    scale_factor: f64,
    object_collection: ObjectCollection, // note: some engine code may have been written on the assumtion that there is only one object collection...
    main_thread_frame_number: u64, // TODO what is this used for? wrap around to handle overflow??
    pending_commands: VecDeque<Command>,
    selected_object_id: Option<ObjectId>,
    selected_primitive_op_index: Option<PrimitiveOpIndex>,
    keyboard_modifier_states: KeyboardModifierStates,
    dragging_source_element: Option<ElementAtPoint>,
    gizmo_visibility: GizmoVisibility,
    hovered_gizmo: Option<GizmoElement>,

    // controllers
    controllers: EngineControllers,

    // settings
    settings: Settings,
    settings_io: SettingsIO,
    camera_control_mappings: CameraControlMappings,

    // window thread (main thread)
    window_thread_channels: WindowThreadChannels,
}

// ~~ Public Functions ~~

impl Engine {
    pub fn new(
        window: Arc<Window>,
        window_thread_channels: WindowThreadChannels,
    ) -> anyhow::Result<Self> {
        let settings = match Settings::load_from_user_settings() {
            Ok(settings) => settings,
            Err(e) => {
                warn!("failed to load settings because: {}", e);
                Settings::default()
            }
        };
        let settings_io = SettingsIO::default();

        let scale_factor_override: Option<f64> = match env::var(config::ENV::SCALE_FACTOR) {
            Ok(s) => s.parse::<f64>().ok(),
            _ => None,
        };
        let scale_factor = scale_factor_override.unwrap_or(window.scale_factor());

        let cursor = Cursor::new();
        let camera = Camera::new(window.inner_size().into())?;

        let mut renderer = RenderManager::new(window.clone(), scale_factor as f32)?;
        renderer.init_camera(&camera, &settings.camera)?;

        let max_texture_size = renderer.max_2d_image_size(); //maxImageDimension2D
        let gui = Gui::new(
            window.clone(),
            &settings_io,
            scale_factor as f32,
            Some(max_texture_size),
        );

        let mut object_collection = ObjectCollection::new();

        // ~~ TESTING OBJECTS START ~~

        object_testing(&mut object_collection);
        //create_default_cube_object(&mut self.object_collection);

        // ~~ TESTING OBJECTS END ~~

        Ok(Engine {
            window,

            view_mode: ViewMode::default(),
            scale_factor,
            object_collection,
            main_thread_frame_number: 0,
            pending_commands: VecDeque::new(),
            selected_object_id: None,
            selected_primitive_op_index: None,
            keyboard_modifier_states: KeyboardModifierStates::default(),
            gizmo_visibility: GizmoVisibility::default(),
            hovered_gizmo: None,
            dragging_source_element: None,

            controllers: EngineControllers {
                cursor,
                camera,
                gui,
                renderer,
            },

            settings,
            settings_io,
            camera_control_mappings: Default::default(),

            window_thread_channels,
        })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            let engine_command = self.window_thread_channels.latest_command();
            match engine_command {
                Some(EngineCommand::Run) => (),
                None => (), // just keep running
                Some(EngineCommand::Pause) => continue,
                Some(EngineCommand::Quit) => {
                    self.shut_down();
                    return Ok(());
                }
            }

            let frame_res = self.run_frame();

            match frame_res {
                Ok(EngineCommand::Quit) => {
                    self.shut_down();
                    return Ok(());
                }
                Err(e) => {
                    self.shut_down();
                    return Err(e);
                }
                _ => (),
            }
        }
    }
}

// ~~ Main Loop Functions ~~

impl Engine {
    /// The main loop of the engine thread
    fn run_frame(&mut self) -> anyhow::Result<EngineCommand> {
        let events = self.window_thread_channels.get_events()?;

        for event in events {
            self.process_window_event(event);
        }

        self.update_engine()?;

        Ok(EngineCommand::Run)
    }

    /// Process window events and update state
    fn process_window_event(&mut self, event: WindowEvent) {
        trace!("winit event: {:?}", event);

        // egui event handling
        let captured_by_gui = self.controllers.gui.process_event(&event).consumed;

        // engine event handling
        match event {
            // cursor moved. triggered when cursor is in window or if currently dragging and started in the window (on linux at least)
            WindowEvent::CursorMoved { position, .. } => {
                self.controllers.cursor.set_position(position.into())
            }

            // send mouse button events to cursor state
            WindowEvent::MouseInput { state, button, .. } => self
                .controllers
                .cursor
                .set_click_state(button, state, captured_by_gui),
            WindowEvent::MouseWheel { delta, .. } => self
                .controllers
                .cursor
                .accumulate_scroll_delta(delta, captured_by_gui),

            // cursor entered window
            WindowEvent::CursorEntered { .. } => self.controllers.cursor.set_in_window_state(true),

            // cursor left window
            WindowEvent::CursorLeft { .. } => self.controllers.cursor.set_in_window_state(false),

            // keyboard
            WindowEvent::KeyboardInput { event, .. } => {
                self.process_keyboard_input(event, captured_by_gui)
            }

            // window resize
            WindowEvent::Resized(new_inner_size) => {
                self.update_window_inner_size(new_inner_size);
            }

            // dpi change
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.set_scale_factor(scale_factor);
            }

            //WindowEvent::ThemeChanged(winit_theme)
            _ => (),
        }
    }

    // Per frame udpates
    fn update_engine(&mut self) -> anyhow::Result<()> {
        // process recieved events for cursor state
        let cursor_event = self.controllers.cursor.process_frame();
        if let Some(cursor_icon) = self.controllers.cursor.cursor_icon() {
            self.controllers.gui.set_cursor_icon(cursor_icon);
        }
        self.process_cursor_event(cursor_event)?;

        // process gui inputs and update layout
        let commands_from_gui = self.controllers.gui.update_gui(
            &mut self.settings,
            &self.settings_io,
            &self.object_collection,
            &self.window,
            self.selected_object_id,
            self.selected_primitive_op_index,
        );
        self.pending_commands.extend(commands_from_gui.into_iter());

        // process commands from gui
        self.execute_engine_commands();

        // object buffer updates
        let objects_delta = self.object_collection.get_and_clear_objects_delta();
        self.controllers.camera.update_camera_objects(
            &mut self.settings.camera,
            &self.object_collection,
            self.selected_object_id,
            self.selected_primitive_op_index,
        );
        self.controllers.renderer.update_objects(objects_delta)?;
        self.update_selection_gizmo()?;

        // submit gui texture updates
        let textures_delta = self.controllers.gui.get_and_clear_textures_delta();
        self.controllers
            .renderer
            .update_gui_textures(textures_delta)?;

        // submit gui primitive updates
        let gui_primitives = self.controllers.gui.mesh_primitives().clone();
        self.controllers.renderer.set_gui_primitives(gui_primitives);

        // renderer
        self.controllers.renderer.render_frame(
            &self.settings.render,
            self.view_mode,
            &self.controllers.camera,
            &self.settings.camera,
            self.gizmo_visibility,
            self.hovered_gizmo,
            self.selected_object_id,
        )?;

        self.main_thread_frame_number += 1;

        Ok(())
    }

    fn shut_down(&self) {
        info!("shutting down...");
        // save gui state
        if let Err(e) = self.controllers.gui.save_gui_state() {
            error!("{}", e);
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        debug!("dropping engine controller");
    }
}
