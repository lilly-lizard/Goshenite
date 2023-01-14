pub type ObjectDataUnit = u32;

#[rustfmt::skip]
pub mod op_codes {
    use super::ObjectDataUnit;
    pub const NULL: 		ObjectDataUnit = 0x00000000;
    pub const UNION: 		ObjectDataUnit = 0x00000001; // OR
    pub const INTERSECTION: ObjectDataUnit = 0x00000002; // AND
    pub const SUBTRACTION: 	ObjectDataUnit = 0x00000003;
    pub const INVALID:      ObjectDataUnit = 0xFFFFFFFF; // better to fail noticably gpu side than fail subtly
}

/// Each primitive is encoded into an array of length `PRIMITIVE_UNIT_LEN`. This value should match the one defined in `primitives.glsl`.
pub const PRIMITIVE_UNIT_LEN: usize = 8;

/// An array which a primitive can be encoded into. Corresponds to the decoding logic in `scene.comp`.
pub type PrimitiveDataSlice = [ObjectDataUnit; PRIMITIVE_UNIT_LEN];

/// Each `PrimitiveDataSlice` begins with a primitive code defining the type of primitive that has been encoded.
/// The values defined here should match the ones defined in `primitives.glsl`.
#[rustfmt::skip]
pub mod primitive_codes {
    use super::ObjectDataUnit;
    pub const NULL:     ObjectDataUnit = 0x00000000;
    pub const SPHERE:   ObjectDataUnit = 0x00000001;
    pub const CUBE:     ObjectDataUnit = 0x00000002;
}
