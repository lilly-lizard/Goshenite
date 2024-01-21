#version 450

layout (location = 0) in vec4 in_position;
layout (location = 1) in uint in_object_id;

layout (set = 0, binding = 0) uniform Camera {
	mat4 view_inverse;
	mat4 proj_inverse;
	vec4 _position;
	vec2 _framebuffer_dims;
	float _near;
	float _far;
    uint _write_linear_color;
} cam;

void main()
{
	gl_Position = inverse(cam.proj_inverse) * inverse(cam.view_inverse) * in_position;
}