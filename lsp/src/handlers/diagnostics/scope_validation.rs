use std::ops::Range;

use cw_model::types::CwtAnalyzer;
use tower_lsp::lsp_types::Diagnostic;

use crate::handlers::{
    cache::TypeCache, diagnostics::diagnostic::create_value_mismatch_diagnostic, scope::ScopeStack,
};

/// Validate a scope reference value (handles dotted paths like "prev.from")
pub fn validate_scope_reference(
    value: &str,
    scope_key: &str,
    scope_stack: &ScopeStack,
    span: Range<usize>,
    content: &str,
) -> Option<Diagnostic> {
    // Handle special case "any" which allows any scope navigation or link
    if scope_key == "any" {
        return validate_scope_path(value, scope_stack, span, content);
    }

    // For specific scope types, validate that the value resolves to that scope type
    if !TypeCache::is_initialized() {
        return None;
    }
    let cache = TypeCache::get().unwrap();
    let analyzer = cache.get_cwt_analyzer();

    // First validate that it's a valid scope path
    if let Some(diagnostic) = validate_scope_path(value, scope_stack, span.clone(), content) {
        return Some(diagnostic);
    }

    // Then check if the final scope matches the expected scope type
    if let Some(final_scope_type) = resolve_scope_path_to_final_type(value, scope_stack, &analyzer)
    {
        if let Some(expected_scope_type) = analyzer.resolve_scope_name(scope_key) {
            if final_scope_type != expected_scope_type {
                return Some(create_value_mismatch_diagnostic(
                    span,
                    &format!(
                        "Expected scope of type '{}' but '{}' resolves to '{}'",
                        scope_key, value, final_scope_type
                    ),
                    content,
                ));
            }
        }
    }

    None
}

/// Validate a scope group reference value (handles dotted paths like "prev.from")
pub fn validate_scopegroup_reference(
    value: &str,
    scopegroup_key: &str,
    scope_stack: &ScopeStack,
    span: Range<usize>,
    content: &str,
) -> Option<Diagnostic> {
    if !TypeCache::is_initialized() {
        return None;
    }
    let cache = TypeCache::get().unwrap();
    let analyzer = cache.get_cwt_analyzer();

    // First validate that it's a valid scope path
    if let Some(diagnostic) = validate_scope_path(value, scope_stack, span.clone(), content) {
        return Some(diagnostic);
    }

    // Then check if the final scope matches one of the scope group members
    if let Some(scope_group) = analyzer.get_scope_group(scopegroup_key) {
        if let Some(final_scope_type) =
            resolve_scope_path_to_final_type(value, scope_stack, &analyzer)
        {
            // Check if final scope type matches any member of the group
            let is_valid = scope_group.members.iter().any(|member| {
                analyzer
                    .resolve_scope_name(member)
                    .map_or(false, |resolved| resolved == final_scope_type)
            });

            if !is_valid {
                return Some(create_value_mismatch_diagnostic(
                    span,
                    &format!(
                        "Expected scope from group '{}' (one of: {}) but '{}' resolves to '{}'",
                        scopegroup_key,
                        scope_group.members.join(", "),
                        value,
                        final_scope_type
                    ),
                    content,
                ));
            }
        }
    }

    None
}

/// Validate that a scope path is structurally valid (handles dotted paths)
fn validate_scope_path(
    value: &str,
    scope_stack: &ScopeStack,
    span: Range<usize>,
    content: &str,
) -> Option<Diagnostic> {
    if !TypeCache::is_initialized() {
        return None;
    }
    let cache = TypeCache::get().unwrap();
    let analyzer = cache.get_cwt_analyzer();

    let parts: Vec<&str> = value.split('.').collect();
    if parts.is_empty() {
        return Some(create_value_mismatch_diagnostic(
            span,
            "Empty scope path",
            content,
        ));
    }

    // Get all valid navigation options from the current scope
    let current_scope_type = &scope_stack.current_scope().scope_type;
    let mut valid_properties = Vec::new();

    // Add scope properties
    valid_properties.extend(scope_stack.available_scope_names());

    // Add links that can be used from the current scope
    let is_unknown_scope = current_scope_type == "unknown";
    for (link_name, link_def) in analyzer.get_links() {
        if is_unknown_scope || link_def.can_be_used_from(current_scope_type, &analyzer) {
            valid_properties.push(link_name.clone());
        }
    }

    // Validate the first part against available options
    let first_part = parts[0];
    if !valid_properties.contains(&first_part.to_string()) {
        return Some(create_value_mismatch_diagnostic(
            span,
            &format!(
                "Invalid scope or link name '{}'. Available options: {}",
                first_part,
                valid_properties.join(", ")
            ),
            content,
        ));
    }

    // For dotted paths, simulate the full navigation step by step
    if parts.len() > 1 {
        let navigation_result = simulate_scope_navigation(&parts, scope_stack, &analyzer);
        if let Err(error_msg) = navigation_result {
            return Some(create_value_mismatch_diagnostic(span, &error_msg, content));
        }
    }

    None
}

