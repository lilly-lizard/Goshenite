use crate::primitives::primitive_collection::PrimitiveCollection;
use std::fmt::{self, Display};

/// Shorthand for the data type in the primitive storage buffer defined in `scene.comp`.
pub type PrimitiveDataUnit = u32;
/// Each primitive is encoded into an array of length `PRIMITIVE_UNIT_LEN`. This value should match the one defined in `primitives.glsl`.
pub const PRIMITIVE_UNIT_LEN: usize = 8;
/// An array which a primitive can be encoded into. Corresponds to the decoding logic in `scene.comp`.
pub type PrimitiveDataSlice = [PrimitiveDataUnit; PRIMITIVE_UNIT_LEN];
/// Each `PrimitiveDataSlice` begins with a primitive code defining the type of primitive that has been encoded.
/// The values defined here should match the ones defined in `primitives.glsl`.
pub mod primitive_codes {
    use super::PrimitiveDataUnit;
    pub const NULL: PrimitiveDataUnit = 0x00000000;
    pub const SPHERE: PrimitiveDataUnit = 0x00000001;
    pub const CUBE: PrimitiveDataUnit = 0x00000002;
}

/// Returns a vector containing data that matches the primitive storage buffer definition in `scene.comp`.
pub fn to_raw_buffer(
    primitives: &PrimitiveCollection,
) -> Result<Vec<PrimitiveDataUnit>, PrimitiveBufferError> {
    let data = primitives.encoded_data();
    let count = data.len();
    if count >= PrimitiveDataUnit::MAX as usize {
        return Err(PrimitiveBufferError::DataLengthOverflow);
    }
    let mut combined_data = vec![count as PrimitiveDataUnit];
    for p in data {
        combined_data.extend_from_slice(p);
    }
    Ok(combined_data)
}

#[derive(Clone, Copy, Debug)]
pub enum PrimitiveBufferError {
    /// The number of primitives passed to [`PrimitiveData::combined_data`] exceeds u32::MAX meaning the count cannot
    /// be encoded accurately.
    DataLengthOverflow,
}
impl std::error::Error for PrimitiveBufferError {}
impl Display for PrimitiveBufferError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "cannot create primitive data structure as the number of primitive exceeds u32::MAX"
        )
    }
}
