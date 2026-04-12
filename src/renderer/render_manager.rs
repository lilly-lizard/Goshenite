use crate::{
    engine::object::objects_delta::ObjectsDelta,
    renderer::{config_renderer::RenderOptions, element_id_reader::ElementAtPoint},
    user_interface::camera::Camera,
};
use egui::{ClippedPrimitive, TexturesDelta};
use std::sync::Arc;
use winit::window::Window;

pub struct RenderManager {}

impl RenderManager {
    pub fn new(window: Arc<Window>, scale_factor: f32) -> anyhow::Result<Self> {
        todo!()
    }
    pub fn max_2d_image_size(&self) -> usize {
        todo!()
    }
    pub fn update_camera(&mut self, camera: &Camera) -> anyhow::Result<()> {
        todo!()
    }
    pub fn update_objects(&mut self, objects_delta: ObjectsDelta) -> anyhow::Result<()> {
        todo!()
    }
    pub fn update_gui_textures(
        &mut self,
        textures_delta: Vec<TexturesDelta>,
    ) -> anyhow::Result<()> {
        todo!()
    }
    pub fn set_gui_primitives(&mut self, gui_primitives: Vec<ClippedPrimitive>) {
        todo!()
    }
    pub fn render_frame(&mut self, overlay_options: RenderOptions) -> anyhow::Result<()> {
        todo!()
    }
    pub fn set_window_just_resized_flag(&mut self) {
        todo!()
    }
    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        todo!()
    }
    pub fn get_element_at_screen_coordinate(
        &mut self,
        screen_coordinate: [f32; 2],
    ) -> anyhow::Result<Option<ElementAtPoint>> {
        todo!()
    }
}
