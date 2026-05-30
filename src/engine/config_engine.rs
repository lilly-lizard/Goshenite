use glam::Vec3;

pub const DEFAULT_RADIUS: f32 = 0.5;
pub const DEFAULT_DIMENSIONS: Vec3 = Vec3::ONE;
pub const DEFAULT_ALBEDO: Vec3 = Vec3::new(0.9, 0.8, 0.2);
pub const DEFAULT_SPECULAR: f32 = 0.5;

pub mod primitive_names {
    pub const SPHERE: &str = "Sphere";
    pub const CUBE: &str = "Cube";
    pub const UBER_PRIMITIVE: &str = "Uber Primitive";
}

pub const AABB_EDGE: f32 = 0.05;

pub const DEFAULT_ORIGIN: Vec3 = Vec3::ZERO;

#[cfg(target_os = "linux")]
/// Note: `~` or `$HOME` cannot be included in this path. `$HOME` has to be queried and prepended to this
pub const USER_CONFIG_DIR: &str = ".config/goshenite";
#[cfg(target_os = "macos")]
pub const USER_CONFIG_DIR: &str = "~/Library/Application Support/Goshenite";
#[cfg(target_os = "windows")]
pub const USER_CONFIG_DIR: &str = "%APPDATA%\\Goshenite";
pub const SETTINGS_FILE_NAME: &str = "settings.json";
pub const HIDDEN_STORAGE_DIR: &str = ".goshenite";
pub const SAVE_STATE_FILENAME_CAMERA: &str = "camera.gsave";
pub const SAVE_STATE_FILENAME_GUI_POSITIONS: &str = "gui_positions.gsave";
pub const SAVE_STATE_FILENAME_SCENE: &str = "scene.gsave";
