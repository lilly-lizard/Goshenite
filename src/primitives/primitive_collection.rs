use super::{primitive::Primitive, primitive::PrimitiveTrait};
use crate::{
    helper::more_errors::{CollectionError, IndexError},
    shaders::primitive_buffer::PrimitiveDataSlice,
};

/// Collection of [`Primitive`]s. Also contains encoded data vector ready to upload to the gpu.
#[derive(Default, Debug, Clone)]
pub struct PrimitiveCollection {
    buffer_data: Vec<PrimitiveDataSlice>,
    primitives: Vec<Primitive>,
    selected_index: Option<usize>,
}
impl PrimitiveCollection {
    pub fn buffer_data(&self) -> &Vec<PrimitiveDataSlice> {
        &self.buffer_data
    }

    pub fn append(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
        self.buffer_data.push(primitive.encode());
    }

    pub fn primitives(&self) -> &Vec<Primitive> {
        &self.primitives
    }

    pub fn update(
        &mut self,
        index: usize,
        new_primitive: Primitive,
    ) -> Result<(), CollectionError> {
        if let Some(s_ref) = self.primitives.get_mut(index) {
            let data_ref = self
                .buffer_data
                .get_mut(index)
                .ok_or(CollectionError::MismatchedDataLength)?;
            let encoded = new_primitive.encode();
            *data_ref = encoded;
            *s_ref = new_primitive;
            Ok(())
        } else {
            Err(IndexError::OutOfBounds {
                index,
                size: self.primitives.len(),
            }
            .into())
        }
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn selected_primitive(&self) -> Option<Primitive> {
        if let Some(index) = self.selected_index {
            self.primitives.get(index).cloned()
        } else {
            None
        }
    }

    pub fn set_selected_index(&mut self, index: usize) -> Result<(), CollectionError> {
        if let Some(_) = self.primitives.get(index) {
            self.selected_index = Some(index);
            Ok(())
        } else {
            Err(IndexError::OutOfBounds {
                index,
                size: self.primitives.len(),
            }
            .into())
        }
    }

    pub fn unset_selected_primitive(&mut self) {
        self.selected_index = None;
    }
}
