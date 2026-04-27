#[derive(Debug, Clone, Copy)]
pub enum GizmoType {
    Linear(GizmoLinear),
}

impl Default for GizmoType {
    fn default() -> Self {
        Self::Linear(Default::default())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GizmoLinear {
    ALL,
    X,
    Y,
    Z,
}

impl Default for GizmoLinear {
    fn default() -> Self {
        Self::ALL
    }
}
