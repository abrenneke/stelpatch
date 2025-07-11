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
        key::{get_property_type_from_expected_type, is_key_valid},
        structural::is_value_structurally_compatible,
        util::get_type_name,
        value::is_value_compatible_with_simple_type_with_scope,
    },
    scope::{ScopeContext, ScopeContextManager},
    scoped_type::{CwtTypeOrSpecial, PropertyNavigationResult, ScopedType},
};

/// Validate an entity value against the expected type structure
pub fn validate_entity_value(
    value: &AstValue<'_>,
    expected_type: &ScopedType,
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

    match value {
        AstValue::Entity(entity) => {
            // Validate each property in the entity
            for item in &entity.items {
                if let AstEntityItem::Expression(expr) = item {
                    let key_name = expr.key.raw_value();

                    eprintln!("DEBUG: Validating key '{}'", key_name);

                    // Check if this key is valid for the expected type
                    if !is_key_valid(expected_type, key_name) {
                        let diagnostic = create_unexpected_key_diagnostic(
                            expr.key.span_range(),
                            key_name,
                            namespace,
                            content,
                        );
                        diagnostics.push(diagnostic);
                    } else {
                        // Get the expected type for this key
                        let property_type =
                            get_property_type_from_expected_type(expected_type, key_name);

                        // Validate the value against the property type
                        let value_diagnostics = validate_value_against_type(
                            &expr.value,
                            &property_type,
                            content,
                            namespace,
                            depth + 1,
                        );
                        diagnostics.extend(value_diagnostics);
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

// === NEW SCOPED TYPE VALIDATION FUNCTIONS ===

/// Validate a value against a scoped type - this is the new recommended approach
pub fn validate_value_against_scoped_type(
    value: &AstValue<'_>,
    scoped_type: &ScopedType,
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

    match (scoped_type.cwt_type(), value) {
        // Block type validation with scope-aware property navigation
        (CwtTypeOrSpecial::CwtType(CwtType::Block(_)), AstValue::Entity(entity)) => {
            for item in &entity.items {
                if let AstEntityItem::Expression(expr) = item {
                    let key_name = expr.key.raw_value();

                    eprintln!("DEBUG: Validating scoped key '{}'", key_name);

                    // Use the resolver to navigate to the property with proper scope handling
                    match cache
                        .get_resolver()
                        .navigate_to_property(scoped_type, key_name)
                    {
                        PropertyNavigationResult::Success(property_scoped_type) => {
                            // Recursively validate the value with the new scoped type
                            let value_diagnostics = validate_value_against_scoped_type(
                                &expr.value,
                                &property_scoped_type,
                                content,
                                namespace,
                                depth + 1,
                            );
                            diagnostics.extend(value_diagnostics);
                        }
                        PropertyNavigationResult::ScopeError(error) => {
                            let diagnostic = create_type_mismatch_diagnostic(
                                expr.key.span_range(),
                                &format!("Scope error for property '{}': {}", key_name, error),
                                content,
                            );
                            diagnostics.push(diagnostic);
                        }
                        PropertyNavigationResult::NotFound => {
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
        }
        (CwtTypeOrSpecial::CwtType(CwtType::Block(_)), _) => {
            let diagnostic = create_type_mismatch_diagnostic(
                value.span_range(),
                "Expected a block/entity",
                content,
            );
            diagnostics.push(diagnostic);
        }

        // Simple type validation with proper scope context
        (CwtTypeOrSpecial::CwtType(CwtType::Simple(simple_type)), _) => {
            if let Some(diagnostic) = is_value_compatible_with_simple_type_with_scope(
                value,
                simple_type,
                content,
                scoped_type.scope_context(),
            ) {
                diagnostics.push(diagnostic);
            }
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
                        "Expected one of {:?} but got '{}'",
                        valid_list,
                        string_value.raw_value()
                    ),
                    content,
                );
                diagnostics.push(diagnostic);
            }
        }
        (CwtTypeOrSpecial::CwtType(CwtType::LiteralSet(_)), _) => {
            let diagnostic = create_type_mismatch_diagnostic(
                value.span_range(),
                "Expected a string value",
                content,
            );
            diagnostics.push(diagnostic);
        }

        // Array type validation
        (CwtTypeOrSpecial::CwtType(CwtType::Array(array_type)), AstValue::Entity(_entity)) => {
            // TODO: Implement proper array validation with scoped types
            let _element_type = &array_type.element_type;
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
            let mut compatible_type = None;

            for union_type in types {
                if is_value_structurally_compatible(
                    value,
                    &ScopedType::new_cwt(union_type.clone(), scoped_type.scope_context().clone()),
                ) {
                    compatible_type = Some(union_type.clone());
                    break;
                }
            }

            if let Some(matching_type) = compatible_type {
                // Create a new scoped type with the matching union member
                let matching_scoped_type =
                    ScopedType::new_cwt(matching_type, scoped_type.scope_context().clone());
                let content_diagnostics = validate_value_against_scoped_type(
                    value,
                    &matching_scoped_type,
                    content,
                    namespace,
                    depth + 1,
                );
                diagnostics.extend(content_diagnostics);
            } else {
                let type_names: Vec<String> = types.iter().map(|t| get_type_name(t)).collect();
                let diagnostic = create_type_mismatch_diagnostic(
                    value.span_range(),
                    &format!(
                        "Value is not compatible with any of the expected types: {}",
                        type_names.join(", ")
                    ),
                    content,
                );
                diagnostics.push(diagnostic);
            }
        }

        // Comparable type validation
        (CwtTypeOrSpecial::CwtType(CwtType::Comparable(base_type)), _) => {
            let base_scoped_type =
                ScopedType::new_cwt((**base_type).clone(), scoped_type.scope_context().clone());
            let base_diagnostics = validate_value_against_scoped_type(
                value,
                &base_scoped_type,
                content,
                namespace,
                depth + 1,
            );
            diagnostics.extend(base_diagnostics);
        }

        // Reference and Unknown types
        (CwtTypeOrSpecial::CwtType(CwtType::Reference(_)), _)
        | (CwtTypeOrSpecial::CwtType(CwtType::Unknown), _) => {
            // Skip validation for these types for now
        }

        (CwtTypeOrSpecial::ScopedUnion(_), _) => {
            todo!()
        }
    }

    diagnostics
}

/// Validate a value against the expected CWT type
fn validate_value_against_type(
    value: &AstValue<'_>,
    expected_type: &ScopedType,
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
    let resolved_type = cache.resolve_type(expected_type);

    match (&resolved_type.cwt_type(), value) {
        // Block type validation
        (CwtTypeOrSpecial::CwtType(CwtType::Block(_)), AstValue::Entity(_)) => {
            // For block types, validate the entity structure recursively
            let entity_diagnostics =
                validate_entity_value(value, &resolved_type, content, namespace, depth);
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
                        "Expected one of {:?} but got '{}'",
                        valid_list,
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
            let scope_manager = ScopeContextManager::default_with_root("unknown");
            if let Some(diagnostic) = is_value_compatible_with_simple_type_with_scope(
                value,
                simple_type,
                content,
                &scope_manager,
            ) {
                diagnostics.push(diagnostic);
            }
        }

        // Array type validation
        (CwtTypeOrSpecial::CwtType(CwtType::Array(array_type)), AstValue::Entity(entity)) => {
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
            // Check if the value is structurally compatible with any of the union members
            let mut compatible_type = None;

            for union_type in types {
                if is_value_structurally_compatible(
                    value,
                    &ScopedType::new_cwt(union_type.clone(), expected_type.scope_context().clone()),
                ) {
                    compatible_type = Some(union_type.clone());
                    break;
                }
            }

            if let Some(matching_type) = compatible_type {
                // Value is structurally compatible with this union member,
                // now validate the content according to this type
                let content_diagnostics = validate_value_against_type(
                    value,
                    &ScopedType::new_cwt(matching_type, expected_type.scope_context().clone()),
                    content,
                    namespace,
                    depth + 1,
                );
                diagnostics.extend(content_diagnostics);
            } else {
                // Value is not structurally compatible with any union member
                let type_names: Vec<String> = types.iter().map(|t| get_type_name(t)).collect();

                let diagnostic = create_type_mismatch_diagnostic(
                    value.span_range(),
                    &format!(
                        "Value is not compatible with any of the expected types: {}",
                        type_names.join(", ")
                    ),
                    content,
                );
                diagnostics.push(diagnostic);
            }
        }

        // Comparable type validation
        (CwtTypeOrSpecial::CwtType(CwtType::Comparable(base_type)), _) => {
            // For comparable types, validate against the base type
            let base_diagnostics = validate_value_against_type(
                value,
                &ScopedType::new_cwt(*base_type.clone(), expected_type.scope_context().clone()),
                content,
                namespace,
                depth + 1,
            );
            diagnostics.extend(base_diagnostics);
        }

        // Reference type validation
        (CwtTypeOrSpecial::CwtType(CwtType::Reference(ref_type)), _) => {
            // For reference types, we need to resolve them through the cache
            // For now, we'll skip validation of reference types as they require complex resolution
            eprintln!(
                "DEBUG: Skipping validation of reference type {:?}",
                ref_type
            );
        }

        // Unknown type - don't validate
        (CwtTypeOrSpecial::CwtType(CwtType::Unknown), _) => {
            // Don't validate unknown types
        }

        (CwtTypeOrSpecial::ScopedUnion(_), _) => todo!(),
    }

    diagnostics
}
