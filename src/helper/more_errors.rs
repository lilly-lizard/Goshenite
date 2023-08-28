use super::unique_id_gen::UniqueId;
use std::{error, fmt};

#[derive(Debug, Clone, Copy)]
pub enum CollectionError {
    OutOfBounds { index: usize, size: usize },
    InvalidId { raw_id: UniqueId },
}
impl fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::OutOfBounds { index, size } => {
                write!(f, "index {} out of bounds. size = {}", index, size)
            }
            Self::InvalidId { raw_id } => write!(f, "invalid id {}", raw_id),
        }
    }
}
impl error::Error for CollectionError {}
