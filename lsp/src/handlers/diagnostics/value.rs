use cw_model::SimpleType;
use cw_parser::{AstNode, AstValue};
use tower_lsp::lsp_types::Diagnostic;

use crate::handlers::{
    cache::{GameDataCache, TypeCache},
    diagnostics::diagnostic::create_type_mismatch_diagnostic,
    scope::ScopeStack,
    settings::VALIDATE_LOCALISATION,
};

/// Check if a value is compatible with a simple type with scope context, returning a diagnostic if incompatible
pub fn is_value_compatible_with_simple_type(
    value: &AstValue<'_>,
    simple_type: &SimpleType,
    content: &str,
    scope_manager: &ScopeStack,
    current_namespace: Option<&str>,
) -> Option<Diagnostic> {
    match (value, simple_type) {
        (AstValue::String(_), SimpleType::Localisation) => {
            if VALIDATE_LOCALISATION {
                // TODO: Implement proper localisation validation
                Some(create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Localisation validation not yet implemented",
                    content,
                ))
            } else {
                None
            }
        }
        (AstValue::String(_), SimpleType::LocalisationSynced) => {
            if VALIDATE_LOCALISATION {
                // TODO: Implement proper localisation validation
                Some(create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Localisation synced validation not yet implemented",
                    content,
                ))
            } else {
                None
            }
        }
        (AstValue::String(_), SimpleType::LocalisationInline) => {
            if VALIDATE_LOCALISATION {
                // TODO: Implement proper localisation validation
                Some(create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Inline localisation validation not yet implemented",
                    content,
                ))
            } else {
                None
            }
        }
        (AstValue::String(_), SimpleType::Filepath) => {
            // TODO: Implement proper filepath validation
            Some(create_type_mismatch_diagnostic(
                value.span_range(),
                "Filepath validation not yet implemented",
                content,
            ))
        }
        (AstValue::String(_), SimpleType::Icon) => {
            // TODO: Implement proper icon validation
            Some(create_type_mismatch_diagnostic(
                value.span_range(),
                "Icon validation not yet implemented",
                content,
            ))
        }
        (AstValue::String(_), SimpleType::VariableField) => {
            // TODO: Implement proper variable field validation
            Some(create_type_mismatch_diagnostic(
                value.span_range(),
                "Variable field validation not yet implemented",
                content,
            ))
        }
        (AstValue::String(scope_field), SimpleType::ScopeField) => {
            let field_name = scope_field.raw_value();

            // Use the unified function to check both scope fields and link properties
            let type_cache = TypeCache::get().unwrap();
            if let Some(_description) = type_cache
                .get_resolver()
                .is_valid_scope_or_link_property(field_name, scope_manager)
            {
                None // Valid scope field or link property
            } else {
                // Neither scope nor link - provide comprehensive error
                let available_properties = type_cache
                    .get_resolver()
                    .get_available_scope_and_link_properties(scope_manager);

                Some(create_type_mismatch_diagnostic(
                    value.span_range(),
                    &format!(
                        "Invalid scope field or link '{}'. Available options: {}",
                        field_name,
                        available_properties.join(", ")
                    ),
                    content,
                ))
            }
        }
        (AstValue::String(_), SimpleType::DateField) => {
            // TODO: Implement proper date field validation
            Some(create_type_mismatch_diagnostic(
                value.span_range(),
                "Date field validation not yet implemented",
                content,
            ))
        }
        (AstValue::String(_), SimpleType::Scalar) => None, // Valid
        (AstValue::String(_), SimpleType::IntVariableField) => {
            // TODO: Implement proper int variable field validation
            Some(create_type_mismatch_diagnostic(
                value.span_range(),
                "Int variable field validation not yet implemented",
                content,
            ))
        }

        (AstValue::Number(_), SimpleType::ValueField) => None, // Valid
        (AstValue::String(s), SimpleType::ValueField) => {
            let val = s.raw_value();
            if val.starts_with("@") {
                validate_scripted_variable(val, value.span_range(), content, current_namespace)
            } else {
                // TODO: Handle other value references
                Some(create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Value field validation for non-variables not yet implemented",
                    content,
                ))
            }
        }
        (AstValue::Number(n), SimpleType::Int) => {
            if n.value.value.find('.').is_none() {
                None // Valid integer
            } else {
                Some(create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Expected integer but got decimal number",
                    content,
                ))
            }
        }
        (AstValue::Number(_), SimpleType::Float) => None, // Valid
        (AstValue::Number(n), SimpleType::PercentageField) => {
            if n.value.value.ends_with("%") {
                None // Valid percentage
            } else {
                Some(create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Expected percentage value (ending with %)",
                    content,
                ))
            }
        }
        (AstValue::Number(n), SimpleType::IntValueField) => {
            if n.value.value.find('.').is_none() {
                None // Valid integer
            } else {
                Some(create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Expected integer but got decimal number",
                    content,
                ))
            }
        }
        (AstValue::String(s), SimpleType::IntValueField) => {
            let val = s.raw_value();
            if val.starts_with("@") {
                validate_scripted_variable(val, value.span_range(), content, current_namespace)
            } else {
                // TODO: Handle other value references
                Some(create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Int value field validation for non-variables not yet implemented",
                    content,
                ))
            }
        }

        (AstValue::String(s), SimpleType::Bool) => {
            let val = s.raw_value();
            if val == "yes" || val == "no" {
                None // Valid boolean
            } else {
                Some(create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Expected boolean value ('yes' or 'no')",
                    content,
                ))
            }
        }

        (AstValue::Color(_), SimpleType::Color) => None, // Valid
        (AstValue::Maths(_), SimpleType::Maths) => None, // Valid

        (AstValue::Maths(_), SimpleType::Float) => None, // Valid, calculated value
        (AstValue::Maths(_), SimpleType::Int) => None,   // Valid, calculated value

        // Type mismatches
        (_, simple_type) => Some(create_type_mismatch_diagnostic(
            value.span_range(),
            &format!(
                "Expected {} but got {}",
                simple_type.id(),
                get_value_type_name(value)
            ),
            content,
        )),
    }
}

