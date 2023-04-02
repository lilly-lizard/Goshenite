
// Represents a signed distance field result
struct SdfResult {
	// Distance from pos to primitive
	float d;
	// operation index
	uint op_index;
};

// ~~~ Signed Distance Fields ~~~
// https://iquilezles.org/articles/distfunctions/

float sdf_sphere(vec3 pos, vec3 center, float radius)
{
	return length(pos - center) - radius;
}

float sdf_box(vec3 pos, vec3 center, vec3 dimensions)
{
	vec3 d = abs(pos - center) - dimensions / 2.;
	float c = min(max(d.x, max(d.y, d.z)), 0.);
	float l = length(max(d, 0.));
	return c + l;
}

float sdf_box_frame(vec3 pos, vec3 center, float dimension_inner, vec3 dimensions_outer)
{
	vec3 pos_relative = pos - center;
	vec3 p = abs(pos_relative)                   - dimensions_outer;
	vec3 q = abs(pos_relative + dimension_inner) - dimension_inner;
	float c1 = length(max(vec3(p.x, q.y, q.z), 0.0)) + min(max(p.x, max(q.y, q.z)), 0.0);
	float c2 = length(max(vec3(q.x, p.y, q.z), 0.0)) + min(max(q.x, max(p.y, q.z)), 0.0);
	float c3 = length(max(vec3(q.x, q.y, p.z), 0.0)) + min(max(q.x, max(q.y, p.z)), 0.0);
	return min(min(c1, c2), c3);
}

// Hole is along z axis
float sdf_torus(vec3 pos, vec3 center, float radius_inner, float radius_outer)
{
	vec3 pos_relative = pos - center;
	float l = length(pos_relative.xy) - radius_inner;
	vec2 q = vec2(l, pos_relative.z);
	return length(q) - radius_outer;
}

// todo wtf arg? https://www.shadertoy.com/view/tl23RK
float sdf_capped_torus(vec3 pos, vec3 center, float radius_inner, float radius_outer, vec2 wtf)
{
	vec3 pos_relative = pos - center;
	pos_relative.x = abs(pos_relative.x);
	bool b = wtf.y * pos_relative.x > wtf.x * pos_relative.y;
	float k = b ? dot(pos_relative.xy, wtf) : length(pos_relative.xy);
	float j = radius_inner * radius_inner - 2. * radius_inner * k;
	return sqrt(j + dot(pos_relative, pos_relative)) - radius_outer;
}

// ~~~ Combination Ops ~~~

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
