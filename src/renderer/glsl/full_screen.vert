// renders a full screen triangle

#version 450

layout (location = 0) out vec2 out_uv;

const vec2 uv[3] = vec2[](
    vec2(-1., -1.),
    vec2( 3., -1.),
    vec2(-1.,  3.)
);

void main() 
{
	gl_Position = vec4(uv[gl_VertexIndex], 0., 1.);
}
