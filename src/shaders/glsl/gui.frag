#version 450

layout(location = 0) in vec4 in_color;
layout(location = 1) in vec2 in_tex_coords;

layout(location = 0) out vec4 out_color;

layout(binding = 0, set = 0) uniform sampler2D font_texture;
layout(push_constant) uniform PushConstants {
    vec2 screen_size;
    uint need_srgb_conv;
} push_constants;

// 0-255 sRGB  from  0-1 linear
vec3 srgb_from_linear(vec3 rgb) {
    bvec3 cutoff = lessThan(rgb, vec3(0.0031308));
    vec3 lower = rgb * vec3(3294.6);
    vec3 higher = vec3(269.025) * pow(rgb, vec3(1.0 / 2.4)) - vec3(14.025);
    return mix(higher, lower, vec3(cutoff));
}

vec4 srgba_from_linear(vec4 rgba) {
    return vec4(srgb_from_linear(rgba.rgb), 255.0 * rgba.a);
}

void main() {
    vec4 texture_color = texture(font_texture, in_tex_coords);

    if (push_constants.need_srgb_conv == 0) {
        out_color = in_color * texture_color;
    } else {
        out_color = srgba_from_linear(in_color * texture_color) / 255.0;
        out_color.a = pow(out_color.a, 1.6);
    }
}