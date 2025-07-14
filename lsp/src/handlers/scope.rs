use std::collections::HashMap;
use std::fmt;

use cw_model::TypeFingerprint;

/// Represents a scope context in Stellaris CWT
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeContext {
    /// The current scope type (e.g., "country", "planet", "fleet", "ship", etc.)
    pub scope_type: String,
}

impl ScopeContext {
    pub fn new(scope_type: impl Into<String>) -> Self {
        Self {
            scope_type: scope_type.into(),
        }
    }
}

impl TypeFingerprint for ScopeContext {
    fn fingerprint(&self) -> String {
        format!("{}", self.scope_type)
    }
}

impl fmt::Display for ScopeContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.scope_type)
    }
}

/// A stack-based scope management system for tracking scope context
/// as we navigate through CWT properties that have push_scope and replace_scope
#[derive(Debug, Clone)]
pub struct ScopeStack {
    /// Stack of scopes, with the most recent (current) scope at the end
    scopes: Vec<ScopeContext>,
    /// Root scope (independent of the stack, only changeable by replace_scope)
    root: ScopeContext,
    /// Maximum allowed stack depth to prevent infinite recursion
    max_depth: usize,
}

impl ScopeStack {
    /// Create a new scope stack with an initial scope
    /// The root scope is initialized to the same value but tracked separately
    pub fn new(initial_scope: ScopeContext) -> Self {
        Self {
            scopes: vec![initial_scope.clone()],
            root: initial_scope, // Root is initialized but only changeable by replace_scope
            max_depth: 50,       // Reasonable limit to prevent stack overflow
        }
    }

    /// Create a new scope stack with a default initial scope
    pub fn default_with_root(initial_scope_type: impl Into<String>) -> Self {
        Self::new(ScopeContext::new(initial_scope_type))
    }

    /// Push a new scope onto the stack
    pub fn push_scope(&mut self, scope: ScopeContext) -> Result<(), ScopeError> {
        if self.scopes.len() >= self.max_depth {
            return Err(ScopeError::StackOverflow {
                max_depth: self.max_depth,
            });
        }
        self.scopes.push(scope);
        Ok(())
    }

    /// Push a new scope onto the stack using a string scope type
    pub fn push_scope_type(&mut self, scope_type: impl Into<String>) -> Result<(), ScopeError> {
        let scope = ScopeContext::new(scope_type);
        self.push_scope(scope)
    }

    /// Replace the entire scope context based on replace_scope specification
    /// This rebuilds the stack from deepest to shallowest scope
    pub fn replace_scope(
        &mut self,
        replacements: HashMap<String, ScopeContext>,
    ) -> Result<(), ScopeError> {
        // Clear the current stack
        self.scopes.clear();

        // Build new stack from deepest to shallowest
        // Order: fromfromfromfrom, fromfromfrom, fromfrom, from, this
        let scope_order = [
            "fromfromfromfrom",
            "fromfromfrom",
            "fromfrom",
            "from",
            "this",
        ];

        for &scope_name in &scope_order {
            if let Some(scope) = replacements.get(scope_name) {
                if self.scopes.len() >= self.max_depth {
                    return Err(ScopeError::StackOverflow {
                        max_depth: self.max_depth,
                    });
                }
                self.scopes.push(scope.clone());
            }
        }

        // Set root if specified (only replace_scope can change root)
        if let Some(root_scope) = replacements.get("root") {
            self.root = root_scope.clone();
        }

        // Ensure we have at least one scope in the stack
        if self.scopes.is_empty() {
            // If no scopes were specified, use the root as the current scope
            self.scopes.push(self.root.clone());
        }

        Ok(())
    }

    /// Helper method to replace scope using string replacements (converts to ScopeContext)
    pub fn replace_scope_from_strings(
        &mut self,
        replacements: HashMap<String, String>,
    ) -> Result<(), ScopeError> {
        let scope_replacements: HashMap<String, ScopeContext> = replacements
            .into_iter()
            .map(|(k, v)| (k, ScopeContext::new(v)))
            .collect();
        self.replace_scope(scope_replacements)
    }

    /// Get the current scope (equivalent to `this` in Stellaris)
    pub fn current_scope(&self) -> &ScopeContext {
        self.scopes.last().expect("Stack should never be empty")
    }

    /// Get the root scope (equivalent to `root` in Stellaris)
    pub fn root_scope(&self) -> &ScopeContext {
        &self.root
    }

