use crate::renderer::shader_interfaces::primitive_op_buffer::{op_codes, PrimitiveOpBufferUnit};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum Operation {
    /// Combination of this primitive and current shape. Equivalent to AND.
    Union,
    /// Intersection of this primitive with current shape. Equivalent to OR.
    Intersection,
    /// Subtract this primitive from current shape.
    Subtraction,
    /// No-op
    Nop,
}

const VARIANTS: &[Operation] = &[
    Operation::Union,
    Operation::Intersection,
    Operation::Subtraction,
    Operation::Nop,
];

impl Operation {
    pub fn op_code(&self) -> PrimitiveOpBufferUnit {
        match *self {
            Self::Union => op_codes::UNION,
            Self::Intersection => op_codes::INTERSECTION,
            Self::Subtraction => op_codes::SUBTRACTION,
            Self::Nop => op_codes::NOP,
        }
    }

    pub fn name(&self) -> &str {
        match *self {
            Self::Union => "Union",
            Self::Intersection => "Intersection",
            Self::Subtraction => "Subtraction",
            Self::Nop => "No-op",
        }
    }

    pub fn variants_with_names() -> Vec<(Self, &'static str)> {
        VARIANTS.iter().map(|op| (*op, op.name())).collect()
    }
}

impl Default for Operation {
    fn default() -> Self {
        Self::Union
    }
}
