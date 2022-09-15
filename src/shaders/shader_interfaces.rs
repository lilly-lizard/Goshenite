//! Contains structs and descriptor set indices/bindings matching the interfaces in shaders

use glam::{Mat4, Vec3, Vec4};

/// Describes the descriptor set containing the render storage image (render.comp) and sampler (post.frag)
pub mod descriptor {
    pub const SET_RENDER_COMP: usize = 0; // descriptor set index in render.comp
    pub const SET_BLIT_FRAG: usize = 0; // descriptor set index in post.frag
    pub const BINDING_IMAGE: u32 = 0; // render storage image binding
    pub const BINDING_SAMPLER: u32 = 0; // render image sampler binding
}

/// Render compute shader push constant struct. size should be no more than 128 bytes for full vulkan coverage
#[allow(non_snake_case)]
pub struct CameraPc {
    /// Inverse of projection matrix multiplied by view matrix. Converts clip space coordinates to world space
    pub projViewInverse: [f32; 16],
    /// Camera position in world space (w component unused)
    pub position: [f32; 4],
}
impl CameraPc {
    pub fn new(proj_view_inverse: Mat4, position: Vec3) -> Self {
        CameraPc {
            projViewInverse: proj_view_inverse.to_cols_array(),
            position: Vec4::from((position, 0.)).to_array(),
        }
    }
}
