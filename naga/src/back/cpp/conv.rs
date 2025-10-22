use crate::{Scalar, ScalarKind, TypeInner, VectorSize};

/// Helper to convert WGSL scalar types to C++ types
pub fn scalar_to_cpp_type(scalar: Scalar) -> &'static str {
    match scalar.kind {
        ScalarKind::Sint => match scalar.width {
            4 => "int32_t",
            8 => "int64_t",
            _ => "int32_t", // default
        },
        ScalarKind::Uint => match scalar.width {
            4 => "uint32_t",
            8 => "uint64_t",
            _ => "uint32_t", // default
        },
        ScalarKind::Float => match scalar.width {
            4 => "float",
            8 => "double",
            _ => "float", // default
        },
        ScalarKind::Bool => "bool",
        ScalarKind::AbstractInt | ScalarKind::AbstractFloat => {
            // These should be resolved during validation
            "float"
        }
    }
}

/// Helper to get vector size as usize
pub fn vector_size_to_usize(size: VectorSize) -> usize {
    match size {
        VectorSize::Bi => 2,
        VectorSize::Tri => 3,
        VectorSize::Quad => 4,
    }
}

/// Helper to convert WGSL binary operators to C++ operators
pub fn binary_op_to_cpp(op: crate::BinaryOperator) -> &'static str {
    use crate::BinaryOperator as Bo;
    match op {
        Bo::Add => "+",
        Bo::Subtract => "-",
        Bo::Multiply => "*",
        Bo::Divide => "/",
        Bo::Modulo => "%",
        Bo::Equal => "==",
        Bo::NotEqual => "!=",
        Bo::Less => "<",
        Bo::LessEqual => "<=",
        Bo::Greater => ">",
        Bo::GreaterEqual => ">=",
        Bo::And => "&",
        Bo::ExclusiveOr => "^",
        Bo::InclusiveOr => "|",
        Bo::LogicalAnd => "&&",
        Bo::LogicalOr => "||",
        Bo::ShiftLeft => "<<",
        Bo::ShiftRight => ">>",
    }
}

/// Helper to convert WGSL unary operators to C++ operators
pub fn unary_op_to_cpp(op: crate::UnaryOperator) -> &'static str {
    use crate::UnaryOperator as Uo;
    match op {
        Uo::Negate => "-",
        Uo::LogicalNot => "!",
        Uo::BitwiseNot => "~",
    }
}

/// Check if a type needs to be passed by reference
pub fn should_pass_by_reference(inner: &TypeInner) -> bool {
    match *inner {
        TypeInner::Scalar { .. } => false,
        TypeInner::Vector { .. } => false, // Small vectors can be passed by value
        TypeInner::Matrix { .. } => true,  // Matrices should be passed by reference
        TypeInner::Atomic { .. } => false,
        TypeInner::Pointer { .. } => false,
        TypeInner::ValuePointer { .. } => false,
        TypeInner::Array { .. } => true,
        TypeInner::Struct { .. } => true,
        TypeInner::Image { .. } => true,
        TypeInner::Sampler { .. } => true,
        TypeInner::AccelerationStructure { .. } => true,
        TypeInner::RayQuery { .. } => true,
        TypeInner::BindingArray { .. } => true,
    }
}
