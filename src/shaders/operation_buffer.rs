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
