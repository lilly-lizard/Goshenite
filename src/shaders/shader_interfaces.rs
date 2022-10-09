//! Contains structs and descriptor set indices/bindings matching the interfaces in shaders.
use crate::primitives::primitive_collection::PrimitiveCollection;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use std::fmt::{self, Display};
use vulkano::format::Format;

/// Function name of the entry point for shaders
pub const SHADER_ENTRY_POINT: &str = "main";

/// G-buffer format
pub const G_BUFFER_FORMAT_NORMAL: Format = Format::R8G8B8A8_UNORM;
pub const G_BUFFER_FORMAT_PRIMITIVE_ID: Format = Format::R32_UINT;

// ~~~ Primitive Data ~~~

/// Shorthand for the data type in the primitive storage buffer defined in `scene.comp`.
pub type PrimitiveDataUnit = u32;
/// Each primitive is encoded into an array of length `PRIMITIVE_LEN`. This value should match the one defined in `primitives.glsl`.
pub const PRIMITIVE_LEN: usize = 8;
/// An array which a primitive can be encoded into. Corresponds to the decoding logic in `scene.comp`.
pub type PrimitiveDataSlice = [PrimitiveDataUnit; PRIMITIVE_LEN];
/// Each `PrimitiveDataSlice` begins with a primitive code defining the type of primitive that has been encoded.
/// The values defined here should match the ones defined in `primitives.glsl`.
pub mod primitive_codes {
    use super::PrimitiveDataUnit;
    pub const NULL: PrimitiveDataUnit = 0x00000000;
    pub const SPHERE: PrimitiveDataUnit = 0x00000001;
    pub const CUBE: PrimitiveDataUnit = 0x00000002;
}

/// Matches the definition of the primitive storage buffer in `scene.comp`. In this case the members are purely for show.
/// Use [`PrimitiveData::combined_data`] when creating the storage buffer.
pub struct PrimitiveData {
    pub _count: PrimitiveDataUnit,
    pub _data: Vec<PrimitiveDataUnit>,
}
impl PrimitiveData {
    /// Returns a vector containing data that will match the primitive storage buffer definition in `scene.comp`.
    pub fn combined_data(
        primitives: &PrimitiveCollection,
    ) -> Result<Vec<PrimitiveDataUnit>, PrimitiveDataError> {
        let data = primitives.encoded_data();
        let count = data.len();
        if count >= PrimitiveDataUnit::MAX as usize {
            return Err(PrimitiveDataError::DataLengthOverflow);
        }
        let mut combined_data = vec![count as PrimitiveDataUnit];
        for p in data {
            combined_data.extend_from_slice(p);
        }
        Ok(combined_data)
    }
}
impl std::error::Error for PrimitiveDataError {}

#[derive(Clone, Copy, Debug)]
pub enum PrimitiveDataError {
    /// The number of primitives passed to [`PrimitiveData::combined_data`] exceeds u32::MAX meaning the count cannot
    /// be encoded accurately.
    DataLengthOverflow,
}
impl Display for PrimitiveDataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "cannot create primitive data structure as the number of primitive exceeds u32::MAX"
        )
    }
}

// ~~~ Push Constants ~~~

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

// ~~~ Vertex Inputs ~~~

/// Should match inputs to `overlay.vert`
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct OverlayVertex {
    pub in_position: [f32; 4],
    pub in_normal: [f32; 4],
    pub in_color: [f32; 4],
}
vulkano::impl_vertex!(OverlayVertex, in_position, in_normal, in_color);
impl OverlayVertex {
    pub const fn new(position: Vec3, normal: Vec3, color: Vec3) -> Self {
        Self {
            in_position: [position.x, position.y, position.z, 1.0],
            in_normal: [normal.x, normal.y, normal.z, 1.0],
            in_color: [color.x, color.y, color.z, 1.0],
        }
    }
}

/// Should match vertex definition for `gui.vert` (except color is `[f32; 4]`)
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct EguiVertex {
    pub in_position: [f32; 2],
    pub in_tex_coords: [f32; 2],
    pub in_color: [f32; 4],
}
vulkano::impl_vertex!(EguiVertex, in_position, in_tex_coords, in_color);
