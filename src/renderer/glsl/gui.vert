#version 450

layout (location = 0) in vec2 in_position;
layout (location = 1) in vec2 in_tex_coords;
layout (location = 2) in vec4 in_color;

layout (location = 0) out vec4 out_color;
layout (location = 1) out vec2 out_tex_coords;

layout (push_constant) uniform GuiPushConstants {
    vec2 screen_size;
    uint need_srgb_conv;
} pc;

// 0-1 linear  from  0-255 sRGB
vec3 linear_from_srgb(vec3 srgb) {
    bvec3 cutoff = lessThan(srgb, vec3(10.31475));
    vec3 lower = srgb / vec3(3294.6);
    vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
    return mix(higher, lower, cutoff);
}

vec4 linear_from_srgba(vec4 srgba) {
    return vec4(linear_from_srgb(srgba.rgb * 255.0), srgba.a);
}

void main() {
  gl_Position =
      vec4(2.0 * in_position.x / pc.screen_size.x - 1.0,
           2.0 * in_position.y / pc.screen_size.y - 1.0, 0.0, 1.0);
  // We must convert vertex color to linear
  out_color = linear_from_srgba(in_color);
  out_tex_coords = in_tex_coords;
}