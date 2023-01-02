use super::operation::OperationTrait;
use crate::shaders::operation_buffer::{op_codes, OperationDataSlice, OperationDataUnit};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Union {
    // todo weak ptr or reference or something
    pub primitive_index_1: usize,
    pub primitive_index_2: usize,
}
impl Union {
    pub fn new(primitive_index_1: usize, primitive_index_2: usize) -> Self {
        Self {
            primitive_index_1,
            primitive_index_2,
        }
    }
}
impl OperationTrait for Union {
    fn encode(&self) -> OperationDataSlice {
        [
            op_codes::UNION,
            self.primitive_index_1 as OperationDataUnit,
            self.primitive_index_2 as OperationDataUnit,
        ]
    }

    fn op_name(&self) -> &'static str {
        "Union"
    }
}
impl Default for Union {
    fn default() -> Self {
        Self {
            primitive_index_1: 0,
            primitive_index_2: 0,
        }
    }
}
