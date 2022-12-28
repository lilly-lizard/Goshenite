use super::operation::OperationTrait;
use crate::shaders::operation_buffer::{op_codes, OperationDataSlice, OperationDataUnit};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Union {
    // todo weak ptr or reference or something
    pub prim_1_index: usize,
    pub prim_2_index: usize,
}
impl OperationTrait for Union {
    fn encode(&self) -> OperationDataSlice {
        [
            op_codes::UNION,
            self.prim_1_index as OperationDataUnit,
            self.prim_2_index as OperationDataUnit,
        ]
    }

    fn op_name(&self) -> &'static str {
        "Union"
    }
}
impl Default for Union {
    fn default() -> Self {
        Self {
            prim_1_index: 0,
            prim_2_index: 0,
        }
    }
}
