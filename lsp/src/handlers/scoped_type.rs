use std::{collections::HashSet, sync::Arc};

use crate::{
    handlers::scope::{ScopeContext, ScopeError, ScopeStack},
    interner::get_interner,
};
use cw_model::{
    ArrayType, BlockType, CwtAnalyzer, CwtType, PatternProperty, Property, ReferenceType,
    SimpleType, SpurMap, TypeFingerprint,
};
use lasso::Spur;

/// A wrapper that combines a CWT type with its scope context
/// This ensures that types always carry information about what scope they exist in
#[derive(Clone, PartialEq)]
pub struct ScopedType {
    /// The actual CWT type definition
    cwt_type: CwtTypeOrSpecial,

    /// The scope context this type exists in
    scope_context: ScopeStack,

    /// The active subtypes (multiple can be active at once)
    subtypes: HashSet<Spur>,

    /// If we're inside a block that's a scripted_effect, this activates VARIABLEs. Value is the name of the scripted_effect.
    in_scripted_effect_block: Option<Spur>,
}

impl std::fmt::Debug for ScopedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.scope_context == ScopeStack::default() && self.subtypes.is_empty() {
            write!(f, "{:?}", self.cwt_type)
        } else {
            write!(f, "ScopedType {{")?;
            write!(f, "cwt_type: {:?}, ", self.cwt_type)?;

            if self.scope_context != ScopeStack::default() {
                write!(f, "scope_context: {:?}, ", self.scope_context)?;
            }

            if !self.subtypes.is_empty() {
                write!(f, "subtypes: {:?}, ", self.subtypes)?;
            }

            write!(f, "}}")
        }
    }
}

