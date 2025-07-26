use std::collections::HashMap;
use std::fmt;

use lasso::Spur;

use cw_model::TypeFingerprint;

use crate::interner::get_interner;

/// Represents a scope context in Stellaris CWT
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeContext {
    /// The current scope type (e.g., "country", "planet", "fleet", "ship", etc.)
    pub scope_type: Spur,
}

impl ScopeContext {
    pub fn new(scope_type: Spur) -> Self {
        Self { scope_type }
    }
}

impl TypeFingerprint for ScopeContext {
    fn fingerprint(&self) -> String {
        format!("{:?}", self.scope_type)
    }
}

impl fmt::Display for ScopeContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.scope_type)
    }
}

/// A stack-based scope management system for tracking scope context
/// as we navigate through CWT properties that have push_scope and replace_scope
#[derive(Clone, PartialEq, Eq)]
pub struct ScopeStack {
    /// Stack of scopes, with the most recent (current) scope at the end
    /// This is accessed via prev/prevprev/prevprevprev/prevprevprevprev
    scopes: Vec<ScopeContext>,
    /// Root scope (independent of the stack, only changeable by replace_scope)
    root: ScopeContext,
    /// Explicit scope references (from/fromfrom/fromfromfrom/fromfromfromfrom)
    /// These are like root - explicit references, not stack positions
    from: Option<ScopeContext>,
    fromfrom: Option<ScopeContext>,
    fromfromfrom: Option<ScopeContext>,
    fromfromfromfrom: Option<ScopeContext>,
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
            from: None,
            fromfrom: None,
            fromfromfrom: None,
            fromfromfromfrom: None,
            max_depth: 50, // Reasonable limit to prevent stack overflow
        }
    }

    /// Create a new scope stack with a default initial scope
    pub fn default_with_root(initial_scope_type: Spur) -> Self {
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
    pub fn push_scope_type(&mut self, scope_type: Spur) -> Result<(), ScopeError> {
        let scope = ScopeContext::new(scope_type);
        self.push_scope(scope)
    }

    /// Replace the entire scope context based on replace_scope specification
    /// This rebuilds the stack from deepest to shallowest scope and sets explicit references
    pub fn replace_scope(
        &mut self,
        replacements: HashMap<Spur, ScopeContext>,
    ) -> Result<(), ScopeError> {
        let interner = get_interner();

        // Clear the current stack
        self.scopes.clear();

        // Build new stack from deepest to shallowest for prev scopes
        // Order: prevprevprevprev, prevprevprev, prevprev, prev, this
        let stack_order = [
            interner.get_or_intern("prevprevprevprev"),
            interner.get_or_intern("prevprevprev"),
            interner.get_or_intern("prevprev"),
            interner.get_or_intern("prev"),
            interner.get_or_intern("this"),
        ];

        for &scope_name in &stack_order {
            if let Some(scope) = replacements.get(&scope_name) {
                if self.scopes.len() >= self.max_depth {
                    return Err(ScopeError::StackOverflow {
                        max_depth: self.max_depth,
                    });
                }
                self.scopes.push(scope.clone());
            }
        }

        // Set explicit scope references
        self.from = replacements.get(&interner.get_or_intern("from")).cloned();
        self.fromfrom = replacements
            .get(&interner.get_or_intern("fromfrom"))
            .cloned();
        self.fromfromfrom = replacements
            .get(&interner.get_or_intern("fromfromfrom"))
            .cloned();
        self.fromfromfromfrom = replacements
            .get(&interner.get_or_intern("fromfromfromfrom"))
            .cloned();

        // Set root if specified (only replace_scope can change root)
        if let Some(root_scope) = replacements.get(&interner.get_or_intern("root")) {
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
        replacements: HashMap<Spur, Spur>,
    ) -> Result<(), ScopeError> {
        let scope_replacements: HashMap<Spur, ScopeContext> = replacements
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

    /// Get the explicit from scope (equivalent to `from` in Stellaris)
    pub fn from_scope(&self) -> Option<&ScopeContext> {
        if let Some(from) = self.from.as_ref() {
            return Some(from);
        }

        if self.current_scope().scope_type == get_interner().get_or_intern("unknown") {
            return Some(self.current_scope());
        }

        None
    }

    /// Get the explicit fromfrom scope (equivalent to `fromfrom` in Stellaris)
    pub fn fromfrom_scope(&self) -> Option<&ScopeContext> {
        if let Some(fromfrom) = self.fromfrom.as_ref() {
            return Some(fromfrom);
        }

        if self.current_scope().scope_type == get_interner().get_or_intern("unknown") {
            return Some(self.current_scope());
        }

        None
    }

    /// Get the explicit fromfromfrom scope (equivalent to `fromfromfrom` in Stellaris)
    pub fn fromfromfrom_scope(&self) -> Option<&ScopeContext> {
        if let Some(fromfromfrom) = self.fromfromfrom.as_ref() {
            return Some(fromfromfrom);
        }

        if self.current_scope().scope_type == get_interner().get_or_intern("unknown") {
            return Some(self.current_scope());
        }

        None
    }

    /// Get the explicit fromfromfromfrom scope (equivalent to `fromfromfromfrom` in Stellaris)
    pub fn fromfromfromfrom_scope(&self) -> Option<&ScopeContext> {
        if let Some(fromfromfromfrom) = self.fromfromfromfrom.as_ref() {
            return Some(fromfromfromfrom);
        }

        if self.current_scope().scope_type == get_interner().get_or_intern("unknown") {
            return Some(self.current_scope());
        }

        None
    }

    /// Get the previous scope in the stack (equivalent to `prev` in Stellaris)
    pub fn prev_scope(&self) -> Option<&ScopeContext> {
        if self.scopes.len() >= 2 {
            Some(&self.scopes[self.scopes.len() - 2])
        } else if self.current_scope().scope_type == get_interner().get_or_intern("unknown") {
            Some(self.current_scope())
        } else {
            None
        }
    }

    /// Get the scope two levels back in the stack (equivalent to `prevprev` in Stellaris)
    pub fn prevprev_scope(&self) -> Option<&ScopeContext> {
        if self.scopes.len() >= 3 {
            Some(&self.scopes[self.scopes.len() - 3])
        } else if self.current_scope().scope_type == get_interner().get_or_intern("unknown") {
            Some(self.current_scope())
        } else {
            None
        }
    }

    /// Get the scope three levels back in the stack (equivalent to `prevprevprev` in Stellaris)
    pub fn prevprevprev_scope(&self) -> Option<&ScopeContext> {
        if self.scopes.len() >= 4 {
            Some(&self.scopes[self.scopes.len() - 4])
        } else if self.current_scope().scope_type == get_interner().get_or_intern("unknown") {
            Some(self.current_scope())
        } else {
            None
        }
    }

    /// Get the scope four levels back in the stack (equivalent to `prevprevprevprev` in Stellaris)
    pub fn prevprevprevprev_scope(&self) -> Option<&ScopeContext> {
        if self.scopes.len() >= 5 {
            Some(&self.scopes[self.scopes.len() - 5])
        } else if self.current_scope().scope_type == get_interner().get_or_intern("unknown") {
            Some(self.current_scope())
        } else {
            None
        }
    }

    pub fn get_all_scope_properties() -> &'static [&'static str] {
        &[
            "this",
            "THIS",
            "root",
            "ROOT",
            "from",
            "FROM",
            "fromfrom",
            "FROMFROM",
            "fromfromfrom",
            "FROMFROMFROM",
            "fromfromfromfrom",
            "FROMFROMFROMFROM",
            "prev",
            "PREV",
            "prevprev",
            "PREVPREV",
            "prevprevprev",
            "PREVPREVPREV",
            "prevprevprevprev",
            "PREVPREVPREVPREV",
        ]
    }

    /// Get scope by name (this, root, from, fromfrom, prev, prevprev, etc.)
    pub fn get_scope_by_name(&self, name: Spur) -> Option<&ScopeContext> {
        let interner = get_interner();
        let name = interner.resolve(&name);
        match name {
            "this" | "THIS" => Some(self.current_scope()),
            "root" | "ROOT" => Some(self.root_scope()),
            "from" | "FROM" => self.from_scope(),
            "fromfrom" | "FROMFROM" => self.fromfrom_scope(),
            "fromfromfrom" | "FROMFROMFROM" => self.fromfromfrom_scope(),
            "fromfromfromfrom" | "FROMFROMFROMFROM" => self.fromfromfromfrom_scope(),
            "prev" | "PREV" => self.prev_scope(),
            "prevprev" | "PREVPREV" => self.prevprev_scope(),
            "prevprevprev" | "PREVPREVPREV" => self.prevprevprev_scope(),
            "prevprevprevprev" | "PREVPREVPREVPREV" => self.prevprevprevprev_scope(),
            _ => None,
        }
    }

    /// Get all available scope names at the current depth
    pub fn available_scope_names(&self) -> Vec<Spur> {
        let interner = get_interner();

        // If current scope is "unknown", return all possible scope properties as fallback
        if self.current_scope().scope_type == get_interner().get_or_intern("unknown") {
            return Self::get_all_scope_properties()
                .iter()
                .map(|s| interner.get_or_intern(s))
                .collect();
        }

        let mut names = vec![
            interner.get_or_intern("this"),
            interner.get_or_intern("root"),
            interner.get_or_intern("THIS"),
            interner.get_or_intern("ROOT"),
        ];

        // Add explicit scope references if they exist
        if self.from.is_some() {
            names.push(interner.get_or_intern("from"));
            names.push(interner.get_or_intern("FROM"));
        }
        if self.fromfrom.is_some() {
            names.push(interner.get_or_intern("fromfrom"));
            names.push(interner.get_or_intern("FROMFROM"));
        }
        if self.fromfromfrom.is_some() {
            names.push(interner.get_or_intern("fromfromfrom"));
            names.push(interner.get_or_intern("FROMFROMFROM"));
        }
        if self.fromfromfromfrom.is_some() {
            names.push(interner.get_or_intern("fromfromfromfrom"));
            names.push(interner.get_or_intern("FROMFROMFROMFROM"));
        }

        // Add stack-based scope references if they exist
        if self.scopes.len() >= 2 {
            names.push(interner.get_or_intern("prev"));
            names.push(interner.get_or_intern("PREV"));
        }
        if self.scopes.len() >= 3 {
            names.push(interner.get_or_intern("prevprev"));
            names.push(interner.get_or_intern("PREVPREV"));
        }
        if self.scopes.len() >= 4 {
            names.push(interner.get_or_intern("prevprevprev"));
            names.push(interner.get_or_intern("PREVPREVPREV"));
        }
        if self.scopes.len() >= 5 {
            names.push(interner.get_or_intern("prevprevprevprev"));
            names.push(interner.get_or_intern("PREVPREVPREVPREV"));
        }
        names
    }

    /// Validate that a scope name is valid in the current context
    pub fn validate_scope_name(&self, name: Spur) -> Result<&ScopeContext, ScopeError> {
        self.get_scope_by_name(name)
            .ok_or_else(|| ScopeError::InvalidScopeName {
                name: get_interner().resolve(&name).to_string(),
            })
    }

    /// Get the current stack depth
    pub fn depth(&self) -> usize {
        self.scopes.len()
    }

    /// Check if a scope name is valid at the current depth
    pub fn is_valid_scope_name(&self, name: Spur) -> bool {
        self.get_scope_by_name(name).is_some()
    }

    /// Create a copy of the stack for branching (e.g., when exploring different paths)
    pub fn branch(&self) -> Self {
        self.clone()
    }
}

impl std::fmt::Debug for ScopeStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self == &ScopeStack::default() {
            write!(f, "ScopeStack::default()")
        } else {
            write!(f, "ScopeStack {{")?;
            write!(f, "scopes: {:?}, ", self.scopes)?;
            write!(f, "root: {:?}, ", self.root)?;
            write!(f, "from: {:?}, ", self.from)?;
            write!(f, "fromfrom: {:?}, ", self.fromfrom)?;
            write!(f, "fromfromfrom: {:?}, ", self.fromfromfrom)?;
            write!(f, "fromfromfromfrom: {:?}, ", self.fromfromfromfrom)?;
            write!(f, "}}")
        }
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

        let explicit_scopes = format!(
            "{}|{}|{}|{}",
            self.from
                .as_ref()
                .map(|s| s.fingerprint())
                .unwrap_or_default(),
            self.fromfrom
                .as_ref()
                .map(|s| s.fingerprint())
                .unwrap_or_default(),
            self.fromfromfrom
                .as_ref()
                .map(|s| s.fingerprint())
                .unwrap_or_default(),
            self.fromfromfromfrom
                .as_ref()
                .map(|s| s.fingerprint())
                .unwrap_or_default(),
        );

        format!(
            "{}|{}|{}",
            self.root.fingerprint(),
            scopes_fingerprint,
            explicit_scopes
        )
    }
}

