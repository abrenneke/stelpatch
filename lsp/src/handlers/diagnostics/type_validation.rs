use std::sync::Arc;

use std::collections::HashSet;

use cw_model::{CwtType, ReferenceType};
use cw_parser::{AstEntityItem, AstNode, AstValue};
use tower_lsp::lsp_types::Diagnostic;

use crate::handlers::utils::contains_scripted_argument;
use crate::handlers::{
    cache::TypeCache,
    diagnostics::{
        diagnostic::{
            create_type_mismatch_diagnostic, create_unexpected_key_diagnostic,
            create_value_mismatch_diagnostic,
        },
        scope_validation::{validate_scope_reference, validate_scopegroup_reference},
        structural::is_value_structurally_compatible,
        value::is_value_compatible_with_simple_type,
    },
    scope::ScopeStack,
    scoped_type::{CwtTypeOrSpecial, CwtTypeOrSpecialRef, PropertyNavigationResult, ScopedType},
};

/// Extract possible string values from a CwtType for error messages
fn extract_possible_values(cwt_type: &CwtType) -> Vec<String> {
    match cwt_type {
        CwtType::Literal(value) => vec![format!("\"{}\"", value)],
        CwtType::LiteralSet(values) => {
            let mut sorted_values: Vec<_> = values.iter().map(|v| format!("\"{}\"", v)).collect();
            sorted_values.sort();
            sorted_values
        }
        CwtType::Simple(simple_type) => vec![format!("<{:?}>", simple_type)],
        CwtType::Block(_) => vec!["<block>".to_string()],
        CwtType::Array(_) => vec!["<array>".to_string()],
        CwtType::Union(types) => {
            let mut all_values = Vec::new();
            for union_type in types {
                all_values.extend(extract_possible_values(union_type));
            }
            all_values
        }
        CwtType::Comparable(base_type) => extract_possible_values(base_type),
        CwtType::Reference(_) => vec!["<reference>".to_string()],
        CwtType::Unknown => vec!["<unknown>".to_string()],
        CwtType::Any => vec!["<any>".to_string()],
    }
}

/// Validate an entity value against the expected type structure
pub fn validate_entity_value(
    value: &AstValue<'_>,
    expected_type: Arc<ScopedType>,
    content: &str,
    namespace: &str,
    depth: usize,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Prevent infinite recursion
    if depth > 20 {
        eprintln!("DEBUG: Max recursion depth reached at depth {}", depth);
        return diagnostics;
    }

    if !TypeCache::is_initialized() {
        return diagnostics;
    }
    let cache = TypeCache::get().unwrap();

    match value {
        AstValue::Entity(entity) => {
            // Validate each property in the entity
            for item in &entity.items {
                if let AstEntityItem::Expression(expr) = item {
                    let key_name = expr.key.raw_value();

                    if let PropertyNavigationResult::Success(property_type) = cache
                        .get_resolver()
                        .navigate_to_property(expected_type.clone(), key_name)
                    {
                        // Validate the value against the property type
                        let value_diagnostics = validate_value_against_type(
                            &expr.value,
                            property_type,
                            content,
                            namespace,
                            depth + 1,
                        );
                        diagnostics.extend(value_diagnostics);
                    } else {
                        let diagnostic = create_unexpected_key_diagnostic(
                            expr.key.span_range(),
                            key_name,
                            &expected_type.type_name_for_display(),
                            content,
                        );
                        diagnostics.push(diagnostic);
                    }
                }
            }
        }
        _ => {
            // For non-entity values, validate the value directly against the expected type
            let value_diagnostics =
                validate_value_against_type(value, expected_type, content, namespace, depth + 1);
            diagnostics.extend(value_diagnostics);
        }
    }

    diagnostics
}

