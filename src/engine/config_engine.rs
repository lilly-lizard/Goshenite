use glam::Vec3;

pub const RENDER_THREAD_WAIT_TIMEOUT_SECONDS: f64 = 2.;

pub const DEFAULT_RADIUS: f32 = 0.5;
pub const DEFAULT_DIMENSIONS: Vec3 = Vec3::ONE;
pub const DEFAULT_ALBEDO: Vec3 = Vec3::new(0.8, 0.8, 0.8);
pub const DEFAULT_SPECULAR: f32 = 0.5;

pub mod primitive_names {
    pub const SPHERE: &str = "Sphere";
    pub const CUBE: &str = "Cube";
    pub const UBER_PRIMITIVE: &str = "Uber Primitive";
}

pub const AABB_EDGE: Vec3 = Vec3::splat(0.5);

pub const LOCAL_STORAGE_DIR: &str = ".goshenite";
pub const SAVE_STATE_FILENAME_CAMERA: &str = "camera.gsave";
pub const SAVE_STATE_FILENAME_OBJECTS: &str = "objects.gsave";