impl std::fmt::Display for ScopeStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();

        // Show scopes with explicit names (this, prev, prevprev, etc.)
        let scope_names = [
            "prevprevprevprev",
            "prevprevprev",
            "prevprev",
            "prev",
            "this",
        ];
        let start_idx = if self.scopes.len() <= 5 {
            5 - self.scopes.len()
        } else {
            0
        };

        for (i, scope) in self.scopes.iter().enumerate() {
            if start_idx + i < scope_names.len() {
                parts.push(format!("{}={}", scope_names[start_idx + i], scope));
            }
        }

        if !parts.is_empty() {
            write!(f, "{}", parts.join(" "))?;
        }

        // Add root reference
        write!(f, " root={}", self.root)?;

        // Add from references if they exist
        if let Some(from) = &self.from {
            write!(f, " from={}", from)?;
        }
        if let Some(fromfrom) = &self.fromfrom {
            write!(f, " fromfrom={}", fromfrom)?;
        }
        if let Some(fromfromfrom) = &self.fromfromfrom {
            write!(f, " fromfromfrom={}", fromfromfrom)?;
        }
        if let Some(fromfromfromfrom) = &self.fromfromfromfrom {
            write!(f, " fromfromfromfrom={}", fromfromfromfrom)?;
        }

        Ok(())
    }
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new(ScopeContext::new(get_interner().get_or_intern("unknown")))
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
        let mut stack = ScopeStack::new(ScopeContext::new(get_interner().get_or_intern("country")));

        // Test initial state
        assert_eq!(
            stack.current_scope().scope_type,
            get_interner().get_or_intern("country")
        );
        assert_eq!(
            stack.root_scope().scope_type,
            get_interner().get_or_intern("country")
        );
        assert_eq!(stack.prev_scope(), None);
        assert_eq!(stack.from_scope(), None);
        assert_eq!(stack.depth(), 1);

        // Test push scope (this affects the stack, not explicit references)
        stack
            .push_scope(ScopeContext::new(get_interner().get_or_intern("planet")))
            .unwrap();
        assert_eq!(
            stack.current_scope().scope_type,
            get_interner().get_or_intern("planet")
        );
        assert_eq!(
            stack.root_scope().scope_type,
            get_interner().get_or_intern("country")
        );
        assert_eq!(
            stack.prev_scope().unwrap().scope_type,
            get_interner().get_or_intern("country")
        );
        assert_eq!(stack.from_scope(), None); // Still no explicit from reference
        assert_eq!(stack.depth(), 2);

        // Test another push
        stack
            .push_scope(ScopeContext::new(get_interner().get_or_intern("pop")))
            .unwrap();
        assert_eq!(
            stack.current_scope().scope_type,
            get_interner().get_or_intern("pop")
        );
        assert_eq!(
            stack.prev_scope().unwrap().scope_type,
            get_interner().get_or_intern("planet")
        );
        assert_eq!(
            stack.prevprev_scope().unwrap().scope_type,
            get_interner().get_or_intern("country")
        );
        assert_eq!(stack.from_scope(), None); // Still no explicit from reference
        assert_eq!(stack.depth(), 3);
    }

    #[test]
    fn test_explicit_scope_references() {
        let mut stack = ScopeStack::new(ScopeContext::new(get_interner().get_or_intern("country")));

        // Set explicit scope references via replace_scope
        let mut replacements = HashMap::new();
        replacements.insert(
            get_interner().get_or_intern("this"),
            ScopeContext::new(get_interner().get_or_intern("pop")),
        );
        replacements.insert(
            get_interner().get_or_intern("from"),
            ScopeContext::new(get_interner().get_or_intern("planet")),
        );
        replacements.insert(
            get_interner().get_or_intern("fromfrom"),
            ScopeContext::new(get_interner().get_or_intern("system")),
        );
        replacements.insert(
            get_interner().get_or_intern("root"),
            ScopeContext::new(get_interner().get_or_intern("empire")),
        );

        stack.replace_scope(replacements).unwrap();

        // Test explicit references
        assert_eq!(
            stack.current_scope().scope_type,
            get_interner().get_or_intern("pop")
        );
        assert_eq!(
            stack.root_scope().scope_type,
            get_interner().get_or_intern("empire")
        );
        assert_eq!(
            stack.from_scope().unwrap().scope_type,
            get_interner().get_or_intern("planet")
        );
        assert_eq!(
            stack.fromfrom_scope().unwrap().scope_type,
            get_interner().get_or_intern("system")
        );
        assert_eq!(stack.fromfromfrom_scope(), None);

        // Test that stack-based scopes are independent
        assert_eq!(stack.prev_scope(), None); // No stack depth for prev
        assert_eq!(stack.depth(), 1); // Only "this" in the stack
    }

    #[test]
    fn test_stack_vs_explicit_scopes() {
        let mut stack = ScopeStack::new(ScopeContext::new(get_interner().get_or_intern("country")));

        // Build up a stack and set explicit references
        let mut replacements = HashMap::new();
        replacements.insert(
            get_interner().get_or_intern("this"),
            ScopeContext::new(get_interner().get_or_intern("pop")),
        );
        replacements.insert(
            get_interner().get_or_intern("prev"),
            ScopeContext::new(get_interner().get_or_intern("planet")),
        );
        replacements.insert(
            get_interner().get_or_intern("prevprev"),
            ScopeContext::new(get_interner().get_or_intern("system")),
        );
        replacements.insert(
            get_interner().get_or_intern("from"),
            ScopeContext::new(get_interner().get_or_intern("fleet")),
        );
        replacements.insert(
            get_interner().get_or_intern("fromfrom"),
            ScopeContext::new(get_interner().get_or_intern("ship")),
        );
        replacements.insert(
            get_interner().get_or_intern("root"),
            ScopeContext::new(get_interner().get_or_intern("empire")),
        );

        stack.replace_scope(replacements).unwrap();

        // Test stack-based scopes
        assert_eq!(
            stack.current_scope().scope_type,
            get_interner().get_or_intern("pop")
        );
        assert_eq!(
            stack.prev_scope().unwrap().scope_type,
            get_interner().get_or_intern("planet")
        );
        assert_eq!(
            stack.prevprev_scope().unwrap().scope_type,
            get_interner().get_or_intern("system")
        );
        assert_eq!(stack.depth(), 3);

        // Test explicit scopes
        assert_eq!(
            stack.from_scope().unwrap().scope_type,
            get_interner().get_or_intern("fleet")
        );
        assert_eq!(
            stack.fromfrom_scope().unwrap().scope_type,
            get_interner().get_or_intern("ship")
        );
        assert_eq!(
            stack.root_scope().scope_type,
            get_interner().get_or_intern("empire")
        );
    }

    #[test]
    fn test_scope_name_resolution() {
        let mut stack = ScopeStack::new(ScopeContext::new(get_interner().get_or_intern("country")));

        // Set up both stack and explicit scopes
        let mut replacements = HashMap::new();
        replacements.insert(
            get_interner().get_or_intern("this"),
            ScopeContext::new(get_interner().get_or_intern("pop")),
        );
        replacements.insert(
            get_interner().get_or_intern("prev"),
            ScopeContext::new(get_interner().get_or_intern("planet")),
        );
        replacements.insert(
            get_interner().get_or_intern("prevprev"),
            ScopeContext::new(get_interner().get_or_intern("system")),
        );
        replacements.insert(
            get_interner().get_or_intern("from"),
            ScopeContext::new(get_interner().get_or_intern("fleet")),
        );
        replacements.insert(
            get_interner().get_or_intern("fromfrom"),
            ScopeContext::new(get_interner().get_or_intern("ship")),
        );
        replacements.insert(
            get_interner().get_or_intern("root"),
            ScopeContext::new(get_interner().get_or_intern("empire")),
        );

        stack.replace_scope(replacements).unwrap();

        // Test scope name resolution
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("this"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("pop")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("root"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("empire")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("prev"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("planet")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("prevprev"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("system")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("from"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("fleet")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("fromfrom"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("ship")
        );
        assert_eq!(
            stack.get_scope_by_name(get_interner().get_or_intern("fromfromfrom")),
            None
        );
        assert_eq!(
            stack.get_scope_by_name(get_interner().get_or_intern("prevprevprev")),
            None
        );

        // Test case variations
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("THIS"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("pop")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("ROOT"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("empire")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("FROM"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("fleet")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("PREV"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("planet")
        );
    }

    #[test]
    fn test_available_scope_names() {
        let mut stack = ScopeStack::new(ScopeContext::new(get_interner().get_or_intern("country")));

        // Initially, only this and root are available
        let mut available = stack.available_scope_names();
        available.sort();
        let mut expected = vec![
            get_interner().get_or_intern("this"),
            get_interner().get_or_intern("root"),
            get_interner().get_or_intern("THIS"),
            get_interner().get_or_intern("ROOT"),
        ];
        expected.sort();
        assert_eq!(available, expected);

        // Add stack scopes
        stack
            .push_scope(ScopeContext::new(get_interner().get_or_intern("planet")))
            .unwrap();
        stack
            .push_scope(ScopeContext::new(get_interner().get_or_intern("pop")))
            .unwrap();

        let mut available = stack.available_scope_names();
        available.sort();
        let mut expected = vec![
            get_interner().get_or_intern("this"),
            get_interner().get_or_intern("root"),
            get_interner().get_or_intern("THIS"),
            get_interner().get_or_intern("ROOT"),
            get_interner().get_or_intern("prev"),
            get_interner().get_or_intern("PREV"),
            get_interner().get_or_intern("prevprev"),
            get_interner().get_or_intern("PREVPREV"),
        ];
        expected.sort();
        assert_eq!(available, expected);

        // Add explicit scopes via replace_scope
        let mut replacements = HashMap::new();
        replacements.insert(
            get_interner().get_or_intern("this"),
            ScopeContext::new(get_interner().get_or_intern("pop")),
        );
        replacements.insert(
            get_interner().get_or_intern("prev"),
            ScopeContext::new(get_interner().get_or_intern("planet")),
        );
        replacements.insert(
            get_interner().get_or_intern("from"),
            ScopeContext::new(get_interner().get_or_intern("fleet")),
        );
        replacements.insert(
            get_interner().get_or_intern("fromfrom"),
            ScopeContext::new(get_interner().get_or_intern("ship")),
        );

        stack.replace_scope(replacements).unwrap();

        let mut available = stack.available_scope_names();
        available.sort();
        let mut expected = vec![
            get_interner().get_or_intern("this"),
            get_interner().get_or_intern("root"),
            get_interner().get_or_intern("THIS"),
            get_interner().get_or_intern("ROOT"),
            get_interner().get_or_intern("prev"),
            get_interner().get_or_intern("PREV"),
            get_interner().get_or_intern("from"),
            get_interner().get_or_intern("FROM"),
            get_interner().get_or_intern("fromfrom"),
            get_interner().get_or_intern("FROMFROM"),
        ];
        expected.sort();
        assert_eq!(available, expected);
    }

    #[test]
    fn test_replace_scope_with_all_levels() {
        let mut stack = ScopeStack::new(ScopeContext::new(
            get_interner().get_or_intern("original_root"),
        ));

        // Test with all levels including prevprevprevprev and fromfromfromfrom
        let mut replacements = HashMap::new();
        replacements.insert(
            get_interner().get_or_intern("this"),
            ScopeContext::new(get_interner().get_or_intern("pop")),
        );
        replacements.insert(
            get_interner().get_or_intern("prev"),
            ScopeContext::new(get_interner().get_or_intern("planet")),
        );
        replacements.insert(
            get_interner().get_or_intern("prevprev"),
            ScopeContext::new(get_interner().get_or_intern("system")),
        );
        replacements.insert(
            get_interner().get_or_intern("prevprevprev"),
            ScopeContext::new(get_interner().get_or_intern("sector")),
        );
        replacements.insert(
            get_interner().get_or_intern("prevprevprevprev"),
            ScopeContext::new(get_interner().get_or_intern("country")),
        );
        replacements.insert(
            get_interner().get_or_intern("from"),
            ScopeContext::new(get_interner().get_or_intern("fleet")),
        );
        replacements.insert(
            get_interner().get_or_intern("fromfrom"),
            ScopeContext::new(get_interner().get_or_intern("ship")),
        );
        replacements.insert(
            get_interner().get_or_intern("fromfromfrom"),
            ScopeContext::new(get_interner().get_or_intern("component")),
        );
        replacements.insert(
            get_interner().get_or_intern("fromfromfromfrom"),
            ScopeContext::new(get_interner().get_or_intern("weapon")),
        );
        replacements.insert(
            get_interner().get_or_intern("root"),
            ScopeContext::new(get_interner().get_or_intern("empire")),
        );

        stack.replace_scope(replacements).unwrap();

        // Verify stack scopes
        assert_eq!(
            stack.current_scope().scope_type,
            get_interner().get_or_intern("pop")
        );
        assert_eq!(
            stack.prev_scope().unwrap().scope_type,
            get_interner().get_or_intern("planet")
        );
        assert_eq!(
            stack.prevprev_scope().unwrap().scope_type,
            get_interner().get_or_intern("system")
        );
        assert_eq!(
            stack.prevprevprev_scope().unwrap().scope_type,
            get_interner().get_or_intern("sector")
        );
        assert_eq!(
            stack.prevprevprevprev_scope().unwrap().scope_type,
            get_interner().get_or_intern("country")
        );
        assert_eq!(stack.depth(), 5);

        // Verify explicit scopes
        assert_eq!(
            stack.from_scope().unwrap().scope_type,
            get_interner().get_or_intern("fleet")
        );
        assert_eq!(
            stack.fromfrom_scope().unwrap().scope_type,
            get_interner().get_or_intern("ship")
        );
        assert_eq!(
            stack.fromfromfrom_scope().unwrap().scope_type,
            get_interner().get_or_intern("component")
        );
        assert_eq!(
            stack.fromfromfromfrom_scope().unwrap().scope_type,
            get_interner().get_or_intern("weapon")
        );
        assert_eq!(
            stack.root_scope().scope_type,
            get_interner().get_or_intern("empire")
        );

        // Test scope name resolution for all levels
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("this"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("pop")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("prev"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("planet")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("prevprev"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("system")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("prevprevprev"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("sector")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("prevprevprevprev"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("country")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("from"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("fleet")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("fromfrom"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("ship")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("fromfromfrom"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("component")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("fromfromfromfrom"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("weapon")
        );
        assert_eq!(
            stack
                .get_scope_by_name(get_interner().get_or_intern("root"))
                .unwrap()
                .scope_type,
            get_interner().get_or_intern("empire")
        );
    }

    #[test]
    fn test_replace_scope_partial() {
        let mut stack = ScopeStack::new(ScopeContext::new(
            get_interner().get_or_intern("original_root"),
        ));

        // Test partial replacement (only some scopes specified)
        let mut partial_replacements = HashMap::new();
        partial_replacements.insert(
            get_interner().get_or_intern("this"),
            ScopeContext::new(get_interner().get_or_intern("pop")),
        );
        partial_replacements.insert(
            get_interner().get_or_intern("prev"),
            ScopeContext::new(get_interner().get_or_intern("planet")),
        );
        partial_replacements.insert(
            get_interner().get_or_intern("from"),
            ScopeContext::new(get_interner().get_or_intern("fleet")),
        );
        // No prevprev, fromfrom, or root specified

        stack.replace_scope(partial_replacements).unwrap();

        // Should only have specified scopes
        assert_eq!(
            stack.current_scope().scope_type,
            get_interner().get_or_intern("pop")
        );
        assert_eq!(
            stack.prev_scope().unwrap().scope_type,
            get_interner().get_or_intern("planet")
        );
        assert_eq!(stack.prevprev_scope(), None);
        assert_eq!(
            stack.from_scope().unwrap().scope_type,
            get_interner().get_or_intern("fleet")
        );
        assert_eq!(stack.fromfrom_scope(), None);
        assert_eq!(
            stack.root_scope().scope_type,
            get_interner().get_or_intern("original_root")
        ); // Root unchanged
        assert_eq!(stack.depth(), 2);
    }

    #[test]
    fn test_replace_scope_empty() {
        let mut stack = ScopeStack::new(ScopeContext::new(
            get_interner().get_or_intern("original_root"),
        ));

        // Replace with empty map
        let replacements = HashMap::new();
        stack.replace_scope(replacements).unwrap();

        // Should have root as the only scope
        assert_eq!(
            stack.current_scope().scope_type,
            get_interner().get_or_intern("original_root")
        );
        assert_eq!(
            stack.root_scope().scope_type,
            get_interner().get_or_intern("original_root")
        );
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.prev_scope(), None);
        assert_eq!(stack.from_scope(), None);
    }

    #[test]
    fn test_push_scope_affects_stack_only() {
        let mut stack = ScopeStack::new(ScopeContext::new(get_interner().get_or_intern("country")));

        // Set explicit from reference
        let mut replacements = HashMap::new();
        replacements.insert(
            get_interner().get_or_intern("from"),
            ScopeContext::new(get_interner().get_or_intern("fleet")),
        );
        stack.replace_scope(replacements).unwrap();

        // Now push some scopes
        stack
            .push_scope(ScopeContext::new(get_interner().get_or_intern("planet")))
            .unwrap();
        stack
            .push_scope(ScopeContext::new(get_interner().get_or_intern("pop")))
            .unwrap();

        // Explicit from should be unchanged
        assert_eq!(
            stack.from_scope().unwrap().scope_type,
            get_interner().get_or_intern("fleet")
        );

        // Stack should have new scopes
        assert_eq!(
            stack.current_scope().scope_type,
            get_interner().get_or_intern("pop")
        );
        assert_eq!(
            stack.prev_scope().unwrap().scope_type,
            get_interner().get_or_intern("planet")
        );
        assert_eq!(
            stack.prevprev_scope().unwrap().scope_type,
            get_interner().get_or_intern("country")
        );
        assert_eq!(stack.depth(), 3);
    }

    #[test]
    fn test_scope_overflow() {
        let mut stack = ScopeStack::new(ScopeContext::new(get_interner().get_or_intern("country")));
        stack.max_depth = 3;

        stack
            .push_scope(ScopeContext::new(get_interner().get_or_intern("planet")))
            .unwrap();
        stack
            .push_scope(ScopeContext::new(get_interner().get_or_intern("pop")))
            .unwrap();

        let result = stack.push_scope(ScopeContext::new(get_interner().get_or_intern("job")));
        assert!(matches!(
            result,
            Err(ScopeError::StackOverflow { max_depth: 3 })
        ));
    }

    #[test]
    fn test_validation() {
        let mut stack = ScopeStack::new(ScopeContext::new(get_interner().get_or_intern("country")));

        // Set up some scopes
        let mut replacements = HashMap::new();
        replacements.insert(
            get_interner().get_or_intern("this"),
            ScopeContext::new(get_interner().get_or_intern("pop")),
        );
        replacements.insert(
            get_interner().get_or_intern("prev"),
            ScopeContext::new(get_interner().get_or_intern("planet")),
        );
        replacements.insert(
            get_interner().get_or_intern("from"),
            ScopeContext::new(get_interner().get_or_intern("fleet")),
        );
        stack.replace_scope(replacements).unwrap();

        // Test valid scope names
        assert!(
            stack
                .validate_scope_name(get_interner().get_or_intern("this"))
                .is_ok()
        );
        assert!(
            stack
                .validate_scope_name(get_interner().get_or_intern("root"))
                .is_ok()
        );
        assert!(
            stack
                .validate_scope_name(get_interner().get_or_intern("prev"))
                .is_ok()
        );
        assert!(
            stack
                .validate_scope_name(get_interner().get_or_intern("from"))
                .is_ok()
        );
        assert!(stack.is_valid_scope_name(get_interner().get_or_intern("THIS")));
        assert!(stack.is_valid_scope_name(get_interner().get_or_intern("ROOT")));
        assert!(stack.is_valid_scope_name(get_interner().get_or_intern("PREV")));
        assert!(stack.is_valid_scope_name(get_interner().get_or_intern("FROM")));

        // Test invalid scope names
        assert!(
            stack
                .validate_scope_name(get_interner().get_or_intern("invalid"))
                .is_err()
        );
        assert!(
            stack
                .validate_scope_name(get_interner().get_or_intern("prevprev"))
                .is_err()
        ); // Not available at depth 2
        assert!(
            stack
                .validate_scope_name(get_interner().get_or_intern("fromfrom"))
                .is_err()
        ); // Not set
        assert!(!stack.is_valid_scope_name(get_interner().get_or_intern("nonexistent")));
    }
}
