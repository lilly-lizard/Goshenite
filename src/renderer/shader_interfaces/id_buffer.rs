/// Data type that the renderer outputs to identify objects in a scene
pub type EncodedId = u32;

/// Set in areas where primitives are being blended together
pub const ID_BLEND: EncodedId = 0x0000FFFE;

// Must match definitions in `config.glsl`
#[allow(unused)]
pub const ID_NULL: EncodedId = 0xFFFFFFFF;
pub const ID_BACKGROUND: EncodedId = 0xFFFFFFFE;
#[allow(unused)]
pub const ID_GISMO_MASK: EncodedId = 0xFFFFFFF0;
pub const ID_GIZMO_X: EncodedId = 0xFFFFFFFD;
pub const ID_GIZMO_Y: EncodedId = 0xFFFFFFFC;
pub const ID_GIZMO_Z: EncodedId = 0xFFFFFFFB;
