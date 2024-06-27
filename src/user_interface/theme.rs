#[derive(Clone, Copy)]
pub enum Theme {
    Dark,
    Light,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Dark
    }
}

impl Theme {
    pub fn as_egui_visuals(&self) -> egui::Visuals {
        match self {
            Self::Dark => egui::Visuals::dark(),
            Self::Light => egui::Visuals::light(),
        }
    }
}
