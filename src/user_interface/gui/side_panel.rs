#[derive(PartialEq, Eq)]
pub enum SidePanelMode {
    Hidden,
    Scene,
    ObjectEditor,
}

impl Default for SidePanelMode {
    fn default() -> Self {
        Self::Scene
    }
}

impl SidePanelMode {
    pub fn bools(&self) -> (bool, bool) {
        (*self == Self::Scene, *self == Self::ObjectEditor)
    }
}
