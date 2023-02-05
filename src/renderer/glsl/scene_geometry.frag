#version 450
#extension GL_EXT_nonuniform_qualifier : require
#extension GL_GOOGLE_include_directive : require
#include "common.glsl"

// Maximum number of ray marching steps before confirming a miss
const uint MAX_STEPS = 50;
// Maximum distance to travel
const float MAX_DIST = 1000.;
// Minimum distance to travel (epsilon)
const float MIN_DIST = 0.0001;

// ~~~ IO ~~~

layout (location = 0) in flat uint in_object_index;
layout (location = 1) in flat uint in_object_id;

layout (location = 0) out vec4 out_normal;
layout (location = 1) out uint out_object_id; // upper 16 bits = object index; lower 16 bits = op index; todo checks for 16bit max on rust side??

layout (set = 0, binding = 0) uniform Camera {
	mat4 proj_view_inverse;
	vec4 position;
	vec2 framebuffer_dims;
	float near;
	float far;
} cam;

layout (set = 1, binding = 0, std430) readonly buffer Object {
	uint _id;
	uint op_count;
	uint primitive_ops[];
} objects[];

// ~~~ Signed Distance Fields ~~~

// Represents a signed distance field result
struct SdfResult {
	// Distance from pos to primitive
	float d;
	// operation index
	uint op_index;
};

// Signed distance function for a sphere
float sdf_sphere(vec3 pos, vec3 center, float radius)
{
	return length(pos - center) - radius;
}

// Signed distance function for a box
float sdf_box(vec3 pos, vec3 center, vec3 dimensions)
{
	vec3 d = abs(pos - center) - dimensions / 2.;
	return min(max(d.x, max(d.y, d.z)), 0.) + length(max(d, 0.));
}

SdfResult process_primitive(uint buffer_index, uint op_index, vec3 pos)
{
	uint primitive_type = objects[in_object_index].primitive_ops[buffer_index++];
	SdfResult res = { MAX_DIST, op_index };

	if (primitive_type == PRIMITIVE_SPHERE)
	{
		vec3 center = vec3(
			uintBitsToFloat(objects[in_object_index].primitive_ops[buffer_index++]),
			uintBitsToFloat(objects[in_object_index].primitive_ops[buffer_index++]),
			uintBitsToFloat(objects[in_object_index].primitive_ops[buffer_index++])
		);
		float radius = uintBitsToFloat(objects[in_object_index].primitive_ops[buffer_index++]);
		res.d = sdf_sphere(pos, center, radius);
	}
	else if (primitive_type == PRIMITIVE_CUBE)
	{
		vec3 center = vec3(
			uintBitsToFloat(objects[in_object_index].primitive_ops[buffer_index++]),
			uintBitsToFloat(objects[in_object_index].primitive_ops[buffer_index++]),
			uintBitsToFloat(objects[in_object_index].primitive_ops[buffer_index++])
		);
		vec3 dimensions = vec3(
			uintBitsToFloat(objects[in_object_index].primitive_ops[buffer_index++]),
			uintBitsToFloat(objects[in_object_index].primitive_ops[buffer_index++]),
			uintBitsToFloat(objects[in_object_index].primitive_ops[buffer_index++])
		);
		res.d = sdf_box(pos, center, dimensions);
	}
	// else do nothing e.g. PRIMITIVE_NULL

	return res;
}

// ~~~ Combination Ops ~~~

// Perform the union combination on 2 sdf calculations for a given position.
// Returns the result with the shortest distance.
SdfResult op_union(SdfResult p1, SdfResult p2)
{
	return p1.d < p2.d ? p1 : p2; // OR
}

// Results in the intersection of 2 primitives
SdfResult op_intersection(SdfResult p1, SdfResult p2)
{
	return p1.d > p2.d ? p1 : p2; // AND
}

// Subtracts the volume of primitive 2 from primitive 1
SdfResult op_subtraction(SdfResult p1, SdfResult p2)
{
	return -p1.d > p2.d ? p1 : p2;
}

SdfResult process_op(uint op, SdfResult p1_res, SdfResult p2_res)
{
	SdfResult res;
	
	switch(op)
	{
	case OP_UNION: 			res = op_union(p1_res, p2_res); break;
	case OP_INTERSECTION: 	res = op_intersection(p1_res, p2_res); break;
	case OP_SUBTRACTION: 	res = op_subtraction(p1_res, p2_res); break;
	default:				res = p1_res; // else do nothing e.g. OP_NULL
	}

	return res;
}

// ~~~ Scene Traversal ~~~

// Calculates the distance to the closest primitive in the scene from `pos`
SdfResult map(vec3 pos)
{
	// the closest primitve and distance to pos p
	SdfResult closest_res = { MAX_DIST, ID_INVALID };

	// loop through the object operations
	uint op_index = 0;
	while (op_index < objects[in_object_index].op_count) {
		uint buffer_index = op_index * OP_UNIT_LENGTH;
		uint op = objects[in_object_index].primitive_ops[buffer_index++];

		SdfResult primitive_res = process_primitive(buffer_index, op_index, pos);
		closest_res = process_op(op, closest_res, primitive_res);

		op_index++;
	}

	return closest_res;
}

// https://iquilezles.org/articles/normalsSDF
// todo pass in hit primitive instead of checking against whole object?
vec3 calcNormal(vec3 pos)
{
	const float EPSILON = 0.001; // defines a threshold for "essentially nothing"
	const vec2 e = vec2(1., -1.) * EPSILON;
	return normalize(e.xyy * map(pos + e.xyy).d +
					 e.yyx * map(pos + e.yyx).d +
					 e.yxy * map(pos + e.yxy).d +
					 e.xxx * map(pos + e.xxx).d);
}

// Render the scene and return the color. Returns the depth of a hit primitive.
// When the ray misses, calls discard.
//https://michaelwalczyk.com/blog-ray-marching.html
float ray_march(const vec3 ray_o, const vec3 ray_d, out vec3 normal, out uint object_id)
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
			object_id = closest_primitive.op_index | (in_object_id << 16);
			return dist;
		}

		// incriment the distance travelled by the distance to the closest primitive
		dist += closest_primitive.d;
	}

	// ray miss
	discard;
}

float depth(float near, float far, float z) {
	return (z - near) / (far - near);
}

// ~~~ Main ~~~

void main()
{
	// ray direction in world space
	vec2 screen_space = gl_FragCoord.xy + vec2(0.5);
	vec2 clip_space = screen_space / cam.framebuffer_dims * 2. - 1.;
	vec4 ray_d = cam.proj_view_inverse * vec4(clip_space.x, -clip_space.y, 1., 1.);
	vec3 ray_d_norm = normalize(ray_d.xyz);

	// render scene
	vec3 normal;
	uint object_id;
	float z = ray_march(cam.position.xyz, ray_d_norm, normal, object_id);

	gl_FragDepth = depth(cam.near, cam.far, z);
	out_normal = vec4(normal, 0.);
	out_object_id = object_id;
}