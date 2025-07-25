//! Specialized visitor for CWT alias definitions
//!
//! This visitor handles the processing of CWT alias definitions, including both
//! regular aliases and single aliases.

use std::sync::Arc;

use cw_parser::{AstCwtIdentifierOrString, AstCwtRule, CwtReferenceType, CwtVisitor};

use crate::{
    AliasDefinition, AliasPattern, ConversionError, CwtAnalysisData, CwtConverter, CwtOptions,
    CwtType,
};

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
                let new_to_type = CwtConverter::convert_value(&rule.value, None);
                let options = CwtOptions::from_rule(rule);

                match &identifier.name.key {
                    AstCwtIdentifierOrString::Identifier(key_id) => match key_id.identifier_type {
                        CwtReferenceType::TypeRef => {
                            let name = key_id.name.raw_value();
                            let alias_pattern = AliasPattern::new_type_ref(category, name);
                            self.insert_or_merge_alias(
                                alias_pattern,
                                category,
                                name,
                                new_to_type,
                                options,
                            );
                        }
                        CwtReferenceType::Enum => {
                            let name = key_id.name.raw_value();
                            let alias_pattern = AliasPattern::new_enum(category, name);
                            self.insert_or_merge_alias(
                                alias_pattern,
                                category,
                                name,
                                new_to_type,
                                options,
                            );
                        }
                        _ => {
                            panic!("Unknown identifier type for alias in rule: {:?}", rule);
                        }
                    },
                    AstCwtIdentifierOrString::String(key_str) => {
                        let name = key_str.raw_value();
                        let alias_pattern = AliasPattern::new_basic(category, name);
                        self.insert_or_merge_alias(
                            alias_pattern,
                            category,
                            name,
                            new_to_type,
                            options,
                        );
                    }
                }
            }
        } else {
            self.data.errors.push(ConversionError::InvalidAliasFormat);
        }
    }

    /// Insert or merge an alias definition, creating unions for duplicates
    fn insert_or_merge_alias(
        &mut self,
        alias_pattern: AliasPattern,
        category: &str,
        name: &str,
        new_to_type: Arc<CwtType>,
        options: CwtOptions,
    ) {
        if let Some(existing_def) = self.data.aliases.get_mut(&alias_pattern) {
            // Merge with existing definition by creating a union
            existing_def.to = match &*existing_def.to {
                CwtType::Union(types) => {
                    // Already a union, add the new type
                    let mut new_types = types.clone();
                    new_types.push(new_to_type);
                    CwtType::Union(new_types).into()
                }
                _ => {
                    // Convert to union
                    CwtType::Union(vec![existing_def.to.clone(), new_to_type.clone()]).into()
                }
            };

            // Merge options - could be more sophisticated, but for now just use the latest
            existing_def.options = options;
        } else {
            // First definition, insert as normal
            let alias_def = AliasDefinition {
                category: category.to_string(),
                name: name.to_string(),
                to: new_to_type,
                options,
            };
            self.data.aliases.insert(alias_pattern, alias_def);
        }
    }

    /// Process a single alias definition
    fn process_single_alias(&mut self, rule: &AstCwtRule) {
        if let Some(identifier) = &rule.key.as_identifier() {
            let name = identifier.name.key.name();
            let new_alias_type = CwtConverter::convert_value(&rule.value, None);
            let _options = CwtOptions::from_rule(rule); // Extract options even if not used for single aliases

            if let Some(existing_type) = self.data.single_aliases.get_mut(name) {
                // Merge with existing single alias by creating a union
                let old_type = std::mem::replace(existing_type, Arc::new(CwtType::Unknown));
                *existing_type = match &*old_type {
                    CwtType::Union(types) => {
                        // Already a union, add the new type
                        let mut new_types = types.clone();
                        new_types.push(new_alias_type.clone());
                        CwtType::Union(new_types).into()
                    }
                    _ => {
                        // Convert to union
                        CwtType::Union(vec![old_type.clone(), new_alias_type.clone()]).into()
                    }
                };
            } else {
                // First definition, insert as normal
                self.data
                    .single_aliases
                    .insert(name.to_string(), new_alias_type);
            }
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

#[cfg(test)]
mod tests {
    use cw_parser::CwtModule;

    use super::*;

    #[test]
    fn scope_exists() {
        let mut data = CwtAnalysisData::new();
        let mut visitor = AliasVisitor::new(&mut data);

        let cwt_text = r#"
#any scope
###Checks if a target scope exists
alias[trigger:exists] = scope[any]
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();

        visitor.visit_module(&module);
    }
}
