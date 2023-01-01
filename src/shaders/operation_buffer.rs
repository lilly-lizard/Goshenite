use core::fmt;
use std::fmt::Display;

use crate::operations::operation_collection::OperationCollection;

pub type OperationDataUnit = u32;

pub const OPERATION_UNIT_LEN: usize = 3;

pub type OperationDataSlice = [OperationDataUnit; OPERATION_UNIT_LEN];

#[rustfmt::skip]
pub mod op_codes {
    use super::OperationDataUnit;
    pub const NULL: 		OperationDataUnit = 0x00000000;
    pub const SINGLE:    	OperationDataUnit = 0x00000001;
    pub const UNION: 		OperationDataUnit = 0x00000002;
    pub const SUBTRACTION: 	OperationDataUnit = 0x00000003;
    pub const INTERSECTION: OperationDataUnit = 0x00000004;
    pub const INVALID:      OperationDataUnit = OperationDataUnit::MAX; // better to fail noticably gpu side than fail subtly
}

/// Returns a vector containing data that matches the operation storage buffer definition in `scene.comp`.
pub fn to_raw_buffer(
    collection: &OperationCollection,
) -> Result<Vec<OperationDataUnit>, OperationBufferError> {
    let data = collection.buffer_data();
    let count = data.len();
    if count >= OperationDataUnit::MAX as usize {
        return Err(OperationBufferError::DataLengthOverflow);
    }
    let mut combined_data = vec![count as OperationDataUnit];
    for p in data {
        combined_data.extend_from_slice(p);
    }
    Ok(combined_data)
}

#[derive(Clone, Copy, Debug)]
pub enum OperationBufferError {
    /// The number of primitives passed to [`OperationData::combined_data`] exceeds u32::MAX meaning the count cannot
    /// be encoded accurately.
    DataLengthOverflow,
}
impl std::error::Error for OperationBufferError {}
impl Display for OperationBufferError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "cannot create primitive data structure as the number of primitive exceeds u32::MAX"
        )
    }
}
