#version 450
#extension GL_GOOGLE_include_directive : require
#include "common.glsl"

// Maximum number of ray marching steps before confirming a miss
const uint MAX_STEPS = 50;
// Maximum distance to travel
const float MAX_DIST = 1000.;
// Minimum distance to travel (epsilon)
const float MIN_DIST = 0.0001;

// input UV from full_screen.vert
layout (location = 0) in vec2 in_uv;

layout (location = 0) out vec4 out_normal;
// upper 16 bits = object index; lower 16 bits = op index; todo checks for 16bit max on rust side
layout (location = 1) out uint out_object_id;

// todo for now just single object
const uint OBJECT_INDEX = 0;
layout (set = 0, binding = 1, std430) readonly buffer Object {
	uint op_count;
	uint data[];
} object;
// push constant with camera data
layout (push_constant) uniform Camera {
	mat4 proj_view_inverse;
	vec4 position;
} cam;

// ~~~ Signed Distance Fields ~~~

// Represents a signed distance field result
struct SdfResult {
	// Distance from pos to primitive
	float d;
	// operation index
	uint op_index;
};

// Signed distance function for a sphere
// * `pos` - pos in space to calculate for
// * `center` - sphere center
// * `radius` - sphere radius
float sdf_sphere(vec3 pos, vec3 center, float radius)
{
	return length(pos - center) - radius;
}

// Signed distance function for a box
// * `pos` - pos in space to calculate for
// * `center` - center of the box
// * `dimensions` - width, length and height of the box
float sdf_box(vec3 pos, vec3 center, vec3 dimensions)
{
	vec3 d = abs(pos - center) - dimensions / 2.;
	return min(max(d.x, max(d.y, d.z)), 0.) + length(max(d, 0.));
}

SdfResult process_primitive(uint buffer_index, uint op_index, vec3 pos)
{
	uint primitive_type = object.data[buffer_index++];
	SdfResult res = { MAX_DIST, op_index };

	if (primitive_type == PRIMITIVE_SPHERE)
	{
		vec3 center = vec3(
			uintBitsToFloat(object.data[buffer_index++]),
			uintBitsToFloat(object.data[buffer_index++]),
			uintBitsToFloat(object.data[buffer_index++])
		);
		float radius = uintBitsToFloat(object.data[buffer_index++]);
		res.d = sdf_sphere(pos, center, radius);
	}
	else if (primitive_type == PRIMITIVE_CUBE)
	{
		vec3 center = vec3(
			uintBitsToFloat(object.data[buffer_index++]),
			uintBitsToFloat(object.data[buffer_index++]),
			uintBitsToFloat(object.data[buffer_index++])
		);
		vec3 dimensions = vec3(
			uintBitsToFloat(object.data[buffer_index++]),
			uintBitsToFloat(object.data[buffer_index++]),
			uintBitsToFloat(object.data[buffer_index++])
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
	SdfResult res = { MAX_DIST, ID_INVALID };

	switch(op)
	{
	case OP_UNION: 			res = op_union(p1_res, p2_res); break;
	case OP_INTERSECTION: 	res = op_intersection(p1_res, p2_res); break;
	case OP_SUBTRACTION: 	res = op_subtraction(p1_res, p2_res); break;
	// else do nothing e.g. OP_NULL
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
	while (op_index < object.op_count) {
		uint buffer_index = op_index * OP_UNIT_LENGTH;
		uint op = object.data[buffer_index++];

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

// Render the scene and return the color
// * `ray_d` - ray direction
//https://michaelwalczyk.com/blog-ray-marching.html
void ray_march(const vec3 ray_o, const vec3 ray_d, out vec3 normal, out uint object_id)
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
			object_id = closest_primitive.op_index | (OBJECT_INDEX << 16);
			return;
		}

		// incriment the distance travelled by the distance to the closest primitive
		dist += closest_primitive.d;
	}

	// ray miss
	object_id = ID_INVALID;
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
	uint object_id;
	ray_march(cam.position.xyz, ray_d, normal, object_id);

	out_normal = vec4(normal, 0.);
	out_object_id = object_id;
}