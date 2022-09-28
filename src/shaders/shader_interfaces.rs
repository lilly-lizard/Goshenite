//! Contains structs and descriptor set indices/bindings matching the interfaces in shaders
use crate::primitives::primitives::PrimitiveCollection;
use glam::{Mat4, Vec3, Vec4};
use vulkano::shader::{SpecializationConstants, SpecializationMapEntry};

pub type PrimitiveDataUnit = u32;
/// bruh
pub const PRIMITIVE_LEN: usize = 8;
/// bruh
pub type PrimitiveDataSlice = [PrimitiveDataUnit; PRIMITIVE_LEN];
/// bruh
pub mod primitive_codes {
    pub const NULL: u32 = 0x00000000;
    pub const SPHERE: u32 = 0x00000001;
    pub const CUBE: u32 = 0x00000002;
}

/// todo doc
#[repr(C)]
pub struct PrimitiveData {
    pub _count: PrimitiveDataUnit,
    pub _data: Vec<PrimitiveDataUnit>,
}
impl PrimitiveData {
    // todo document code
    pub fn combined_data(primitives: &PrimitiveCollection) -> Vec<PrimitiveDataUnit> {
        let data = primitives.encoded_data();
        // todo return err instead of assert, also checked in buffer()??
        let count = data.len();
        assert!(
            count < PrimitiveDataUnit::MAX as usize,
            "primitive count exceeded `PrimitiveDataUnit::MAX`! todo handle this..."
        );
        let mut combined_data = vec![count as PrimitiveDataUnit];
        for p in data {
            combined_data.extend_from_slice(p);
        }
        combined_data
    }
}

/// Render compute shader push constant struct. Size should be no more than 128 bytes for full vulkan coverage
#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
#[allow(non_snake_case)]
pub struct CameraPushConstant {
    /// Inverse of projection matrix multiplied by view matrix. Converts clip space coordinates to world space
    pub projViewInverse: [f32; 16],
    /// Camera position in world space (w component unused)
    pub position: [f32; 4],
}
impl CameraPushConstant {
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
pub struct GuiPushConstant {
    pub screen_size: [f32; 2],
    /// use bool.into()
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
