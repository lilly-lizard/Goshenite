
// Shaves `shave_radius` units off the sdf. Corners will be rounded.
SdfResult op_shave(SdfResult p, float shave_radius)
{
	SdfResult res = { p.d - shave_radius, p.op_index };
	return res;
}