/// Resolve a scope path to its final scope type (full implementation)
fn resolve_scope_path_to_final_type(
    value: &str,
    scope_stack: &ScopeStack,
    analyzer: &CwtAnalyzer,
) -> Option<String> {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.is_empty() {
        return None;
    }

    // Use the full simulation to get the final scope type
    simulate_scope_navigation(&parts, scope_stack, analyzer).ok()
}

/// Simulate full scope navigation step by step for dotted paths
fn simulate_scope_navigation(
    parts: &[&str],
    scope_stack: &ScopeStack,
    analyzer: &CwtAnalyzer,
) -> Result<String, String> {
    if parts.is_empty() {
        return Err("Empty scope path".to_string());
    }

    // Start with the original scope stack and current scope
    let mut simulated_scope_stack = scope_stack.clone();
    let mut current_scope_type = scope_stack.current_scope().scope_type.clone();

    // Navigate through each part of the dotted path
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            return Err(format!("Empty scope name at position {}", i + 1));
        }

        // Basic validation: scope/link names should be valid identifiers
        if !part.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(format!(
                "Invalid scope or link name '{}' at position {} (contains invalid characters)",
                part,
                i + 1
            ));
        }

        // Navigate using the current simulated scope stack context
        let navigation_result =
            navigate_from_scope(part, &current_scope_type, &simulated_scope_stack, analyzer);

        match navigation_result {
            Ok(new_scope_type) => {
                current_scope_type = new_scope_type.clone();

                // Update the simulated scope stack by pushing the new scope
                // This simulates what would happen if we actually navigated to this scope
                if let Ok(_) = simulated_scope_stack.push_scope_type(new_scope_type) {
                    // Successfully updated simulated stack
                } else {
                    // If we can't push (stack overflow), continue without updating stack
                    // but still track the current scope type for link validation
                }
            }
            Err(error) => {
                if i == 0 {
                    return Err(format!("At '{}': {}", part, error));
                } else {
                    return Err(format!(
                        "At '{}' from scope '{}': {}",
                        part, current_scope_type, error
                    ));
                }
            }
        }
    }

    Ok(current_scope_type)
}

/// Navigate from a specific scope using a property name or link
fn navigate_from_scope(
    property_name: &str,
    from_scope_type: &str,
    scope_stack: &ScopeStack,
    analyzer: &CwtAnalyzer,
) -> Result<String, String> {
    let is_unknown_scope = from_scope_type == "unknown";

    // Check if it's a scope property first
    if let Some(scope_context) = scope_stack.get_scope_by_name(property_name) {
        return Ok(scope_context.scope_type.clone());
    }

    // Check if it's a link that can be used from the current scope
    if let Some(link_def) = analyzer.get_link(property_name) {
        if is_unknown_scope || link_def.can_be_used_from(from_scope_type, analyzer) {
            if let Some(resolved_output) = analyzer.resolve_scope_name(&link_def.output_scope) {
                return Ok(resolved_output.to_string());
            } else {
                return Err(format!(
                    "Link '{}' has unresolvable output scope '{}'",
                    property_name, link_def.output_scope
                ));
            }
        } else {
            return Err(format!(
                "Link '{}' cannot be used from scope '{}' (allowed from: {})",
                property_name,
                from_scope_type,
                link_def.input_scopes.join(", ")
            ));
        }
    }

    // If we reach here, the property/link name is not valid
    let mut available_options = Vec::new();

    // Add available scope properties
    available_options.extend(scope_stack.available_scope_names());

    // Add available links
    for (link_name, link_def) in analyzer.get_links() {
        if is_unknown_scope || link_def.can_be_used_from(from_scope_type, analyzer) {
            available_options.push(link_name.clone());
        }
    }

    Err(format!(
        "Invalid scope or link name '{}'. Available options from scope '{}': {}",
        property_name,
        from_scope_type,
        available_options.join(", ")
    ))
}
