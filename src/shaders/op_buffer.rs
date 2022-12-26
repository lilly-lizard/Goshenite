pub type OpDataUnit = u32;
pub const OP_UNIT_LEN: usize = 3;
pub type OpDataSlice = [OpDataUnit; OP_UNIT_LEN];
pub mod op_codes {
    use super::OpDataUnit;
    pub const NULL: OpDataUnit = 0x00000000;
    pub const PRIMITIVE_1: OpDataUnit = 0x00000001;
    pub const PRIMITIVE_2: OpDataUnit = 0x00000002;
    pub const UNION: OpDataUnit = 0x00000003;
    pub const SUBTRACTION: OpDataUnit = 0x00000004;
    pub const INTERSECTION: OpDataUnit = 0x00000005;
}
