use std::sync::Arc;

use std::collections::HashSet;

use cw_model::{CwtType, ReferenceType};
use cw_parser::{AstEntityItem, AstNode, AstValue};
use tower_lsp::lsp_types::Diagnostic;

use crate::handlers::utils::contains_scripted_argument;
use crate::handlers::{
    cache::{FileIndex, TypeCache},
    diagnostics::{
        diagnostic::{
            create_type_mismatch_diagnostic, create_unexpected_key_diagnostic,
            create_value_mismatch_diagnostic,
        },
        scope_validation::{validate_scope_reference, validate_scopegroup_reference},
        structural::calculate_structural_compatibility_score,
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

/// Helper function to validate a value against multiple union types with structural scoring
fn validate_union_types(
    value: &AstValue<'_>,
    union_types: Vec<Arc<ScopedType>>,
    content: &str,
    namespace: &str,
    depth: usize,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Step 1: Validate against ALL union types and calculate structural scores
    let mut all_validation_results = Vec::new();
    let mut type_scores = Vec::new();

    for union_type in union_types {
        let validation_result =
            validate_value_against_type(value, union_type.clone(), content, namespace, depth + 1);
        let structural_score = calculate_structural_compatibility_score(value, union_type.clone());

        all_validation_results.push(validation_result);
        type_scores.push((union_type, structural_score));
    }

    // Step 2: Check if ANY union type validates successfully (0 diagnostics)
    let any_validation_passed = all_validation_results.iter().any(|diags| diags.is_empty());

    if any_validation_passed {
        // If any type validates successfully, the union passes - report no errors
        return diagnostics; // Empty diagnostics = success
    }

    // Step 3: All types have errors - find the highest structural score
    let max_score = type_scores
        .iter()
        .map(|(_, score)| *score)
        .fold(0.0, f64::max);

    if max_score > 0.0 {
        // Step 4: Report errors only from the most structurally compatible types
        let best_indices: Vec<usize> = type_scores
            .iter()
            .enumerate()
            .filter(|(_, (_, score))| (*score - max_score).abs() < f64::EPSILON)
            .map(|(index, _)| index)
            .collect();

        for index in best_indices {
            diagnostics.extend(all_validation_results[index].iter().cloned());
        }
    } else {
        // No structural compatibility at all - provide general error
        let mut all_possible_values = Vec::new();
        for (union_type, _) in type_scores {
            if let CwtTypeOrSpecial::CwtType(cwt_type) = union_type.cwt_type() {
                all_possible_values.extend(extract_possible_values(cwt_type));
            }
        }

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
            if string_value.raw_value().to_lowercase() != *literal_value.to_lowercase() {
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

        (CwtTypeOrSpecialRef::Literal(literal_value), AstValue::Number(number_value)) => {
            if number_value.value.value != *literal_value {
                let diagnostic = create_value_mismatch_diagnostic(
                    value.span_range(),
                    &format!(
                        "Expected '{}' but got '{}'",
                        literal_value, number_value.value.value
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
                if !valid_values.contains(&string_value.raw_value().to_lowercase()) {
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
            // Resolve all union types first
            let resolved_union_types: Vec<Arc<ScopedType>> = types
                .iter()
                .map(|union_type| {
                    cache.resolve_type(Arc::new(ScopedType::new_cwt(
                        union_type.clone(),
                        expected_type.scope_stack().clone(),
                        expected_type.in_scripted_effect_block().cloned(),
                    )))
                })
                .collect();

            let union_diagnostics =
                validate_union_types(value, resolved_union_types, content, namespace, depth);
            diagnostics.extend(union_diagnostics);
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
            ReferenceType::Icon { path } => {
                if let AstValue::String(string_value) = value {
                    if let Some(file_index) = FileIndex::get() {
                        // Construct the full path for the icon file (append .dds extension)
                        let icon_filename = string_value.raw_value();
                        let icon_filename_with_ext = format!("{}.dds", icon_filename);
                        let full_path = if path.is_empty() {
                            icon_filename_with_ext
                        } else {
                            format!("{}/{}", path.trim_end_matches('/'), icon_filename_with_ext)
                        };

                        if !file_index.file_exists(&full_path) {
                            let diagnostic = create_type_mismatch_diagnostic(
                                value.span_range(),
                                &format!(
                                    "Icon file '{}' does not exist in path '{}'",
                                    icon_filename, path
                                ),
                                content,
                            );
                            diagnostics.push(diagnostic);
                        }
                    } else {
                        let diagnostic = create_type_mismatch_diagnostic(
                            value.span_range(),
                            "File index not initialized, cannot validate icon path",
                            content,
                        );
                        diagnostics.push(diagnostic);
                    }
                } else {
                    let diagnostic = create_type_mismatch_diagnostic(
                        value.span_range(),
                        "Expected a string value for icon reference",
                        content,
                    );
                    diagnostics.push(diagnostic);
                }
            }
            ReferenceType::Filepath { path } => {
                if let AstValue::String(string_value) = value {
                    if let Some(file_index) = FileIndex::get() {
                        // Split the path by comma to get prefix and suffix
                        let parts: Vec<&str> = path.split(',').collect();
                        if parts.len() == 2 {
                            let prefix = parts[0];
                            let suffix = parts[1];
                            let filename = string_value.raw_value();
                            let full_path = format!("{}{}{}", prefix, filename, suffix);

                            if !file_index.file_exists(&full_path) {
                                let diagnostic = create_type_mismatch_diagnostic(
                                    value.span_range(),
                                    &format!(
                                        "File '{}' does not exist (expected at '{}')",
                                        filename, full_path
                                    ),
                                    content,
                                );
                                diagnostics.push(diagnostic);
                            }
                        } else {
                            let diagnostic = create_type_mismatch_diagnostic(
                                value.span_range(),
                                &format!(
                                    "Invalid filepath pattern '{}' - expected format 'prefix,suffix'",
                                    path
                                ),
                                content,
                            );
                            diagnostics.push(diagnostic);
                        }
                    } else {
                        let diagnostic = create_type_mismatch_diagnostic(
                            value.span_range(),
                            "File index not initialized, cannot validate filepath",
                            content,
                        );
                        diagnostics.push(diagnostic);
                    }
                } else {
                    let diagnostic = create_type_mismatch_diagnostic(
                        value.span_range(),
                        "Expected a string value for filepath reference",
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
            // ScopedUnion types are already resolved, so we can use them directly
            let union_types: Vec<Arc<ScopedType>> = scoped_types.iter().cloned().collect();

            let union_diagnostics =
                validate_union_types(value, union_types, content, namespace, depth);
            diagnostics.extend(union_diagnostics);
        }
    }

    diagnostics
}
