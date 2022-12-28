use std::{error, fmt};

use super::from_enum_impl::from_enum_impl;

#[derive(Debug)]
pub enum IndexError {
    OutOfBounds { index: usize, size: usize },
    Invalid,
}
impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::OutOfBounds { index, size } => {
                write!(f, "index {} out of bounds. size = {}", index, size)
            }
            Self::Invalid => write!(f, "invalid index"),
        }
    }
}
impl error::Error for IndexError {}

#[derive(Debug)]
pub enum CollectionError {
    IndexError(IndexError),
    /// The encoded data length doesn't match the collection. Indicates a bug.
    MismatchedDataLength,
}
impl fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::IndexError(e) => e.fmt(f),
            Self::MismatchedDataLength =>
                write!(f, "the encoded data vector length doesn't match the collection. this is a PrimitiveCollection or OperationCollection bug!!!"), // todo test
        }
    }
}
impl error::Error for CollectionError {}
from_enum_impl!(CollectionError, IndexError);
