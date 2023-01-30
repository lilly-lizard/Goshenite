#[derive(PartialEq, Clone, Copy)]
pub enum Theme {
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Dark
    }
}

impl From<winit::window::Theme> for Theme {
    fn from(value: winit::window::Theme) -> Self {
        match value {
            winit::window::Theme::Light => Self::Light,
            winit::window::Theme::Dark => Self::Dark,
        }
    }
}
