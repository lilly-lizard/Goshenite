use super::unique_id_gen::{UniqueId, UniqueIdError};
use egui_dnd::utils::ShiftSliceError;
use std::{error, fmt, io};

// ~~ Collections ~~

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

// ~~ File IO ~~

#[derive(Debug)]
pub enum IoError {
    CreateDirectoryFailed(String, io::Error),
    SerializeFailed(bincode::Error),
    DeserializeFailed(bincode::Error),
    WriteFileFailed(String, io::Error),
    FileDoesntExist(String, io::Error),
    ReadExistingFileFailed(String, io::Error),
    ReadBufferFailed(io::Error),
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateDirectoryFailed(directory_name, e) => write!(
                f,
                "the directory \"{}\" does not exist
				and attempting to create it failed due to: {}",
                directory_name, e,
            ),
            Self::SerializeFailed(e) => write!(f, "data serialization failed: {}", e),
            Self::DeserializeFailed(e) => write!(f, "data deserialization failed: {}", e),
            Self::WriteFileFailed(file_name, e) => write!(
                f,
                "failed to open file \"{}\" for writing due to: {}",
                file_name, e
            ),
            Self::FileDoesntExist(file_name, e) => write!(
                f,
                "attempted to read a non-existant file \"{}\". io error: {}",
                file_name, e
            ),
            Self::ReadExistingFileFailed(file_name, e) => write!(
                f,
                "reading an existing file \"{}\" failed due to: {}",
                file_name, e
            ),
            Self::ReadBufferFailed(e) => {
                write!(f, "failed to read from a file buffer due to: {}", e)
            }
        }
    }
}

impl std::error::Error for IoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CreateDirectoryFailed(_, e) => Some(e),
            Self::SerializeFailed(e) => Some(e),
            Self::DeserializeFailed(e) => Some(e),
            Self::WriteFileFailed(_, e) => Some(e),
            Self::FileDoesntExist(_, e) => Some(e),
            Self::ReadExistingFileFailed(_, e) => Some(e),
            Self::ReadBufferFailed(e) => Some(e),
        }
    }
}

impl IoError {
    pub fn read_file_error(io_error: io::Error, file_path_string: String) -> Self {
        match io_error.kind() {
            io::ErrorKind::NotFound => IoError::FileDoesntExist(file_path_string, io_error),
            _ => IoError::ReadExistingFileFailed(file_path_string, io_error),
        }
    }
}
