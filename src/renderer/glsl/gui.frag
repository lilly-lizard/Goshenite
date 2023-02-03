#version 450

layout (location = 0) in vec4 in_color;
layout (location = 1) in vec2 in_tex_coords;

layout (location = 0) out vec4 out_color;

layout (binding = 0, set = 0) uniform sampler2D font_texture;
layout (push_constant) uniform GuiPushConstant {
    vec2 screen_size;
    uint srgb_framebuffer;
} push_constants;

void main() {
    vec4 texture_color = texture(font_texture, in_tex_coords);
    out_color = in_color * texture_color;

    if (push_constants.srgb_framebuffer == 0) {
        // vertex colors and textures in srgb, so we need to convert to linear
        out_color.xyz = pow(out_color.xyz, vec3(1. / 2.2));
    }
}