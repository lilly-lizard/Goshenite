#version 450

layout (location = 0) out uint out_object_index;
layout (location = 1) out vec2 out_uv;

void main() 
{
	out_object_index = 0;
	out_uv = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
	gl_Position = vec4(out_uv * 2.0f + -1.0f, 0.0f, 1.0f);
}