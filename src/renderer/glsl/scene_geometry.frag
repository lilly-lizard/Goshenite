#version 450
#extension GL_EXT_nonuniform_qualifier : require
#extension GL_GOOGLE_include_directive : require
#include "config.glsl"
#include "sdf_functions.glsl"

// Maximum number of ray marching steps before confirming a miss
const uint MAX_STEPS = 50;
// Distance required to confirm a hit
const float MIN_MARCH_STEP = 0.001;
// Offset used for calculating normals.
const float NORMAL_EPSILON = 0.001;
const vec2 NORMAL_OFFSET = vec2(NORMAL_EPSILON, -NORMAL_EPSILON);

// ~~~ IO ~~~

layout (location = 0) in flat uint in_object_id;

layout (location = 0) out vec4 out_normal;
layout (location = 1) out uint out_object_id; // upper 16 bits = object index; lower 16 bits = op index; todo checks for 16bit max on rust side??

layout (set = 0, binding = 0) uniform Camera {
	mat4 proj_view_inverse;
	vec4 position;
	vec2 framebuffer_dims;
	float near;
	float far;
    uint _is_srgb_framebuffer;
} cam;

layout (set = 1, binding = 0, std430) readonly buffer Object {
	uint _id;
	uint op_count;
	uint primitive_ops[];
} object;

// todo layout(depth_less) out float gl_FragDepth; https://github.com/KhronosGroup/Vulkan-Guide/blob/main/chapters/depth.adoc

// ~~~ Code interpreters ~~~

// todo need radius?
float sdf_sphere(vec3 pos, float radius)
{
	return length(pos) - radius;
}

float sdf_box(vec3 pos, vec3 dimensions)
{
	vec3 d = abs(pos) - dimensions / 2.;
	float c = min(max(d.x, max(d.y, d.z)), 0.);
	float l = length(max(d, 0.));
	return c + l;
}

float sdf_box_frame(vec3 pos, float dimension_inner, vec3 dimensions_outer)
{
	vec3 p = abs(pos)                   - dimensions_outer;
	vec3 q = abs(pos + dimension_inner) - dimension_inner;
	float c1 = length(max(vec3(p.x, q.y, q.z), 0.0)) + min(max(p.x, max(q.y, q.z)), 0.0);
	float c2 = length(max(vec3(q.x, p.y, q.z), 0.0)) + min(max(q.x, max(p.y, q.z)), 0.0);
	float c3 = length(max(vec3(q.x, q.y, p.z), 0.0)) + min(max(q.x, max(q.y, p.z)), 0.0);
	return min(min(c1, c2), c3);
}

// Hole is along z axis
float sdf_torus(vec3 pos, float radius_inner, float radius_outer)
{
	float l = length(pos.xy) - radius_inner;
	vec2 q = vec2(l, pos.z);
	return length(q) - radius_outer;
}

// todo wtf arg? https://www.shadertoy.com/view/tl23RK
float sdf_capped_torus(vec3 pos, float radius_inner, float radius_outer, vec2 wtf)
{
	pos.x = abs(pos.x);
	bool b = wtf.y * pos.x > wtf.x * pos.y;
	float k = b ? dot(pos.xy, wtf) : length(pos.xy);
	float j = radius_inner * radius_inner - 2. * radius_inner * k;
	return sqrt(j + dot(pos, pos)) - radius_outer;
}

