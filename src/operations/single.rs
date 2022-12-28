use super::operation::OperationTrait;
use crate::shaders::operation_buffer::{op_codes, OperationDataSlice, OperationDataUnit};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Single {
    // todo weak ptr or reference or something
    pub primitive_index: usize,
}
impl Single {
    pub fn new(primitive_index: usize) -> Self {
        Self { primitive_index }
    }
}
impl OperationTrait for Single {
    fn encode(&self) -> OperationDataSlice {
        [
            op_codes::UNION,
            self.primitive_index as OperationDataUnit,
            op_codes::INVALID,
        ]
    }

    fn op_name(&self) -> &'static str {
        "Single Primitive"
    }
}
impl Default for Single {
    fn default() -> Self {
        Self {
            primitive_index: op_codes::INVALID as usize,
        }
    }
}
