//! Specialized visitor for CWT value set definitions
//!
//! This visitor handles the processing of CWT value set definitions, which are
//! collections of valid string values for specific contexts.

use super::super::conversion::ConversionError;
use super::registry::CwtAnalysisData;
use cw_parser::cwt::{AstCwtBlock, AstCwtRule, CwtReferenceType, AstCwtRuleKey, CwtValue, CwtVisitor};
use std::collections::HashSet;

/// Specialized visitor for value set definitions
pub struct ValueSetVisitor<'a> {
    data: &'a mut CwtAnalysisData,
    in_values_section: bool,
}

impl<'a> ValueSetVisitor<'a> {
    /// Create a new value set visitor
    pub fn new(data: &'a mut CwtAnalysisData) -> Self {
        Self {
            data,
            in_values_section: false,
        }
    }

    /// Set whether we're in a values section
    pub fn set_in_values_section(&mut self, in_section: bool) {
        self.in_values_section = in_section;
    }

    /// Check if this visitor can handle the given rule
    fn can_handle_rule(&self, rule: &AstCwtRule) -> bool {
        let key = rule.key.name();

        // Check for typed identifiers
        if let AstCwtRuleKey::Identifier(identifier) = &rule.key {
            return matches!(identifier.identifier_type, CwtReferenceType::ValueSet);
        }

        // Check for legacy string-based format
        if key.starts_with("value_set[") && key.ends_with("]") {
            return true;
        }

        // If we're in a values section, we can handle any rule
        self.in_values_section
    }

    /// Process a value set definition rule
    fn process_value_set_definition(&mut self, rule: &AstCwtRule) {
        let value_set_name = self.extract_value_set_name(rule);

        if let Some(name) = value_set_name {
            let mut values = HashSet::new();

            // Extract values from the block
            if let CwtValue::Block(block) = &rule.value {
                Self::extract_values(&mut values, block);
            } else {
                // Handle single value case
                if let CwtValue::String(s) = &rule.value {
                    values.insert(s.raw_value().to_string());
                }
            }

            self.data.value_sets.insert(name, values);
        } else {
            self.data.errors.push(ConversionError::InvalidEnumFormat); // TODO: Add specific error type
        }
    }

    /// Extract the value set name from a rule
    fn extract_value_set_name(&self, rule: &AstCwtRule) -> Option<String> {
        if let AstCwtRuleKey::Identifier(identifier) = &rule.key {
            if matches!(identifier.identifier_type, CwtReferenceType::ValueSet) {
                return Some(identifier.name.raw_value().to_string());
            }
        }

        // Legacy string-based handling
        let key = rule.key.name();
        if key.starts_with("value_set[") && key.ends_with("]") {
            Some(key[10..key.len() - 1].to_string())
        } else if self.in_values_section {
            Some(key.to_string())
        } else {
            None
        }
    }

    /// Extract values from a value set definition block
    fn extract_values(values: &mut HashSet<String>, block: &AstCwtBlock) {
        for item in &block.items {
            match item {
                cw_parser::cwt::AstCwtExpression::String(s) => {
                    values.insert(s.raw_value().to_string());
                }
                cw_parser::cwt::AstCwtExpression::Identifier(id) => {
                    values.insert(id.name.raw_value().to_string());
                }
                // Handle nested rules for more complex value sets
                cw_parser::cwt::AstCwtExpression::Rule(rule) => {
                    let key = rule.key.name();
                    values.insert(key.to_string());

                    // If the rule has a block value, extract values from it too
                    if let CwtValue::Block(inner_block) = &rule.value {
                        Self::extract_values(values, inner_block);
                    }
                }
                _ => {}
            }
        }
    }
}

impl<'a> CwtVisitor<'a> for ValueSetVisitor<'a> {
    fn visit_rule(&mut self, rule: &AstCwtRule<'a>) {
        if self.can_handle_rule(rule) {
            self.process_value_set_definition(rule);
        }

        // Continue walking for nested rules
        self.walk_rule(rule);
    }
}