    /// Get the parent scope (equivalent to `from` in Stellaris)
    pub fn from_scope(&self) -> Option<&ScopeContext> {
        if self.scopes.len() >= 2 {
            Some(&self.scopes[self.scopes.len() - 2])
        } else {
            None
        }
    }

    /// Get the grandparent scope (equivalent to `fromfrom` in Stellaris)
    pub fn fromfrom_scope(&self) -> Option<&ScopeContext> {
        if self.scopes.len() >= 3 {
            Some(&self.scopes[self.scopes.len() - 3])
        } else {
            None
        }
    }

    /// Get the great-grandparent scope (equivalent to `fromfromfrom` in Stellaris)
    pub fn fromfromfrom_scope(&self) -> Option<&ScopeContext> {
        if self.scopes.len() >= 4 {
            Some(&self.scopes[self.scopes.len() - 4])
        } else {
            None
        }
    }

    /// Get the great-great-grandparent scope (equivalent to `fromfromfromfrom` in Stellaris)
    pub fn fromfromfromfrom_scope(&self) -> Option<&ScopeContext> {
        if self.scopes.len() >= 5 {
            Some(&self.scopes[self.scopes.len() - 5])
        } else {
            None
        }
    }

    pub fn get_all_scope_properties() -> Vec<String> {
        vec![
            "this".to_string(),
            "THIS".to_string(),
            "root".to_string(),
            "ROOT".to_string(),
            "from".to_string(),
            "FROM".to_string(),
            "fromfrom".to_string(),
            "FROMFROM".to_string(),
            "fromfromfrom".to_string(),
            "FROMFROMFROM".to_string(),
            "fromfromfromfrom".to_string(),
            "FROMFROMFROMFROM".to_string(),
        ]
    }

    /// Get scope by name (this, root, from, fromfrom, etc.)
    pub fn get_scope_by_name(&self, name: &str) -> Option<&ScopeContext> {
        match name {
            "this" | "THIS" => Some(self.current_scope()),
            "root" | "ROOT" => Some(self.root_scope()),
            "from" | "FROM" => self.from_scope(),
            "fromfrom" | "FROMFROM" => self.fromfrom_scope(),
            "fromfromfrom" | "FROMFROMFROM" => self.fromfromfrom_scope(),
            "fromfromfromfrom" | "FROMFROMFROMFROM" => self.fromfromfromfrom_scope(),
            _ => None,
        }
    }

    /// Get all available scope names at the current depth
    pub fn available_scope_names(&self) -> Vec<String> {
        let mut names = vec![
            "this".to_string(),
            "root".to_string(),
            "THIS".to_string(),
            "ROOT".to_string(),
        ];

        if self.scopes.len() >= 2 {
            names.push("from".to_string());
            names.push("FROM".to_string());
        }
        if self.scopes.len() >= 3 {
            names.push("fromfrom".to_string());
            names.push("FROMFROM".to_string());
        }
        if self.scopes.len() >= 4 {
            names.push("fromfromfrom".to_string());
            names.push("FROMFROMFROM".to_string());
        }
        if self.scopes.len() >= 5 {
            names.push("fromfromfromfrom".to_string());
            names.push("FROMFROMFROMFROM".to_string());
        }
        names
    }

    /// Validate that a scope name is valid in the current context
    pub fn validate_scope_name(&self, name: &str) -> Result<&ScopeContext, ScopeError> {
        self.get_scope_by_name(name)
            .ok_or_else(|| ScopeError::InvalidScopeName {
                name: name.to_string(),
            })
    }

    /// Get the current stack depth
    pub fn depth(&self) -> usize {
        self.scopes.len()
    }

    /// Check if a scope name is valid at the current depth
    pub fn is_valid_scope_name(&self, name: &str) -> bool {
        self.get_scope_by_name(name).is_some()
    }

    /// Create a copy of the stack for branching (e.g., when exploring different paths)
    pub fn branch(&self) -> Self {
        self.clone()
    }
}

impl TypeFingerprint for ScopeStack {
    fn fingerprint(&self) -> String {
        let scopes_fingerprint = self
            .scopes
            .iter()
            .map(|s| s.fingerprint())
            .collect::<Vec<_>>()
            .join(",");
        format!("{}|{}", self.root.fingerprint(), scopes_fingerprint)
    }
}

impl std::fmt::Display for ScopeStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}",
            self.scopes
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join("->"),
            self.root.to_string()
        )?;
        Ok(())
    }
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new(ScopeContext::new("unknown"))
    }
}

/// Scope-related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeError {
    /// Stack overflow - too many nested scopes
    StackOverflow { max_depth: usize },
    /// Invalid scope name
    InvalidScopeName { name: String },
}

