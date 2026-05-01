#version 430

// output color to swapchain image
layout (location = 0) out vec4 out_color;
layout (location = 1) out uint out_id;

layout (push_constant) uniform GizmoPushConstantFrag {
	vec3 color;
	uint object_id;
} pc;

void main()
{
	out_color = vec4(pc.color, 1.);
	out_id = pc.object_id;
}