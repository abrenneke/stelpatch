use std::sync::Arc;

use std::collections::{HashMap, HashSet};

use cw_model::CwtType;
use cw_parser::{AstEntityItem, AstNode, AstValue};
use tower_lsp::lsp_types::Diagnostic;

use crate::handlers::{
    cache::TypeCache,
    diagnostics::{
        diagnostic::{
            create_type_mismatch_diagnostic, create_unexpected_key_diagnostic,
            create_value_mismatch_diagnostic,
        },
        structural::is_value_structurally_compatible,
        value::is_value_compatible_with_simple_type,
    },
    scope::ScopeStack,
    scoped_type::{CwtTypeOrSpecial, PropertyNavigationResult, ScopedType},
};

/// Extract property data from an AST entity for subtype condition checking
fn extract_property_data_from_entity(entity: &cw_parser::AstEntity<'_>) -> HashMap<String, String> {
    let mut property_data = HashMap::new();

    for item in &entity.items {
        if let AstEntityItem::Expression(expr) = item {
            let key_name = expr.key.raw_value();

            // Extract simple string values for condition matching
            match &expr.value {
                AstValue::String(string_val) => {
                    property_data.insert(key_name.to_string(), string_val.raw_value().to_string());
                }
                AstValue::Number(num_val) => {
                    property_data.insert(key_name.to_string(), num_val.value.value.to_string());
                }
                AstValue::Entity(_) => {
                    // For entities, just mark that the property exists
                    property_data.insert(key_name.to_string(), "{}".to_string());
                }
                AstValue::Color(_) => {
                    // For colors, just mark that the property exists
                    property_data.insert(key_name.to_string(), "color".to_string());
                }
                AstValue::Maths(_) => {
                    // For math expressions, just mark that the property exists
                    property_data.insert(key_name.to_string(), "math".to_string());
                }
            }
        }
    }

    property_data
}

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
    if depth > 10 {
        eprintln!("DEBUG: Max recursion depth reached at depth {}", depth);
        return diagnostics;
    }

    if !TypeCache::is_initialized() {
        return diagnostics;
    }
    let cache = TypeCache::get().unwrap();

    match value {
        AstValue::Entity(entity) => {
            // Check if we need to determine a subtype for this entity
            let actual_expected_type =
                if let CwtTypeOrSpecial::CwtType(CwtType::Block(block_type)) =
                    expected_type.cwt_type()
                {
                    if !block_type.subtypes.is_empty() {
                        // Extract property data from the entity
                        let property_data = extract_property_data_from_entity(entity);

                        // Try to determine the matching subtypes
                        let detected_subtypes = cache
                            .get_resolver()
                            .determine_matching_subtypes(expected_type.clone(), &property_data);

                        if !detected_subtypes.is_empty() {
                            // Create a new scoped type with the detected subtypes
                            Arc::new(expected_type.with_subtypes(detected_subtypes))
                        } else {
                            expected_type
                        }
                    } else {
                        expected_type
                    }
                } else {
                    expected_type
                };

            // Validate each property in the entity
            for item in &entity.items {
                if let AstEntityItem::Expression(expr) = item {
                    let key_name = expr.key.raw_value();

                    if let PropertyNavigationResult::Success(property_type) = cache
                        .get_resolver()
                        .navigate_to_property(actual_expected_type.clone(), key_name)
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
                            namespace,
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
    if depth > 10 {
        eprintln!("DEBUG: Max recursion depth reached at depth {}", depth);
        return diagnostics;
    }

    if !TypeCache::is_initialized() {
        return diagnostics;
    }

    let cache = TypeCache::get().unwrap();
    let resolved_type = cache.resolve_type(expected_type.clone());

    match (&resolved_type.cwt_type(), value) {
        // Block type validation
        (CwtTypeOrSpecial::CwtType(CwtType::Block(_)), AstValue::Entity(_)) => {
            // For block types, validate the entity structure recursively
            let entity_diagnostics =
                validate_entity_value(value, resolved_type, content, namespace, depth);
            diagnostics.extend(entity_diagnostics);
        }
        (CwtTypeOrSpecial::CwtType(CwtType::Block(_)), _) => {
            // Expected a block but got something else
            let diagnostic = create_type_mismatch_diagnostic(
                value.span_range(),
                "Expected a block/entity",
                content,
            );
            diagnostics.push(diagnostic);
        }

        // Literal value validation
        (
            CwtTypeOrSpecial::CwtType(CwtType::Literal(literal_value)),
            AstValue::String(string_value),
        ) => {
            if string_value.raw_value() != literal_value {
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
        (CwtTypeOrSpecial::CwtType(CwtType::Literal(literal_value)), _) => {
            // Expected a literal string but got something else
            let diagnostic = create_type_mismatch_diagnostic(
                value.span_range(),
                &format!("Expected string literal '{}'", literal_value),
                content,
            );
            diagnostics.push(diagnostic);
        }

        // Literal set validation
        (
            CwtTypeOrSpecial::CwtType(CwtType::LiteralSet(valid_values)),
            AstValue::String(string_value),
        ) => {
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
        (CwtTypeOrSpecial::CwtType(CwtType::LiteralSet(_)), _) => {
            // Expected a string from literal set but got something else
            let diagnostic = create_type_mismatch_diagnostic(
                value.span_range(),
                "Expected a string value",
                content,
            );
            diagnostics.push(diagnostic);
        }

        // Simple type validation
        (CwtTypeOrSpecial::CwtType(CwtType::Simple(simple_type)), _) => {
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
        (CwtTypeOrSpecial::CwtType(CwtType::Array(array_type)), AstValue::Entity(_entity)) => {
            // Arrays in CW are represented as entities with numbered keys
            // For now, we'll just validate that it's an entity - more complex validation would require
            // checking that all keys are valid indices and values match the element type
            let _element_type = &array_type.element_type;
            // TODO: Implement array element validation
        }
        (CwtTypeOrSpecial::CwtType(CwtType::Array(_)), _) => {
            let diagnostic = create_type_mismatch_diagnostic(
                value.span_range(),
                "Expected an array (entity with indexed elements)",
                content,
            );
            diagnostics.push(diagnostic);
        }

        // Union type validation
        (CwtTypeOrSpecial::CwtType(CwtType::Union(types)), _) => {
            // Find all structurally compatible union members
            let mut compatible_resolved_types = Vec::new();

            for union_type in types {
                // Resolve the union type first in case it's a reference
                let resolved_union_type = cache.resolve_type(Arc::new(ScopedType::new_cwt(
                    union_type.clone(),
                    expected_type.scope_stack().clone(),
                )));

                if is_value_structurally_compatible(value, resolved_union_type.clone()) {
                    compatible_resolved_types.push(resolved_union_type);
                }
            }

            if compatible_resolved_types.is_empty() {
                // Value is not structurally compatible with any union member
                let mut all_possible_values = Vec::new();
                for union_type in types {
                    // Resolve each union type to get its actual structure for error messages
                    let resolved_union_type = cache.resolve_type(Arc::new(ScopedType::new_cwt(
                        union_type.clone(),
                        expected_type.scope_stack().clone(),
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
                    &format!("Expected one of: {}", unique_values.join(", ")),
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
                    // All compatible types have validation errors - create a comprehensive error message
                    // showing all possible union values as a flat list
                    let mut all_possible_values = Vec::new();
                    for compatible_resolved_type in &compatible_resolved_types {
                        if let CwtTypeOrSpecial::CwtType(cwt_type) =
                            compatible_resolved_type.cwt_type()
                        {
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
                        &format!("Expected one of: {}", unique_values.join(", ")),
                        content,
                    );
                    diagnostics.push(diagnostic);
                }
            }
        }

        // Comparable type validation
        (CwtTypeOrSpecial::CwtType(CwtType::Comparable(base_type)), _) => {
            // For comparable types, validate against the base type
            let base_diagnostics = validate_value_against_type(
                value,
                Arc::new(ScopedType::new_cwt(
                    *base_type.clone(),
                    expected_type.scope_stack().clone(),
                )),
                content,
                namespace,
                depth + 1,
            );
            diagnostics.extend(base_diagnostics);
        }

        // Reference type validation
        (CwtTypeOrSpecial::CwtType(CwtType::Reference(_)), _) => {
            let diagnostic = create_type_mismatch_diagnostic(
                value.span_range(),
                "Reference types are not supported yet",
                content,
            );
            diagnostics.push(diagnostic);
        }

        // Unknown type - don't validate
        (CwtTypeOrSpecial::CwtType(CwtType::Unknown), _) => {
            // Don't validate unknown types
        }

        (CwtTypeOrSpecial::ScopedUnion(_), _) => todo!(),
    }

    diagnostics
}
