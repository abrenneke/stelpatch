//! Specialized visitor for CWT alias definitions
//!
//! This visitor handles the processing of CWT alias definitions, including both
//! regular aliases and single aliases.

use cw_parser::{AstCwtIdentifierOrString, AstCwtRule, CwtReferenceType, CwtVisitor};

use crate::{AliasDefinition, AliasPattern, ConversionError, CwtAnalysisData, CwtConverter};

/// Specialized visitor for alias definitions
pub struct AliasVisitor<'a> {
    data: &'a mut CwtAnalysisData,
}

impl<'a> AliasVisitor<'a> {
    /// Create a new alias visitor
    pub fn new(data: &'a mut CwtAnalysisData) -> Self {
        Self { data }
    }

    /// Check if this visitor can handle the given rule
    fn can_handle_rule(&self, rule: &AstCwtRule) -> bool {
        // Check for typed identifiers
        if let AstCwtIdentifierOrString::Identifier(identifier) = &rule.key {
            return matches!(
                identifier.identifier_type,
                CwtReferenceType::Alias | CwtReferenceType::SingleAlias
            );
        }

        false
    }

    /// Process an alias definition rule
    fn process_alias_definition(&mut self, rule: &AstCwtRule) {
        let is_single = self.is_single_alias(rule);

        if is_single {
            self.process_single_alias(rule);
        } else {
            self.process_regular_alias(rule);
        }
    }

    /// Check if this is a single alias
    fn is_single_alias(&self, rule: &AstCwtRule) -> bool {
        if let AstCwtIdentifierOrString::Identifier(identifier) = &rule.key {
            matches!(identifier.identifier_type, CwtReferenceType::SingleAlias)
        } else {
            false
        }
    }

    /// Process a regular alias definition
    fn process_regular_alias(&mut self, rule: &AstCwtRule) {
        if let Some(identifier) = &rule.key.as_identifier() {
            if let Some(scope) = &identifier.name.scope {
                let category = scope.raw_value();
                match &identifier.name.key {
                    AstCwtIdentifierOrString::Identifier(key_id) => match key_id.identifier_type {
                        CwtReferenceType::TypeRef => {
                            let name = key_id.name.raw_value();
                            let alias_def = AliasDefinition {
                                category: category.to_string(),
                                name: name.to_string(),
                                to: CwtConverter::convert_value(&rule.value),
                            };
                            self.data
                                .aliases
                                .insert(AliasPattern::new_type_ref(category, name), alias_def);
                        }
                        CwtReferenceType::Enum => {
                            let name = key_id.name.raw_value();
                            let alias_def = AliasDefinition {
                                category: category.to_string(),
                                name: name.to_string(),
                                to: CwtConverter::convert_value(&rule.value),
                            };
                            self.data
                                .aliases
                                .insert(AliasPattern::new_enum(category, name), alias_def);
                        }
                        _ => {
                            panic!("Unknown identifier type for alias in rule: {:?}", rule);
                        }
                    },
                    AstCwtIdentifierOrString::String(key_str) => {
                        let name = key_str.raw_value();
                        let alias_def = AliasDefinition {
                            category: category.to_string(),
                            name: name.to_string(),
                            to: CwtConverter::convert_value(&rule.value),
                        };
                        self.data
                            .aliases
                            .insert(AliasPattern::new_basic(category, name), alias_def);
                    }
                }
            }
        } else {
            self.data.errors.push(ConversionError::InvalidAliasFormat);
        }
    }

    /// Process a single alias definition
    fn process_single_alias(&mut self, rule: &AstCwtRule) {
        if let Some(identifier) = &rule.key.as_identifier() {
            let name = identifier.name.key.name();
            let alias_type = CwtConverter::convert_value(&rule.value);
            self.data
                .single_aliases
                .insert(name.to_string(), alias_type);
        } else {
            self.data.errors.push(ConversionError::InvalidAliasFormat);
        }
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
