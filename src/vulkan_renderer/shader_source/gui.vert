#version 450

layout (location = 0) in vec2 in_position;
layout (location = 1) in vec2 in_tex_coords;
layout (location = 2) in vec4 in_color;

layout (location = 0) out vec4 out_color;
layout (location = 1) out vec2 out_tex_coords;

layout (push_constant) uniform GuiPushConstant {
    vec2 screen_size;
    uint _write_linear_color;
} pc;

void main()
{
    gl_Position = vec4(2.0 * in_position.x / pc.screen_size.x - 1.0,
                       2.0 * in_position.y / pc.screen_size.y - 1.0,
                       0.0, 1.0);

    out_color = in_color;
    out_tex_coords = in_tex_coords;
}