use super::operation::Operation;
use crate::{
    engine::primitives::{primitive::Primitive, primitive_transform::PrimitiveTransform},
    helper::unique_id_gen::{UniqueId, UniqueIdType},
};

// PRIMITIVE OP

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrimitiveOp {
    id: PrimitiveOpId,
    pub op: Operation,
    pub primitive: Primitive,
    pub primitive_transform: PrimitiveTransform,
}

impl PrimitiveOp {
    pub fn new(
        id: PrimitiveOpId,
        op: Operation,
        primitive: Primitive,
        primitive_transform: PrimitiveTransform,
    ) -> Self {
        Self {
            id,
            op,
            primitive,
            primitive_transform,
        }
    }

    #[inline]
    pub fn id(&self) -> PrimitiveOpId {
        self.id
    }
}

// PRIMITIVE OP ID

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
