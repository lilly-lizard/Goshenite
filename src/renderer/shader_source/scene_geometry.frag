#version 450
#extension GL_EXT_nonuniform_qualifier : require
#extension GL_GOOGLE_include_directive : require
#include "config.glsl"

// Maximum number of ray marching steps before confirming a miss
const uint MAX_STEPS = 100;
// Distance required to confirm a hit todo make this dynamic with depth (like LOD)
const float MIN_MARCH_STEP = 0.001;
// Offset used for calculating normals.
const float NORMAL_EPSILON = 0.001;
const vec2 NORMAL_OFFSET = vec2(NORMAL_EPSILON, -NORMAL_EPSILON);

// ~~~ IO ~~~

layout (location = 0) in flat uint in_object_id;
layout (location = 1) in noperspective vec2 in_clip_space_uv; // clip space position [-1, 1]

layout (location = 0) out vec4 out_normal;
layout (location = 1) out vec4 out_albedo;
layout (location = 2) out uint out_object_id; // upper 16 bits = object index; lower 16 bits = op index; todo checks for 16bit max on rust side
layout (depth_greater) out float gl_FragDepth; // although drivers probably can't optimize with this anyway because we use discard... https://github.com/KhronosGroup/Vulkan-Guide/blob/main/chapters/depth.adoc

layout (set = 0, binding = 0) uniform Camera {
	mat4 proj_view_inverse;
	vec4 position;
	vec2 framebuffer_dims;
	float near;
	float far;
    uint _write_linear_color;
} cam;

layout (set = 1, binding = 0, std430) readonly buffer Object {
	uint _id;
	uint op_count;
	uint primitive_ops[];
} object;

// ~~~ Signed Distance Fields ~~~
// https://www.shadertoy.com/view/MsVGWG

float sdf_uber_primitive(vec3 pos, vec4 s, vec2 r)
{
	vec3 d = abs(pos) - s.xyz;
	float q_1 = length(max(d.xy + r.x, 0.));
	float q_2 = min(-r.x, max(d.x, d.y) + s.w);
	float q = abs(q_1 + q_2) - s.w;
	vec2 ret_1 = max(vec2(q, d.z) + r.y, 0.);
	float ret_2 = min(-r.y, max(q, d.z));
	return length(ret_1) + ret_2;
}

// ~~~ Combination Ops ~~~

// Represents a signed distance field result
struct SdfResult {
	// Distance from pos to primitive
	float d;
	// operation index
	uint op_index;
};

// Results in the union (min) of 2 primitives
SdfResult op_union(SdfResult p1, SdfResult p2, float blend)
{
	float d_delta = p2.d - p1.d;
	if (abs(d_delta) >= blend) {
		return p1.d < p2.d ? p1 : p2;
	}
	float h = clamp(0.5 + 0.5 * d_delta / blend, 0., 1.);
	float d_ret = mix(p2.d, p1.d, h) - blend * h * (1. - h);
	SdfResult ret = { d_ret, ID_BLEND };
	return ret;
}

// Results in the intersection (max) of 2 primitives
SdfResult op_intersection(SdfResult p1, SdfResult p2, float blend)
{
	return p1.d > p2.d ? p1 : p2;
}

// Subtracts the volume of primitive 2 (max) from primitive 1 (max inverted)
SdfResult op_subtraction(SdfResult p1, SdfResult p2, float blend)
{
	SdfResult p2_neg = { -p2.d, p2.op_index };
	return op_intersection(p1, p2_neg, blend);
}

// ~~~ Primitive-Op Processing ~~~

SdfResult process_primitive(uint op_index, vec3 pos)
{
	// todo perf comparison: load OP_UNIT_LENGTH values at once then decode below
	uint buffer_index = op_index * OP_UNIT_LENGTH;

	vec3 center = vec3(
		uintBitsToFloat(object.primitive_ops[buffer_index++]),
		uintBitsToFloat(object.primitive_ops[buffer_index++]),
		uintBitsToFloat(object.primitive_ops[buffer_index++])
	);

	mat3 rotation;
	rotation[0] = vec3(
		uintBitsToFloat(object.primitive_ops[buffer_index++]),
		uintBitsToFloat(object.primitive_ops[buffer_index++]),
		uintBitsToFloat(object.primitive_ops[buffer_index++])
	); // column 1
	rotation[1] = vec3(
		uintBitsToFloat(object.primitive_ops[buffer_index++]),
		uintBitsToFloat(object.primitive_ops[buffer_index++]),
		uintBitsToFloat(object.primitive_ops[buffer_index++])
	); // column 2
	rotation[2] = vec3(
		uintBitsToFloat(object.primitive_ops[buffer_index++]),
		uintBitsToFloat(object.primitive_ops[buffer_index++]),
		uintBitsToFloat(object.primitive_ops[buffer_index++])
	); // column 3

	vec4 s = vec4(
		uintBitsToFloat(object.primitive_ops[buffer_index++]),
		uintBitsToFloat(object.primitive_ops[buffer_index++]),
		uintBitsToFloat(object.primitive_ops[buffer_index++]),
		uintBitsToFloat(object.primitive_ops[buffer_index++])
	);

	vec2 r = vec2(
		uintBitsToFloat(object.primitive_ops[buffer_index++]),
		uintBitsToFloat(object.primitive_ops[buffer_index++])
	);

	pos = pos - center;
	pos = pos * rotation;

	float dist = sdf_uber_primitive(pos, s, r);

	SdfResult ret = { dist, op_index };
	return ret;
}

