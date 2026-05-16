use crate::user_interface::{
    config_ui::{DEFAULT_SIDE_PANEL_WIDTH, MIN_SIDE_PANEL_WIDTH},
    gui::Gui,
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
    pub(super) fn draw_side_panel(ui: &mut egui::Ui, side_panel_mode: SidePanelMode) {
        egui::Panel::left("side panel")
            .resizable(true)
            .default_size(DEFAULT_SIDE_PANEL_WIDTH)
            .min_size(MIN_SIDE_PANEL_WIDTH)
            .show_inside(ui, |ui| {
                ui.vertical(|ui| {
                    side_panel_layout(ui, side_panel_mode);
                });
            });
    }
}

fn side_panel_layout(ui: &mut egui::Ui, side_panel_mode: SidePanelMode) {}
