use super::operation::Operation;
use crate::{engine::primitives::primitive::Primitive, helper::unique_id_gen::UniqueId};

// PRIMITIVE OP

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrimitiveOp {
    id: PrimitiveOpId,
    pub op: Operation,
    pub primitive: Primitive,
}

impl PrimitiveOp {
    pub fn new(id: PrimitiveOpId, op: Operation, primitive: Primitive) -> Self {
        Self { id, op, primitive }
    }

    #[inline]
    pub fn id(&self) -> PrimitiveOpId {
        self.id
    }
}

// PRIMITIVE OP ID

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrimitiveOpId(pub UniqueId);
impl PrimitiveOpId {
    pub const fn raw_id(&self) -> UniqueId {
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
