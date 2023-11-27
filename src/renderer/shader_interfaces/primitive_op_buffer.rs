pub type PrimitiveOpBufferUnit = u32;

/// Inicates an unset primitive id
pub const PRIMITIVE_ID_INVALID: PrimitiveOpBufferUnit = 0xFFFFFFFF;

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
    op_code: PrimitiveOpBufferUnit,
    transform: PrimitiveTransformSlice,
    props: PrimitivePropsSlice,
) -> PrimitiveOpPacket {
    [
        op_code,
        transform[0],
        transform[1],
        transform[2],
        transform[3],
        transform[4],
        transform[5],
        transform[6],
        transform[7],
        transform[8],
        transform[9],
        transform[10],
        transform[11],
        props[0],
        props[1],
        props[2],
        props[3],
        props[4],
        props[5],
        0,
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
