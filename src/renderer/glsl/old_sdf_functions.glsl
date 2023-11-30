
// Shaves `shave_radius` units off the sdf. Corners will be rounded.
SdfResult op_shave(SdfResult p, float shave_radius)
{
	SdfResult res = { p.d - shave_radius, p.op_index };
	return res;
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