/// Validate a value against the expected CWT type
fn validate_value_against_type(
    value: &AstValue<'_>,
    expected_type: Arc<ScopedType>,
    content: &str,
    namespace: &str,
    depth: usize,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Prevent infinite recursion
    if depth > 20 {
        eprintln!("DEBUG: Max recursion depth reached at depth {}", depth);
        return diagnostics;
    }

    if !TypeCache::is_initialized() {
        return diagnostics;
    }

    let cache = TypeCache::get().unwrap();
    let resolved_type = cache.resolve_type(expected_type.clone());

    match (&resolved_type.cwt_type_for_matching(), value) {
        // Block type validation
        (CwtTypeOrSpecialRef::Block(_), AstValue::Entity(_)) => {
            // For block types, validate the entity structure recursively
            let entity_diagnostics =
                validate_entity_value(value, resolved_type, content, namespace, depth);
            diagnostics.extend(entity_diagnostics);
        }
        (CwtTypeOrSpecialRef::Block(_), _) => {
            // Expected a block but got something else
            let diagnostic = create_type_mismatch_diagnostic(
                value.span_range(),
                "Expected a block/entity",
                content,
            );
            diagnostics.push(diagnostic);
        }

        // Literal value validation
        (CwtTypeOrSpecialRef::Literal(literal_value), AstValue::String(string_value)) => {
            if string_value.raw_value() != *literal_value {
                let diagnostic = create_value_mismatch_diagnostic(
                    value.span_range(),
                    &format!(
                        "Expected '{}' but got '{}'",
                        literal_value,
                        string_value.raw_value()
                    ),
                    content,
                );
                diagnostics.push(diagnostic);
            }
        }
        (CwtTypeOrSpecialRef::Literal(literal_value), _) => {
            // Expected a literal string but got something else
            let diagnostic = create_type_mismatch_diagnostic(
                value.span_range(),
                &format!("Expected string literal '{}'", literal_value),
                content,
            );
            diagnostics.push(diagnostic);
        }

        // Literal set validation
        (CwtTypeOrSpecialRef::LiteralSet(valid_values), AstValue::String(string_value)) => {
            // Allow $ARGUMENT$ to be used as a value
            if !contains_scripted_argument(string_value.raw_value()) {
                if !valid_values.contains(string_value.raw_value()) {
                    let valid_list: Vec<_> = valid_values.iter().collect();
                    let diagnostic = create_value_mismatch_diagnostic(
                        value.span_range(),
                        &format!(
                            "Expected one of {} but got '{}'",
                            valid_list
                                .iter()
                                .map(|v| format!("\"{}\"", v))
                                .collect::<Vec<_>>()
                                .join(", "),
                            string_value.raw_value()
                        ),
                        content,
                    );
                    diagnostics.push(diagnostic);
                }
            }
        }

        (CwtTypeOrSpecialRef::LiteralSet(set), AstValue::Number(num)) => {
            // A number is valid if when converted to a string, it is in the set
            let number_str = num.value.value;
            if !set.iter().any(|s| s == &number_str) {
                let diagnostic = create_value_mismatch_diagnostic(
                    value.span_range(),
                    &format!(
                        "Expected one of {} but got '{}'",
                        set.iter()
                            .map(|s| format!("\"{}\"", s))
                            .collect::<Vec<_>>()
                            .join(", "),
                        number_str
                    ),
                    content,
                );
                diagnostics.push(diagnostic);
            }
        }

        (CwtTypeOrSpecialRef::LiteralSet(_), _) => {
            // Expected a string from literal set but got something else
            let diagnostic = create_type_mismatch_diagnostic(
                value.span_range(),
                "Expected a string value",
                content,
            );
            diagnostics.push(diagnostic);
        }

        // Simple type validation
        (CwtTypeOrSpecialRef::Simple(simple_type), _) => {
            // Create a default scope for backward compatibility
            let scope_manager = ScopeStack::default_with_root("unknown");
            if let Some(diagnostic) = is_value_compatible_with_simple_type(
                value,
                simple_type,
                content,
                &scope_manager,
                Some(namespace),
            ) {
                diagnostics.push(diagnostic);
            }
        }

        // Array type validation
        (CwtTypeOrSpecialRef::Array(array_type), AstValue::Entity(_entity)) => {
            // Arrays in CW are represented as entities with numbered keys
            // For now, we'll just validate that it's an entity - more complex validation would require
            // checking that all keys are valid indices and values match the element type
            let _element_type = &array_type.element_type;
            // TODO: Implement array element validation
        }
        (CwtTypeOrSpecialRef::Array(_), _) => {
            let diagnostic = create_type_mismatch_diagnostic(
                value.span_range(),
                "Expected an array (entity with indexed elements)",
                content,
            );
            diagnostics.push(diagnostic);
        }

        // Union type validation
        (CwtTypeOrSpecialRef::Union(types), _) => {
            // Find all structurally compatible union members
            let mut compatible_resolved_types = Vec::new();

            for union_type in *types {
                // Resolve the union type first in case it's a reference
                let resolved_union_type = cache.resolve_type(Arc::new(ScopedType::new_cwt(
                    union_type.clone(),
                    expected_type.scope_stack().clone(),
                    expected_type.in_scripted_effect_block().cloned(),
                )));

                if is_value_structurally_compatible(value, resolved_union_type.clone()) {
                    compatible_resolved_types.push(resolved_union_type);
                }
            }

            if compatible_resolved_types.is_empty() {
                // Value is not structurally compatible with any union member
                let mut all_possible_values = Vec::new();
                for union_type in *types {
                    // Resolve each union type to get its actual structure for error messages
                    let resolved_union_type = cache.resolve_type(Arc::new(ScopedType::new_cwt(
                        union_type.clone(),
                        expected_type.scope_stack().clone(),
                        expected_type.in_scripted_effect_block().cloned(),
                    )));

                    if let CwtTypeOrSpecial::CwtType(cwt_type) = resolved_union_type.cwt_type() {
                        all_possible_values.extend(extract_possible_values(cwt_type));
                    }
                }

                // Remove duplicates and sort
                let mut unique_values: Vec<_> = all_possible_values
                    .into_iter()
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect();
                unique_values.sort();

                let diagnostic = create_type_mismatch_diagnostic(
                    value.span_range(),
                    &format!(
                        "Expected one of: {}, found: {:?}",
                        unique_values.join(", "),
                        value.type_name(),
                    ),
                    content,
                );
                diagnostics.push(diagnostic);
            } else {
                // Value is structurally compatible with at least one union member
                // Validate against each compatible type and use "any" logic:
                // only report errors if ALL compatible types have validation errors
                let mut all_validation_results = Vec::new();

                for compatible_resolved_type in &compatible_resolved_types {
                    let content_diagnostics = validate_value_against_type(
                        value,
                        compatible_resolved_type.clone(),
                        content,
                        namespace,
                        depth + 1,
                    );
                    all_validation_results.push(content_diagnostics);
                }

                // If ANY compatible type validates without errors, the union validation passes
                let any_validation_passed =
                    all_validation_results.iter().any(|diags| diags.is_empty());

                if !any_validation_passed {
                    // All compatible types have validation errors - report the inner errors
                    // from the first compatible type (they should be similar anyway)
                    if let Some(first_errors) = all_validation_results.first() {
                        diagnostics.extend(first_errors.iter().cloned());
                    }
                }
            }
        }

        // Comparable type validation
        (CwtTypeOrSpecialRef::Comparable(base_type), _) => {
            // For comparable types, validate against the base type
            let base_diagnostics = validate_value_against_type(
                value,
                Arc::new(ScopedType::new_cwt(
                    (***base_type).clone(),
                    expected_type.scope_stack().clone(),
                    expected_type.in_scripted_effect_block().cloned(),
                )),
                content,
                namespace,
                depth + 1,
            );
            diagnostics.extend(base_diagnostics);
        }

        (CwtTypeOrSpecialRef::Reference(ReferenceType::ValueSet { .. }), AstValue::String(_)) => {
            // Any string is allowed for value_set
        }

        (
            CwtTypeOrSpecialRef::Reference(ReferenceType::Colour { format }),
            AstValue::Color(color),
        ) => {
            if color.color_type.value != format {
                let diagnostic = create_type_mismatch_diagnostic(
                    value.span_range(),
                    &format!(
                        "Expected color format '{}' but got '{}'",
                        format, color.color_type.value
                    ),
                    content,
                );
                diagnostics.push(diagnostic);
            }
        }

        // Reference type validation
        (CwtTypeOrSpecialRef::Reference(ref_type), _) => match ref_type {
            ReferenceType::Scope { key } => {
                if let AstValue::String(string_value) = value {
                    if let Some(diagnostic) = validate_scope_reference(
                        string_value.raw_value(),
                        key,
                        expected_type.scope_stack(),
                        value.span_range(),
                        content,
                    ) {
                        diagnostics.push(diagnostic);
                    }
                } else {
                    let diagnostic = create_type_mismatch_diagnostic(
                        value.span_range(),
                        "Expected a string value for scope reference",
                        content,
                    );
                    diagnostics.push(diagnostic);
                }
            }
            ReferenceType::ScopeGroup { key } => {
                if let AstValue::String(string_value) = value {
                    if let Some(diagnostic) = validate_scopegroup_reference(
                        string_value.raw_value(),
                        key,
                        expected_type.scope_stack(),
                        value.span_range(),
                        content,
                    ) {
                        diagnostics.push(diagnostic);
                    }
                } else {
                    let diagnostic = create_type_mismatch_diagnostic(
                        value.span_range(),
                        "Expected a string value for scope group reference",
                        content,
                    );
                    diagnostics.push(diagnostic);
                }
            }
            _ => {
                let diagnostic = create_type_mismatch_diagnostic(
                    value.span_range(),
                    &format!(
                        "Reference type validation not implemented yet, found: {:?}",
                        ref_type
                    ),
                    content,
                );
                diagnostics.push(diagnostic);
            }
        },

        (CwtTypeOrSpecialRef::Any, _) => {
            // Any type is valid for anything
        }

        // Unknown type - don't validate
        (CwtTypeOrSpecialRef::Unknown, _) => {
            // Don't validate unknown types
        }

        (CwtTypeOrSpecialRef::ScopedUnion(scoped_types), _) => {
            // Find all structurally compatible union members
            let mut compatible_types = Vec::new();

            for scoped_type in *scoped_types {
                let scoped_type_arc = scoped_type.clone();
                if is_value_structurally_compatible(value, scoped_type_arc.clone()) {
                    compatible_types.push(scoped_type_arc);
                }
            }

            if compatible_types.is_empty() {
                // Value is not structurally compatible with any union member
                let mut all_possible_values = Vec::new();
                for scoped_type in *scoped_types {
                    if let CwtTypeOrSpecial::CwtType(cwt_type) = scoped_type.cwt_type() {
                        all_possible_values.extend(extract_possible_values(cwt_type));
                    }
                }

                // Remove duplicates and sort
                let mut unique_values: Vec<_> = all_possible_values
                    .into_iter()
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect();
                unique_values.sort();

                let diagnostic = create_type_mismatch_diagnostic(
                    value.span_range(),
                    &format!(
                        "Expected one of: {}, found: {:?}",
                        unique_values.join(", "),
                        value.type_name(),
                    ),
                    content,
                );
                diagnostics.push(diagnostic);
            } else {
                // Value is structurally compatible with at least one union member
                // Validate against each compatible type and use "any" logic:
                // only report errors if ALL compatible types have validation errors
                let mut all_validation_results = Vec::new();

                for compatible_type in &compatible_types {
                    let content_diagnostics = validate_value_against_type(
                        value,
                        compatible_type.clone(),
                        content,
                        namespace,
                        depth + 1,
                    );
                    all_validation_results.push(content_diagnostics);
                }

                // If ANY compatible type validates without errors, the union validation passes
                let any_validation_passed =
                    all_validation_results.iter().any(|diags| diags.is_empty());

                if !any_validation_passed {
                    // All compatible types have validation errors - report the inner errors
                    // from the first compatible type (they should be similar anyway)
                    if let Some(first_errors) = all_validation_results.first() {
                        diagnostics.extend(first_errors.iter().cloned());
                    }
                }
            }
        }
    }

    diagnostics
}
