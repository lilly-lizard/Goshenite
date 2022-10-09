#version 450
#extension GL_GOOGLE_include_directive : require
#include "primitives.glsl"

// g-buffer input attachments
layout (input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput in_normal;
layout (input_attachment_index = 1, set = 0, binding = 1) uniform usubpassInput in_prmitive_id;
// input UV from full_screen.vert
layout (location = 0) in vec2 in_uv;

// output color to swapchain image
layout (location = 0) out vec4 out_color;

// push constant with camera data
layout (push_constant) uniform Camera {
	mat4 proj_view_inverse;
	vec4 position;
} cam;

/// Returns a sky color for a ray miss
/// * `ray_d` - ray direction
vec3 background(const vec3 ray_d)
{
	return vec3(0.3, 0.4, 0.5) + 0.3 * ray_d * WORLD_SPACE_UP;
}

void main() 
{
	// decode g-buffer
	vec3 normal = subpassLoad(in_normal).xyz;
	uint primitive_id = subpassLoad(in_prmitive_id).x;
	
	if (primitive_id == ID_INVALID) {
		// ray miss: draw background

		// clip space position in frame (between -1 and 1)
		vec2 pos_uv = in_uv * 2. - 1.;
		// ray direction in world space
		vec3 ray_d = normalize((cam.proj_view_inverse * vec4(pos_uv.x, -pos_uv.y, 1., 1.)).xyz);
		out_color = vec4(background(ray_d), 1.);
	} else {
		// ray hit: just output normal as color for now
		out_color = vec4(normal, 1.);
	}
}