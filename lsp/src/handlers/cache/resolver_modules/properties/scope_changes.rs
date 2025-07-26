use std::{collections::HashMap, sync::Arc};

use cw_model::{AliasDefinition, CwtAnalyzer};

use crate::handlers::scope::{ScopeError, ScopeStack};

/// Apply scope changes from alias definition options
pub fn apply_alias_scope_changes(
    cwt_analyzer: Arc<CwtAnalyzer>,
    scope_stack: &ScopeStack,
    alias_def: &AliasDefinition,
) -> Result<ScopeStack, ScopeError> {
    let mut new_scope = scope_stack.branch();

    // Apply push_scope if present
    if let Some(push_scope) = &alias_def.options.push_scope {
        if let Some(scope_name) = cwt_analyzer.resolve_scope_name(*push_scope) {
            new_scope.push_scope_type(scope_name)?;
        }
    }

    // Apply replace_scope if present
    if let Some(replace_scope) = &alias_def.options.replace_scope {
        let mut new_scopes = HashMap::new();

        for (key, value) in replace_scope {
            if let Some(scope_name) = cwt_analyzer.resolve_scope_name(*value) {
                new_scopes.insert(*key, scope_name);
            }
        }

        new_scope.replace_scope_from_strings(new_scopes)?;
    }

    Ok(new_scope)
}
