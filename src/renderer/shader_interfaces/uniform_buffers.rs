use crate::user_interface::camera::{Camera, ProjectionMatrixReturn};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct CameraUniformBuffer {
    /// Inverse of projection matrix multiplied by view matrix. Converts clip space coordinates to world space
    pub proj_view_inverse: [f32; 16],
    /// Camera position in world space (w component unused)
    pub position: [f32; 4],
    /// Framebuffer dimensions
    pub framebuffer_dims: [f32; 2],
    /// Near plane
    pub near: f32,
    /// Far plane
    pub far: f32,
    // 0 if false, 1 if true
    pub is_srgb_framebuffer: u32,
}
impl CameraUniformBuffer {
    pub fn new(
        proj_view_inverse: Mat4,
        position: Vec3,
        framebuffer_dimensions: [f32; 2],
        near: f32,
        far: f32,
        is_srgb_framebuffer: bool,
    ) -> Self {
        Self {
            proj_view_inverse: proj_view_inverse.to_cols_array(),
            position: [position.x, position.y, position.z, 0.0],
            framebuffer_dims: framebuffer_dimensions,
            near,
            far,
            is_srgb_framebuffer: is_srgb_framebuffer as u32,
        }
    }

    pub fn from_camera(
        camera: &mut Camera,
        framebuffer_dimensions: [f32; 2],
        is_srgb_framebuffer: bool,
    ) -> Self {
        let ProjectionMatrixReturn {
            proj,
            proj_inverse: _,
            proj_a: _,
            proj_b: _,
        } = camera.projection_matrices();

        let proj_view = proj * camera.view_matrix();
        let proj_view_inverse = Mat4::inverse(&proj_view);

        Self::new(
            proj_view_inverse,
            camera.position().as_vec3(),
            framebuffer_dimensions,
            camera.near_plane() as f32,
            camera.far_plane() as f32,
            is_srgb_framebuffer,
        )
    }
}
