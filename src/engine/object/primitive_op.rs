use super::operation::Operation;
use crate::{
    engine::primitives::{null_primitive::NullPrimitive, primitive::Primitive},
    helper::unique_id_gen::UniqueId,
};

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

pub struct PrimitiveOp {
    id: PrimitiveOpId,
    pub op: Operation,
    pub primitive: Box<dyn Primitive>,
}

impl PrimitiveOp {
    pub fn new(id: PrimitiveOpId, op: Operation, primitive: Box<dyn Primitive>) -> Self {
        Self { id, op, primitive }
    }

    pub fn new_null() -> Self {
        Self::new(
            usize::MAX.into(),
            Operation::NOP,
            Box::new(NullPrimitive::new()),
        )
    }

    pub fn id(&self) -> PrimitiveOpId {
        self.id
    }
}
