use super::operation::OperationTrait;
use crate::{
    helper::more_errors::{CollectionError, IndexError},
    shaders::operation_buffer::OperationDataSlice,
};

/// Collection of [`Operation`]s. Also contains encoded data vector ready to upload to the gpu.
#[derive(Default)]
pub struct OperationCollection {
    buffer_data: Vec<OperationDataSlice>,
    operations: Vec<Box<dyn OperationTrait>>,
}
impl OperationCollection {
    pub fn buffer_data(&self) -> &Vec<OperationDataSlice> {
        &self.buffer_data
    }

    pub fn append(&mut self, operation: Box<dyn Operation>) {
        self.operations.push(operation);
        self.buffer_data.push(operation.encode());
    }

    pub fn update(
        &mut self,
        index: usize,
        new_operation: Operation,
    ) -> Result<(), CollectionError> {
        if let Some(s_ref) = self.operations.get_mut(index) {
            let data_ref = self
                .buffer_data
                .get_mut(index)
                .ok_or(CollectionError::MismatchedDataLength)?;
            let encoded = new_operation.encode();
            *data_ref = encoded;
            *s_ref = new_operation;
            Ok(())
        } else {
            Err(IndexError::OutOfBounds {
                index,
                size: self.operations.len(),
            }
            .into())
        }
    }
}
