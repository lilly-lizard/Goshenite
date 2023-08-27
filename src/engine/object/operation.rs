use crate::renderer::shader_interfaces::primitive_op_buffer::{op_codes, PrimitiveOpBufferUnit};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Operation {
    /// No-op
    NOP,
    /// Combination of this primitive and current shape. Equivalent to AND.
    Union,
    /// Intersection of this primitive with current shape. Equivalent to OR.
    Intersection,
    /// Subtract this primitive from current shape.
    Subtraction,
}

static VARIANTS: &[Operation] = &[
    Operation::NOP,
    Operation::Union,
    Operation::Intersection,
    Operation::Subtraction,
];

impl Operation {
    pub fn op_code(&self) -> PrimitiveOpBufferUnit {
        match *self {
            Self::NOP => op_codes::NOP,
            Self::Union => op_codes::UNION,
            Self::Intersection => op_codes::INTERSECTION,
            Self::Subtraction => op_codes::SUBTRACTION,
        }
    }

    pub fn name(&self) -> &'static str {
        match *self {
            Self::NOP => "No-op",
            Self::Union => "Union",
            Self::Intersection => "Intersection",
            Self::Subtraction => "Subtraction",
        }
    }

    pub fn variant_names() -> Vec<(Self, &'static str)> {
        VARIANTS
            .iter()
            .map(|op| (*op, op.name()))
            .collect::<Vec<(Self, &'static str)>>()
    }
}

impl Default for Operation {
    fn default() -> Self {
        Self::NOP
    }
}
