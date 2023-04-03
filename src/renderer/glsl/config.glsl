
/// Defines the up/down axis in the world space coordinate system
const vec3 WORLD_SPACE_UP = vec3(0., 0., 1.);

/// Number of 32-bit values used to store data for an operation and associated primitive
/// Note: this is half of typical amd/nvidia cache line (128 bytes)
const uint OP_UNIT_LENGTH = 20;

/// Inicates an unset primitive id
const uint ID_INVALID = 0xFFFFFFFFu;

/// The codes for different primitive types
const uint PRIM_NULL      = 0x00000000u;
const uint PRIM_SPHERE    = 0x00000001u;
const uint PRIM_BOX       = 0x00000002u;
const uint PRIM_BOX_FRAME = 0x00000003u;
const uint PRIM_TORUS     = 0x00000004u;
const uint PRIM_TORUS_CAP = 0x00000005u;

/// The codes for different ops to execute
const uint OP_NULL 			= 0x00000000u;
const uint OP_UNION 		= 0x00000001u;
const uint OP_INTERSECTION 	= 0x00000002u;
const uint OP_SUBTRACTION 	= 0x00000003u;
