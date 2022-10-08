#version 450
#extension GL_GOOGLE_include_directive : require
#include "primitives.glsl"

/// Defines the up/down axis in the world space coordinate system
const vec3 WORLD_SPACE_UP = vec3(0., 0., 1.);
/// Maximum number of ray marching steps before confirming a miss
const uint MAX_STEPS = 50;
/// Maximum distance to travel
const float MAX_DIST = 1000.;
/// Minimum distance to travel (epsilon)
const float MIN_DIST = 0.0001;

// input UV from full_screen.vert
layout (location = 0) in vec2 inUV;

// output g-buffer
layout (location = 0) out vec4 outColor;

// encoded object data to render
layout (set = 0, binding = 0, std430) readonly buffer PrimitiveData {
	uint count;
	uint data[];
} primitives;
// push constant with camera data
layout (push_constant) uniform Camera {
	mat4 proj_view_inverse;
	vec4 position;
} cam;

/// Signed distance function for a sphere
/// * `point` - point in space to calculate for
/// * `center` - sphere center
/// * `radius` - sphere radius
float sdfSphere(vec3 point, vec3 center, float radius)
{
	return length(point - center) - radius;
}

/// Signed distance function for a box
/// * `point` - point in space to calculate for
/// * `center` - center of the box
/// * `dimensions` - width, length and height of the box
float sdfBox(vec3 point, vec3 center, vec3 dimensions)
{
    vec3 d = abs(point - center) - dimensions / 2.;
    return min(max(d.x, max(d.y, d.z)), 0.) + length(max(d, 0.));
}

/// Returns a sky color for a ray miss
/// * `ray_d` - ray direction
vec3 background(const vec3 ray_d)
{
	return vec3(0.3, 0.4, 0.5) + 0.3 * ray_d * WORLD_SPACE_UP;
}

/// Calculates the distance to the closest signed distance field in the scene
float map(vec3 pos)
{
	float dist = MAX_DIST;

	uint primitive_index = 0;
	while (primitive_index < primitives.count) {
		uint buffer_pos = primitive_index * PRIMITIVE_UNIT_LENGTH;
		uint primitive_type = primitives.data[buffer_pos++];

		if (primitive_type == SPHERE) {
			vec3 center = vec3(
				uintBitsToFloat(primitives.data[buffer_pos++]),
				uintBitsToFloat(primitives.data[buffer_pos++]),
				uintBitsToFloat(primitives.data[buffer_pos++])
			);
			float radius = uintBitsToFloat(primitives.data[buffer_pos++]);
			dist = min(dist, sdfSphere(pos, center, radius));
		} else if (primitive_type == CUBE) {
			vec3 center = vec3(
				uintBitsToFloat(primitives.data[buffer_pos++]),
				uintBitsToFloat(primitives.data[buffer_pos++]),
				uintBitsToFloat(primitives.data[buffer_pos++])
			);
			vec3 dimensions = vec3(
				uintBitsToFloat(primitives.data[buffer_pos++]),
				uintBitsToFloat(primitives.data[buffer_pos++]),
				uintBitsToFloat(primitives.data[buffer_pos++])
			);
			dist = min(dist, sdfBox(pos, center, dimensions));
		}
		
		primitive_index++;
	}

	return dist;
}

// https://iquilezles.org/articles/normalsSDF
// todo pass in hit primitive instead of checking against whole scene...
vec3 calcNormal(vec3 pos)
{
	const float EPSILON = 0.001; // defines a threshold for "essentially nothing"
    const vec2 e = vec2(1., -1.) * EPSILON;
    return normalize(e.xyy * map(pos + e.xyy) + 
					 e.yyx * map(pos + e.yyx) + 
					 e.yxy * map(pos + e.yxy) + 
					 e.xxx * map(pos + e.xxx));
}

/// Render the scene and return the color
/// * `ray_d` - ray direction
//https://michaelwalczyk.com/blog-ray-marching.html
vec3 ray_march(const vec3 ray_o, const vec3 ray_d)
{
	float dist = 0.; // total distance traveled
	for (int i = 0; i < MAX_STEPS && dist < MAX_DIST; i++) {
		vec3 current_pos = ray_o + dist * ray_d; // get the world space position from the current marching distance
		float dist_to_closest = map(current_pos); // get the distance to the closest primitive
		if (dist_to_closest < MIN_DIST) {
			// ray hit
			return calcNormal(current_pos) / 2. + .5; // colot output = normal for now
		}
		dist += dist_to_closest; // incriment the distance travelled along the ray path
	}

	// ray miss
	return background(ray_d);
}

void main()
{
	vec2 pos_uv = inUV * 2. - 1.; // clip space position in frame (between -1 and 1)
	vec3 ray_d = normalize((cam.proj_view_inverse * vec4(pos_uv.x, -pos_uv.y, 1., 1.)).xyz); // ray direction in world space

	// render scene
	vec3 color = ray_march(cam.position.xyz, ray_d);

	outColor = vec4(color, 1.);
}