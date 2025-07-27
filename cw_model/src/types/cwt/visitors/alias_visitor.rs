//! Specialized visitor for CWT alias definitions
//!
//! This visitor handles the processing of CWT alias definitions, including both
//! regular aliases and single aliases.

use std::sync::Arc;

use cw_parser::{AstCwtIdentifierOrString, AstCwtRule, CwtReferenceType, CwtVisitor};
use lasso::Spur;

use crate::{
    AliasDefinition, AliasPattern, CaseInsensitiveInterner, ConversionError, CwtAnalysisData,
    CwtConverter, CwtOptions, CwtType,
};

/// Specialized visitor for alias definitions
pub struct AliasVisitor<'a, 'interner> {
    data: &'a mut CwtAnalysisData,
    interner: &'interner CaseInsensitiveInterner,
}

impl<'a, 'interner> AliasVisitor<'a, 'interner> {
    /// Create a new alias visitor
    pub fn new(
        data: &'a mut CwtAnalysisData,
        interner: &'interner CaseInsensitiveInterner,
    ) -> Self {
        Self { data, interner }
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
                let category = self.interner.get_or_intern(scope.raw_value());
                let new_to_type = CwtConverter::convert_value(&rule.value, None, self.interner);
                let options = CwtOptions::from_rule(rule, self.interner);

                match &identifier.name.key {
                    AstCwtIdentifierOrString::Identifier(key_id) => match key_id.identifier_type {
                        CwtReferenceType::TypeRef => {
                            let name = self.interner.get_or_intern(key_id.name.raw_value());
                            let alias_pattern =
                                AliasPattern::new_type_ref(category, name, self.interner);
                            self.insert_or_merge_alias(
                                alias_pattern,
                                category,
                                name,
                                new_to_type,
                                options,
                            );
                        }
                        CwtReferenceType::Enum => {
                            let name = self.interner.get_or_intern(key_id.name.raw_value());
                            let alias_pattern =
                                AliasPattern::new_enum(category, name, self.interner);
                            self.insert_or_merge_alias(
                                alias_pattern,
                                category,
                                name,
                                new_to_type,
                                options,
                            );
                        }
                        CwtReferenceType::TypeRefWithPrefixSuffix(prefix, suffix) => {
                            let name = self.interner.get_or_intern(key_id.name.raw_value());
                            let alias_pattern = AliasPattern::new_type_ref_with_prefix_suffix(
                                category,
                                name,
                                prefix.map(|p| self.interner.get_or_intern(p)),
                                suffix.map(|s| self.interner.get_or_intern(s)),
                                self.interner,
                            );
                            self.insert_or_merge_alias(
                                alias_pattern,
                                category,
                                name,
                                new_to_type,
                                options,
                            );
                        }
                        ref unknown => {
                            panic!("Unknown identifier type for alias in rule: {:?}", unknown);
                        }
                    },
                    AstCwtIdentifierOrString::String(key_str) => {
                        let name = self.interner.get_or_intern(key_str.raw_value());
                        let alias_pattern = AliasPattern::new_basic(category, name, self.interner);
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
        category: Spur,
        name: Spur,
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
                category,
                name,
                to: new_to_type,
                options,
            };
            self.data.aliases.insert(alias_pattern, alias_def);
        }
    }

    /// Process a single alias definition
    fn process_single_alias(&mut self, rule: &AstCwtRule) {
        if let Some(identifier) = &rule.key.as_identifier() {
            let name = self.interner.get_or_intern(identifier.name.key.name());
            let new_alias_type = CwtConverter::convert_value(&rule.value, None, self.interner);
            let _options = CwtOptions::from_rule(rule, self.interner); // Extract options even if not used for single aliases

            if let Some(existing_type) = self.data.single_aliases.get_mut(&name) {
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
                self.data.single_aliases.insert(name, new_alias_type);
            }
        } else {
            self.data.errors.push(ConversionError::InvalidAliasFormat);
        }
    }
}

impl<'a, 'interner> CwtVisitor<'a> for AliasVisitor<'a, 'interner> {
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
        let interner = CaseInsensitiveInterner::new();
        let mut visitor = AliasVisitor::new(&mut data, &interner);

        let cwt_text = r#"
#any scope
###Checks if a target scope exists
alias[trigger:exists] = scope[any]
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();

        visitor.visit_module(&module);
    }
}
