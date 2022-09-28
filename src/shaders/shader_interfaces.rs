//! Contains structs and descriptor set indices/bindings matching the interfaces in shaders.
use std::fmt::{self, Display};

use crate::primitives::primitives::PrimitiveCollection;
use glam::{Mat4, Vec3, Vec4};
use vulkano::shader::{SpecializationConstants, SpecializationMapEntry};

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

/// Render compute shader push constant struct. Size should be no more than 128 bytes for full vulkan coverage
#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
pub struct CameraPushConstant {
    /// Inverse of projection matrix multiplied by view matrix. Converts clip space coordinates to world space
    pub proj_view_inverse: [f32; 16],
    /// Camera position in world space (w component unused)
    pub position: [f32; 4],
}
impl CameraPushConstant {
    pub fn new(proj_view_inverse: Mat4, position: Vec3) -> Self {
        Self {
            proj_view_inverse: proj_view_inverse.to_cols_array(),
            position: Vec4::from((position, 0.)).to_array(),
        }
    }
}

/// Scene render compute shader specialization constants. Used for setting the local work group size
#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
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

/// Gui shader push constants. Should match definitions in `gui.vert` and `gui.frag`.
#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
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
