use crate::user_interface::camera::Camera;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
pub struct CameraUniformBuffer {
    /// Inverse of projection matrix multiplied by view matrix. Converts clip space coordinates to world space
    pub proj_view_inverse: [f32; 16],
    /// Camera position in world space (w component unused)
    pub position: [f32; 4],
}
impl CameraUniformBuffer {
    pub fn new(proj_view_inverse: Mat4, position: Vec3) -> Self {
        Self {
            proj_view_inverse: proj_view_inverse.to_cols_array(),
            position: [position.x, position.y, position.z, 0.0],
        }
    }

    pub fn from_camera(camera: &Camera) -> Self {
        Self::new(
            glam::DMat4::inverse(&(camera.proj_matrix() * camera.view_matrix())).as_mat4(),
            camera.position().as_vec3(),
        )
    }
}
