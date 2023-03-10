use crate::engine::object::object::ObjectId;
use ash::vk;
use bort::VertexInputState;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use memoffset::offset_of;

/// Should match inputs in `overlay.vert`
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)] // todo bytemuck needed now?
pub struct OverlayVertex {
    pub in_position: [f32; 4],
    pub in_normal: [f32; 4],
    pub in_color: [f32; 4],
}

impl OverlayVertex {
    pub const fn new(position: Vec3, normal: Vec3, color: Vec3) -> Self {
        Self {
            in_position: [position.x, position.y, position.z, 1.],
            in_normal: [normal.x, normal.y, normal.z, 1.],
            in_color: [color.x, color.y, color.z, 1.],
        }
    }
}

/// Should match vertex definition for `gui.vert` (except color is `[f32; 4]`)
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct EguiVertex {
    pub in_position: [f32; 2],
    pub in_tex_coords: [f32; 2],
    pub in_color: [f32; 4],
}

impl EguiVertex {
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }

    pub fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            // in_position
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Self, in_position) as u32,
            },
            // in_tex_coords
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Self, in_tex_coords) as u32,
            },
            // in_color
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Self, in_color) as u32,
            },
        ]
    }

    pub fn vertex_input_state() -> VertexInputState {
        VertexInputState {
            vertex_binding_descriptions: vec![Self::binding_description()],
            vertex_attribute_descriptions: Self::attribute_descriptions(),
            ..Default::default()
        }
    }
}

/// Should match inputs in `bounding_box.vert`
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct BoundingBoxVertex {
    pub in_position: [f32; 4],
    pub in_object_id: u32,
}

impl BoundingBoxVertex {
    pub const fn new(position: Vec3, object_id: ObjectId) -> Self {
        Self {
            in_position: [position.x, position.y, position.z, 1.],
            in_object_id: object_id as u32,
        }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }

    pub fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            // in_position
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Self, in_position) as u32,
            },
            // in_object_id
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32_UINT,
                offset: offset_of!(Self, in_object_id) as u32,
            },
        ]
    }

    pub fn vertex_input_state() -> VertexInputState {
        VertexInputState {
            vertex_binding_descriptions: vec![Self::binding_description()],
            vertex_attribute_descriptions: Self::attribute_descriptions(),
            ..Default::default()
        }
    }
}
