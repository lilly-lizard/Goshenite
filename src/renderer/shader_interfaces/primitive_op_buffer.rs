use bytemuck::NoUninit;

use crate::engine::{object::primitive_op::PrimitiveOp, primitives::primitive::EncodablePrimitive};

// this is because the shaders store the primitive op index in the lower 16 bits of a u32
pub const MAX_PRIMITIVE_OP_COUNT: usize = u16::MAX as usize;

#[rustfmt::skip]
#[allow(dead_code)]
pub mod op_codes {
    pub const NOP: 		    u32 = 0x00000000;
    pub const UNION: 		u32 = 0x00000001; // OR
    pub const INTERSECTION: u32 = 0x00000002; // AND
    pub const SUBTRACTION: 	u32 = 0x00000003;
    pub const INVALID:      u32 = 0xFFFFFFFF;
}

#[repr(C)]
#[derive(Default, Clone, Copy, NoUninit)]
pub struct PrimitiveOpPacket {
    op: u32,
    blend: f32,
    primitive_center: [f32; 3],
    primitive_rotation: [f32; 9],
    s: [f32; 4],
    r: [f32; 2],
    albedo: [f32; 3],
    specular: f32,
}

pub fn create_primitive_op_packet(primitive_op: &PrimitiveOp) -> PrimitiveOpPacket {
    PrimitiveOpPacket {
        op: primitive_op.op.op_code(),
        blend: primitive_op.blend,
        primitive_center: primitive_op.transform.translation.to_array(),
        primitive_rotation: primitive_op.transform.rotation_matrix().to_cols_array(),
        s: primitive_op.primitive.uber_s(),
        r: primitive_op.primitive.uber_r(),
        albedo: primitive_op.albedo.to_array(),
        specular: primitive_op.specular,
    }
}

pub fn nop_primitive_op_packet() -> PrimitiveOpPacket {
    PrimitiveOpPacket::default()
}