impl fmt::Display for ScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScopeError::StackOverflow { max_depth } => {
                write!(f, "Scope stack overflow (max depth: {})", max_depth)
            }
            ScopeError::InvalidScopeName { name } => {
                write!(f, "Invalid scope name: {}", name)
            }
        }
    }
}

impl std::error::Error for ScopeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_stack_basic() {
        let mut stack = ScopeStack::new(ScopeContext::new("country"));

        // Test initial state
        assert_eq!(stack.current_scope().scope_type, "country");
        assert_eq!(stack.root_scope().scope_type, "country");
        assert_eq!(stack.from_scope(), None);
        assert_eq!(stack.depth(), 1);

        // Test push scope
        stack.push_scope(ScopeContext::new("planet")).unwrap();
        assert_eq!(stack.current_scope().scope_type, "planet");
        assert_eq!(stack.root_scope().scope_type, "country");
        assert_eq!(stack.from_scope().unwrap().scope_type, "country");
        assert_eq!(stack.depth(), 2);

        // Test another push
        stack.push_scope(ScopeContext::new("pop")).unwrap();
        assert_eq!(stack.current_scope().scope_type, "pop");
        assert_eq!(stack.from_scope().unwrap().scope_type, "planet");
        assert_eq!(stack.fromfrom_scope().unwrap().scope_type, "country");
        assert_eq!(stack.depth(), 3);
    }

    #[test]
    fn test_scope_names() {
        let mut stack = ScopeStack::new(ScopeContext::new("country"));
        stack.push_scope(ScopeContext::new("planet")).unwrap();
        stack.push_scope(ScopeContext::new("pop")).unwrap();

        assert_eq!(stack.get_scope_by_name("this").unwrap().scope_type, "pop");
        assert_eq!(
            stack.get_scope_by_name("root").unwrap().scope_type,
            "country"
        );
        assert_eq!(
            stack.get_scope_by_name("from").unwrap().scope_type,
            "planet"
        );
        assert_eq!(
            stack.get_scope_by_name("fromfrom").unwrap().scope_type,
            "country"
        );
        assert_eq!(stack.get_scope_by_name("fromfromfrom"), None);

        let available = stack.available_scope_names();
        assert_eq!(available, vec!["this", "root", "from", "fromfrom"]);
    }

    #[test]
    fn test_scope_context_manager() {
        let mut manager = ScopeStack::new(ScopeContext::new("country"));

        // Test push scope
        manager.push_scope_type("planet").unwrap();
        assert_eq!(manager.current_scope().scope_type, "planet");

        // Test replace_scope functionality
        let mut replacements = HashMap::new();
        replacements.insert("this".to_string(), ScopeContext::new("pop"));
        replacements.insert("from".to_string(), ScopeContext::new("planet"));
        replacements.insert("fromfrom".to_string(), ScopeContext::new("country"));
        replacements.insert("root".to_string(), ScopeContext::new("empire"));

        manager.replace_scope(replacements).unwrap();

        // After replace_scope, the stack should be rebuilt
        assert_eq!(manager.current_scope().scope_type, "pop");
        assert_eq!(manager.from_scope().unwrap().scope_type, "planet");
        assert_eq!(manager.fromfrom_scope().unwrap().scope_type, "country");
        assert_eq!(manager.root_scope().scope_type, "empire");

        // Test validation
        assert!(manager.validate_scope_name("root").is_ok());
        assert!(manager.validate_scope_name("this").is_ok());
        assert!(manager.validate_scope_name("from").is_ok());
        assert!(manager.validate_scope_name("fromfrom").is_ok());
        assert!(manager.validate_scope_name("invalid").is_err());
    }

    #[test]
    fn test_replace_scope() {
        let mut stack = ScopeStack::new(ScopeContext::new("original_root"));

        // Push some scopes first
        stack
            .push_scope(ScopeContext::new("original_scope1"))
            .unwrap();
        stack
            .push_scope(ScopeContext::new("original_scope2"))
            .unwrap();

        // Verify original state
        assert_eq!(stack.current_scope().scope_type, "original_scope2");
        assert_eq!(stack.root_scope().scope_type, "original_root");
        assert_eq!(stack.depth(), 3);

        // Now replace the scope context
        let mut replacements = HashMap::new();
        replacements.insert("this".to_string(), ScopeContext::new("planet"));
        replacements.insert("from".to_string(), ScopeContext::new("ship"));
        replacements.insert("fromfrom".to_string(), ScopeContext::new("country"));
        replacements.insert("root".to_string(), ScopeContext::new("empire"));

        stack.replace_scope(replacements).unwrap();

        // Verify the new state
        assert_eq!(stack.current_scope().scope_type, "planet");
        assert_eq!(stack.from_scope().unwrap().scope_type, "ship");
        assert_eq!(stack.fromfrom_scope().unwrap().scope_type, "country");
        assert_eq!(stack.root_scope().scope_type, "empire");
        assert_eq!(stack.depth(), 3);

        // Test partial replacement (only some scopes specified)
        let mut partial_replacements = HashMap::new();
        partial_replacements.insert("this".to_string(), ScopeContext::new("pop"));
        partial_replacements.insert("from".to_string(), ScopeContext::new("district"));
        // No fromfrom, fromfromfrom, or root specified

        stack.replace_scope(partial_replacements).unwrap();

        // Should only have this and from, root should remain the same
        assert_eq!(stack.current_scope().scope_type, "pop");
        assert_eq!(stack.from_scope().unwrap().scope_type, "district");
        assert_eq!(stack.fromfrom_scope(), None);
        assert_eq!(stack.root_scope().scope_type, "empire"); // Root unchanged
        assert_eq!(stack.depth(), 2);
    }

    #[test]
    fn test_replace_scope_empty() {
        let mut stack = ScopeStack::new(ScopeContext::new("original_root"));

        // Replace with empty map
        let replacements = HashMap::new();
        stack.replace_scope(replacements).unwrap();

        // Should have root as the only scope
        assert_eq!(stack.current_scope().scope_type, "original_root");
        assert_eq!(stack.root_scope().scope_type, "original_root");
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.from_scope(), None);
    }

    #[test]
    fn test_replace_scope_with_fromfromfromfrom() {
        let mut stack = ScopeStack::new(ScopeContext::new("original_root"));

        // Test with all levels including fromfromfromfrom
        let mut replacements = HashMap::new();
        replacements.insert("this".to_string(), ScopeContext::new("pop"));
        replacements.insert("from".to_string(), ScopeContext::new("planet"));
        replacements.insert("fromfrom".to_string(), ScopeContext::new("system"));
        replacements.insert("fromfromfrom".to_string(), ScopeContext::new("sector"));
        replacements.insert("fromfromfromfrom".to_string(), ScopeContext::new("country"));
        replacements.insert("root".to_string(), ScopeContext::new("empire"));

        stack.replace_scope(replacements).unwrap();

        // Verify all levels
        assert_eq!(stack.current_scope().scope_type, "pop");
        assert_eq!(stack.from_scope().unwrap().scope_type, "planet");
        assert_eq!(stack.fromfrom_scope().unwrap().scope_type, "system");
        assert_eq!(stack.fromfromfrom_scope().unwrap().scope_type, "sector");
        assert_eq!(
            stack.fromfromfromfrom_scope().unwrap().scope_type,
            "country"
        );
        assert_eq!(stack.root_scope().scope_type, "empire");
        assert_eq!(stack.depth(), 5);

        // Test scope name resolution
        assert_eq!(stack.get_scope_by_name("this").unwrap().scope_type, "pop");
        assert_eq!(
            stack.get_scope_by_name("from").unwrap().scope_type,
            "planet"
        );
        assert_eq!(
            stack.get_scope_by_name("fromfrom").unwrap().scope_type,
            "system"
        );
        assert_eq!(
            stack.get_scope_by_name("fromfromfrom").unwrap().scope_type,
            "sector"
        );
        assert_eq!(
            stack
                .get_scope_by_name("fromfromfromfrom")
                .unwrap()
                .scope_type,
            "country"
        );
        assert_eq!(
            stack.get_scope_by_name("root").unwrap().scope_type,
            "empire"
        );

        // Test available scope names
        let available = stack.available_scope_names();
        assert_eq!(
            available,
            vec![
                "this",
                "root",
                "from",
                "fromfrom",
                "fromfromfrom",
                "fromfromfromfrom"
            ]
        );
    }

    #[test]
    fn test_scope_overflow() {
        let mut stack = ScopeStack::new(ScopeContext::new("country"));
        stack.max_depth = 3;

        stack.push_scope(ScopeContext::new("planet")).unwrap();
        stack.push_scope(ScopeContext::new("pop")).unwrap();

        let result = stack.push_scope(ScopeContext::new("job"));
        assert!(matches!(
            result,
            Err(ScopeError::StackOverflow { max_depth: 3 })
        ));
    }
}
