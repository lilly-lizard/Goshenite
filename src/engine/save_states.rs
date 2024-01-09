use super::{
    config_engine::{LOCAL_STORAGE_DIR, SAVE_STATE_FILENAME_CAMERA, SAVE_STATE_FILENAME_OBJECTS},
    object::{object::Object, object_collection::ObjectCollection},
};
use crate::{
    config::{PRECURSOR_BYTES, PRECURSOR_BYTE_COUNT},
    user_interface::camera::Camera,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{fs, io, path::PathBuf};

// ~~ Public ~~

pub fn save_state_camera(camera: &Camera) -> Result<(), SaveStateError> {
    save_state(camera, SAVE_STATE_FILENAME_CAMERA)
}

pub fn load_state_camera() -> Result<Camera, SaveStateError> {
    load_state::<Camera>(SAVE_STATE_FILENAME_CAMERA)
}

pub fn save_all_objects(object_collection: &ObjectCollection) -> Result<(), SaveStateError> {
    let object_list: Vec<Object> = object_collection.objects().values().cloned().collect();
    save_state(&object_list, SAVE_STATE_FILENAME_OBJECTS)
}

pub fn load_objects() -> Result<Vec<Object>, SaveStateError> {
    load_state::<Vec<Object>>(SAVE_STATE_FILENAME_OBJECTS)
}

// ~~ Private ~~

fn save_state(to_serialize: &impl Serialize, file_name: &str) -> Result<(), SaveStateError> {
    let encoded_bytes =
        bincode::serialize(to_serialize).map_err(|e| SaveStateError::SerializeFailed(e))?;
    save_state_bytes(file_name, encoded_bytes)
}

fn save_state_bytes(file_name: &str, mut encoded_bytes: Vec<u8>) -> Result<(), SaveStateError> {
    // prepend encoded bytes with engine info
    let mut write_bytes = PRECURSOR_BYTES.to_vec();
    write_bytes.append(&mut encoded_bytes);

    let file_path = validated_file_path(file_name)?;
    fs::write(file_path.clone(), write_bytes).map_err(|e| {
        let file_path_string = file_path.to_str().unwrap_or(file_name).to_string();
        SaveStateError::WriteFileFailed(file_path_string, e)
    })?;
    Ok(())
}

fn load_state<T>(file_path: &str) -> Result<T, SaveStateError>
where
    T: DeserializeOwned,
{
    let encoded_bytes = load_state_bytes(file_path)?;
    bincode::deserialize::<T>(&encoded_bytes).map_err(|e| SaveStateError::DeserializeFailed(e))
}

fn load_state_bytes(file_name: &str) -> Result<Vec<u8>, SaveStateError> {
    let file_path = validated_file_path(file_name)?;
    let read_res = fs::read(file_path.clone());

    let io_error = match read_res {
        Ok(mut read_bytes) => {
            // ignore engine info for now
            let _read_precursor_bytes: Vec<u8> =
                read_bytes.drain(0..PRECURSOR_BYTE_COUNT).collect();
            return Ok(read_bytes);
        }
        Err(io_error) => io_error,
    };

    let file_path_string = file_path.to_str().unwrap_or(file_name).to_string();
    match io_error.kind() {
        io::ErrorKind::NotFound => Err(SaveStateError::FileDoesntExist(file_path_string, io_error)),
        _ => Err(SaveStateError::ReadExistingFileFailed(
            file_path_string,
            io_error,
        )),
    }
}

/// Ensures containing directories exist, but not the actual file
fn validated_file_path(file_name: &str) -> Result<PathBuf, SaveStateError> {
    // create dir if missing
    fs::create_dir_all(LOCAL_STORAGE_DIR)
        .map_err(|e| SaveStateError::CreateSaveDirectoryFailed(LOCAL_STORAGE_DIR.to_string(), e))?;

    let mut file_path = PathBuf::from(LOCAL_STORAGE_DIR);
    file_path.push(file_name);
    Ok(file_path)
}

// ~~ Errors ~~

#[derive(Debug)]
pub enum SaveStateError {
    CreateSaveDirectoryFailed(String, io::Error),
    SerializeFailed(bincode::Error),
    DeserializeFailed(bincode::Error),
    WriteFileFailed(String, io::Error),
    FileDoesntExist(String, io::Error),
    ReadExistingFileFailed(String, io::Error),
}

impl std::fmt::Display for SaveStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateSaveDirectoryFailed(directory_name, e) => write!(
                f,
                "the save state directory \"{}\" does not exist
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
        }
    }
}

impl std::error::Error for SaveStateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CreateSaveDirectoryFailed(_, e) => Some(e),
            Self::SerializeFailed(e) => Some(e),
            Self::DeserializeFailed(e) => Some(e),
            Self::WriteFileFailed(_, e) => Some(e),
            Self::FileDoesntExist(_, e) => Some(e),
            Self::ReadExistingFileFailed(_, e) => Some(e),
        }
    }
}

// ~~ Tests ~~

mod tests {
    #[allow(unused_imports)]
    use super::*;

    const TEST_FILE_NAME: &str = "_testing.gsave";

    #[test]
    fn camera_store() {
        let camera = Camera::default();
        save_state(&camera, TEST_FILE_NAME).unwrap();
    }

    #[test]
    fn camera_save_and_load() {
        let saved_camera = Camera::default();
        save_state(&saved_camera, TEST_FILE_NAME).unwrap();
        let loaded_camera = load_state(TEST_FILE_NAME).unwrap();
        assert_eq!(saved_camera, loaded_camera);
    }
}
