#version 450

layout (set = 0, binding = 0) uniform sampler2D samplerColor;

layout (location = 0) in vec2 inUV;
layout (location = 0) out vec4 outColor;

void main() 
{
	// todo sRGB?
	outColor = texture(samplerColor, vec2(inUV.s, 1.0 - inUV.t));
}