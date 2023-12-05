use glam::Vec3;

pub const RENDER_THREAD_WAIT_TIMEOUT_SECONDS: f64 = 2.;

pub const DEFAULT_RADIUS: f32 = 0.5;
pub const DEFAULT_DIMENSIONS: Vec3 = Vec3::ONE;

pub mod primitive_names {
    pub const SPHERE: &'static str = "Sphere";
    pub const CUBE: &'static str = "Cube";
    pub const UBER_PRIMITIVE: &'static str = "Uber Primitive";
}

pub const AABB_EDGE: Vec3 = Vec3::splat(0.1);