/// Validate a scripted variable reference
fn validate_scripted_variable(
    variable_name: &str,
    span_range: std::ops::Range<usize>,
    content: &str,
    current_namespace: Option<&str>,
) -> Option<Diagnostic> {
    use crate::handlers::cache::EntityRestructurer;

    if let Some(cache) = GameDataCache::get() {
        // Check global scripted variables first
        if cache.scripted_variables.contains_key(variable_name) {
            None // Valid scripted variable
        } else if let Some(namespace_name) = current_namespace {
            // Check only the current namespace's scripted variables using EntityRestructurer
            if let Some(namespace_variables) =
                EntityRestructurer::get_namespace_scripted_variables(namespace_name)
            {
                if namespace_variables.contains_key(variable_name) {
                    None // Valid scripted variable
                } else {
                    Some(create_type_mismatch_diagnostic(
                        span_range,
                        &format!(
                            "Unknown scripted variable '{}' in namespace '{}'",
                            variable_name, namespace_name
                        ),
                        content,
                    ))
                }
            } else {
                Some(create_type_mismatch_diagnostic(
                    span_range,
                    &format!(
                        "Unknown scripted variable '{}' (namespace '{}' not found)",
                        variable_name, namespace_name
                    ),
                    content,
                ))
            }
        } else {
            // No namespace context, assume valid for now
            None
        }
    } else {
        // Game data cache not initialized, assume valid for now
        None
    }
}

/// Get a human-readable name for a value type
pub fn get_value_type_name(value: &AstValue<'_>) -> &'static str {
    match value {
        AstValue::String(_) => "string",
        AstValue::Number(_) => "number",
        AstValue::Entity(_) => "entity/block",
        AstValue::Color(_) => "color",
        AstValue::Maths(_) => "math expression",
    }
}
