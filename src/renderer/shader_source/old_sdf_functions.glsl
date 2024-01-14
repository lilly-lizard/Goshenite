
// ~~~ Combination Ops ~~~

// Represents a signed distance field result
struct SdfResult {
	// Distance from pos to primitive
	float d;
	// operation index
	uint op_index;
};

// Results in the union (min) of 2 primitives
SdfResult op_union(SdfResult p1, SdfResult p2)
{
	return p1.d < p2.d ? p1 : p2;
}

// Results in the intersection (max) of 2 primitives
SdfResult op_intersection(SdfResult p1, SdfResult p2)
{
	return p1.d > p2.d ? p1 : p2;
}

// Subtracts the volume of primitive 2 (max) from primitive 1 (max inverted)
SdfResult op_subtraction(SdfResult p1, SdfResult p2)
{
	SdfResult p2_neg = { -p2.d, p2.op_index };
	return op_intersection(p1, p2_neg);
}

// Shaves `shave_radius` units off the sdf. Corners will be rounded.
SdfResult op_shave(SdfResult p, float shave_radius)
{
	SdfResult res = { p.d - shave_radius, p.op_index };
	return res;
}

// ~~~ Uber Primitive ~~~
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

// cube debugging
float sdf_uber_primitive(vec3 pos, vec4 s, vec2 r)
{
	vec3 dimensions = s.xyz;
	vec3 d = abs(pos) - dimensions;
	float c = min(max(d.x, max(d.y, d.z)), 0.);
	float l = length(max(d, 0.));
	return c + l;
}

// ~~~ Signed Distance Fields ~~~
// https://iquilezles.org/articles/distfunctions/

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

/// Normalized ray direction in world space
vec3 ray_direction() {
	// can use clip_space_uv instead of in_uv clip space position in frame (between -1 and 1)
	//vec2 screen_space = gl_FragCoord.xy + vec2(0.5);
	//vec2 clip_space_uv = screen_space / cam.framebuffer_dims * 2. - 1.;
	float clip_space_depth = -cam.near / cam.far;
	vec4 ray_d = cam.proj_view_inverse * vec4(in_clip_space_uv, clip_space_depth, 1.);
	return normalize(ray_d.xyz);
}
