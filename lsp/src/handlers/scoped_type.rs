use std::{collections::HashMap, sync::Arc};

use crate::handlers::scope::{ScopeContext, ScopeError, ScopeStack};
use cw_model::{CwtAnalyzer, CwtType, PatternProperty, Property, SimpleType, TypeFingerprint};

/// A wrapper that combines a CWT type with its scope context
/// This ensures that types always carry information about what scope they exist in
#[derive(Debug, Clone)]
pub struct ScopedType {
    /// The actual CWT type definition
    cwt_type: CwtTypeOrSpecial,
    /// The scope context this type exists in
    scope_context: ScopeStack,
}

impl TypeFingerprint for ScopedType {
    fn fingerprint(&self) -> String {
        format!(
            "{}(scope:{})",
            self.cwt_type.fingerprint(),
            self.scope_context.fingerprint()
        )
    }
}

#[derive(Debug, Clone)]
pub enum CwtTypeOrSpecial {
    CwtType(CwtType),
    ScopedUnion(Vec<ScopedType>),
}

impl TypeFingerprint for CwtTypeOrSpecial {
    fn fingerprint(&self) -> String {
        match self {
            CwtTypeOrSpecial::CwtType(cwt_type) => cwt_type.fingerprint(),
            CwtTypeOrSpecial::ScopedUnion(scoped_types) => scoped_types
                .iter()
                .map(|s| s.fingerprint())
                .collect::<Vec<_>>()
                .join("|"),
        }
    }
}

impl ScopedType {
    pub fn new(cwt_type: CwtTypeOrSpecial, scope_context: ScopeStack) -> Self {
        Self {
            cwt_type,
            scope_context,
        }
    }

    /// Create a new scoped type
    pub fn new_cwt(cwt_type: CwtType, scope_context: ScopeStack) -> Self {
        Self {
            cwt_type: CwtTypeOrSpecial::CwtType(cwt_type),
            scope_context,
        }
    }

    pub fn new_scoped_union(scoped_types: Vec<ScopedType>, scope_context: ScopeStack) -> Self {
        Self {
            cwt_type: CwtTypeOrSpecial::ScopedUnion(scoped_types),
            scope_context,
        }
    }

    /// Create a scoped type with a default root scope
    pub fn with_root_scope(cwt_type: CwtType, root_scope_type: impl Into<String>) -> Self {
        Self {
            cwt_type: CwtTypeOrSpecial::CwtType(cwt_type),
            scope_context: ScopeStack::default_with_root(root_scope_type),
        }
    }

    pub fn child(&self, cwt_type: CwtType) -> Self {
        Self {
            cwt_type: CwtTypeOrSpecial::CwtType(cwt_type),
            scope_context: self.scope_context.clone(),
        }
    }

    /// Get the underlying CWT type
    pub fn cwt_type(&self) -> &CwtTypeOrSpecial {
        &self.cwt_type
    }

    /// Get the scope context
    pub fn scope_stack(&self) -> &ScopeStack {
        &self.scope_context
    }

    pub fn scope_stack_mut(&mut self) -> &mut ScopeStack {
        &mut self.scope_context
    }

    /// Check if this is a scope field type
    pub fn is_scope_field(&self) -> bool {
        matches!(
            &self.cwt_type,
            CwtTypeOrSpecial::CwtType(CwtType::Simple(SimpleType::ScopeField))
        )
    }

    /// Get available scope field names for this scoped type
    pub fn available_scope_fields(&self) -> Vec<String> {
        self.scope_context.available_scope_names()
    }

    /// Validate a scope field value in this type's context
    pub fn validate_scope_field(&self, field_name: &str) -> Result<&ScopeContext, ScopeError> {
        self.scope_context.validate_scope_name(field_name)
    }

    /// Get the current scope type (equivalent to `this` in Stellaris)
    pub fn current_scope_type(&self) -> &str {
        &self.scope_context.current_scope().scope_type
    }

    /// Get the root scope type
    pub fn root_scope_type(&self) -> &str {
        &self.scope_context.root_scope().scope_type
    }

    /// Check if a scope field name is valid in the current context
    pub fn is_valid_scope_field(&self, field_name: &str) -> bool {
        self.scope_context.is_valid_scope_name(field_name)
    }

    /// Create a branch of this scoped type for exploration
    pub fn branch(&self) -> Self {
        Self {
            cwt_type: self.cwt_type.clone(),
            scope_context: self.scope_context.branch(),
        }
    }
}

/// Result of navigating to a property - either a new scoped type or an error
#[derive(Debug)]
pub enum PropertyNavigationResult {
    /// Successfully navigated to a property
    Success(Arc<ScopedType>),
    /// Property exists but has invalid scope configuration
    ScopeError(ScopeError),
    /// Property doesn't exist
    NotFound,
}

impl PropertyNavigationResult {
    /// Convert to Option, losing error information
    pub fn ok(self) -> Option<Arc<ScopedType>> {
        match self {
            PropertyNavigationResult::Success(scoped_type) => Some(scoped_type),
            _ => None,
        }
    }

    /// Check if the navigation was successful
    pub fn is_success(&self) -> bool {
        matches!(self, PropertyNavigationResult::Success(_))
    }

    /// Check if the property was not found
    pub fn is_not_found(&self) -> bool {
        matches!(self, PropertyNavigationResult::NotFound)
    }

