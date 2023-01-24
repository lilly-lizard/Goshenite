#version 450

layout (location = 0) in vec4 in_color;
layout (location = 1) in vec4 in_normal;

layout (location = 0) out vec4 out_color;

const vec3 SUN_VECTOR = vec3(0.25, 0.559, 0.75); // length ~= 1

void main() {
	float light_factor = (dot(in_normal.xyz, SUN_VECTOR) + 1) / 2;
	out_color = in_color * light_factor;
}