#version 430

layout (location = 0) out vec4 out_normal;
layout (location = 1) out vec4 out_albedo_specular;
layout (location = 2) out uint out_object_op_id;
layout (depth_less) out float gl_FragDepth; // https://docs.vulkan.org/guide/latest/depth.html#conservative-depth

layout (push_constant) uniform GizmoPushConstant {
	vec3 color;
	uint object_id;
} pc;

void main()
{
	out_normal = vec4(1., 0., 0., 0.);
	out_albedo_specular = vec4(pc.color, 1.);
	out_object_op_id = pc.object_id;
	float infinity = 1.0 / 0.0;
	gl_FragDepth = infinity;
}