
// Represents a signed distance field result
struct SdfResult {
	// Distance from pos to primitive
	float d;
	// operation index
	uint op_index;
};

// ~~~ Signed Distance Fields ~~~
// https://iquilezles.org/articles/distfunctions/

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
