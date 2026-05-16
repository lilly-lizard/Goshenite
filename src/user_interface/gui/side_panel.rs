use crate::{
    engine::{
        commands::Command,
        object::{
            object::ObjectId, object_collection::ObjectCollection, primitive_op::PrimitiveOpIndex,
        },
    },
    user_interface::{
        config_ui::{DEFAULT_SIDE_PANEL_WIDTH, MIN_SIDE_PANEL_WIDTH},
        gui::{object_editor::layout_object_editor, scene_editor::layout_scene_editor, Gui},
        gui_state::ValueState,
    },
};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum SidePanelMode {
    Scene,
    ObjectEditor,
}
impl Default for SidePanelMode {
    fn default() -> Self {
        Self::Scene
    }
}
impl SidePanelMode {
    /// Returns a boolean for each possible mode
    pub fn bools(mode: Option<Self>) -> (bool, bool) {
        match mode {
            Some(some_mode) => (some_mode == Self::Scene, some_mode == Self::ObjectEditor),
            None => (false, false),
        }
    }
}

impl Gui {
    pub(super) fn draw_side_panel(
        ui: &mut egui::Ui,
        side_panel_mode: SidePanelMode,
        value_state: &mut ValueState,
        object_collection: &ObjectCollection,
        selected_object_id: Option<ObjectId>,
        selected_primitive_op_index: Option<PrimitiveOpIndex>,
    ) -> Vec<Command> {
        let mut commands = Vec::<Command>::new();

        egui::Panel::left("side panel")
            .resizable(true)
            .default_size(DEFAULT_SIDE_PANEL_WIDTH)
            .min_size(MIN_SIDE_PANEL_WIDTH)
            .show_inside(ui, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(6.0);
                    commands = match side_panel_mode {
                        SidePanelMode::ObjectEditor => layout_object_editor(
                            ui,
                            value_state,
                            object_collection,
                            selected_object_id,
                            selected_primitive_op_index,
                        ),
                        SidePanelMode::Scene => {
                            layout_scene_editor(ui, selected_object_id, object_collection)
                        }
                    };
                });
            });

        commands
    }
}
