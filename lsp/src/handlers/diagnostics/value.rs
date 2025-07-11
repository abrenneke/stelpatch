use cw_parser::{AstNode, AstValue};
use tower_lsp::lsp_types::Diagnostic;

use crate::handlers::{
    diagnostics::diagnostic::create_type_mismatch_diagnostic, scope::ScopeContextManager,
};

/// Check if a value is compatible with a simple type with scope context, returning a diagnostic if incompatible
pub fn is_value_compatible_with_simple_type_with_scope(
    value: &AstValue<'_>,
    simple_type: &cw_model::SimpleType,
    content: &str,
    scope_manager: &ScopeContextManager,
) -> Option<Diagnostic> {
    use cw_model::SimpleType;

    match (value, simple_type) {
        (AstValue::String(_), SimpleType::Localisation) => {
            // TODO: Implement proper localisation validation
            Some(create_type_mismatch_diagnostic(
                value.span_range(),
                "Localisation validation not yet implemented",
                content,
            ))
        }
        (AstValue::String(_), SimpleType::LocalisationSynced) => {
            // TODO: Implement proper localisation validation
            Some(create_type_mismatch_diagnostic(
                value.span_range(),
                "Localisation synced validation not yet implemented",
                content,
            ))
        }
        (AstValue::String(_), SimpleType::LocalisationInline) => {
            // TODO: Implement proper localisation validation
            Some(create_type_mismatch_diagnostic(
                value.span_range(),
                "Inline localisation validation not yet implemented",
                content,
            ))
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
            // Now we can properly validate scope fields using the scope context!
            let field_name = scope_field.raw_value();
            match scope_manager.validate_scope_name(field_name) {
                Ok(_) => None, // Valid scope field
                Err(error) => {
                    let available_scopes = scope_manager.available_scope_names();
                    Some(create_type_mismatch_diagnostic(
                        value.span_range(),
                        &format!(
                            "Invalid scope field '{}'. Available scopes: {}. {}",
                            field_name,
                            available_scopes.join(", "),
                            error
                        ),
                        content,
                    ))
                }
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
        (AstValue::String(_), SimpleType::IntValueField) => {
            // TODO: Implement proper int value field validation
            Some(create_type_mismatch_diagnostic(
                value.span_range(),
                "Int value field validation not yet implemented",
                content,
            ))
        }

        (AstValue::Number(_), SimpleType::ValueField) => None, // Valid
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

        // Type mismatches
        (_, simple_type) => Some(create_type_mismatch_diagnostic(
            value.span_range(),
            &format!(
                "Expected {:?} but got {}",
                simple_type,
                get_value_type_name(value)
            ),
            content,
        )),
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
