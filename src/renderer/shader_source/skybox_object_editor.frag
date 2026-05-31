#version 430
#extension GL_GOOGLE_include_directive : require
#include "config.glsl"

layout(location = 0) out vec4 out_normal;
layout(location = 1) out vec4 out_albedo_specular;
layout(location = 2) out uint out_object_op_id;

layout(set = 0, binding = 0) uniform Camera {
    mat4 view_inverse;
    mat4 proj_inverse;
    vec2 framebuffer_dims;
    float _near;
    float _far;
    vec3 _position;
    vec3 _direction;
    uint _write_linear_color;
} cam;

/// Returns a sky color for a ray miss
/// * `ray_d` - ray direction
vec3 background(const vec3 ray_d)
{
    return vec3(0.2, 0.2, 0.2) + 0.3 * dot(ray_d, WORLD_SPACE_UP);
}

/// Normalized ray direction in world space
vec3 ray_direction()
{
    vec2 clip_space_uv = gl_FragCoord.xy / cam.framebuffer_dims; // todo divide by gl_FragCoord.w?
    vec4 origin = cam.view_inverse * vec4(0, 0, 0, 1);
    vec4 target = cam.proj_inverse * vec4(clip_space_uv, 1, 1);
    vec4 direction = cam.view_inverse * vec4(normalize(target.xyz / target.w), 0);
    return normalize(direction.xyz); // todo what changes when normalize is removed here?
}

void main()
{
    vec3 ray_d = ray_direction();
    out_albedo_specular = vec4(background(ray_d), 1.);
    //out_albedo_specular = vec4(0., 0.5, 1.0, 1.);
    out_object_op_id = ID_BACKGROUND;
}
