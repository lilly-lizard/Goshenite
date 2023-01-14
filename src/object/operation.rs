use crate::shaders::object_buffer::{op_codes, ObjectDataUnit};

pub enum Operation {
    /// No-op
    None,
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
            Self::None => op_codes::NULL,
            Self::Union => op_codes::UNION,
            Self::Intersection => op_codes::INTERSECTION,
            Self::Subtraction => op_codes::SUBTRACTION,
        }
    }

    pub fn name(&self) -> &'static str {
        match *self {
            Self::None => "None",
            Self::Union => "Union",
            Self::Intersection => "Intersection",
            Self::Subtraction => "Subtraction",
        }
    }
}
