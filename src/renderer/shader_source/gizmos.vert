#version 450

// per vertex
layout (location = 0) in vec4 in_position;
// per instance
layout (location = 1) in mat4 in_orientation; // consumes locations 1-4 inclusive

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
	float gizmo_scale;
} param;

void main()
{
	vec4 view_offset = inverse(cam.view_inverse) * param.object_center;
	vec4 proj_offset = inverse(cam.proj_inverse) * view_offset;
	float scale = param.gizmo_scale / (proj_offset.z / proj_offset.w);
	//float scale = 0.1;

	vec4 pos = in_orientation * in_position;

	// scale the vertex positions to keep the gizmo at a constant size relative to the screen
	pos.x *= scale;
	pos.y *= scale;
	pos.z *= scale;

	pos.x += param.object_center.x;
	pos.y += param.object_center.y;
	pos.z += param.object_center.z;

	vec4 view = inverse(cam.view_inverse) * pos;
	vec4 proj = inverse(cam.proj_inverse) * view;

	gl_Position = proj;
}
