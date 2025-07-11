use std::collections::HashMap;
use std::fmt;

use cw_model::TypeFingerprint;

/// Represents a scope context in Stellaris CWT
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeContext {
    /// The current scope type (e.g., "country", "planet", "fleet", "ship", etc.)
    pub scope_type: String,
    /// Optional scope identifier or name
    pub scope_id: Option<String>,
}

impl ScopeContext {
    pub fn new(scope_type: impl Into<String>) -> Self {
        Self {
            scope_type: scope_type.into(),
            scope_id: None,
        }
    }

    pub fn with_id(scope_type: impl Into<String>, scope_id: impl Into<String>) -> Self {
        Self {
            scope_type: scope_type.into(),
            scope_id: Some(scope_id.into()),
        }
    }
}

impl TypeFingerprint for ScopeContext {
    fn fingerprint(&self) -> String {
        format!(
            "{}:{}",
            self.scope_type,
            self.scope_id.as_ref().unwrap_or(&"".to_string())
        )
    }
}

impl fmt::Display for ScopeContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(id) = &self.scope_id {
            write!(f, "{}:{}", self.scope_type, id)
        } else {
            write!(f, "{}", self.scope_type)
        }
    }
}

/// A stack-based scope management system for tracking scope context
/// as we navigate through CWT properties that have push_scope
#[derive(Debug, Clone)]
pub struct ScopeStack {
    /// Stack of scopes, with the most recent (current) scope at the end
    scopes: Vec<ScopeContext>,
    /// Maximum allowed stack depth to prevent infinite recursion
    max_depth: usize,
}

impl ScopeStack {
    /// Create a new scope stack with a root scope
    pub fn new(root_scope: ScopeContext) -> Self {
        Self {
            scopes: vec![root_scope],
            max_depth: 50, // Reasonable limit to prevent stack overflow
        }
    }

    /// Create a new scope stack with a default root scope
    pub fn default_with_root(root_scope_type: impl Into<String>) -> Self {
        Self::new(ScopeContext::new(root_scope_type))
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

    /// Pop the current scope, returning to the previous scope
    pub fn pop_scope(&mut self) -> Result<ScopeContext, ScopeError> {
        if self.scopes.len() <= 1 {
            return Err(ScopeError::CannotPopRoot);
        }
        self.scopes.pop().ok_or(ScopeError::EmptyStack)
    }

    /// Get the current scope (equivalent to `this` in Stellaris)
    pub fn current_scope(&self) -> &ScopeContext {
        self.scopes.last().expect("Stack should never be empty")
    }

    /// Get the root scope (equivalent to `root` in Stellaris)
    pub fn root_scope(&self) -> &ScopeContext {
        self.scopes.first().expect("Stack should never be empty")
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

    /// Get scope by name (this, root, from, fromfrom, etc.)
    pub fn get_scope_by_name(&self, name: &str) -> Option<&ScopeContext> {
        match name {
            "this" | "THIS" => Some(self.current_scope()),
            "root" | "ROOT" => Some(self.root_scope()),
            "from" | "FROM" => self.from_scope(),
            "fromfrom" | "FROMFROM" => self.fromfrom_scope(),
            "fromfromfrom" | "FROMFROMFROM" => self.fromfromfrom_scope(),
            _ => None,
        }
    }

    /// Get all available scope names at the current depth
    pub fn available_scope_names(&self) -> Vec<String> {
        let mut names = vec!["this".to_string(), "root".to_string()];
        if self.scopes.len() >= 2 {
            names.push("from".to_string());
        }
        if self.scopes.len() >= 3 {
            names.push("fromfrom".to_string());
        }
        if self.scopes.len() >= 4 {
            names.push("fromfromfrom".to_string());
        }
        names
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
        self.scopes
            .iter()
            .map(|s| s.fingerprint())
            .collect::<Vec<_>>()
            .join(",")
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
    /// Attempted to pop the root scope
    CannotPopRoot,
    /// Empty stack (should never happen in practice)
    EmptyStack,
    /// Invalid scope name
    InvalidScopeName { name: String },
    /// Scope type mismatch
    ScopeTypeMismatch { expected: String, actual: String },
}

impl fmt::Display for ScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScopeError::StackOverflow { max_depth } => {
                write!(f, "Scope stack overflow (max depth: {})", max_depth)
            }
            ScopeError::CannotPopRoot => {
                write!(f, "Cannot pop root scope")
            }
            ScopeError::EmptyStack => {
                write!(f, "Scope stack is empty")
            }
            ScopeError::InvalidScopeName { name } => {
                write!(f, "Invalid scope name: {}", name)
            }
            ScopeError::ScopeTypeMismatch { expected, actual } => {
                write!(
                    f,
                    "Scope type mismatch: expected {}, got {}",
                    expected, actual
                )
            }
        }
    }
}

