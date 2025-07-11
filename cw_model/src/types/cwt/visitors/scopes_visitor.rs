//! Specialized visitor for CWT scopes definitions
//!
//! This visitor handles the processing of CWT scopes definitions, which define
//! scope types and scope groups used in script validation.

use cw_parser::{AstCwtRule, CwtValue, CwtVisitor};

use crate::{ConversionError, CwtAnalysisData, ScopeDefinition, ScopeGroupDefinition};

/// Specialized visitor for scopes definitions
pub struct ScopesVisitor<'a> {
    data: &'a mut CwtAnalysisData,
}

impl<'a> ScopesVisitor<'a> {
    /// Create a new scopes visitor
    pub fn new(data: &'a mut CwtAnalysisData) -> Self {
        Self { data }
    }

    /// Check if this visitor can handle the given rule
    fn can_handle_rule(&self, rule: &AstCwtRule) -> bool {
        // Check if this is a scopes or scope_groups section
        let key = rule.key.name();
        key == "scopes" || key == "scope_groups"
    }

    /// Process a scopes or scope_groups section
    fn process_scopes_section(&mut self, rule: &AstCwtRule) {
        let key = rule.key.name();

        match key {
            "scopes" => self.process_scopes_block(rule),
            "scope_groups" => self.process_scope_groups_block(rule),
            _ => {}
        }
    }

    /// Process a scopes block
    fn process_scopes_block(&mut self, rule: &AstCwtRule) {
        if let CwtValue::Block(block) = &rule.value {
            for item in &block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(scope_rule) = item {
                    self.process_scope_definition(scope_rule);
                }
            }
        }
    }

    /// Process a scope_groups block
    fn process_scope_groups_block(&mut self, rule: &AstCwtRule) {
        if let CwtValue::Block(block) = &rule.value {
            for item in &block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(group_rule) = item {
                    self.process_scope_group_definition(group_rule);
                }
            }
        }
    }

    /// Process a single scope definition
    fn process_scope_definition(&mut self, rule: &AstCwtRule) {
        let scope_name = rule.key.name();

        if let CwtValue::Block(block) = &rule.value {
            let mut aliases = Vec::new();

            // Parse the scope properties
            for item in &block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(prop_rule) = item {
                    let prop_key = prop_rule.key.name();

                    if prop_key == "aliases" {
                        if let Some(parsed_aliases) = self.parse_aliases_list(&prop_rule.value) {
                            aliases = parsed_aliases;
                        }
                    }
                }
            }

            // Create and store the scope definition
            let scope_def = ScopeDefinition::new(scope_name.to_string(), aliases);
            self.data.scopes.insert(scope_name.to_string(), scope_def);
        } else {
            self.data
                .errors
                .push(ConversionError::InvalidScopeFormat(format!(
                    "Scope '{}' must have a block value",
                    scope_name
                )));
        }
    }

    /// Process a single scope group definition
    fn process_scope_group_definition(&mut self, rule: &AstCwtRule) {
        let group_name = rule.key.name();

        if let CwtValue::Block(block) = &rule.value {
            let mut members = Vec::new();

            // Parse the scope group members (they're just listed as values)
            for item in &block.items {
                if let cw_parser::cwt::AstCwtExpression::Value(value) = item {
                    if let Some(member) = self.parse_scope_member(value) {
                        members.push(member);
                    }
                }
            }

            // Create and store the scope group definition
            let group_def = ScopeGroupDefinition::new(group_name.to_string(), members);
            self.data
                .scope_groups
                .insert(group_name.to_string(), group_def);
        } else {
            self.data
                .errors
                .push(ConversionError::InvalidScopeFormat(format!(
                    "Scope group '{}' must have a block value",
                    group_name
                )));
        }
    }

    /// Parse a list of aliases from a CWT value
    fn parse_aliases_list(&self, value: &CwtValue) -> Option<Vec<String>> {
        match value {
            CwtValue::Block(block) => {
                let mut aliases = Vec::new();
                for item in &block.items {
                    if let cw_parser::cwt::AstCwtExpression::Value(val) = item {
                        if let Some(alias) = self.parse_scope_member(val) {
                            aliases.push(alias);
                        }
                    }
                }
                Some(aliases)
            }
            _ => None,
        }
    }

    /// Parse a single scope member from a CWT value
    fn parse_scope_member(&self, value: &CwtValue) -> Option<String> {
        match value {
            CwtValue::String(s) => Some(s.raw_value().to_string()),
            _ => None,
        }
    }
}

impl<'a> CwtVisitor<'a> for ScopesVisitor<'a> {
    fn visit_rule(&mut self, rule: &AstCwtRule<'a>) {
        if self.can_handle_rule(rule) {
            self.process_scopes_section(rule);
        }

        // Continue walking for nested rules
        self.walk_rule(rule);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cw_parser::CwtModule;

    #[test]
    fn test_scopes_visitor() {
        let mut data = CwtAnalysisData::new();
        let mut visitor = ScopesVisitor::new(&mut data);

        let cwt_text = r#"
scopes = {
    Country = {
        aliases = { country }
    }
    Leader = {
        aliases = { leader }
    }
}

scope_groups = {
    celestial_coordinate = {
        planet ship fleet system
    }
}
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();

        // Test scopes section
        if let Some(scopes_rule) = module.find_rule("scopes") {
            visitor.visit_rule(scopes_rule);
        }

        // Test scope_groups section
        if let Some(scope_groups_rule) = module.find_rule("scope_groups") {
            visitor.visit_rule(scope_groups_rule);
        }

        assert_eq!(data.scopes.len(), 2);
        assert_eq!(data.scope_groups.len(), 1);

        let country_scope = data.scopes.get("Country").unwrap();
        assert_eq!(country_scope.aliases, vec!["country"]);

        let leader_scope = data.scopes.get("Leader").unwrap();
        assert_eq!(leader_scope.aliases, vec!["leader"]);

        let celestial_group = data.scope_groups.get("celestial_coordinate").unwrap();
        assert_eq!(
            celestial_group.members,
            vec!["planet", "ship", "fleet", "system"]
        );
    }
}
