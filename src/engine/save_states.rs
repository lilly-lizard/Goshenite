use super::{
    config_engine::{SAVE_STATE_FILENAME_CAMERA, SAVE_STATE_FILENAME_SCENE},
    object::{object::Object, object_collection::ObjectCollection},
};
use crate::{
    config::{PRECURSOR_BYTES, PRECURSOR_BYTE_COUNT},
    engine::{
        commands::{command_failed_warn, Command},
        config_engine::{HIDDEN_STORAGE_DIR, SAVE_STATE_FILENAME_GUI_POSITIONS},
    },
    helper::more_errors::IoError,
    user_interface::camera::Camera,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{fs, path::PathBuf};

// ~~ Generalized Save Functions ~~

fn save_state(
    to_serialize: &impl Serialize,
    file_name: &str,
    save_directory: Option<&str>,
) -> Result<(), IoError> {
    let mut encoded_bytes =
        bincode::serialize(to_serialize).map_err(|e| IoError::SerializeFailed(e))?;

    // prepend encoded bytes with engine info
    let mut write_bytes = PRECURSOR_BYTES.to_vec();
    write_bytes.append(&mut encoded_bytes);

    let file_path = if let Some(directory) = save_directory {
        validated_file_path(file_name, directory)?
    } else {
        file_path(file_name)
    };
    fs::write(file_path.clone(), write_bytes).map_err(|e| {
        let file_path_string = file_path.to_str().unwrap_or(file_name).to_string();
        IoError::WriteFileFailed(file_path_string, e)
    })?;
    Ok(())
}

fn load_state<T>(file_path: &str, load_directory: Option<&str>) -> Result<T, IoError>
where
    T: DeserializeOwned,
{
    let encoded_bytes = load_state_bytes(file_path, load_directory)?;
    bincode::deserialize::<T>(&encoded_bytes).map_err(|e| IoError::DeserializeFailed(e))
}

fn load_state_bytes(file_name: &str, load_directory: Option<&str>) -> Result<Vec<u8>, IoError> {
    let file_path = if let Some(directory) = load_directory {
        validated_file_path(file_name, directory)?
    } else {
        file_path(file_name)
    };
    let read_res = fs::read(file_path.clone());

    let mut read_bytes = match read_res {
        Ok(read_bytes) => read_bytes,
        Err(io_error) => {
            let file_path_string = file_path.to_str().unwrap_or(file_name).to_string();
            return Err(IoError::read_file_error(io_error, file_path_string));
        }
    };

    // ignore engine info for now
    let _read_precursor_bytes: Vec<u8> = read_bytes.drain(0..PRECURSOR_BYTE_COUNT).collect();
    return Ok(read_bytes);
}

/// Ensures containing directories exist, but not the actual file
fn file_path(file_name: &str) -> PathBuf {
    PathBuf::from(file_name)
}

/// Ensures containing directories exist, but not the actual file
fn validated_file_path(file_name: &str, directory: &str) -> Result<PathBuf, IoError> {
    // create dir if missing
    fs::create_dir_all(directory)
        .map_err(|e| IoError::CreateDirectoryFailed(directory.to_string(), e))?;

    let mut file_path = PathBuf::from(directory);
    file_path.push(file_name);
    Ok(file_path)
}

// ~~ Type Specific Functions ~~

pub fn save_all_objects(object_collection: &ObjectCollection, command: Command) {
    let object_list: Vec<Object> = object_collection.objects().values().cloned().collect();
    let save_state_res = save_state(&object_list, SAVE_STATE_FILENAME_SCENE, None);
    if let Err(e) = save_state_res {
        let failed_because = format!("error while saving objects: {}", e);
        command_failed_warn(command, &failed_because);
    }
}

pub fn load_objects(object_collection: &mut ObjectCollection, command: Command) {
    let load_state_res = load_state::<Vec<Object>>(SAVE_STATE_FILENAME_SCENE, None);
    let loaded_objects = match load_state_res {
        Ok(o) => o,
        Err(e) => {
            let failed_because = format!("error while loading saved objects: {}", e);
            command_failed_warn(command, &failed_because);
            return;
        }
    };

    let insert_objects_res = object_collection.push_objects(loaded_objects);
    if let Err(e) = insert_objects_res {
        let failed_because = format!("error while inserting loaded objects: {}", e);
        command_failed_warn(command, &failed_because);
    }
}

pub fn save_state_camera(camera: &Camera, command: Command) {
    let save_state_res = save_state(camera, SAVE_STATE_FILENAME_CAMERA, Some(HIDDEN_STORAGE_DIR));
    if let Err(e) = save_state_res {
        let failed_because = format!("error while saving camera state: {}", e);
        command_failed_warn(command, &failed_because);
    }
}

pub fn load_state_camera(camera: &mut Camera, command: Command) {
    let load_state_res = load_state::<Camera>(SAVE_STATE_FILENAME_CAMERA, Some(HIDDEN_STORAGE_DIR));
    let loaded_camera = match load_state_res {
        Ok(c) => c,
        Err(e) => {
            let failed_because = format!("error while loading saved camera state: {}", e);
            command_failed_warn(command, &failed_because);
            return;
        }
    };
    *camera = loaded_camera;
}

pub fn save_state_gui_positions(gui_memory: &egui::Memory) -> Result<(), IoError> {
    save_state(
        gui_memory,
        SAVE_STATE_FILENAME_GUI_POSITIONS,
        Some(HIDDEN_STORAGE_DIR),
    )
}
pub fn load_state_gui_positions() -> Result<egui::Memory, IoError> {
    load_state::<egui::Memory>(SAVE_STATE_FILENAME_GUI_POSITIONS, Some(HIDDEN_STORAGE_DIR))
}

// ~~ Tests ~~

#[allow(dead_code)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    const TEST_FILE_NAME: &str = "_testing.gsave";

    #[test]
    fn camera_store() {
        let camera = Camera::default();
        save_state(&camera, TEST_FILE_NAME, Some(HIDDEN_STORAGE_DIR)).unwrap();
    }

    #[test]
    fn camera_save_and_load() {
        let saved_camera = Camera::default();
        save_state(&saved_camera, TEST_FILE_NAME, Some(HIDDEN_STORAGE_DIR)).unwrap();
        let loaded_camera = load_state(TEST_FILE_NAME, Some(HIDDEN_STORAGE_DIR)).unwrap();
        assert_eq!(saved_camera, loaded_camera);
    }
}