SdfResult process_op(uint op, float blend, SdfResult lhs, SdfResult rhs)
{
	SdfResult res;
	
	switch(op)
	{
	case OP_UNION: 			res = op_union(lhs, rhs, blend); break;
	case OP_INTERSECTION: 	res = op_intersection(lhs, rhs, blend); break;
	case OP_SUBTRACTION: 	res = op_subtraction(lhs, rhs, blend); break;
	default:				res = lhs; // else do nothing e.g. OP_NULL
	}

	return res;
}

// ~~~ Scene Traversal ~~~

// Calculates the distance to the closest primitive in the scene from `pos`
SdfResult map(vec3 pos)
{
	// the closest primitve and distance to pos p
	SdfResult closest_res = { cam.far, ID_BACKGROUND };

	// loop through the primitive operations
	for (uint op_index = 0; op_index < object.op_count; op_index++) {
		
		SdfResult primitive_res = process_primitive(op_index, pos);

		uint buffer_index = op_index * OP_UNIT_LENGTH + 18;
		uint op = object.primitive_ops[buffer_index++];
		float blend = uintBitsToFloat(object.primitive_ops[buffer_index++]);

		closest_res = process_op(op, blend, closest_res, primitive_res);
	}

	return closest_res;
}

// https://iquilezles.org/articles/normalsSDF
vec3 calcNormal(vec3 pos)
{
	const vec2 e = NORMAL_OFFSET;
	return normalize(e.xyy * map(pos + e.xyy).d +
					 e.yyx * map(pos + e.yyx).d +
					 e.yxy * map(pos + e.yxy).d +
					 e.xxx * map(pos + e.xxx).d);
}

float dist_to_depth(float dist, float near, float far) {
	float b = near / (far - near);
	float a = far * b;
	return a / dist - b;
}

float depth_to_dist(float depth, float near, float far) {
	float b = near / (far - near);
	float a = far * b;
	return a / (depth + b);
}

// Render the scene with sphere tracing and write the normal and object id.
// When the ray misses, calls discard. Otherwise writes depth of a hit primitive.
// https://michaelwalczyk.com/blog-ray-marching.html
void ray_march(const vec3 ray_o, const vec3 ray_d, out float o_dist,
			   out vec3 o_normal, out vec4 o_albedo, out uint o_object_id)
{
	// total distance traveled. start at the frag depth
	float dist = cam.near;
	const float dist_max = depth_to_dist(gl_FragCoord.z, cam.near, cam.far);

	for (int i = 0; dist < dist_max && i < MAX_STEPS; i++) {
		// get the world space position from the current marching distance
		vec3 current_pos = ray_o + ray_d * dist;
		// get the distance to the closest primitive
		SdfResult closest_primitive = map(current_pos);

		// ray hit
		if (closest_primitive.d < MIN_MARCH_STEP) {
			o_normal = calcNormal(current_pos);
			o_object_id = (closest_primitive.op_index & 0xFFFF) | (in_object_id << 16);
			o_albedo = vec4(0.8, 0.2, 0.1, 1.0);
			o_dist = dist;
			return;
		}

		// incriment the distance travelled by the distance to the closest primitive
		dist += closest_primitive.d;
	}

	// ray miss
	// see create_clear_values() in vulkan_init.rs for the default framebuffer values
	discard;
}

// ~~~ Main ~~~

/// Normalized ray direction in world space
vec3 ray_direction() {
	float clip_space_depth = -cam.near / cam.far; // results in z = 1 after multiplying my the proj matrix
	vec4 ray_d = cam.proj_view_inverse * vec4(in_clip_space_uv, clip_space_depth, 1.);
	return normalize(ray_d.xyz);
}

void main()
{
	vec3 ray_d_norm = ray_direction();
	
	float z;
	vec3 normal;
	vec4 albedo;
	uint object_id;
	ray_march(cam.position.xyz, ray_d_norm, z, normal, albedo, object_id);

	gl_FragDepth = dist_to_depth(z, cam.near, cam.far);
	out_normal = vec4(normal / 2. + 0.5, 0.); // fit [-1, 1] in unorm range
	out_albedo = albedo;
	out_object_id = object_id;
}