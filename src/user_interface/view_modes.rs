#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    SceneEditor,
    ObjectEditor,
}

impl Default for ViewMode {
    fn default() -> Self {
        ViewMode::SceneEditor
    }
}
