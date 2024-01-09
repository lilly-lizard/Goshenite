use super::operation::Operation;
use crate::{
    engine::primitives::{primitive::Primitive, primitive_transform::PrimitiveTransform},
    helper::unique_id_gen::{UniqueId, UniqueIdType},
};
use serde::{Deserialize, Serialize};

// PRIMITIVE OP

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PrimitiveOp {
    id: PrimitiveOpId,
    pub primitive: Primitive,
    pub transform: PrimitiveTransform,
    pub op: Operation,
    /// Amount of blending between this primitive op and the previous ops in world-space units.
    pub blend: f32,
}

impl PrimitiveOp {
    pub fn new(
        id: PrimitiveOpId,
        primitive: Primitive,
        transform: PrimitiveTransform,
        op: Operation,
        blend: f32,
    ) -> Self {
        Self {
            id,
            primitive,
            transform,
            op,
            blend,
        }
    }

    #[inline]
    pub fn id(&self) -> PrimitiveOpId {
        self.id
    }
}

// PRIMITIVE OP ID

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct PrimitiveOpId(pub UniqueId);

impl UniqueIdType for PrimitiveOpId {
    fn raw_id(&self) -> UniqueId {
        self.0
    }
}

impl From<UniqueId> for PrimitiveOpId {
    fn from(id: UniqueId) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for PrimitiveOpId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw_id())
    }
}