impl std::error::Error for ScopeError {}

/// Scope context manager for managing scope transitions
/// during type resolution and validation
#[derive(Debug, Clone)]
pub struct ScopeContextManager {
    /// Current scope stack
    scope_stack: ScopeStack,
    /// Scope mappings for replace_scope functionality
    scope_replacements: HashMap<String, String>,
}

impl ScopeContextManager {
    /// Create a new scope context manager with a root scope
    pub fn new(root_scope: ScopeContext) -> Self {
        Self {
            scope_stack: ScopeStack::new(root_scope),
            scope_replacements: HashMap::new(),
        }
    }

    /// Create a new scope context manager with a default root scope
    pub fn default_with_root(root_scope_type: impl Into<String>) -> Self {
        Self::new(ScopeContext::new(root_scope_type))
    }

    /// Push a new scope (for push_scope functionality)
    pub fn push_scope(&mut self, scope_type: impl Into<String>) -> Result<(), ScopeError> {
        let scope = ScopeContext::new(scope_type);
        self.scope_stack.push_scope(scope)
    }

    /// Pop the current scope
    pub fn pop_scope(&mut self) -> Result<ScopeContext, ScopeError> {
        self.scope_stack.pop_scope()
    }

    /// Set scope replacements (for replace_scope functionality)
    pub fn set_scope_replacements(&mut self, replacements: HashMap<String, String>) {
        self.scope_replacements = replacements;
    }

    /// Get the current scope stack
    pub fn scope_stack(&self) -> &ScopeStack {
        &self.scope_stack
    }

    /// Get the current scope stack mutably
    pub fn scope_stack_mut(&mut self) -> &mut ScopeStack {
        &mut self.scope_stack
    }

    /// Get scope by name, applying replacements if configured
    pub fn get_scope_by_name(&self, name: &str) -> Option<&ScopeContext> {
        // First check if there's a replacement for this scope name
        let effective_name = self
            .scope_replacements
            .get(name)
            .map(|s| s.as_str())
            .unwrap_or(name);

        // Then get the scope from the stack
        self.scope_stack.get_scope_by_name(effective_name)
    }

    /// Get all available scope names, considering replacements
    pub fn available_scope_names(&self) -> Vec<String> {
        let mut names = self.scope_stack.available_scope_names();

        // Add replacement keys
        for replacement_key in self.scope_replacements.keys() {
            if !names.contains(replacement_key) {
                names.push(replacement_key.clone());
            }
        }

        names.sort();
        names
    }

    /// Validate that a scope name is valid in the current context
    pub fn validate_scope_name(&self, name: &str) -> Result<&ScopeContext, ScopeError> {
        self.get_scope_by_name(name)
            .ok_or_else(|| ScopeError::InvalidScopeName {
                name: name.to_string(),
            })
    }

    /// Create a branch of this context manager for exploring different paths
    pub fn branch(&self) -> Self {
        Self {
            scope_stack: self.scope_stack.branch(),
            scope_replacements: self.scope_replacements.clone(),
        }
    }
}

impl TypeFingerprint for ScopeContextManager {
    fn fingerprint(&self) -> String {
        self.scope_stack.fingerprint()
    }
}

impl Default for ScopeContextManager {
    fn default() -> Self {
        Self::new(ScopeContext::new("unknown"))
    }
}

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

        // Test pop
        let popped = stack.pop_scope().unwrap();
        assert_eq!(popped.scope_type, "pop");
        assert_eq!(stack.current_scope().scope_type, "planet");
        assert_eq!(stack.depth(), 2);
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
        let mut manager = ScopeContextManager::new(ScopeContext::new("country"));

        // Test push scope
        manager.push_scope("planet").unwrap();
        assert_eq!(manager.scope_stack().current_scope().scope_type, "planet");

        // Test scope replacements
        let mut replacements = HashMap::new();
        replacements.insert("this".to_string(), "from".to_string());
        manager.set_scope_replacements(replacements);

        // "this" should now resolve to "from" scope
        assert_eq!(
            manager.get_scope_by_name("this").unwrap().scope_type,
            "country"
        );

        // Test validation
        assert!(manager.validate_scope_name("root").is_ok());
        assert!(manager.validate_scope_name("invalid").is_err());
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

    #[test]
    fn test_cannot_pop_root() {
        let mut stack = ScopeStack::new(ScopeContext::new("country"));
        let result = stack.pop_scope();
        assert!(matches!(result, Err(ScopeError::CannotPopRoot)));
    }
}
