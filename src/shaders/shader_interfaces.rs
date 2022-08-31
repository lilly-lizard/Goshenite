//! Contains structs matching the interfaces in shaders

pub mod render_comp {
    use bytemuck::Zeroable;
    use glam::{Mat4, Vec3, Vec4};

    pub const set: usize = 0; // descriptor set index
    pub const binding_render_image: u32 = 0; // render image descriptor binding
    pub const binding_camera: u32 = 1; // camera descriptor binding

    // Render compute shader push constant struct
    #[derive(Zeroable, Copy, Clone)]
    pub struct Camera {
        pub viewInverse: [[f32; 4]; 4],
        pub projInverse: [[f32; 4]; 4],
        pub position: [f32; 4],
    }
    // allows vulkano::buffer::BufferContents to be implimented
    unsafe impl Send for Camera {}
    unsafe impl Sync for Camera {}
    unsafe impl bytemuck::Pod for Camera {}

    impl Camera {
        pub fn new(view_inverse: Mat4, proj_inverse: Mat4, position: Vec3) -> Self {
            Camera {
                viewInverse: view_inverse.to_cols_array_2d(),
                projInverse: proj_inverse.to_cols_array_2d(),
                position: Vec4::from((position, 0.)).to_array(),
            }
        }
    }
}
