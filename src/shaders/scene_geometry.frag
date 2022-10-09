#version 450
#extension GL_GOOGLE_include_directive : require
#include "primitives.glsl"

/// Maximum number of ray marching steps before confirming a miss
const uint MAX_STEPS = 50;
/// Maximum distance to travel
const float MAX_DIST = 1000.;
/// Minimum distance to travel (epsilon)
const float MIN_DIST = 0.0001;

// input UV from full_screen.vert
layout (location = 0) in vec2 in_uv;

// output g-buffer
layout (location = 0) out vec4 out_gbuffer;

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

// ~~~ Primitive Data Decoding ~~~

// todo

// ~~~ Signed Distance Fields ~~~

/// Represents a signed distance field result
struct SdfResult {
	/// Distance from point to primitive
	float d;
	/// Primitive id
	uint primitive_id;
};

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

// ~~~ Combination Ops ~~~

/// Perform the union combination on 2 sdf calculations for a given point.
/// Returns the result with the shortest distance.
SdfResult op_union(SdfResult p1, SdfResult p2)
{
	return p1.d < p2.d ? p1 : p2;
}

// ~~~ Scene Traversal ~~~

/// Calculates the distance to the closest primitive in the scene from `point`
SdfResult map(vec3 point)
{
	// the closest primitve and distance to point p
	SdfResult closest_res = { MAX_DIST, ID_INVALID };

	// loop through the primitives
	uint primitive_index = 0;
	while (primitive_index < primitives.count) {
		// we'll store the distance from `point` to this primitive here
		SdfResult test_res = { MAX_DIST, primitive_index };

		// decode primitive data and perform sdf calculation depending on the type of primitive
		uint buffer_pos = primitive_index * PRIMITIVE_UNIT_LENGTH;
		uint primitive_type = primitives.data[buffer_pos++];
		if (primitive_type == PRIMITIVE_SPHERE) {
			vec3 center = vec3(
				uintBitsToFloat(primitives.data[buffer_pos++]),
				uintBitsToFloat(primitives.data[buffer_pos++]),
				uintBitsToFloat(primitives.data[buffer_pos++])
			);
			float radius = uintBitsToFloat(primitives.data[buffer_pos++]);
			test_res.d = sdfSphere(point, center, radius);
		} else if (primitive_type == PRIMITIVE_CUBE) {
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
			test_res.d = sdfBox(point, center, dimensions);
		}

		// update `closest_res` if `test_res` is closer to `point`
		closest_res = op_union(closest_res, test_res);
		
		primitive_index++;
	}

	return closest_res;
}

// https://iquilezles.org/articles/normalsSDF
// todo pass in hit primitive instead of checking against whole scene...
vec3 calcNormal(vec3 pos)
{
	const float EPSILON = 0.001; // defines a threshold for "essentially nothing"
    const vec2 e = vec2(1., -1.) * EPSILON;
    return normalize(e.xyy * map(pos + e.xyy).d + 
					 e.yyx * map(pos + e.yyx).d + 
					 e.yxy * map(pos + e.yxy).d + 
					 e.xxx * map(pos + e.xxx).d);
}

/// Render the scene and return the color
/// * `ray_d` - ray direction
//https://michaelwalczyk.com/blog-ray-marching.html
void ray_march(const vec3 ray_o, const vec3 ray_d, out vec3 normal, out uint primitive_id)
{
	// total distance traveled
	float dist = 0.;
	for (int i = 0; i < MAX_STEPS && dist < MAX_DIST; i++) {
		// get the world space position from the current marching distance
		vec3 current_pos = ray_o + ray_d * dist;
		// get the distance to the closest primitive
		SdfResult closest_primitive = map(current_pos);

		// ray hit
		if (closest_primitive.d < MIN_DIST) {
			normal = calcNormal(current_pos) / 2. + .5;
			primitive_id = closest_primitive.primitive_id;
			return;
		}

		// incriment the distance travelled by the distance to the closest primitive
		dist += closest_primitive.d;
	}

	// ray miss
	primitive_id = ID_INVALID;
	normal = vec3(0.);
}

// ~~~ Main ~~~

void main()
{
	// clip space position in frame (between -1 and 1)
	vec2 pos_uv = in_uv * 2. - 1.;
	// ray direction in world space
	vec3 ray_d = normalize((cam.proj_view_inverse * vec4(pos_uv.x, -pos_uv.y, 1., 1.)).xyz);

	// render scene
	vec3 normal;
	uint primitive_id;
	ray_march(cam.position.xyz, ray_d, normal, primitive_id);

	GBufferValue store = {
		normal,
		primitive_id,
	};
	out_gbuffer = gbuffer_encode(store);
}