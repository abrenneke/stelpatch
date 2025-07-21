use std::sync::Arc;

use cw_parser::{AstEntityItem, AstValue};

use crate::handlers::{
    cache::TypeCache,
    scoped_type::{CwtTypeOrSpecialRef, PropertyNavigationResult, ScopedType},
};

/// Check if a value is structurally compatible with a type (without content validation)
pub fn is_value_structurally_compatible(
    value: &AstValue<'_>,
    expected_type: Arc<ScopedType>,
) -> bool {
    is_value_structurally_compatible_with_depth(value, expected_type, 0)
}

/// Calculate a structural compatibility score (higher is better match)
/// Returns a score from 0.0 to 1.0, where 1.0 is perfect structural match
pub fn calculate_structural_compatibility_score(
    value: &AstValue<'_>,
    expected_type: Arc<ScopedType>,
) -> f64 {
    calculate_structural_compatibility_score_with_depth(value, expected_type, 0)
}

/// Calculate structural compatibility score with recursion depth limit
fn calculate_structural_compatibility_score_with_depth(
    value: &AstValue<'_>,
    expected_type: Arc<ScopedType>,
    depth: usize,
) -> f64 {
    // Prevent infinite recursion
    if depth > 10 {
        return 0.0;
    }

    if !TypeCache::is_initialized() {
        return 0.5; // Default to neutral score if cache not available
    }

    let cache = TypeCache::get().unwrap();
    let resolved_type = cache.resolve_type(expected_type.clone());

    match (&resolved_type.cwt_type_for_matching(), value) {
        // Block types with entities - score based on key matches
        (CwtTypeOrSpecialRef::Block(_), AstValue::Entity(entity)) => {
            let mut total_keys = 0;
            let mut matching_keys = 0;

            for item in &entity.items {
                if let AstEntityItem::Expression(expr) = item {
                    total_keys += 1;
                    let key_name = expr.key.raw_value();

                    if let PropertyNavigationResult::Success(_) = cache
                        .get_resolver()
                        .navigate_to_property(expected_type.clone(), key_name)
                    {
                        matching_keys += 1;
                    }
                }
            }

            if total_keys == 0 {
                return 1.0; // Empty entity matches empty block perfectly
            }

            matching_keys as f64 / total_keys as f64
        }

        // Exact type matches get high scores
        (CwtTypeOrSpecialRef::Literal(_), AstValue::String(_)) => 0.9,
        (CwtTypeOrSpecialRef::LiteralSet(_), AstValue::String(_)) => 0.9,
        (CwtTypeOrSpecialRef::Array(_), AstValue::Entity(_)) => 0.8,

        // Simple types - check basic compatibility
        (CwtTypeOrSpecialRef::Simple(simple_type), _) => {
            if is_value_compatible_with_simple_type_structurally(value, simple_type) {
                0.9
            } else {
                0.0
            }
        }

        // Union types - return the best score from any member
        (CwtTypeOrSpecialRef::Union(types), _) => types
            .iter()
            .map(|union_type| {
                calculate_structural_compatibility_score_with_depth(
                    value,
                    Arc::new(ScopedType::new_cwt(
                        union_type.clone(),
                        expected_type.scope_stack().clone(),
                        expected_type.in_scripted_effect_block().cloned(),
                    )),
                    depth + 1,
                )
            })
            .fold(0.0, f64::max),

        // Comparable types - check compatibility with base type
        (CwtTypeOrSpecialRef::Comparable(base_type), _) => {
            calculate_structural_compatibility_score_with_depth(
                value,
                Arc::new(ScopedType::new_cwt(
                    (***base_type).clone(),
                    expected_type.scope_stack().clone(),
                    expected_type.in_scripted_effect_block().cloned(),
                )),
                depth + 1,
            )
        }

        // Reference types - moderate score
        (CwtTypeOrSpecialRef::Reference(_), _) => 0.7,

        // Unknown types get neutral score
        (CwtTypeOrSpecialRef::Unknown, _) => 0.5,

        // Any type accepts everything
        (CwtTypeOrSpecialRef::Any, _) => 0.6,

        // ScopedUnion - return best score from any member
        (CwtTypeOrSpecialRef::ScopedUnion(scoped_types), _) => scoped_types
            .iter()
            .map(|scoped_type| {
                calculate_structural_compatibility_score_with_depth(
                    value,
                    scoped_type.clone(),
                    depth + 1,
                )
            })
            .fold(0.0, f64::max),

        // Everything else is incompatible
        _ => 0.0,
    }
}

