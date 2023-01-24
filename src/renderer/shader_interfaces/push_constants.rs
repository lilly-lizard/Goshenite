use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};

/// Render compute shader push constant struct. Size should be no more than 128 bytes for full vulkan coverage
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
pub struct CameraPushConstants {
    /// Inverse of projection matrix multiplied by view matrix. Converts clip space coordinates to world space
    pub proj_view_inverse: [f32; 16],
    /// Camera position in world space (w component unused)
    pub position: [f32; 4],
}
impl CameraPushConstants {
    pub fn new(proj_view_inverse: Mat4, position: Vec3) -> Self {
        Self {
            proj_view_inverse: proj_view_inverse.to_cols_array(),
            position: [position.x, position.y, position.z, 0.0],
        }
    }
}

/// Gui shader push constants. Should match definitions in `gui.vert` and `gui.frag`.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
pub struct GuiPushConstants {
    /// Framebuffer dimensions.
    pub screen_size: [f32; 2],
    /// Wherver the render target is in srgb format.
    pub need_srgb_conv: u32,
}
impl GuiPushConstants {
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
pub struct OverlayPushConstants {
    pub proj_view: [f32; 16],
    pub offset: [f32; 4],
}
impl OverlayPushConstants {
    pub fn new(proj_view: Mat4, offset: Vec4) -> Self {
        Self {
            proj_view: proj_view.to_cols_array(),
            offset: offset.into(),
        }
    }
}
