use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec4};

/// Should match definition in `bounding_box.vert`
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
pub struct ObjectIndexPushConstant {
    pub object_index: u32,
}
impl ObjectIndexPushConstant {
    pub fn new(object_index: u32) -> Self {
        Self { object_index }
    }
}

/// Should match definitions in `gui.vert` and `gui.frag`.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
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
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
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
