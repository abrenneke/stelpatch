use cw_model::CwtType;
use cw_parser::AstValue;

use crate::handlers::cache::TypeCache;

/// Check if a value is structurally compatible with a type (without content validation)
pub fn is_value_structurally_compatible(value: &AstValue<'_>, expected_type: &CwtType) -> bool {
    is_value_structurally_compatible_with_depth(value, expected_type, 0)
}

/// Check if a value is structurally compatible with a type with recursion depth limit
fn is_value_structurally_compatible_with_depth(
    value: &AstValue<'_>,
    expected_type: &CwtType,
    depth: usize,
) -> bool {
    // Prevent infinite recursion
    if depth > 10 {
        return false;
    }

    if !TypeCache::is_initialized() {
        return true; // Default to compatible if cache not available
    }

    let cache = TypeCache::get().unwrap();
    let resolved_type = cache.resolve_type(expected_type);

    match (&resolved_type, value) {
        // Block types are compatible with entities
        (CwtType::Block(_), AstValue::Entity(_)) => true,

        // Literal types are compatible with strings
        (CwtType::Literal(_), AstValue::String(_)) => true,

        // Literal sets are compatible with strings
        (CwtType::LiteralSet(_), AstValue::String(_)) => true,

        // Simple types - check basic compatibility
        (CwtType::Simple(simple_type), _) => {
            is_value_compatible_with_simple_type_structurally(value, simple_type)
        }

        // Array types are compatible with entities
        (CwtType::Array(_), AstValue::Entity(_)) => true,

        // Union types - check if compatible with any member
        (CwtType::Union(types), _) => types.iter().any(|union_type| {
            is_value_structurally_compatible_with_depth(value, union_type, depth + 1)
        }),

        // Comparable types - check compatibility with base type
        (CwtType::Comparable(base_type), _) => {
            is_value_structurally_compatible_with_depth(value, base_type, depth + 1)
        }

        // Reference types - resolve and check
        (CwtType::Reference(_), _) => {
            // For now, assume references are compatible
            // TODO: Implement proper reference resolution
            true
        }

        // Unknown types are always compatible
        (CwtType::Unknown, _) => true,

        // Everything else is incompatible
        _ => false,
    }
}

/// Check if a value is structurally compatible with a simple type
fn is_value_compatible_with_simple_type_structurally(
    value: &AstValue<'_>,
    simple_type: &cw_model::SimpleType,
) -> bool {
    use cw_model::SimpleType;

    match (value, simple_type) {
        // String-based types
        (AstValue::String(_), SimpleType::Localisation) => true,
        (AstValue::String(_), SimpleType::LocalisationSynced) => true,
        (AstValue::String(_), SimpleType::LocalisationInline) => true,
        (AstValue::String(_), SimpleType::Filepath) => true,
        (AstValue::String(_), SimpleType::Icon) => true,
        (AstValue::String(_), SimpleType::VariableField) => true,
        (AstValue::String(_), SimpleType::ScopeField) => true,
        (AstValue::String(_), SimpleType::DateField) => true,
        (AstValue::String(_), SimpleType::Scalar) => true,
        (AstValue::String(_), SimpleType::IntVariableField) => true,
        (AstValue::String(_), SimpleType::IntValueField) => true,
        (AstValue::String(_), SimpleType::Bool) => true,

        // Number-based types
        (AstValue::Number(_), SimpleType::ValueField) => true,
        (AstValue::Number(_), SimpleType::Int) => true,
        (AstValue::Number(_), SimpleType::Float) => true,
        (AstValue::Number(_), SimpleType::PercentageField) => true,

        // Specialized types
        (AstValue::Color(_), SimpleType::Color) => true,
        (AstValue::Maths(_), SimpleType::Maths) => true,

        // Everything else is incompatible
        _ => false,
    }
}
