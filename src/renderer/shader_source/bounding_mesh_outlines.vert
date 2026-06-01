#version 450

layout (location = 0) in vec4 in_position;
layout (location = 1) in uint _in_object_id;
// per instance
layout (location = 2) in vec4 in_translation;
layout (location = 3) in mat4 in_rotation; // consumes 4 locations

layout (set = 0, binding = 0) uniform Camera {
	mat4 view_inverse;
	mat4 proj_inverse;
	vec2 _framebuffer_dims;
	float _near;
	float _far;
	vec3 _position;
	vec3 _direction;
    uint _write_linear_color;
} cam;

void main()
{
	gl_Position = inverse(cam.proj_inverse) * inverse(cam.view_inverse) * (in_position + in_translation);
}
