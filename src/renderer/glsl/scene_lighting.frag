#version 450
#extension GL_GOOGLE_include_directive : require
#include "common.glsl"

// g-buffer input attachments
layout (set = 0, binding = 0, input_attachment_index = 0) uniform subpassInput in_normal;
layout (set = 0, binding = 1, input_attachment_index = 1) uniform usubpassInput in_prmitive_id;
// input UV from full_screen.vert
layout (location = 0) in vec2 in_uv;

// output color to swapchain image
layout (location = 0) out vec4 out_color;

layout (set = 1, binding = 0) uniform Camera {
	mat4 proj_view_inverse;
	vec4 _position;
	vec2 _framebuffer_dims;
	float _near;
	float _far;
    uint is_srgb_framebuffer;
} cam;

/// Returns a sky color for a ray miss
/// * `ray_d` - ray direction
vec3 background(const vec3 ray_d)
{
	return vec3(0.45, 0.55, 0.7) + 0.3 * dot(ray_d, WORLD_SPACE_UP);
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

    if (cam.is_srgb_framebuffer == 1) {
        // need to convert linear colors to srgb
        out_color.xyz = pow(out_color.xyz, vec3(2.2));
    }
}