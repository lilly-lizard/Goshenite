// renders a full screen triangle

#version 450

layout (location = 0) out vec2 out_uv;

void main() 
{
	vec2 uv = vec2((gl_VertexIndex << 1u) & 2u, gl_VertexIndex & 2u);
	out_uv = uv * 2. - 1.;
	gl_Position = vec4(out_uv, 0., 1.);
}
