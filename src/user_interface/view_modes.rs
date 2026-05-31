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
impl ViewMode {
    pub const VARIANTS: [Self; 2] = [Self::SceneEditor, Self::ObjectEditor];
    pub fn name(&self) -> &str {
        match self {
            Self::SceneEditor => "Scene Editor",
            Self::ObjectEditor => "Object Editor",
        }
    }
}