/// Check if a value is structurally compatible with a type with recursion depth limit
fn is_value_structurally_compatible_with_depth(
    value: &AstValue<'_>,
    expected_type: Arc<ScopedType>,
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
    let resolved_type = cache.resolve_type(expected_type.clone());

    match (&resolved_type.cwt_type_for_matching(), value) {
        // Block types are compatible with entities
        (CwtTypeOrSpecialRef::Block(_), AstValue::Entity(_)) => true,

        // Literal types are compatible with strings
        (CwtTypeOrSpecialRef::Literal(_), AstValue::String(_)) => true,

        // Literal sets are compatible with strings
        (CwtTypeOrSpecialRef::LiteralSet(_), AstValue::String(_)) => true,

        // Simple types - check basic compatibility
        (CwtTypeOrSpecialRef::Simple(simple_type), _) => {
            is_value_compatible_with_simple_type_structurally(value, simple_type)
        }

        // Array types are compatible with entities
        (CwtTypeOrSpecialRef::Array(_), AstValue::Entity(_)) => true,

        // Union types - check if compatible with any member
        (CwtTypeOrSpecialRef::Union(types), _) => types.iter().any(|union_type| {
            is_value_structurally_compatible_with_depth(
                value,
                Arc::new(ScopedType::new_cwt(
                    union_type.clone(),
                    expected_type.scope_stack().clone(),
                    expected_type.in_scripted_effect_block().cloned(),
                )),
                depth + 1,
            )
        }),

        // Comparable types - check compatibility with base type
        (CwtTypeOrSpecialRef::Comparable(base_type), _) => {
            is_value_structurally_compatible_with_depth(
                value,
                Arc::new(ScopedType::new_cwt(
                    (***base_type).clone(),
                    expected_type.scope_stack().clone(),
                    expected_type.in_scripted_effect_block().cloned(),
                )),
                depth + 1,
            )
        }

        // Reference types - resolve and check
        (CwtTypeOrSpecialRef::Reference(_), _) => {
            // For now, assume references are compatible
            // TODO: Implement proper reference resolution
            true
        }

        // Unknown types are always compatible
        (CwtTypeOrSpecialRef::Unknown, _) => true,

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
        (AstValue::String(_), SimpleType::ValueField) => true,
        (AstValue::String(_), SimpleType::Bool) => true,

        // Variables (strings starting with @) are compatible with number types
        (AstValue::String(s), SimpleType::Int) if s.raw_value().starts_with('@') => true,
        (AstValue::String(s), SimpleType::Float) if s.raw_value().starts_with('@') => true,
        (AstValue::String(s), SimpleType::PercentageField) if s.raw_value().starts_with('@') => {
            true
        }

        // Number-based types
        (AstValue::Number(_), SimpleType::ValueField) => true,
        (AstValue::Number(_), SimpleType::Int) => true,
        (AstValue::Number(_), SimpleType::Float) => true,
        (AstValue::Number(_), SimpleType::PercentageField) => true,
        (AstValue::Number(_), SimpleType::IntValueField) => true,

        // Specialized types
        (AstValue::Color(_), SimpleType::Color) => true,
        (AstValue::Maths(_), SimpleType::Maths) => true,

        // Everything else is incompatible
        _ => false,
    }
}