    /// Check if there was a scope error
    pub fn is_scope_error(&self) -> bool {
        matches!(self, PropertyNavigationResult::ScopeError(_))
    }
}

/// Helper trait for working with properties and scope
pub trait ScopeAwareProperty {
    /// Check if this property changes scope context
    fn changes_scope(&self) -> bool;

    /// Get the scope that this property pushes to, if any
    fn pushed_scope(&self) -> Option<&str>;

    /// Get scope replacements, if any
    fn scope_replacements(&self) -> Option<&std::collections::HashMap<String, String>>;

    /// Apply scope changes to a scope stack
    fn apply_scope_changes(
        &self,
        scope_manager: &ScopeStack,
        analyzer: &CwtAnalyzer,
    ) -> Result<ScopeStack, ScopeError>;
}

impl ScopeAwareProperty for Property {
    fn changes_scope(&self) -> bool {
        self.options.push_scope.is_some() || self.options.replace_scope.is_some()
    }

    fn pushed_scope(&self) -> Option<&str> {
        self.options.push_scope.as_deref()
    }

    fn scope_replacements(&self) -> Option<&std::collections::HashMap<String, String>> {
        self.options.replace_scope.as_ref()
    }

    fn apply_scope_changes(
        &self,
        scope_manager: &ScopeStack,
        analyzer: &CwtAnalyzer,
    ) -> Result<ScopeStack, ScopeError> {
        let mut new_scope = scope_manager.branch();

        // Apply push_scope if present
        if let Some(push_scope) = &self.options.push_scope {
            if let Some(scope_name) = analyzer.resolve_scope_name(push_scope) {
                new_scope.push_scope_type(scope_name.to_string())?;
            }
        }

        // Apply replace_scope if present
        if let Some(replace_scope) = &self.options.replace_scope {
            let mut new_scopes = HashMap::new();

            for (key, value) in replace_scope {
                if let Some(scope_name) = analyzer.resolve_scope_name(value) {
                    new_scopes.insert(key.clone(), scope_name.to_string());
                }
            }

            new_scope
                .replace_scope_from_strings(new_scopes)
                .expect("Failed to replace scope");
        }

        Ok(new_scope)
    }
}

impl ScopeAwareProperty for PatternProperty {
    fn changes_scope(&self) -> bool {
        self.options.push_scope.is_some() || self.options.replace_scope.is_some()
    }

    fn pushed_scope(&self) -> Option<&str> {
        self.options.push_scope.as_deref()
    }

    fn scope_replacements(&self) -> Option<&std::collections::HashMap<String, String>> {
        self.options.replace_scope.as_ref()
    }

    fn apply_scope_changes(
        &self,
        scope_manager: &ScopeStack,
        analyzer: &CwtAnalyzer,
    ) -> Result<ScopeStack, ScopeError> {
        let mut new_scope = scope_manager.branch();

        // Apply push_scope if present
        if let Some(push_scope) = &self.options.push_scope {
            if let Some(scope_name) = analyzer.resolve_scope_name(push_scope) {
                new_scope.push_scope_type(scope_name.to_string())?;
            }
        }

        // Apply replace_scope if present
        if let Some(replace_scope) = &self.options.replace_scope {
            let mut new_scopes = HashMap::new();

            for (key, value) in replace_scope {
                if let Some(scope_name) = analyzer.resolve_scope_name(value) {
                    new_scopes.insert(key.clone(), scope_name.to_string());
                }
            }

            new_scope.replace_scope_from_strings(new_scopes).unwrap();
        }

        Ok(new_scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cw_model::CwtOptions;
    use std::collections::HashMap;

    #[test]
    fn test_scoped_type_creation() {
        let cwt_type = CwtType::Simple(SimpleType::ScopeField);
        let scoped_type = ScopedType::with_root_scope(cwt_type, "country");

        assert!(scoped_type.is_scope_field());
        assert_eq!(scoped_type.current_scope_type(), "country");
        assert_eq!(scoped_type.root_scope_type(), "country");
    }

    #[test]
    fn test_scope_field_validation() {
        let cwt_type = CwtType::Simple(SimpleType::ScopeField);
        let mut scope_manager = ScopeStack::default_with_root("country");
        scope_manager.push_scope_type("planet").unwrap();

        let scoped_type = ScopedType::new_cwt(cwt_type, scope_manager);

        // Valid scope fields
        assert!(scoped_type.is_valid_scope_field("this"));
        assert!(scoped_type.is_valid_scope_field("root"));
        assert!(scoped_type.is_valid_scope_field("from"));

        // Invalid scope field
        assert!(!scoped_type.is_valid_scope_field("invalid"));

        // Test validation
        assert!(scoped_type.validate_scope_field("this").is_ok());
        assert!(scoped_type.validate_scope_field("invalid").is_err());
    }

    #[test]
    fn test_scoped_type_branching() {
        let cwt_type = CwtType::Simple(SimpleType::ScopeField);
        let scoped_type = ScopedType::with_root_scope(cwt_type, "country");

        let branched = scoped_type.branch();

        // Should be equal but independent
        assert_eq!(
            scoped_type.current_scope_type(),
            branched.current_scope_type()
        );

        // Verify they're independent (this is more of a conceptual test)
        assert_eq!(
            scoped_type.scope_context.depth(),
            branched.scope_context.depth()
        );
    }
}
