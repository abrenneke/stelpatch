//! Specialized visitor for CWT value set definitions
//!
//! This visitor handles the processing of CWT value set definitions, which are
//! collections of valid string values for specific contexts.

use super::super::conversion::ConversionError;
use super::registry::CwtAnalysisData;
use cw_parser::cwt::{
    AstCwtBlock, AstCwtIdentifierOrString, AstCwtRule, CwtReferenceType, CwtValue, CwtVisitor,
};
use lasso::{Spur, ThreadedRodeo};
use std::collections::HashSet;

/// Specialized visitor for value set definitions
pub struct ValueSetVisitor<'a, 'interner> {
    data: &'a mut CwtAnalysisData,
    interner: &'interner ThreadedRodeo,
}

impl<'a, 'interner> ValueSetVisitor<'a, 'interner> {
    /// Create a new value set visitor
    pub fn new(data: &'a mut CwtAnalysisData, interner: &'interner ThreadedRodeo) -> Self {
        Self { data, interner }
    }

    /// Check if this visitor can handle the given rule
    fn can_handle_rule(&self, rule: &AstCwtRule) -> bool {
        // Check for typed identifiers
        if let AstCwtIdentifierOrString::Identifier(identifier) = &rule.key {
            matches!(identifier.identifier_type, CwtReferenceType::ValueSet)
        } else {
            false
        }
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
    fn extract_value_set_name(&self, rule: &AstCwtRule) -> Option<Spur> {
        if let AstCwtIdentifierOrString::Identifier(identifier) = &rule.key {
            if matches!(identifier.identifier_type, CwtReferenceType::ValueSet) {
                return Some(self.interner.get_or_intern(identifier.name.raw_value()));
            }
        }

        None
    }

    /// Extract values from a value set definition block
    fn extract_values(values: &mut HashSet<String>, block: &AstCwtBlock) {
        for item in &block.items {
            match item {
                cw_parser::cwt::AstCwtExpression::Value(v) => match v {
                    CwtValue::String(s) => {
                        values.insert(s.raw_value().to_string());
                    }
                    _ => {}
                },
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

impl<'a, 'interner> CwtVisitor<'a> for ValueSetVisitor<'a, 'interner> {
    fn visit_rule(&mut self, rule: &AstCwtRule<'a>) {
        if self.can_handle_rule(rule) {
            self.process_value_set_definition(rule);
        }

        // Continue walking for nested rules
        self.walk_rule(rule);
    }
}
