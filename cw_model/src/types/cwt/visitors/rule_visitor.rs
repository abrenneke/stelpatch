//! Specialized visitor for CWT rule definitions
//!
//! This visitor handles the processing of regular CWT rule definitions that define
//! validation structure for game entities (e.g., ambient_object, asteroid_belt_type).

use cw_parser::{AstCwtIdentifierOrString, AstCwtRule, CwtVisitor};

use crate::{ConversionError, CwtAnalysisData, CwtConverter, CwtType, RuleOptions, TypeDefinition};

/// Specialized visitor for regular rule definitions
pub struct RuleVisitor<'a> {
    data: &'a mut CwtAnalysisData,
}

impl<'a> RuleVisitor<'a> {
    /// Create a new rule visitor
    pub fn new(data: &'a mut CwtAnalysisData) -> Self {
        Self { data }
    }

    /// Check if this visitor can handle the given rule
    fn can_handle_rule(&self, rule: &AstCwtRule) -> bool {
        match &rule.key {
            AstCwtIdentifierOrString::Identifier(_) => {
                // Don't handle typed identifiers - those are handled by specialized visitors
                false
            }
            AstCwtIdentifierOrString::String(_) => {
                // Handle string-based rule keys (regular rule definitions)
                true
            }
        }
    }

    /// Process a regular rule definition
    fn process_rule_definition(&mut self, rule: &AstCwtRule) {
        let rule_name = self.extract_rule_name(rule);

        if let Some(name) = rule_name {
            // Skip special section names that are handled by other visitors
            if name == "types" || name == "enums" || name == "links" {
                return;
            }

            // Parse rule options
            let options = RuleOptions::from_rule(rule);

            // Convert the rule definition to an inferred type
            let rule_type = CwtConverter::convert_value(&rule.value);

            // Store the rule definition as a TypeDefinition (merge with existing if present)
            let mut type_def = TypeDefinition::new(rule_type);
            type_def.rule_options = options;
            self.data.insert_or_merge_type(name.clone(), type_def);
        } else {
            let key_name = match &rule.key {
                AstCwtIdentifierOrString::Identifier(identifier) => identifier.name.raw_value(),
                AstCwtIdentifierOrString::String(string) => string.raw_value(),
            };

            self.data
                .errors
                .push(ConversionError::InvalidRuleDefinition(format!(
                    "Could not extract rule name from rule: {}",
                    key_name
                )));
        }
    }

    /// Extract the rule name from a rule
    fn extract_rule_name(&self, rule: &AstCwtRule) -> Option<String> {
        match &rule.key {
            AstCwtIdentifierOrString::Identifier(_) => {
                // Don't handle typed identifiers - those are handled by specialized visitors
                None
            }
            AstCwtIdentifierOrString::String(string) => {
                // Handle string-based rule keys (regular rule definitions)
                Some(string.raw_value().to_string())
            }
        }
    }
}

impl<'a> CwtVisitor<'a> for RuleVisitor<'a> {
    fn visit_rule(&mut self, rule: &AstCwtRule<'a>) {
        if self.can_handle_rule(rule) {
            self.process_rule_definition(rule);
        }
    }
}

#[cfg(test)]
mod tests {
    use cw_parser::CwtModule;

    use super::*;

    #[test]
    fn test_rule_visitor_basic() {
        let mut data = CwtAnalysisData::new();
        let mut visitor = RuleVisitor::new(&mut data);

        let cwt_text = r#"
ambient_object = {
    name = localisation
    entity = <model_entity>
    selectable = bool
}

asteroid_belt_type = {
    mesh = scalar
    shader = scalar
    width = float
    density = float
}
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();
        visitor.visit_module(&module);

        // Check that we have 2 types (formerly rules)
        assert_eq!(data.types.len(), 2);
        assert!(data.types.contains_key("ambient_object"));
        assert!(data.types.contains_key("asteroid_belt_type"));
    }

    #[test]
    fn test_rule_visitor_with_options() {
        let mut data = CwtAnalysisData::new();
        let mut visitor = RuleVisitor::new(&mut data);

        let cwt_text = r#"
## cardinality = 0..1
attitude = {
    type = scalar
    behaviour = {
        attack = bool
        weaken = bool
    }
}
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();
        visitor.visit_module(&module);

        // Check that we have 1 type (formerly rule)
        assert_eq!(data.types.len(), 1);
        assert!(data.types.contains_key("attitude"));
    }

    #[test]
    fn test_rule_visitor_ignores_special_sections() {
        let mut data = CwtAnalysisData::new();
        let mut visitor = RuleVisitor::new(&mut data);

        let cwt_text = r#"
types = {
    type[test_type] = {
        path = "test_path"
    }
}

enums = {
    enum[test_enum] = {
        value1
        value2
    }
}

ambient_object = {
    name = localisation
}
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();
        visitor.visit_module(&module);

        // Check that we only have the ambient_object type, not the special sections
        assert_eq!(data.types.len(), 1);
        assert!(data.types.contains_key("ambient_object"));
        assert!(!data.types.contains_key("types"));
        assert!(!data.types.contains_key("enums"));
    }

    #[test]
    fn test_rule_visitor_with_string_keys() {
        let mut data = CwtAnalysisData::new();
        let mut visitor = RuleVisitor::new(&mut data);

        let cwt_text = r#"
"string_key_rule" = {
    field1 = scalar
    field2 = int
}
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();
        visitor.visit_module(&module);

        // Check that we have 1 type with string key
        assert_eq!(data.types.len(), 1);
        assert!(data.types.contains_key("string_key_rule"));
    }

    #[test]
    fn test_rule_visitor_with_nested_types() {
        let mut data = CwtAnalysisData::new();
        let mut visitor = RuleVisitor::new(&mut data);

        let cwt_text = r#"
tradable_actions = {
    fire_and_forget = bool
    ai_weight = {
        weight = float
        modifier = {
            factor = float
            weight = float
        }
    }
}
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();
        visitor.visit_module(&module);

        // Check that we have the main type
        assert_eq!(data.types.len(), 1);
        assert!(data.types.contains_key("tradable_actions"));

        // Verify the nested structure exists
        let tradable_actions_type = &data.types["tradable_actions"];
        if let CwtType::Block(block) = &tradable_actions_type.rules {
            // Should have ai_weight property
            assert!(block.properties.contains_key("ai_weight"));

            // ai_weight should itself be a block
            let ai_weight_prop = &block.properties["ai_weight"];
            if let CwtType::Block(ai_weight_block) = &ai_weight_prop.property_type {
                // Should have modifier property
                assert!(ai_weight_block.properties.contains_key("modifier"));

                // modifier should be a block with factor and weight
                let modifier_prop = &ai_weight_block.properties["modifier"];
                if let CwtType::Block(modifier_block) = &modifier_prop.property_type {
                    assert!(modifier_block.properties.contains_key("factor"));
                    assert!(modifier_block.properties.contains_key("weight"));
                } else {
                    panic!("modifier should be a block type");
                }
            } else {
                panic!("ai_weight should be a block type");
            }
        } else {
            panic!("tradable_actions should be a block type");
        }
    }
}
