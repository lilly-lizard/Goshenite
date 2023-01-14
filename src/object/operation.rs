use crate::shaders::object_buffer::{op_codes, ObjectDataUnit};

pub enum Operation {
    None,
    Union,
    Intersection,
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
