use super::{primitive::Primitive, primitive::PrimitiveTrait};
use crate::shaders::shader_interfaces::PrimitiveDataSlice;
use std::{error, fmt};

/// Collection of [`Primitive`]s. Also contains encoded data to upload to the gpu.
#[derive(Default, Debug, Clone)]
pub struct PrimitiveCollection {
    /// Encoded primitive data.
    data: Vec<PrimitiveDataSlice>,
    primitives: Vec<Primitive>,
    selected_primitive_index: Option<usize>,
}
impl PrimitiveCollection {
    /// Returns vector containing encoded data for all the primitives in the collection.
    pub fn encoded_data(&self) -> &Vec<PrimitiveDataSlice> {
        &self.data
    }

    /// Appends a new primitive to the primitive collection.
    pub fn add_primitive(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
        self.data.push(primitive.encode());
    }

    /// Returns a reference to the primitives collection.
    pub fn primitives(&self) -> &Vec<Primitive> {
        &self.primitives
    }

    /// Updates an existing primitive in collection at `index`.
    pub fn update_primitive(
        &mut self,
        index: usize,
        new_primitive: Primitive,
    ) -> Result<(), PrimitiveCollectionError> {
        if let Some(s_ref) = self.primitives.get_mut(index) {
            let data_ref = self
                .data
                .get_mut(index)
                .ok_or(PrimitiveCollectionError::MismatchedDataLength)?;
            let encoded = new_primitive.encode();
            *data_ref = encoded;
            *s_ref = new_primitive;
            Ok(())
        } else {
            Err(PrimitiveCollectionError::InvalidPrimitiveIndex {
                index,
                primitive_count: self.primitives.len(),
            })
        }
    }

    // todo doc
    pub fn selected_primitive_index(&self) -> Option<usize> {
        self.selected_primitive_index
    }

    // todo doc
    pub fn selected_primitive(&self) -> Option<Primitive> {
        if let Some(index) = self.selected_primitive_index {
            self.primitives.get(index).cloned()
        } else {
            None
        }
    }

    // todo doc
    pub fn set_selected_primitive(&mut self, index: usize) -> Result<(), PrimitiveCollectionError> {
        if let Some(_) = self.primitives.get(index) {
            self.selected_primitive_index = Some(index);
            Ok(())
        } else {
            Err(PrimitiveCollectionError::InvalidPrimitiveIndex {
                index,
                primitive_count: self.primitives.len(),
            })
        }
    }

    // todo doc
    pub fn unset_selected_primitive(&mut self) {
        self.selected_primitive_index = None;
    }
}

#[derive(Debug)]
pub enum PrimitiveCollectionError {
    /// Attempted to access primitive with out of bounds index.
    InvalidPrimitiveIndex {
        index: usize,
        primitive_count: usize,
    },
    /// The data vector length doesn't match the primitive vector. This is a bug!!!
    MismatchedDataLength,
}
impl fmt::Display for PrimitiveCollectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidPrimitiveIndex { index, primitive_count } =>
                write!(f, "attempted to access primitive without out of bounds index. index = {}, primitive count = {}",
                index, primitive_count),
            Self::MismatchedDataLength =>
                write!(f, "the data vector length doesn't match the primitive vector. this is a PrimitiveCollection bug!!!"),
        }
    }
}
impl error::Error for PrimitiveCollectionError {}
