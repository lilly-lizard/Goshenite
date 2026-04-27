/// Data type that the renderer outputs to identify objects in a scene
pub type EncodedId = u32;

/// Set in areas where primitives are being blended together
pub const ID_BLEND: EncodedId = 0x0000FFFE;

pub const ID_BACKGROUND: EncodedId = 0xFFFFFFFF;
pub const ID_GIZMO_X: EncodedId = 0xFFFFFFFE;
pub const ID_GIZMO_Y: EncodedId = 0xFFFFFFFD;
pub const ID_GIZMO_Z: EncodedId = 0xFFFFFFFC;
