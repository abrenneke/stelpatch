//! Specialized visitor for CWT alias definitions
//!
//! This visitor handles the processing of CWT alias definitions, including both
//! regular aliases and single aliases.

use cw_parser::{AstCwtRule, CwtReferenceType, AstCwtRuleKey, CwtVisitor};

use crate::{AliasDefinition, ConversionError, CwtAnalysisData, CwtConverter};

/// Specialized visitor for alias definitions
pub struct AliasVisitor<'a> {
    data: &'a mut CwtAnalysisData,
    in_aliases_section: bool,
}

impl<'a> AliasVisitor<'a> {
    /// Create a new alias visitor
    pub fn new(data: &'a mut CwtAnalysisData) -> Self {
        Self {
            data,
            in_aliases_section: false,
        }
    }

    /// Set whether we're in an aliases section
    pub fn set_in_aliases_section(&mut self, in_section: bool) {
        self.in_aliases_section = in_section;
    }

    /// Check if this visitor can handle the given rule
    fn can_handle_rule(&self, rule: &AstCwtRule) -> bool {
        let key = rule.key.name();

        // Check for typed identifiers
        if let AstCwtRuleKey::Identifier(identifier) = &rule.key {
            return matches!(
                identifier.identifier_type,
                CwtReferenceType::Alias | CwtReferenceType::SingleAlias
            );
        }

        // Check for legacy string-based format
        if key.starts_with("alias[") && key.ends_with("]") {
            return true;
        }

        if key.starts_with("single_alias[") && key.ends_with("]") {
            return true;
        }

        // If we're in an aliases section, we can handle any rule
        self.in_aliases_section
    }

    /// Process an alias definition rule
    fn process_alias_definition(&mut self, rule: &AstCwtRule) {
        let alias_name = self.extract_alias_name(rule);
        let is_single = self.is_single_alias(rule);

        if let Some(name) = alias_name {
            if is_single {
                self.process_single_alias(&name, rule);
            } else {
                self.process_regular_alias(&name, rule);
            }
        } else {
            self.data.errors.push(ConversionError::InvalidAliasFormat);
        }
    }

    /// Extract the alias name from a rule
    fn extract_alias_name(&self, rule: &AstCwtRule) -> Option<String> {
        if let AstCwtRuleKey::Identifier(identifier) = &rule.key {
            if matches!(
                identifier.identifier_type,
                CwtReferenceType::Alias | CwtReferenceType::SingleAlias
            ) {
                return Some(identifier.name.raw_value().to_string());
            }
        }

        // Legacy string-based handling
        let key = rule.key.name();
        if key.starts_with("alias[") && key.ends_with("]") {
            Some(key[6..key.len() - 1].to_string())
        } else if key.starts_with("single_alias[") && key.ends_with("]") {
            Some(key[13..key.len() - 1].to_string())
        } else if self.in_aliases_section {
            Some(key.to_string())
        } else {
            None
        }
    }

    /// Check if this is a single alias
    fn is_single_alias(&self, rule: &AstCwtRule) -> bool {
        if let AstCwtRuleKey::Identifier(identifier) = &rule.key {
            matches!(identifier.identifier_type, CwtReferenceType::SingleAlias)
        } else {
            let key = rule.key.name();
            key.starts_with("single_alias[") && key.ends_with("]")
        }
    }

    /// Process a regular alias definition
    fn process_regular_alias(&mut self, alias_full: &str, rule: &AstCwtRule) {
        if let Some((category, name)) = alias_full.split_once(':') {
            let alias_def = AliasDefinition {
                category: category.to_string(),
                name: name.to_string(),
                rules: CwtConverter::convert_value(&rule.value),
            };

            self.data
                .aliases
                .insert(format!("{}:{}", category, name), alias_def);
        } else {
            self.data.errors.push(ConversionError::InvalidAliasFormat);
        }
    }

    /// Process a single alias definition
    fn process_single_alias(&mut self, alias_name: &str, rule: &AstCwtRule) {
        let alias_type = CwtConverter::convert_value(&rule.value);
        self.data
            .single_aliases
            .insert(alias_name.to_string(), alias_type);
    }
}

impl<'a> CwtVisitor<'a> for AliasVisitor<'a> {
    fn visit_rule(&mut self, rule: &AstCwtRule<'a>) {
        if self.can_handle_rule(rule) {
            self.process_alias_definition(rule);
        }

        // Continue walking for nested rules
        self.walk_rule(rule);
    }
}
