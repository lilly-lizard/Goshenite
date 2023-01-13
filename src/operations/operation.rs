use crate::shaders::operation_buffer::OperationDataSlice;

pub trait OperationTrait {
    /// Returns buffer compatible operation data as a [`PrimitiveDataSlice`].
    ///
    /// _Note: must match the decode process in `scene.comp`_
    fn encode(&self) -> OperationDataSlice;
    /// Returns the (capitalised) name of the operation as a str
    fn op_name(&self) -> &'static str;
}
