
/// Defines the up/down axis in the world space coordinate system
const vec3 WORLD_SPACE_UP = vec3(0., 0., 1.);

/// Number of 32-bit values used to store data for an operation and associated primitive
/// Note: this is half of typical amd/nvidia cache line (128 bytes)
const uint OP_UNIT_LENGTH = 20;

/// Inicates an unset primitive id
const uint ID_INVALID = 0xFFFFFFFFu;

/// The codes for different ops to execute
const uint OP_NULL 			= 0x00000000u;
const uint OP_UNION 		= 0x00000001u;
const uint OP_INTERSECTION 	= 0x00000002u;
const uint OP_SUBTRACTION 	= 0x00000003u;
