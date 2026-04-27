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
	float view_depth;
} param;

void main()
{
	vec4 pos = in_orientation * in_position;
	vec4 view_pos = inverse(cam.view_inverse) * pos;
	vec4 view_offset = inverse(cam.view_inverse) * param.object_center;
	vec4 view_total = view_pos + view_offset;

	// keep gizmo at a constant size/depth
	view_total.z = -param.view_depth;

	vec4 proj_pos = inverse(cam.proj_inverse) * view_total;
	gl_Position = proj_pos;
}