SdfResult process_primitive(uint buffer_index, uint op_index, vec3 pos)
{
	uint primitive_type = object.primitive_ops[buffer_index++];

	if (primitive_type == PRIM_NULL) {
		SdfResult ret = { cam.far, op_index };
		return ret;
	}

	// todo perf comparison: load OP_UNIT_LENGTH values at once then decode below

	// decode primitive translation vector
	vec3 center = vec3(
		uintBitsToFloat(object.primitive_ops[buffer_index++]),
		uintBitsToFloat(object.primitive_ops[buffer_index++]),
		uintBitsToFloat(object.primitive_ops[buffer_index++])
	);

	// decode primitive rotation matrix
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

	// apply to position
	pos = pos - center; // todo before or after transform?
	pos = pos * rotation;

	float dist;
	if (primitive_type == PRIM_SPHERE)
	{
		float radius = uintBitsToFloat(object.primitive_ops[buffer_index++]);
		dist = sdf_sphere(pos, radius);
	}
	else if (primitive_type == PRIM_BOX)
	{
		vec3 dimensions = vec3(
			uintBitsToFloat(object.primitive_ops[buffer_index++]),
			uintBitsToFloat(object.primitive_ops[buffer_index++]),
			uintBitsToFloat(object.primitive_ops[buffer_index++])
		);
		dist = sdf_box(pos, dimensions);
	}
	else
	{
		// else do nothing
		dist = cam.far;
	}

	SdfResult ret = { dist, op_index };
	return ret;
}

SdfResult process_op(uint op, SdfResult lhs, SdfResult rhs)
{
	SdfResult res;
	
	switch(op)
	{
	case OP_UNION: 			res = op_union(lhs, rhs); break;
	case OP_INTERSECTION: 	res = op_intersection(lhs, rhs); break;
	case OP_SUBTRACTION: 	res = op_subtraction(lhs, rhs); break;
	default:				res = lhs; // else do nothing e.g. OP_NULL
	}

	return res;
}

// ~~~ Scene Traversal ~~~

// Calculates the distance to the closest primitive in the scene from `pos`
SdfResult map(vec3 pos)
{
	// the closest primitve and distance to pos p
	SdfResult closest_res = { cam.far, ID_INVALID };

	// loop through the primitive operations
	for (uint op_index = 0; op_index < object.op_count; op_index++) {

		uint buffer_index = op_index * OP_UNIT_LENGTH;
		uint op = object.primitive_ops[buffer_index++];

		SdfResult primitive_res = process_primitive(buffer_index, op_index, pos);
		closest_res = process_op(op, closest_res, primitive_res);
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
	return near + depth * (far - near);
}

// Render the scene with sphere tracing and write the normal and object id.
// When the ray misses, calls discard. Otherwise writes depth of a hit primitive.
// https://michaelwalczyk.com/blog-ray-marching.html
void ray_march(const vec3 ray_o, const vec3 ray_d, out float o_dist, out vec3 o_normal, out uint o_object_id)
{
	// total distance traveled. start at the frag depth
	float dist = cam.near;// + gl_FragCoord.z * (cam.far - cam.near);

	// todo MAX_STEPS as hit condition instead of miss? may get rid of some wacky artifacts...
	for (int i = 0; i < MAX_STEPS && dist < cam.far; i++) {
		// get the world space position from the current marching distance
		vec3 current_pos = ray_o + ray_d * dist;
		// get the distance to the closest primitive
		SdfResult closest_primitive = map(current_pos);

		// ray hit
		if (closest_primitive.d < MIN_MARCH_STEP) {
			o_normal = calcNormal(current_pos) / 2. + .5;
			o_object_id = closest_primitive.op_index | (in_object_id << 16);
			o_dist = dist;
			return;
		}

		// incriment the distance travelled by the distance to the closest primitive
		dist += closest_primitive.d;
	}

	// ray miss
	discard;
}

// ~~~ Main ~~~

void main()
{
	// out_normal = vec4(0.9, 0.7, 0.5, 1.);
	// out_object_id = 1;
	// return;
	
	// clip space position in frame (between -1 and 1)
	vec2 screen_space = gl_FragCoord.xy + vec2(0.5);
	vec2 clip_space_uv = screen_space / cam.framebuffer_dims * 2. - 1.;
	float clip_space_depth = -cam.near / cam.far;

	// ray direction in world space
	vec4 ray_d = cam.proj_view_inverse * vec4(clip_space_uv, clip_space_depth, 1.);
	vec3 ray_d_norm = normalize(ray_d.xyz);

	// render scene
	float z;
	vec3 normal;
	uint object_id;
	ray_march(cam.position.xyz, ray_d_norm, z, normal, object_id);

	gl_FragDepth = dist_to_depth(z, cam.near, cam.far);
	out_normal = vec4(normal, 0.);
	out_object_id = object_id;
}