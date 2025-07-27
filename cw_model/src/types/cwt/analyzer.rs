//! Main CWT analyzer that coordinates specialized visitors
//!
//! This is the main entry point for CWT analysis, using a visitor pattern
//! with specialized visitors for different CWT constructs.

use crate::{AliasPattern, CaseInsensitiveInterner, CwtType, SpurMap};

use super::conversion::ConversionError;
use super::definitions::*;
use super::visitors::{CwtAnalysisData, CwtVisitorRegistry};
use cw_parser::cwt::CwtModule;
use lasso::Spur;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Main analyzer for CWT (Clausewitz Type) files
///
/// This analyzer converts CWT AST structures to the rich CwtType system
/// using specialized visitors for different CWT constructs.
pub struct CwtAnalyzer {
    /// Internal analysis data
    data: CwtAnalysisData,

    /// Pre-computed category to aliases mapping for performance
    category_index: SpurMap<Vec<AliasPattern>>,
}

impl CwtAnalyzer {
    /// Create a new CWT analyzer
    pub fn new() -> Self {
        Self {
            data: CwtAnalysisData::new(),
            category_index: SpurMap::new(),
        }
    }

    /// Convert a CWT module to CwtType definitions
    pub fn convert_module(
        &mut self,
        module: &CwtModule,
        interner: &CaseInsensitiveInterner,
    ) -> Result<(), Vec<ConversionError>> {
        // Use the visitor registry to process the module
        CwtVisitorRegistry::process_module(&mut self.data, module, interner);

        // Rebuild category index after processing
        self.rebuild_category_index();

        if self.data.errors.is_empty() {
            Ok(())
        } else {
            Err(self.data.errors.clone())
        }
    }

    /// Get all defined types
    pub fn get_types(&self) -> &SpurMap<TypeDefinition> {
        &self.data.types
    }

    /// Get all defined enums
    pub fn get_enums(&self) -> &SpurMap<EnumDefinition> {
        &self.data.enums
    }

    /// Get all defined value sets
    pub fn get_value_sets(&self) -> &SpurMap<HashSet<String>> {
        &self.data.value_sets
    }

    /// Get all defined aliases
    pub fn get_aliases(&self) -> &HashMap<AliasPattern, AliasDefinition> {
        &self.data.aliases
    }

    /// Get single aliases
    pub fn get_single_aliases(&self) -> &SpurMap<Arc<CwtType>> {
        &self.data.single_aliases
    }

    /// Get a specific scope group
    pub fn get_scope_group(&self, name: Spur) -> Option<&ScopeGroupDefinition> {
        self.data.scope_groups.get(&name)
    }

    /// Get all defined links
    pub fn get_links(&self) -> &SpurMap<super::definitions::LinkDefinition> {
        &self.data.links
    }

    /// Get conversion errors
    pub fn get_errors(&self) -> &Vec<ConversionError> {
        &self.data.errors
    }

    /// Clear all definitions and errors
    pub fn clear(&mut self) {
        self.data.clear();
        self.category_index.clear();
    }

    /// Rebuild the category index from current aliases
    fn rebuild_category_index(&mut self) {
        self.category_index.clear();
        for alias_pattern in self.data.aliases.keys() {
            self.category_index
                .entry(alias_pattern.category)
                .or_insert_with(Vec::new)
                .push(alias_pattern.clone());
        }
    }

    /// Get all aliases for a specific category (O(1) lookup)
    pub fn get_aliases_for_category(&self, category: Spur) -> Option<&[AliasPattern]> {
        self.category_index.get(&category).map(|v| v.as_slice())
    }

    /// Check if a category has any aliases
    pub fn has_category(&self, category: Spur) -> bool {
        self.category_index.contains_key(&category)
    }

    /// Get all available categories
    pub fn get_categories(&self) -> Vec<Spur> {
        self.category_index.keys().collect()
    }

    /// Get a specific type definition
    pub fn get_type(&self, name: Spur) -> Option<&TypeDefinition> {
        self.data.types.get(&name)
    }

    /// Get a specific enum definition
    pub fn get_enum(&self, name: Spur) -> Option<&EnumDefinition> {
        self.data.enums.get(&name)
    }

    /// Get a specific value set
    pub fn get_value_set(&self, name: Spur) -> Option<&HashSet<String>> {
        self.data.value_sets.get(&name)
    }

    /// Get a specific alias definition
    pub fn get_alias(&self, pattern: &AliasPattern) -> Option<&AliasDefinition> {
        self.data.aliases.get(pattern)
    }

    /// Get a specific single alias
    pub fn get_single_alias(&self, name: Spur) -> Option<&Arc<CwtType>> {
        self.data.single_aliases.get(&name)
    }

    /// Get a specific link definition
    pub fn get_link(&self, name: Spur) -> Option<&LinkDefinition> {
        self.data.links.get(&name)
    }

    /// Check if a type is defined
    pub fn has_type(&self, name: Spur) -> bool {
        self.data.types.contains_key(&name)
    }

    /// Check if an enum is defined
    pub fn has_enum(&self, name: Spur) -> bool {
        self.data.enums.contains_key(&name)
    }

    /// Check if a value set is defined
    pub fn has_value_set(&self, name: Spur) -> bool {
        self.data.value_sets.contains_key(&name)
    }

    /// Check if an alias is defined
    pub fn has_alias(&self, pattern: &AliasPattern) -> bool {
        self.data.aliases.contains_key(pattern)
    }

    /// Check if a single alias is defined
    pub fn has_single_alias(&self, name: Spur) -> bool {
        self.data.single_aliases.contains_key(&name)
    }

