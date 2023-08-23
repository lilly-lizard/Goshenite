use super::operation::Operation;
use crate::{engine::primitives::primitive::Primitive, helper::unique_id_gen::UniqueId};

// PRIMITIVE OP

pub struct PrimitiveOp {
    id: PrimitiveOpId,
    pub op: Operation,
    pub primitive: Primitive,
}

impl PrimitiveOp {
    pub fn new(id: PrimitiveOpId, op: Operation, primitive: Primitive) -> Self {
        Self { id, op, primitive }
    }

    pub fn id(&self) -> PrimitiveOpId {
        self.id
    }

    pub fn duplicate(&self) -> PrimitiveOpDuplicate {
        PrimitiveOpDuplicate {
            id: self.id,
            op: self.op,
            primitive: self.primitive,
        }
    }
}

#[derive(Clone)]
pub struct PrimitiveOpDuplicate {
    pub id: PrimitiveOpId,
    pub op: Operation,
    pub primitive: Primitive,
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
