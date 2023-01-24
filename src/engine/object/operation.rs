use crate::renderer::shader_interfaces::object_buffer::{op_codes, ObjectDataUnit};

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

impl Operation {
    pub fn op_code(&self) -> ObjectDataUnit {
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
}
