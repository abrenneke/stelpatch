//! Registry that coordinates all specialized CWT visitors
//!
//! This module provides the main coordinator for all CWT visitors, determining which
//! visitor should handle each rule based on its type and context.

use crate::{AliasPattern, CwtType};

use super::super::conversion::ConversionError;
use super::super::definitions::*;
use super::{AliasVisitor, EnumVisitor, LinksVisitor, RuleVisitor, TypeVisitor, ValueSetVisitor};
use cw_parser::cwt::{
    AstCwtIdentifierOrString, AstCwtRule, CwtModule, CwtReferenceType, CwtValue, CwtVisitor,
};
use std::collections::{HashMap, HashSet};

/// Master data structure that owns all CWT analysis results
#[derive(Debug, Default)]
pub struct CwtAnalysisData {
    /// Known types registry
    pub types: HashMap<String, TypeDefinition>,
    /// Known enums registry
    pub enums: HashMap<String, EnumDefinition>,
    /// Known value sets registry
    pub value_sets: HashMap<String, HashSet<String>>,
    /// Known aliases registry
    pub aliases: HashMap<AliasPattern, AliasDefinition>,
    /// Known single aliases registry
    pub single_aliases: HashMap<String, CwtType>,
    /// Known links registry
    pub links: HashMap<String, LinkDefinition>,
    /// Errors encountered during conversion
    pub errors: Vec<ConversionError>,
}

impl CwtAnalysisData {
    /// Create a new empty analysis data structure
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.types.clear();
        self.enums.clear();
        self.value_sets.clear();
        self.aliases.clear();
        self.single_aliases.clear();
        self.links.clear();
        self.errors.clear();
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the number of definitions
    pub fn definition_count(&self) -> usize {
        self.types.len()
            + self.enums.len()
            + self.value_sets.len()
            + self.aliases.len()
            + self.single_aliases.len()
            + self.links.len()
    }

    /// Insert or merge a type definition
    pub fn insert_or_merge_type(&mut self, name: String, type_def: TypeDefinition) {
        if let Some(existing) = self.types.get_mut(&name) {
            existing.merge_with(type_def);
        } else {
            self.types.insert(name, type_def);
        }
    }
}

/// Registry that coordinates all specialized visitors
pub struct CwtVisitorRegistry;

impl CwtVisitorRegistry {
    /// Process a CWT module using specialized visitors
    pub fn process_module(data: &mut CwtAnalysisData, module: &CwtModule) {
        let mut registry = CwtRegistryVisitor::new(data);
        registry.visit_module(module);
    }
}

/// Internal visitor that coordinates with specialized visitors
struct CwtRegistryVisitor<'a> {
    data: &'a mut CwtAnalysisData,
}

impl<'a> CwtRegistryVisitor<'a> {
    /// Create a new registry visitor
    fn new(data: &'a mut CwtAnalysisData) -> Self {
        Self { data }
    }

    /// Handle a types section
    fn handle_types_section(&mut self, rule: &AstCwtRule) {
        if let CwtValue::Block(block) = &rule.value {
            let mut type_visitor = TypeVisitor::new(self.data);
            type_visitor.set_in_types_section(true);

            for item in &block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(child_rule) = item {
                    type_visitor.visit_rule(child_rule);
                }
            }
        }
    }

    /// Handle an enums section
    fn handle_enums_section(&mut self, rule: &AstCwtRule) {
        if let CwtValue::Block(block) = &rule.value {
            let mut enum_visitor = EnumVisitor::new(self.data);
            enum_visitor.set_in_enums_section(true);

            for item in &block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(child_rule) = item {
                    enum_visitor.visit_rule(child_rule);
                }
            }
        }
    }

    /// Handle a links section
    fn handle_links_section(&mut self, rule: &AstCwtRule) {
        let mut links_visitor = LinksVisitor::new(self.data);
        links_visitor.visit_rule(rule);
    }
}

impl<'a> CwtVisitor<'a> for CwtRegistryVisitor<'a> {
    fn visit_rule(&mut self, rule: &AstCwtRule<'a>) {
        let key = rule.key.name();

        // Handle special top-level sections
        match key {
            "types" => {
                self.handle_types_section(rule);
            }
            "enums" => {
                self.handle_enums_section(rule);
            }
            "links" => {
                self.handle_links_section(rule);
            }
            _ => {
                // Check for typed identifiers in the rule key
                match &rule.key {
                    AstCwtIdentifierOrString::Identifier(identifier) => {
                        match &identifier.identifier_type {
                            CwtReferenceType::Type => {
                                let mut type_visitor = TypeVisitor::new(self.data);
                                type_visitor.visit_rule(rule);
                            }
                            CwtReferenceType::Enum => {
                                let mut enum_visitor = EnumVisitor::new(self.data);
                                enum_visitor.visit_rule(rule);
                            }
                            CwtReferenceType::ComplexEnum => {
                                let mut enum_visitor = EnumVisitor::new(self.data);
                                enum_visitor.visit_rule(rule);
                            }
                            CwtReferenceType::ValueSet => {
                                let mut value_set_visitor = ValueSetVisitor::new(self.data);
                                value_set_visitor.visit_rule(rule);
                            }
                            CwtReferenceType::Alias => {
                                let mut alias_visitor = AliasVisitor::new(self.data);
                                alias_visitor.visit_rule(rule);
                            }
                            CwtReferenceType::SingleAlias => {
                                let mut alias_visitor = AliasVisitor::new(self.data);
                                alias_visitor.visit_rule(rule);
                            }
                            _ => {
                                // Default handling - treat as regular rule definition
                                let mut rule_visitor = RuleVisitor::new(self.data);
                                rule_visitor.visit_rule(rule);
                            }
                        }
                    }
                    AstCwtIdentifierOrString::String(_) => {
                        // Default handling - treat as regular rule definition
                        let mut rule_visitor = RuleVisitor::new(self.data);
                        rule_visitor.visit_rule(rule);
                    }
                }
            }
        }
    }
}
