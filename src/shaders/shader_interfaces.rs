//! Contains structs and descriptor set indices/bindings matching the interfaces in shaders

use glam::{Mat4, Vec3, Vec4};
use vulkano::shader::{SpecializationConstants, SpecializationMapEntry};

/// Describes the descriptor set containing the render storage image (render.comp) and sampler (post.frag)
pub mod descriptor {
    pub const SET_RENDER_COMP: usize = 0; // descriptor set index in render.comp
    pub const SET_BLIT_FRAG: usize = 0; // descriptor set index in post.frag
    pub const BINDING_IMAGE: u32 = 0; // render storage image binding
    pub const BINDING_SAMPLER: u32 = 0; // render image sampler binding
}

/// Render compute shader push constant struct. Size should be no more than 128 bytes for full vulkan coverage
#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
#[allow(non_snake_case)]
pub struct CameraPc {
    /// Inverse of projection matrix multiplied by view matrix. Converts clip space coordinates to world space
    pub projViewInverse: [f32; 16],
    /// Camera position in world space (w component unused)
    pub position: [f32; 4],
}
impl CameraPc {
    pub fn new(proj_view_inverse: Mat4, position: Vec3) -> Self {
        Self {
            projViewInverse: proj_view_inverse.to_cols_array(),
            position: Vec4::from((position, 0.)).to_array(),
        }
    }
}

/// Render compute shader specialization constants. Used for setting the local work group size
#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
#[allow(non_snake_case)]
pub struct ComputeSpecConstant {
    /// Local work group size x value
    pub local_size_x: u32,
    /// Local work group size y value
    pub local_size_y: u32,
}
unsafe impl SpecializationConstants for ComputeSpecConstant {
    /// Returns descriptors of the struct's layout.
    fn descriptors() -> &'static [SpecializationMapEntry] {
        &[
            // local_size_x
            SpecializationMapEntry {
                constant_id: 0,
                offset: 0,
                size: 4,
            },
            // local_size_y
            SpecializationMapEntry {
                constant_id: 1,
                offset: 4,
                size: 4,
            },
        ]
    }
}

// todo doc
#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
#[allow(non_snake_case)]
pub struct GuiPc {
    pub screen_size: [f32; 2],
    /// use bool.into()
    pub need_srgb_conv: u32,
}
impl GuiPc {
    pub fn new(screen_size: [f32; 2], need_srgb_conv: bool) -> Self {
        Self {
            screen_size,
            need_srgb_conv: need_srgb_conv as u32,
        }
    }
}
