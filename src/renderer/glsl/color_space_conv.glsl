vec3 linear_to_srgb(vec3 color) {
    return pow(color, vec3(1. / 2.2));
}

vec3 srgb_to_linear(vec3 color) {
    return pow(color, vec3(2.2));
}
