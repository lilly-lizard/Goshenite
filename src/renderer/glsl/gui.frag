#version 450

layout (location = 0) in vec4 in_color; // linear
layout (location = 1) in vec2 in_tex_coords;

layout (location = 0) out vec4 out_color;

layout (binding = 0, set = 0) uniform sampler2D font_texture; // srgb
layout (push_constant) uniform GuiPushConstant {
    vec2 _screen_size;
    uint is_srgb_framebuffer;
} push_constants;

void main() {
    vec4 texture_color = texture(font_texture, in_tex_coords);

    // convert srgb texture color to linear for multiplication with vertext colors
    texture_color.xyz = pow(texture_color.xyz, vec3(1. / 2.2));

    out_color = in_color * texture_color;

    if (push_constants.is_srgb_framebuffer == 1) {
        // convert color back to srgb
        out_color.xyz = pow(out_color.xyz, vec3(1. * 2.2));
    }
}