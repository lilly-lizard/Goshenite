#version 450

layout (location = 0) out uint out_object_index;

void main() 
{
	out_object_index = 0;
	vec2 uv = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
	gl_Position = vec4(uv * 2.0f + -1.0f, 0.0f, 1.0f);
}

/*
#version 450

layout (location = 0) in vec4 in_position;
layout (location = 1) in uint in_object_index;

layout (location = 0) out uint out_object_index;

layout (push_constant) uniform PushConstants {
	mat4 proj_view;
} pc;

void main() 
{
	gl_Position = pc.proj_view * in_position;
	gl_Position.y = -gl_Position.y; // fuck knows why. do this cpu side to save a couple of gpu instructions?

	out_object_index = in_object_index;
}
*/