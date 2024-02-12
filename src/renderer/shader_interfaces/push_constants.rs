use bytemuck::{Pod, Zeroable};

/// Should match definitions in `gui.vert` and `gui.frag`.
#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct GuiPushConstant {
    /// Framebuffer dimensions.
    pub screen_size: [f32; 2],
    /// Wherver the render target is in srgb format.
    pub need_srgb_conv: u32,
}
impl GuiPushConstant {
    pub fn new(screen_size: [f32; 2], need_srgb_conv: bool) -> Self {
        Self {
            screen_size,
            need_srgb_conv: need_srgb_conv as u32,
        }
    }
}

/// Should match definition in `gizmos.frag`
#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct GizmosPushConstant {
    pub color: [f32; 3],
    pub object_id: u32,
}
