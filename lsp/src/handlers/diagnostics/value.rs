use std::ops::Range;

use cw_model::SimpleType;
use cw_parser::{AstNode, AstValue};
use tower_lsp::lsp_types::Diagnostic;

use crate::handlers::{
    cache::{EntityRestructurer, FileIndex, FullAnalysis, GameDataCache, ModDataCache, TypeCache},
    diagnostics::diagnostic::create_type_mismatch_diagnostic,
    scope::ScopeStack,
    settings::VALIDATE_LOCALISATION,
    utils::contains_scripted_argument,
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
        (AstValue::String(string), SimpleType::Filepath) => {
            if let Some(file_index) = FileIndex::get() {
                if file_index.file_exists(string.raw_value()) {
                    None
                } else {
                    Some(create_type_mismatch_diagnostic(
                        value.span_range(),
                        "Filepath does not exist",
                        content,
                    ))
                }
            } else {
                Some(create_type_mismatch_diagnostic(
                    value.span_range(),
                    "File index not initialized, cannot validate filepath",
                    content,
                ))
            }
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
        (AstValue::Number(_), SimpleType::Scalar) => None, // Valid
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
            validate_value_field_string(
                s.raw_value(),
                value.span_range(),
                content,
                current_namespace,
                false, // Don't include "integer" in error message
            )
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
        (AstValue::String(s), SimpleType::Int) => {
            let val = s.raw_value();
            if val.starts_with("@") {
                validate_scripted_variable(val, value.span_range(), content, current_namespace)
            } else if val.starts_with("$") && val.ends_with("$") {
                None // Argument in scripted effect
            } else {
                Some(create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Expected integer value or scripted variable",
                    content,
                ))
            }
        }
        (AstValue::Number(_), SimpleType::Float) => None, // Valid
        (AstValue::String(s), SimpleType::Float) => {
            let val = s.raw_value();
            if val.starts_with("@") {
                validate_scripted_variable(val, value.span_range(), content, current_namespace)
            } else if val.starts_with("$") && val.ends_with("$") {
                None // Argument in scripted effect
            } else {
                Some(create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Expected float value or scripted variable",
                    content,
                ))
            }
        }
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
            validate_value_field_string(
                s.raw_value(),
                value.span_range(),
                content,
                current_namespace,
                true, // Include "integer" in error message
            )
        }

        (AstValue::String(s), SimpleType::Bool) => {
            let val = s.raw_value();
            if val == "yes" || val == "no" {
                None // Valid boolean
            } else if val.starts_with("$") && val.ends_with("$") {
                None // Argument in scripted effect
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
    span_range: Range<usize>,
    content: &str,
    current_namespace: Option<&str>,
) -> Option<tower_lsp::lsp_types::Diagnostic> {
    validate_scripted_variable_exists(variable_name, span_range, content, current_namespace)
}

/// Helper function to validate value field strings (used by both ValueField and IntValueField)
fn validate_value_field_string(
    value_str: &str,
    span_range: Range<usize>,
    content: &str,
    current_namespace: Option<&str>,
    include_integer_in_error: bool,
) -> Option<Diagnostic> {
    if value_str.starts_with("@") {
        validate_scripted_variable(value_str, span_range, content, current_namespace)
    } else if contains_scripted_argument(value_str) {
        None // Argument in scripted effect
    } else if value_str.starts_with("modifier:") {
        None // Modifier reference - anything after modifier: is valid for now
    } else if let Some(colon_pos) = value_str.find(':') {
        // Check if there's a dot before the colon (complex path like "from.trigger:empire_size")
        if value_str[..colon_pos].contains('.') {
            None // Complex path - skip validation
        } else if value_str.starts_with("value:") {
            // Extract the script value name, handling parameterized format
            // Format: value:my_value|PARAM1|value1|PARAM2|value2|
            let value_part = value_str.split("value:").nth(1).unwrap();
            let value_name = if let Some(pipe_pos) = value_part.find('|') {
                &value_part[..pipe_pos]
            } else {
                value_part
            };

            let entity = EntityRestructurer::get_entity("common/script_values", value_name);

            if entity.is_none() {
                Some(create_type_mismatch_diagnostic(
                    span_range,
                    &format!("Script value '{}' does not exist", value_name),
                    content,
                ))
            } else {
                None
            }
        } else {
            // Other colon-based values, for now let them through
            None
        }
    } else {
        // Check if the value exists in any value set
        if let Some(full_analysis) = FullAnalysis::get() {
            // Check if this value exists in any of the dynamic value sets
            for (_key, value_set) in &full_analysis.dynamic_value_sets {
                if value_set.contains(value_str) {
                    return None; // Valid value from a value set
                }
            }
        }

        let error_msg = if include_integer_in_error {
            "Expected integer, script value (value: prefix), scripted variable (@), argument ($...$), or value from value set"
        } else {
            "Expected number, script value (value: prefix), scripted variable (@), argument ($...$), or value from value set"
        };

        Some(create_type_mismatch_diagnostic(
            span_range, error_msg, content,
        ))
    }
}

/// Check if a scripted variable exists in the game data
fn validate_scripted_variable_exists(
    variable_name: &str,
    span_range: Range<usize>,
    content: &str,
    current_namespace: Option<&str>,
) -> Option<tower_lsp::lsp_types::Diagnostic> {
    if let Some(cache) = GameDataCache::get() {
        // Check global scripted variables from base game
        if cache.scripted_variables.contains_key(variable_name) {
            return None; // Valid scripted variable
        }

        // Check global scripted variables from mod data
        let mod_scripted_variables = ModDataCache::get_scripted_variables();
        if mod_scripted_variables.contains_key(variable_name) {
            return None; // Valid scripted variable
        }

        if let Some(namespace_name) = current_namespace {
            // Check namespace-specific scripted variables using EntityRestructurer
            // (This will automatically include both base game and mod data)
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
