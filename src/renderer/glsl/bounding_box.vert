#version 450

layout (location = 0) in vec4 in_position;
layout (location = 1) in uint in_object_id;

layout (location = 0) out uint out_object_index;
layout (location = 1) out uint out_object_id;

layout (push_constant) uniform PushConstant {
	uint object_index;
} pc;

layout (set = 0, binding = 0) uniform Camera {
	mat4 proj_view_inverse;
	vec4 _position;
	vec2 _framebuffer_dims;
	float _near;
	float _far;
} cam;

void main()
{
	gl_Position = inverse(cam.proj_view_inverse) * in_position;
	gl_Position.y = -gl_Position.y; // fuck knows why. fix this cpu side to save a couple of cycles?

	out_object_index = pc.object_index;
	out_object_id = in_object_id;
	//out_uv = gl_Position.xy / gl_Position.w; // clip space xy position (between -1 and 1)
	// todo this could remove a few cycles from the frag shader but it results in warping along the aabb edges! i assume the interpolation is to blame somehow but idk tbh
}
