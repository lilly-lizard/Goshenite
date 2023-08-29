use super::operation::Operation;
use crate::{engine::primitives::primitive::Primitive, helper::unique_id_gen::UniqueId};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrimitiveOpWithId(pub PrimitiveOpId, pub PrimitiveOp);

// PRIMITIVE OP

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrimitiveOp {
    pub op: Operation,
    pub primitive: Primitive,
}

impl PrimitiveOp {
    pub fn new(op: Operation, primitive: Primitive) -> Self {
        Self { op, primitive }
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
