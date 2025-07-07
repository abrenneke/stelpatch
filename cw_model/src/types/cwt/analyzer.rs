//! Main CWT analyzer that coordinates specialized visitors
//!
//! This is the main entry point for CWT analysis, using a visitor pattern
//! with specialized visitors for different CWT constructs.

use super::super::inference::*;
use super::conversion::ConversionError;
use super::definitions::*;
use super::visitors::{CwtAnalysisData, CwtVisitorRegistry};
use cw_parser::cwt::CwtModule;
use std::collections::{HashMap, HashSet};

/// Main analyzer for CWT (Clausewitz Type) files
///
/// This analyzer converts CWT AST structures to the rich InferredType system
/// using specialized visitors for different CWT constructs.
pub struct CwtAnalyzer {
    /// Internal analysis data
    data: CwtAnalysisData,
}

impl CwtAnalyzer {
    /// Create a new CWT analyzer
    pub fn new() -> Self {
        Self {
            data: CwtAnalysisData::new(),
        }
    }

    /// Convert a CWT module to InferredType definitions
    pub fn convert_module(&mut self, module: &CwtModule) -> Result<(), Vec<ConversionError>> {
        // Use the visitor registry to process the module
        CwtVisitorRegistry::process_module(&mut self.data, module);

        if self.data.errors.is_empty() {
            Ok(())
        } else {
            Err(self.data.errors.clone())
        }
    }

    /// Get all defined types
    pub fn get_types(&self) -> &HashMap<String, TypeDefinition> {
        &self.data.types
    }

    /// Get all defined enums
    pub fn get_enums(&self) -> &HashMap<String, EnumDefinition> {
        &self.data.enums
    }

    /// Get all defined value sets
    pub fn get_value_sets(&self) -> &HashMap<String, HashSet<String>> {
        &self.data.value_sets
    }

    /// Get all defined aliases
    pub fn get_aliases(&self) -> &HashMap<String, AliasDefinition> {
        &self.data.aliases
    }

    /// Get single aliases
    pub fn get_single_aliases(&self) -> &HashMap<String, InferredType> {
        &self.data.single_aliases
    }

    /// Get conversion errors
    pub fn get_errors(&self) -> &Vec<ConversionError> {
        &self.data.errors
    }

    /// Clear all definitions and errors
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Get a specific type definition
    pub fn get_type(&self, name: &str) -> Option<&TypeDefinition> {
        self.data.types.get(name)
    }

    /// Get a specific enum definition
    pub fn get_enum(&self, name: &str) -> Option<&EnumDefinition> {
        self.data.enums.get(name)
    }

    /// Get a specific value set
    pub fn get_value_set(&self, name: &str) -> Option<&HashSet<String>> {
        self.data.value_sets.get(name)
    }

    /// Get a specific alias definition
    pub fn get_alias(&self, name: &str) -> Option<&AliasDefinition> {
        self.data.aliases.get(name)
    }

    /// Get a specific single alias
    pub fn get_single_alias(&self, name: &str) -> Option<&InferredType> {
        self.data.single_aliases.get(name)
    }

    /// Check if a type is defined
    pub fn has_type(&self, name: &str) -> bool {
        self.data.types.contains_key(name)
    }

    /// Check if an enum is defined
    pub fn has_enum(&self, name: &str) -> bool {
        self.data.enums.contains_key(name)
    }

    /// Check if a value set is defined
    pub fn has_value_set(&self, name: &str) -> bool {
        self.data.value_sets.contains_key(name)
    }

    /// Check if an alias is defined
    pub fn has_alias(&self, name: &str) -> bool {
        self.data.aliases.contains_key(name)
    }

    /// Check if a single alias is defined
    pub fn has_single_alias(&self, name: &str) -> bool {
        self.data.single_aliases.contains_key(name)
    }

    /// Get statistics about the analyzer
    pub fn get_stats(&self) -> AnalyzerStats {
        AnalyzerStats {
            types_count: self.data.types.len(),
            enums_count: self.data.enums.len(),
            value_sets_count: self.data.value_sets.len(),
            aliases_count: self.data.aliases.len(),
            single_aliases_count: self.data.single_aliases.len(),
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
        self.data.errors.extend(other.data.errors);
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
            "Types: {}, Enums: {}, Value Sets: {}, Aliases: {}, Single Aliases: {}, Errors: {}",
            self.types_count,
            self.enums_count,
            self.value_sets_count,
            self.aliases_count,
            self.single_aliases_count,
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
        assert_eq!(stats.errors_count, 0);
        assert_eq!(stats.total_definitions(), 0);
        assert!(!stats.has_errors());
        assert!(stats.is_empty());
    }

    #[test]
    fn test_analyzer_clear() {
        let mut analyzer = CwtAnalyzer::new();

        // Add some fake data
        analyzer.data.types.insert(
            "test".to_string(),
            TypeDefinition::new(super::super::super::inference::InferredType::Primitive(
                super::super::super::inference::PrimitiveType::String,
            )),
        );

        assert_eq!(analyzer.get_stats().types_count, 1);

        analyzer.clear();

        let stats = analyzer.get_stats();
        assert_eq!(stats.total_definitions(), 0);
        assert!(stats.is_empty());
    }

    #[test]
    fn test_analyzer_merge() {
        let mut analyzer1 = CwtAnalyzer::new();
        let mut analyzer2 = CwtAnalyzer::new();

        // Add some fake data to analyzer2
        analyzer2.data.types.insert(
            "test".to_string(),
            TypeDefinition::new(super::super::super::inference::InferredType::Primitive(
                super::super::super::inference::PrimitiveType::String,
            )),
        );

        assert_eq!(analyzer1.get_stats().types_count, 0);
        assert_eq!(analyzer2.get_stats().types_count, 1);

        analyzer1.merge(analyzer2);

        assert_eq!(analyzer1.get_stats().types_count, 1);
    }

    #[test]
    fn test_analyzer_getters() {
        let analyzer = CwtAnalyzer::new();

        assert!(analyzer.get_types().is_empty());
        assert!(analyzer.get_enums().is_empty());
        assert!(analyzer.get_value_sets().is_empty());
        assert!(analyzer.get_aliases().is_empty());
        assert!(analyzer.get_single_aliases().is_empty());
        assert!(analyzer.get_errors().is_empty());

        assert!(!analyzer.has_type("test"));
        assert!(!analyzer.has_enum("test"));
        assert!(!analyzer.has_value_set("test"));
        assert!(!analyzer.has_alias("test"));
        assert!(!analyzer.has_single_alias("test"));

        assert!(analyzer.get_type("test").is_none());
        assert!(analyzer.get_enum("test").is_none());
        assert!(analyzer.get_value_set("test").is_none());
        assert!(analyzer.get_alias("test").is_none());
        assert!(analyzer.get_single_alias("test").is_none());
    }
}
