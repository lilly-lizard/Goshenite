#version 450

layout (input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput inColor;
layout (location = 0) in vec2 inUV;
layout (location = 0) out vec4 outColor;

void main() 
{
	// todo sRGB?
	outColor = subpassLoad(inColor).rgba;
}