use crate::{
    helper::from_enum_impl::from_enum_impl,
    shaders::operation_buffer::{
        op_codes, OperationDataSlice, OperationDataUnit, OPERATION_UNIT_LEN,
    },
};

use super::single::Single;

pub trait OperationTrait: Default + PartialEq + Clone {
    /// Returns buffer compatible operation data as a [`PrimitiveDataSlice`].
    ///
    /// _Note: must match the decode process in `scene.comp`_
    fn encode(&self) -> OperationDataSlice;
    /// Returns the (capitalised) name of the operation as a str
    fn op_name(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operation {
    Null,
    Raw {
        op: OperationDataUnit,
        p1_index: OperationDataUnit,
        p2_index: OperationDataUnit,
    },
    Single(Single),
}
impl OperationTrait for Operation {
    fn encode(&self) -> OperationDataSlice {
        match self {
            Self::Null => [op_codes::NULL; OPERATION_UNIT_LEN],
            Self::Raw {
                op,
                p1_index,
                p2_index,
            } => [*op, *p1_index, *p2_index],
            Self::Single(op) => op.encode(),
        }
    }

    fn op_name(&self) -> &'static str {
        match self {
            Self::Null => "Null",
            Self::Raw { .. } => "Raw",
            Self::Single(op) => op.op_name(),
        }
    }
}
impl Default for Operation {
    fn default() -> Self {
        Self::Null
    }
}
from_enum_impl!(Operation, Single);
