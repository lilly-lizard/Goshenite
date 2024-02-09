#version 430

layout (location = 0) out vec4 out_color;
layout (location = 1) out uint out_object_id;

layout (push_constant) uniform GizmoPushConstant {
	vec3 color;
	uint object_id;
};