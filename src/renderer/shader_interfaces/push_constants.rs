use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec4};

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

/// Should match definition in `overlay.vert`
#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct OverlayPushConstant {
    pub proj_view: [f32; 16],
    pub offset: [f32; 4],
}
impl OverlayPushConstant {
    pub fn new(proj_view: Mat4, offset: Vec4) -> Self {
        Self {
            proj_view: proj_view.to_cols_array(),
            offset: offset.into(),
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
