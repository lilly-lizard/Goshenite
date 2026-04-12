#version 450
#extension GL_GOOGLE_include_directive : require
#include "color_space_conv.glsl"

layout (location = 0) in vec4 in_color; // srgb
layout (location = 1) in vec2 in_tex_coords;

layout (location = 0) out vec4 out_color;

layout (binding = 0, set = 0) uniform sampler2D font_texture; // srgb
layout (push_constant) uniform GuiPushConstant {
    vec2 _screen_size;
    uint write_linear_color;
} push_constants;

void main() {
    vec4 texture_color = texture(font_texture, in_tex_coords);

    out_color = in_color * texture_color;

    if (push_constants.write_linear_color == 1) {
        // surface format/color space combination requires us to write out linear color
        // see https://stackoverflow.com/questions/66401081/vulkan-swapchain-format-unorm-vs-srgb for more info
        out_color.xyz = srgb_to_linear(out_color.xyz);
    }
}