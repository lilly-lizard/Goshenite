use crate::engine::{object::primitive_op::PrimitiveOp, primitives::primitive::EncodablePrimitive};
use glam::Vec3;

pub type PrimitiveOpBufferUnit = u32;

// this is because the shaders store the primitive op index in the lower 16 bits of a u32
pub const MAX_PRIMITIVE_OP_COUNT: usize = u16::MAX as usize;

/// Set in areas where primitives are being blended together
pub const PRIMITIVE_ID_BLEND: PrimitiveOpBufferUnit = 0xFFFFFFFE;
/// Inicates an unset primitive id
pub const PRIMITIVE_ID_BACKGROUND: PrimitiveOpBufferUnit = 0xFFFFFFFF;

#[rustfmt::skip]
#[allow(dead_code)]
pub mod op_codes {
    use super::PrimitiveOpBufferUnit;
    pub const NOP: 		    PrimitiveOpBufferUnit = 0x00000000;
    pub const UNION: 		PrimitiveOpBufferUnit = 0x00000001; // OR
    pub const INTERSECTION: PrimitiveOpBufferUnit = 0x00000002; // AND
    pub const SUBTRACTION: 	PrimitiveOpBufferUnit = 0x00000003;
    pub const INVALID:      PrimitiveOpBufferUnit = 0xFFFFFFFF;
}

/// Number of 32-bit values to store an op_code and a primitive. Note that this value should equal
/// to 1 + `TRANSFORM_DATA_LEN` + `PRIMITIVE_DATA_LEN` for the total primitive data packet.
///
/// _Must match value defined in `confg.glsl`_
pub const PRIMITIVE_PACKET_LEN: usize = 20;
/// Each primitive has a 3x3 matrix associated with it for transformations. This defines that size.
pub const PRIMITIVE_TRANSFORM_LEN: usize = 12;
/// Each primitive type has unique properties encoded into an array of this length.
pub const PRIMITIVE_PROPS_LEN: usize = 6;

/// Array for data describing a primitive operation.
/// Corresponds to decoding logic in `scene_geometry.frag`.
pub type PrimitiveOpPacket = [PrimitiveOpBufferUnit; PRIMITIVE_PACKET_LEN];
/// Array for per-primitive translation and rotation data decoded by shaders.
/// Corresponds to decoding logic in `scene_geometry.frag`.
pub type PrimitiveTransformSlice = [PrimitiveOpBufferUnit; PRIMITIVE_TRANSFORM_LEN];
/// Array for properties specific to a given primitive type.
/// Corresponds to decoding logic in `scene_geometry.frag`.
pub type PrimitivePropsSlice = [PrimitiveOpBufferUnit; PRIMITIVE_PROPS_LEN];

pub fn create_primitive_op_packet(
    primitive_op: &PrimitiveOp,
    object_origin: Vec3,
) -> PrimitiveOpPacket {
    let encoded_op_code = primitive_op.op.op_code();
    let encoded_transform = primitive_op.transform.gpu_encoded(object_origin);
    let encoded_props = primitive_op.primitive.encoded_props();
    let encoded_blend = primitive_op.blend.to_bits();
    [
        encoded_transform[0],
        encoded_transform[1],
        encoded_transform[2],
        encoded_transform[3],
        encoded_transform[4],
        encoded_transform[5],
        encoded_transform[6],
        encoded_transform[7],
        encoded_transform[8],
        encoded_transform[9],
        encoded_transform[10],
        encoded_transform[11],
        encoded_props[0],
        encoded_props[1],
        encoded_props[2],
        encoded_props[3],
        encoded_props[4],
        encoded_props[5],
        encoded_op_code,
        encoded_blend,
    ]
}

pub fn nop_primitive_op_packet() -> PrimitiveOpPacket {
    [
        op_codes::NOP,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ]
}
