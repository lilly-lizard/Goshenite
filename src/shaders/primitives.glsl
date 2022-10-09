/// Number of 32-but uint values used to store data for a primitive
const uint PRIMITIVE_UNIT_LENGTH = 8;

/// Inicates an unset primitive id
const uint ID_INVALID = 0xFFFFFFFFu;

/// The codes for different primitive types
const uint PRIMITIVE_NULL = 0x00000000u;
const uint PRIMITIVE_SPHERE = 0x00000001u;
const uint PRIMITIVE_CUBE = 0x00000002u;

/// Defines the up/down axis in the world space coordinate system
const vec3 WORLD_SPACE_UP = vec3(0., 0., 1.);

/// Data to be stored in the g-buffer
struct GBufferValue {
	vec3 normal;
	uint primitive_id;
};

/// Encode g-buffer data
vec4 gbuffer_encode(GBufferValue value) {
	return vec4(value.normal, uintBitsToFloat(value.primitive_id));
}

/// Decode g-buffer data
GBufferValue gbuffer_decode(vec4 encoded_value) {
	GBufferValue ret = {
		encoded_value.xyz,
		floatBitsToUint(encoded_value.w),
	};
	return ret;
}
