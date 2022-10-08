[[spirv::out(0)]]
vec2 out_uv;

[[spirv::vert]]
void main()
{ 
	out_uv = vec2((glvert_VertexIndex << 1) & 2, glvert_VertexIndex & 2);
	glvert_Output.Position = vec4(out_uv * 2.0f + -1., 0., 1.);
}