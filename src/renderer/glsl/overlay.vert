#version 450

layout (location = 0) in vec4 in_position;
layout (location = 1) in vec4 in_normal;
layout (location = 2) in vec4 in_color;

layout (location = 0) out vec4 out_color;
layout (location = 1) out vec4 out_normal;

layout (push_constant) uniform OverlayParams {
	mat4 proj_view;
	vec4 offset;
} pc;

void main() {
	gl_Position = pc.proj_view * (in_position + pc.offset);
	gl_Position.y = -gl_Position.y; // fuck knows why. do this cpu side to save a couple of gpu instructions?

	out_color = in_color;
	
	out_normal = in_normal;
}