impl TypeFingerprint for ScopedType {
    fn fingerprint(&self) -> String {
        let base = format!(
            "{}(scope:{})",
            self.cwt_type.fingerprint(),
            self.scope_context.fingerprint()
        );

        if !self.subtypes.is_empty() {
            let mut subtypes_vec: Vec<_> = self
                .subtypes
                .iter()
                .map(|s| get_interner().resolve(s))
                .collect();
            subtypes_vec.sort(); // Ensure consistent ordering for fingerprints
            let subtypes_str = subtypes_vec.join(",");
            format!("{}[subtypes:{}]", base, subtypes_str)
        } else {
            base
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CwtTypeOrSpecial {
    CwtType(Arc<CwtType>),
    ScopedUnion(Vec<Arc<ScopedType>>),
}

pub enum CwtTypeOrSpecialRef<'a> {
    /// Unknown type
    Unknown,

    /// Simple primitive types (bool, int, float, scalar, etc.)
    Simple(&'a SimpleType),

    /// Reference types (<type>, enum[key], scope[key], etc.)
    Reference(&'a ReferenceType),

    /// Block/object types with properties
    Block(&'a BlockType),

    /// Array types
    Array(&'a ArrayType),

    /// Union types (multiple alternatives)
    Union(&'a Vec<Arc<CwtType>>),

    /// Literal string values
    Literal(&'a Spur),

    /// Set of literal values
    LiteralSet(&'a HashSet<Spur>),

    /// Comparable types (for triggers with == operator)
    Comparable(&'a Box<Arc<CwtType>>),

    /// Any type
    Any,

    /// Union of scoped types (multiple alternatives)
    ScopedUnion(&'a Vec<Arc<ScopedType>>),
}

impl CwtTypeOrSpecial {
    pub fn get_type_name(&self) -> Option<Spur> {
        match self {
            CwtTypeOrSpecial::CwtType(cwt_type) => cwt_type.get_type_name(),
            CwtTypeOrSpecial::ScopedUnion(_) => None,
        }
    }

    pub fn type_name_for_display(&self) -> String {
        match self {
            CwtTypeOrSpecial::CwtType(cwt_type) => cwt_type.type_name_for_display(get_interner()),
            CwtTypeOrSpecial::ScopedUnion(union_types) => {
                if union_types.is_empty() {
                    "(empty scoped union)".to_string()
                } else {
                    union_types
                        .iter()
                        .map(|t| t.type_name_for_display())
                        .collect::<Vec<_>>()
                        .join(" | ")
                }
            }
        }
    }
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
            subtypes: HashSet::new(),
            in_scripted_effect_block: None,
        }
    }

    pub fn get_type_name(&self) -> Option<Spur> {
        self.cwt_type.get_type_name()
    }

    pub fn type_name_for_display(&self) -> String {
        if !self.subtypes.is_empty() {
            return format!(
                "{} ({})",
                self.cwt_type.type_name_for_display(),
                self.subtypes
                    .iter()
                    .map(|s| get_interner().resolve(s))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        self.cwt_type.type_name_for_display()
    }

    pub fn new_with_subtype(
        cwt_type: CwtTypeOrSpecial,
        scope_context: ScopeStack,
        subtype: Option<Spur>,
        scripted_effect_name: Option<Spur>,
    ) -> Self {
        Self {
            cwt_type,
            scope_context,
            subtypes: subtype.into_iter().collect(),
            in_scripted_effect_block: scripted_effect_name,
        }
    }

    pub fn new_with_subtypes(
        cwt_type: CwtTypeOrSpecial,
        scope_context: ScopeStack,
        subtypes: HashSet<Spur>,
        scripted_effect_name: Option<Spur>,
    ) -> Self {
        Self {
            cwt_type,
            scope_context,
            subtypes,
            in_scripted_effect_block: scripted_effect_name,
        }
    }

    /// Create a new scoped type
    pub fn new_cwt(
        cwt_type: Arc<CwtType>,
        scope_context: ScopeStack,
        scripted_effect_name: Option<Spur>,
    ) -> Self {
        Self {
            cwt_type: CwtTypeOrSpecial::CwtType(cwt_type),
            scope_context,
            subtypes: HashSet::new(),
            in_scripted_effect_block: scripted_effect_name,
        }
    }

    pub fn new_cwt_with_subtypes(
        cwt_type: Arc<CwtType>,
        scope_context: ScopeStack,
        subtypes: HashSet<Spur>,
        scripted_effect_name: Option<Spur>,
    ) -> Self {
        Self {
            cwt_type: CwtTypeOrSpecial::CwtType(cwt_type),
            scope_context,
            subtypes,
            in_scripted_effect_block: scripted_effect_name,
        }
    }

    pub fn new_scoped_union(
        scoped_types: Vec<Arc<ScopedType>>,
        scope_context: ScopeStack,
        scripted_effect_name: Option<Spur>,
    ) -> Self {
        Self {
            cwt_type: CwtTypeOrSpecial::ScopedUnion(scoped_types),
            scope_context,
            subtypes: HashSet::new(),
            in_scripted_effect_block: scripted_effect_name,
        }
    }

    /// Create a scoped type with a default root scope
    pub fn with_root_scope(
        cwt_type: Arc<CwtType>,
        root_scope_type: Spur,
        scripted_effect_name: Option<Spur>,
    ) -> Self {
        Self {
            cwt_type: CwtTypeOrSpecial::CwtType(cwt_type),
            scope_context: ScopeStack::default_with_root(root_scope_type),
            subtypes: HashSet::new(),
            in_scripted_effect_block: scripted_effect_name,
        }
    }

    pub fn with_root_scope_and_subtype(
        cwt_type: Arc<CwtType>,
        root_scope_type: Spur,
        subtype: Option<Spur>,
        scripted_effect_name: Option<Spur>,
    ) -> Self {
        Self {
            cwt_type: CwtTypeOrSpecial::CwtType(cwt_type),
            scope_context: ScopeStack::default_with_root(root_scope_type),
            subtypes: subtype.into_iter().collect(),
            in_scripted_effect_block: scripted_effect_name,
        }
    }

    pub fn with_root_scope_and_subtypes(
        cwt_type: Arc<CwtType>,
        root_scope_type: Spur,
        subtypes: HashSet<Spur>,
        scripted_effect_name: Option<Spur>,
    ) -> Self {
        Self {
            cwt_type: CwtTypeOrSpecial::CwtType(cwt_type),
            scope_context: ScopeStack::default_with_root(root_scope_type),
            subtypes,
            in_scripted_effect_block: scripted_effect_name,
        }
    }

    pub fn child(&self, cwt_type: Arc<CwtType>) -> Self {
        Self {
            cwt_type: CwtTypeOrSpecial::CwtType(cwt_type),
            scope_context: self.scope_context.clone(),
            subtypes: self.subtypes.clone(),
            in_scripted_effect_block: self.in_scripted_effect_block.clone(),
        }
    }

    pub fn child_with_subtype(&self, cwt_type: Arc<CwtType>, subtype: Option<Spur>) -> Self {
        Self {
            cwt_type: CwtTypeOrSpecial::CwtType(cwt_type),
            scope_context: self.scope_context.clone(),
            subtypes: subtype.into_iter().collect(),
            in_scripted_effect_block: self.in_scripted_effect_block.clone(),
        }
    }

    pub fn child_with_subtypes(&self, cwt_type: Arc<CwtType>, subtypes: HashSet<Spur>) -> Self {
        Self {
            cwt_type: CwtTypeOrSpecial::CwtType(cwt_type),
            scope_context: self.scope_context.clone(),
            subtypes,
            in_scripted_effect_block: self.in_scripted_effect_block.clone(),
        }
    }

    pub fn cwt_type(&self) -> &CwtTypeOrSpecial {
        &self.cwt_type
    }

    /// Get the underlying CWT type
    pub fn cwt_type_for_matching<'a>(&'a self) -> CwtTypeOrSpecialRef<'a> {
        match &self.cwt_type {
            CwtTypeOrSpecial::CwtType(cwt_type) => match &**cwt_type {
                CwtType::Simple(simple_type) => CwtTypeOrSpecialRef::Simple(simple_type),
                CwtType::Reference(reference_type) => {
                    CwtTypeOrSpecialRef::Reference(reference_type)
                }
                CwtType::Block(block_type) => CwtTypeOrSpecialRef::Block(block_type),
                CwtType::Array(array_type) => CwtTypeOrSpecialRef::Array(array_type),
                CwtType::Union(union_type) => CwtTypeOrSpecialRef::Union(union_type),
                CwtType::Literal(literal_type) => CwtTypeOrSpecialRef::Literal(literal_type),
                CwtType::LiteralSet(literal_set_type) => {
                    CwtTypeOrSpecialRef::LiteralSet(literal_set_type)
                }
                CwtType::Comparable(comparable_type) => {
                    CwtTypeOrSpecialRef::Comparable(comparable_type)
                }
                CwtType::Any => CwtTypeOrSpecialRef::Any,
                CwtType::Unknown => CwtTypeOrSpecialRef::Unknown,
            },
            CwtTypeOrSpecial::ScopedUnion(scoped_union) => {
                CwtTypeOrSpecialRef::ScopedUnion(scoped_union)
            }
        }
    }

    /// Get the scope context
    pub fn scope_stack(&self) -> &ScopeStack {
        &self.scope_context
    }

    pub fn scope_stack_mut(&mut self) -> &mut ScopeStack {
        &mut self.scope_context
    }

    pub fn in_scripted_effect_block(&self) -> Option<&Spur> {
        self.in_scripted_effect_block.as_ref()
    }

    /// Get the active subtypes, if any
    pub fn subtypes(&self) -> &HashSet<Spur> {
        &self.subtypes
    }

    /// Set the active subtypes
    pub fn set_subtypes(&mut self, subtypes: HashSet<Spur>) {
        self.subtypes = subtypes;
    }

    /// Add a subtype to the active subtypes
    pub fn add_subtype(&mut self, subtype: Spur) {
        self.subtypes.insert(subtype);
    }

    /// Remove a subtype from the active subtypes
    pub fn remove_subtype(&mut self, subtype: Spur) {
        self.subtypes.remove(&subtype);
    }

    /// Clear all active subtypes
    pub fn clear_subtypes(&mut self) {
        self.subtypes.clear();
    }

    /// Create a new instance with a different set of subtypes
    pub fn with_subtypes(&self, subtypes: HashSet<Spur>) -> Self {
        Self {
            cwt_type: self.cwt_type.clone(),
            scope_context: self.scope_context.clone(),
            subtypes,
            in_scripted_effect_block: self.in_scripted_effect_block.clone(),
        }
    }

    /// Create a new instance with a single subtype (for backward compatibility)
    pub fn with_subtype(&self, subtype: Option<Spur>) -> Self {
        let subtypes = subtype.into_iter().collect();
        Self {
            cwt_type: self.cwt_type.clone(),
            scope_context: self.scope_context.clone(),
            subtypes,
            in_scripted_effect_block: self.in_scripted_effect_block.clone(),
        }
    }

    /// Create a new instance with additional subtypes
    pub fn with_additional_subtypes(&self, additional_subtypes: HashSet<Spur>) -> Self {
        let mut new_subtypes = self.subtypes.clone();
        new_subtypes.extend(additional_subtypes);
        Self {
            cwt_type: self.cwt_type.clone(),
            scope_context: self.scope_context.clone(),
            subtypes: new_subtypes,
            in_scripted_effect_block: self.in_scripted_effect_block.clone(),
        }
    }

    pub fn set_in_scripted_effect_block(&mut self, scripted_effect_name: Spur) {
        self.in_scripted_effect_block = Some(scripted_effect_name);
    }

    /// Check if this scoped type has a specific subtype
    pub fn has_subtype(&self, subtype_name: Spur) -> bool {
        self.subtypes.contains(&subtype_name)
    }

    /// Check if this scoped type has any subtype
    pub fn has_any_subtype(&self) -> bool {
        !self.subtypes.is_empty()
    }

    /// Check if this is a scope field type
    pub fn is_scope_field(&self) -> bool {
        match &self.cwt_type {
            CwtTypeOrSpecial::CwtType(cwt_type) => match &**cwt_type {
                CwtType::Simple(SimpleType::ScopeField) => true,
                _ => false,
            },
            _ => false,
        }
    }

    /// Get available scope field names for this scoped type
    pub fn available_scope_fields(&self) -> Vec<Spur> {
        self.scope_context.available_scope_names()
    }

    /// Validate a scope field value in this type's context
    pub fn validate_scope_field(&self, field_name: Spur) -> Result<&ScopeContext, ScopeError> {
        self.scope_context.validate_scope_name(field_name)
    }

    /// Get the current scope type (equivalent to `this` in Stellaris)
    pub fn current_scope_type(&self) -> Spur {
        self.scope_context.current_scope().scope_type
    }

    /// Get the root scope type
    pub fn root_scope_type(&self) -> Spur {
        self.scope_context.root_scope().scope_type
    }

    /// Check if a scope field name is valid in the current context
    pub fn is_valid_scope_field(&self, field_name: Spur) -> bool {
        self.scope_context.is_valid_scope_name(field_name)
    }

    /// Create a branch of this scoped type for exploration
    pub fn branch(&self) -> Self {
        Self {
            cwt_type: self.cwt_type.clone(),
            scope_context: self.scope_context.branch(),
            subtypes: self.subtypes.clone(),
            in_scripted_effect_block: self.in_scripted_effect_block.clone(),
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

    fn apply_scope_changes(
        &self,
        scope_manager: &ScopeStack,
        analyzer: &CwtAnalyzer,
    ) -> Result<ScopeStack, ScopeError> {
        let mut new_scope = scope_manager.branch();

        // Apply push_scope if present
        if let Some(push_scope) = &self.options.push_scope {
            if let Some(scope_name) = analyzer.resolve_scope_name(*push_scope) {
                new_scope.push_scope_type(scope_name)?;
            }
        }

        // Apply replace_scope if present
        if let Some(replace_scope) = &self.options.replace_scope {
            let mut new_scopes = SpurMap::new();

            for (key, value) in replace_scope {
                if let Some(scope_name) = analyzer.resolve_scope_name(*value) {
                    new_scopes.insert(key.clone(), scope_name);
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

    fn apply_scope_changes(
        &self,
        scope_manager: &ScopeStack,
        analyzer: &CwtAnalyzer,
    ) -> Result<ScopeStack, ScopeError> {
        let mut new_scope = scope_manager.branch();

        // Apply push_scope if present
        if let Some(push_scope) = &self.options.push_scope {
            if let Some(scope_name) = analyzer.resolve_scope_name(*push_scope) {
                new_scope.push_scope_type(scope_name)?;
            }
        }

        // Apply replace_scope if present
        if let Some(replace_scope) = &self.options.replace_scope {
            let mut new_scopes = SpurMap::new();

            for (key, value) in replace_scope {
                if let Some(scope_name) = analyzer.resolve_scope_name(*value) {
                    new_scopes.insert(key.clone(), scope_name);
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

    #[test]
    fn test_scoped_type_creation() {
        let cwt_type = Arc::new(CwtType::Simple(SimpleType::ScopeField));
        let scoped_type =
            ScopedType::with_root_scope(cwt_type, get_interner().get_or_intern("country"), None);

        assert!(scoped_type.is_scope_field());
        assert_eq!(
            scoped_type.current_scope_type(),
            get_interner().get_or_intern("country")
        );
        assert_eq!(
            scoped_type.root_scope_type(),
            get_interner().get_or_intern("country")
        );
        assert!(scoped_type.subtypes().is_empty());
    }

    #[test]
    fn test_scoped_type_with_subtype() {
        let cwt_type = Arc::new(CwtType::Simple(SimpleType::ScopeField));
        let scoped_type = ScopedType::with_root_scope_and_subtype(
            cwt_type,
            get_interner().get_or_intern("country"),
            Some(get_interner().get_or_intern("pop_spawned")),
            None,
        );

        assert!(scoped_type.is_scope_field());
        assert_eq!(
            scoped_type.current_scope_type(),
            get_interner().get_or_intern("country")
        );
        assert_eq!(
            scoped_type.root_scope_type(),
            get_interner().get_or_intern("country")
        );
        assert_eq!(
            scoped_type.subtypes(),
            &HashSet::from([get_interner().get_or_intern("pop_spawned")])
        );
        assert!(scoped_type.has_subtype(get_interner().get_or_intern("pop_spawned")));
        assert!(!scoped_type.has_subtype(get_interner().get_or_intern("buildable")));
        assert!(scoped_type.has_any_subtype());
    }

    #[test]
    fn test_subtype_manipulation() {
        let cwt_type = Arc::new(CwtType::Simple(SimpleType::ScopeField));
        let mut scoped_type =
            ScopedType::with_root_scope(cwt_type, get_interner().get_or_intern("country"), None);

        // Initially no subtypes
        assert!(scoped_type.subtypes().is_empty());
        assert!(!scoped_type.has_any_subtype());

        // Set subtypes
        scoped_type.set_subtypes(HashSet::from([get_interner().get_or_intern("pop_spawned")]));
        assert_eq!(
            scoped_type.subtypes(),
            &HashSet::from([get_interner().get_or_intern("pop_spawned")])
        );
        assert!(scoped_type.has_subtype(get_interner().get_or_intern("pop_spawned")));
        assert!(scoped_type.has_any_subtype());

        // Create new instance with different subtypes
        let new_scoped_type =
            scoped_type.with_subtype(Some(get_interner().get_or_intern("buildable")));
        assert_eq!(
            new_scoped_type.subtypes(),
            &HashSet::from([get_interner().get_or_intern("buildable")])
        );
        assert!(new_scoped_type.has_subtype(get_interner().get_or_intern("buildable")));

        // Original should be unchanged
        assert_eq!(
            scoped_type.subtypes(),
            &HashSet::from([get_interner().get_or_intern("pop_spawned")])
        );
    }

    #[test]
    fn test_scope_field_validation() {
        let cwt_type = Arc::new(CwtType::Simple(SimpleType::ScopeField));
        let mut scope_manager =
            ScopeStack::default_with_root(get_interner().get_or_intern("country"));
        scope_manager
            .push_scope_type(get_interner().get_or_intern("planet"))
            .unwrap();

        let scoped_type = ScopedType::new_cwt(cwt_type, scope_manager, None);

        // Valid scope fields
        assert!(scoped_type.is_valid_scope_field(get_interner().get_or_intern("this")));
        assert!(scoped_type.is_valid_scope_field(get_interner().get_or_intern("root")));
        assert!(scoped_type.is_valid_scope_field(get_interner().get_or_intern("prev"))); // Stack-based previous scope

        // Invalid scope field
        assert!(!scoped_type.is_valid_scope_field(get_interner().get_or_intern("invalid")));
        assert!(!scoped_type.is_valid_scope_field(get_interner().get_or_intern("from"))); // Explicit scope reference not set

        // Test validation
        assert!(
            scoped_type
                .validate_scope_field(get_interner().get_or_intern("this"))
                .is_ok()
        );
        assert!(
            scoped_type
                .validate_scope_field(get_interner().get_or_intern("invalid"))
                .is_err()
        );
    }

    #[test]
    fn test_scoped_type_branching() {
        let cwt_type = Arc::new(CwtType::Simple(SimpleType::ScopeField));
        let scoped_type = ScopedType::with_root_scope_and_subtype(
            cwt_type,
            get_interner().get_or_intern("country"),
            Some(get_interner().get_or_intern("pop_spawned")),
            None,
        );

        let branched = scoped_type.branch();

        // Should be equal but independent
        assert_eq!(
            scoped_type.current_scope_type(),
            branched.current_scope_type()
        );
        assert_eq!(scoped_type.subtypes(), branched.subtypes());

        // Verify they're independent (this is more of a conceptual test)
        assert_eq!(
            scoped_type.scope_context.depth(),
            branched.scope_context.depth()
        );
    }
}
