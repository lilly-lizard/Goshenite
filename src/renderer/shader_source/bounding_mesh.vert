#version 450

// per vertex
layout (location = 0) in vec4 in_position;
layout (location = 1) in uint in_object_id;
// per instance
layout (location = 2) in vec4 in_translation;
layout (location = 3) in mat4 in_rotation; // consumes 4 locations

layout (location = 0) out uint out_object_id;
layout (location = 1) out vec2 out_clip_space_uv; // clip space xy position (between -1 and 1)
layout (location = 2) out float out_camera_distance; // distance from camera to fragment
layout (location = 3) out vec4 out_translation;
layout (location = 4) out mat3 out_rotation; // consumes 3 locations

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
	vec4 view_space = inverse(cam.view_inverse) * (in_rotation * in_position + in_translation);
	out_camera_distance = length(view_space.xyz);
	gl_Position = inverse(cam.proj_inverse) * view_space;

	out_object_id = in_object_id;
	out_clip_space_uv = gl_Position.xy / gl_Position.w;
	out_translation = in_translation;
	out_rotation = mat3(in_rotation);
}
