#version 450
#extension GL_GOOGLE_include_directive : require
#include "config.glsl"
#include "color_space_conv.glsl"

// g-buffer input attachments
layout (set = 0, binding = 0, input_attachment_index = 0) uniform subpassInput in_normal;
layout (set = 0, binding = 1, input_attachment_index = 1) uniform subpassInput in_albedo;
layout (set = 0, binding = 2, input_attachment_index = 2) uniform usubpassInput in_prmitive_id;

// input UV from full_screen.vert
layout (location = 0) in vec2 in_uv; // clip space position [-1, 1]

// output color to swapchain image
layout (location = 0) out vec4 out_color;

layout (set = 1, binding = 0) uniform Camera {
	mat4 proj_view_inverse;
	vec4 _position;
	vec2 framebuffer_dims;
	float near;
	float far;
    uint write_linear_color;
} cam;

/// Returns a sky color for a ray miss
/// * `ray_d` - ray direction
vec3 background(const vec3 ray_d)
{
	return vec3(0.45, 0.55, 0.7) + 0.3 * dot(ray_d, WORLD_SPACE_UP);
}

/// Normalized ray direction in world space
vec3 ray_direction() {
	float clip_space_depth = -cam.near / cam.far; // results in z = 1 after multiplying my the proj matrix
	vec4 ray_d = cam.proj_view_inverse * vec4(in_uv, clip_space_depth, 1.);
	return normalize(ray_d.xyz);
}

void main() 
{
	// decode g-buffer
	uint primitive_id = subpassLoad(in_prmitive_id).x;

	vec3 ray_d = ray_direction();
	
	if (primitive_id == ID_BACKGROUND) {
		// ray miss: draw background
		out_color = vec4(background(ray_d), 1.);
	} else {
		// ray hit: calculate color (https://learnopengl.com/Lighting/Basic-Lighting)

		const vec3 SUN_DIR = vec3(-0.57735, -0.57735, -0.57735); // normalized
		const vec3 SUN_COLOR = vec3(1., 1., 0.8);
		const float AMBIENT_STRENGTH = 0.18;

		vec3 normal = (subpassLoad(in_normal).xyz - 0.5) * 2.;
		vec4 albedo = subpassLoad(in_albedo);
		float specular_strength = 0.5; // hard-coded for now

		vec3 ambient = AMBIENT_STRENGTH * SUN_COLOR;
		
		float diffuse_factor = max(dot(normal, -SUN_DIR), 0.);
		vec3 diffuse = diffuse_factor * SUN_COLOR;

		vec3 reflect_d = reflect(-SUN_DIR, normal);
		float specular_factor = pow(max(dot(ray_d, reflect_d), 0.), 32);
		vec3 specular = specular_strength * specular_factor * SUN_COLOR;

		vec4 ambient_diffuse_specular = vec4(ambient + diffuse + specular, 1.);
		out_color = albedo * ambient_diffuse_specular;
	}

    if (cam.write_linear_color == 1) {
        // surface format/color space combination requires us to write out linear color
        // see https://stackoverflow.com/questions/66401081/vulkan-swapchain-format-unorm-vs-srgb for more info
        out_color.xyz = srgb_to_linear(out_color.xyz);
    }
}