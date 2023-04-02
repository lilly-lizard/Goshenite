pub type PrimitiveOpBufferUnit = u32;

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

/// Number of 32-bit values to store an op_code and a primitive.
/// _Must match value defined in `confg.glsl`_
pub const PRIMITIVE_OP_UNIT_LEN: usize = 8;
/// Each primitive is encoded into an array of length `PRIMITIVE_UNIT_LEN`.
pub const PRIMITIVE_UNIT_LEN: usize = 7;

/// An array which a primitive can be encoded into. Corresponds to the decoding logic in `scene.comp`.
pub type PrimitiveDataSlice = [PrimitiveOpBufferUnit; PRIMITIVE_UNIT_LEN];

/// Each `PrimitiveDataSlice` begins with a primitive code defining the type of primitive that has been encoded.
/// The values defined here should match the ones defined in `primitives.glsl`.
#[rustfmt::skip]
#[allow(dead_code)]
pub mod primitive_codes {
    use super::PrimitiveOpBufferUnit;
    pub const NULL:     PrimitiveOpBufferUnit = 0x00000000;
    pub const SPHERE:   PrimitiveOpBufferUnit = 0x00000001;
    pub const CUBE:     PrimitiveOpBufferUnit = 0x00000002;
    pub const INVALID:  PrimitiveOpBufferUnit = 0xFFFFFFFF;
}
