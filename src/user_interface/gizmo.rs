use crate::helper::axis::CartesianAxis;

#[derive(Default, Debug, Clone, Copy)]
pub struct GizmoVisibility {
    pub linear: bool,
    // rotate
    // linear_plane
    // scale
}
impl GizmoVisibility {
    pub fn any_visible(&self) -> bool {
        return self.linear; // || self.rotate || ...
    }
    pub fn hide_all(&mut self) {
        self.linear = false;
    }
    pub fn show_all(&mut self) {
        self.linear = true;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GizmoElement {
    Linear(CartesianAxis),
    // Rotate
    // LinearPlane
    // Scale
}
impl Default for GizmoElement {
    fn default() -> Self {
        Self::Linear(Default::default())
    }
}
