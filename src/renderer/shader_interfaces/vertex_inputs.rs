use crate::{engine::object::object::ObjectId, helper::unique_id_gen::UniqueIdType};
use ash::vk;
use bort_vk::VertexInputState;
use bytemuck::NoUninit;
use glam::{Mat4, Vec3};
use memoffset::offset_of;

pub trait VulkanVertex {
    fn binding_descriptions() -> Vec<vk::VertexInputBindingDescription>;
    fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription>;
    fn vertex_input_state() -> VertexInputState {
        VertexInputState {
            vertex_binding_descriptions: Self::binding_descriptions(),
            vertex_attribute_descriptions: Self::attribute_descriptions(),
            flags: Default::default(),
        }
    }
}

/// Should match vertex definition for `gui.vert` (except color is `[f32; 4]`)
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, NoUninit)]
pub struct EguiVertex {
    pub in_position: [f32; 2],
    pub in_tex_coords: [f32; 2],
    pub in_color: [f32; 4],
}

impl EguiVertex {
    pub fn from_egui_vertex(egui_vertex: &egui::epaint::Vertex) -> Self {
        let color = [
            egui_vertex.color.r() as f32 / 255.,
            egui_vertex.color.g() as f32 / 255.,
            egui_vertex.color.b() as f32 / 255.,
            egui_vertex.color.a() as f32 / 255.,
        ];

        Self {
            in_position: egui_vertex.pos.into(),
            in_tex_coords: egui_vertex.uv.into(),
            in_color: color,
        }
    }
}

impl VulkanVertex for EguiVertex {
    fn binding_descriptions() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            // in_position
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Self, in_position) as u32,
            },
            // in_tex_coords
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Self, in_tex_coords) as u32,
            },
            // in_color
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Self, in_color) as u32,
            },
        ]
    }
}

/// Should match inputs in `bounding_mesh.vert`
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, NoUninit)]
pub struct BoundingBoxVertex {
    pub in_position: [f32; 4],
    pub in_object_id: u32,
}

impl BoundingBoxVertex {
    pub fn new(position: Vec3, object_id: ObjectId) -> Self {
        Self {
            in_position: [position.x, position.y, position.z, 1.],
            in_object_id: object_id.raw_id() as u32,
        }
    }
}

impl VulkanVertex for BoundingBoxVertex {
    fn binding_descriptions() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            // in_position
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Self, in_position) as u32,
            },
            // in_object_id
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32_UINT,
                offset: offset_of!(Self, in_object_id) as u32,
            },
        ]
    }
}

/// Should match inputs in `gizmos.frag`
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, NoUninit)]
pub struct GizmoVertex {
    // location 0: in_position: Vec4,
    // location 1: in_orientation: Mat4,
}

impl VulkanVertex for GizmoVertex {
    fn binding_descriptions() -> Vec<vk::VertexInputBindingDescription> {
        vec![
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: std::mem::size_of::<[f32; 4]>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            },
            vk::VertexInputBindingDescription {
                binding: 1,
                stride: std::mem::size_of::<Mat4>() as u32,
                input_rate: vk::VertexInputRate::INSTANCE,
            },
        ]
    }

    fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            // in_position: Vec4
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 0,
            },
            // in_orientation: Mat4
            // in order for a mat4 to be used as a vertex input, 4 vec4 locations are used
            // the driver then combines these locations into 1 so the shader can access the mat4. pretty neat!
            // https://old.reddit.com/r/vulkan/comments/8zx1hn/matrix_as_vertex_input/
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: std::mem::size_of::<[f32; 4]>() as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 3,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: std::mem::size_of::<[f32; 8]>() as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 4,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: std::mem::size_of::<[f32; 12]>() as u32,
            },
        ]
    }
}

/// Should match inputs in `skybox.vert`
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, NoUninit)]
pub struct SkyboxVertex {
    // location 0: in_position: Vec4,
}

impl VulkanVertex for SkyboxVertex {
    fn binding_descriptions() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<[f32; 4]>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            // in_position: Vec4
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 0,
            },
        ]
    }
}
