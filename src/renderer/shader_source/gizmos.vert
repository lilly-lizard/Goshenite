#version 450

// per vertex
layout (location = 0) in vec4 in_position;
// per instance
layout (location = 1) in mat4 in_orientation;

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

layout (set = 1, binding = 0) uniform GizmoParams {
	vec4 object_center;
} param;

void main()
{
	vec4 pos = in_orientation * in_position + param.object_center;
	gl_Position = inverse(cam.proj_inverse) * inverse(cam.view_inverse) * pos;
}
