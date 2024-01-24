use super::unique_id_gen::{UniqueId, UniqueIdError};
use egui_dnd::utils::ShiftSliceError;
use std::{error, fmt};

#[derive(Debug)]
pub enum CollectionError {
    OutOfBounds { index: usize, size: usize },
    InvalidId { raw_id: UniqueId },
    UniqueIdError(UniqueIdError),
    ShiftSliceError(ShiftSliceError),
}

impl fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::OutOfBounds { index, size } => {
                write!(f, "index {} out of bounds. size = {}", index, size)
            }
            Self::InvalidId { raw_id } => write!(f, "invalid id {}", raw_id),
            Self::UniqueIdError(e) => e.fmt(f),
            Self::ShiftSliceError(e) => e.fmt(f),
        }
    }
}

impl error::Error for CollectionError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::UniqueIdError(e) => Some(e),
            Self::ShiftSliceError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<UniqueIdError> for CollectionError {
    fn from(value: UniqueIdError) -> Self {
        Self::UniqueIdError(value)
    }
}

impl From<ShiftSliceError> for CollectionError {
    fn from(value: ShiftSliceError) -> Self {
        Self::ShiftSliceError(value)
    }
}