    /// Check if a link is defined
    pub fn has_link(&self, name: Spur) -> bool {
        self.data.links.contains_key(&name)
    }

    pub fn add_type(&mut self, name: Spur, type_definition: TypeDefinition) {
        self.data.types.insert(name, type_definition);
    }

    /// Resolves a scope alias to a scope's canonical name
    pub fn resolve_scope_name(&self, name: Spur) -> Option<Spur> {
        if let Some(scope) = self.data.scopes.get(&name) {
            return Some(scope.name);
        }

        self.data.scopes.iter().find_map(|(_, scope)| {
            if scope.aliases.iter().any(|alias| *alias == name) {
                Some(scope.name)
            } else {
                None
            }
        })
    }

    /// Get statistics about the analyzer
    pub fn get_stats(&self) -> AnalyzerStats {
        AnalyzerStats {
            types_count: self.data.types.len(),
            enums_count: self.data.enums.len(),
            value_sets_count: self.data.value_sets.len(),
            aliases_count: self.data.aliases.len(),
            single_aliases_count: self.data.single_aliases.len(),
            links_count: self.data.links.len(),
            errors_count: self.data.errors.len(),
        }
    }

    /// Merge another analyzer's results into this one
    pub fn merge(&mut self, other: CwtAnalyzer) {
        self.data.types.extend(other.data.types);
        self.data.enums.extend(other.data.enums);
        self.data.value_sets.extend(other.data.value_sets);
        self.data.aliases.extend(other.data.aliases);
        self.data.single_aliases.extend(other.data.single_aliases);
        self.data.links.extend(other.data.links);
        self.data.errors.extend(other.data.errors);

        // Rebuild category index after merging
        self.rebuild_category_index();
    }

    /// Get a reference to the internal analysis data
    pub fn get_analysis_data(&self) -> &CwtAnalysisData {
        &self.data
    }

    /// Get a mutable reference to the internal analysis data
    pub fn get_analysis_data_mut(&mut self) -> &mut CwtAnalysisData {
        &mut self.data
    }
}

impl Default for CwtAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the analyzer's contents
#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzerStats {
    pub types_count: usize,
    pub enums_count: usize,
    pub value_sets_count: usize,
    pub aliases_count: usize,
    pub single_aliases_count: usize,
    pub links_count: usize,
    pub errors_count: usize,
}

impl AnalyzerStats {
    /// Get the total number of definitions
    pub fn total_definitions(&self) -> usize {
        self.types_count
            + self.enums_count
            + self.value_sets_count
            + self.aliases_count
            + self.single_aliases_count
            + self.links_count
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.errors_count > 0
    }

    /// Check if there are any definitions
    pub fn is_empty(&self) -> bool {
        self.total_definitions() == 0
    }
}

impl std::fmt::Display for AnalyzerStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Types: {}, Enums: {}, Value Sets: {}, Aliases: {}, Single Aliases: {}, Links: {}, Errors: {}",
            self.types_count,
            self.enums_count,
            self.value_sets_count,
            self.aliases_count,
            self.single_aliases_count,
            self.links_count,
            self.errors_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let analyzer = CwtAnalyzer::new();
        let stats = analyzer.get_stats();
        assert_eq!(stats.total_definitions(), 0);
        assert!(!stats.has_errors());
        assert!(stats.is_empty());
    }

    #[test]
    fn test_analyzer_stats() {
        let analyzer = CwtAnalyzer::new();
        let stats = analyzer.get_stats();

        assert_eq!(stats.types_count, 0);
        assert_eq!(stats.enums_count, 0);
        assert_eq!(stats.value_sets_count, 0);
        assert_eq!(stats.aliases_count, 0);
        assert_eq!(stats.single_aliases_count, 0);
        assert_eq!(stats.links_count, 0);
        assert_eq!(stats.errors_count, 0);
        assert_eq!(stats.total_definitions(), 0);
        assert!(!stats.has_errors());
        assert!(stats.is_empty());
    }

    #[test]
    fn test_analyzer_getters() {
        let interner = CaseInsensitiveInterner::new();
        let analyzer = CwtAnalyzer::new();

        assert!(analyzer.get_types().is_empty());
        assert!(analyzer.get_enums().is_empty());
        assert!(analyzer.get_value_sets().is_empty());
        assert!(analyzer.get_aliases().is_empty());
        assert!(analyzer.get_single_aliases().is_empty());
        assert!(analyzer.get_links().is_empty());
        assert!(analyzer.get_errors().is_empty());

        assert!(!analyzer.has_type(interner.get_or_intern("test")));
        assert!(!analyzer.has_enum(interner.get_or_intern("test")));
        assert!(!analyzer.has_value_set(interner.get_or_intern("test")));
        assert!(!analyzer.has_alias(&AliasPattern::new_basic(
            interner.get_or_intern("test"),
            interner.get_or_intern("test"),
            &interner
        )));
        assert!(!analyzer.has_single_alias(interner.get_or_intern("test")));
        assert!(!analyzer.has_link(interner.get_or_intern("test")));

        assert!(analyzer.get_type(interner.get_or_intern("test")).is_none());
        assert!(analyzer.get_enum(interner.get_or_intern("test")).is_none());
        assert!(
            analyzer
                .get_value_set(interner.get_or_intern("test"))
                .is_none()
        );
        assert!(
            analyzer
                .get_alias(&AliasPattern::new_basic(
                    interner.get_or_intern("test"),
                    interner.get_or_intern("test"),
                    &interner
                ))
                .is_none()
        );
        assert!(
            analyzer
                .get_single_alias(interner.get_or_intern("test"))
                .is_none()
        );
        assert!(analyzer.get_link(interner.get_or_intern("test")).is_none());
    }
}
