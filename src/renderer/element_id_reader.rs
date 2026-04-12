use super::shader_interfaces::primitive_op_buffer::{PRIMITIVE_ID_BACKGROUND, PRIMITIVE_ID_BLEND};
use crate::engine::object::{object::ObjectId, primitive_op::PrimitiveOpIndex};

#[derive(Debug, Clone, Copy)]
pub enum ElementAtPoint {
    Object {
        object_id: ObjectId,
        primitive_op_index: PrimitiveOpIndex,
    },
    Background,
    BlendArea {
        object_id: ObjectId,
    },
    // X, Y, Z manipulation ui elements
}

impl ElementAtPoint {
    pub fn from_rendered_id(rendered_id: u32) -> Self {
        match rendered_id {
            PRIMITIVE_ID_BACKGROUND => Self::Background,
            encoded_id => {
                let object_id_u32 = encoded_id >> 16;
                let object_id = ObjectId::from(object_id_u32 as u16);
                let primitive_op_index = (encoded_id & 0x0000FFFF) as usize;

                if primitive_op_index == PRIMITIVE_ID_BLEND as usize {
                    Self::BlendArea { object_id }
                } else {
                    Self::Object {
                        object_id,
                        primitive_op_index,
                    }
                }
            }
        }
    }
}
