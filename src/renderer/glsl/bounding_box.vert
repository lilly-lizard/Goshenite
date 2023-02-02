#version 450

layout (location = 0) in vec4 in_position;
layout (location = 1) in uint in_object_index;

layout (location = 0) out uint out_object_index;
layout (location = 1) out vec2 out_uv;

layout (push_constant) uniform Transform {
	mat4 proj_view;
} pc;

void main() 
{
	gl_Position = pc.proj_view * in_position;
	gl_Position.y = -gl_Position.y; // fuck knows why. fix this cpu side to save a couple of cycles?

	out_uv = gl_Position.xy; // clip space xy position (between -1 and 1)
	
	out_object_index = in_object_index;
}

/*
#version 450

layout (location = 0) out uint out_object_index;
layout (location = 1) out vec2 out_uv;

void main() 
{
	out_object_index = 0;

	vec2 uv = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
	gl_Position = vec4(uv * 2.0f + -1.0f, 0.0f, 1.0f);

	out_uv = gl_Position.xy; // clip space xy position (between -1 and 1)
}
*/