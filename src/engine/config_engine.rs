use glam::Vec3;

pub const JOIN_THREAD_WAIT_TIMEOUT_SECONDS: f64 = 2.;

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

pub const LOCAL_STORAGE_DIR: &str = ".goshenite";
pub const SAVE_STATE_FILENAME_CAMERA: &str = "camera.gsave";
pub const SAVE_STATE_FILENAME_OBJECTS: &str = "objects.gsave";
