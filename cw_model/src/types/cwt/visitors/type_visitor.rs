//! Specialized visitor for CWT type definitions
//!
//! This visitor handles the processing of CWT type definitions, including nested
//! subtypes, localisation requirements, and type options.

use std::collections::HashMap;

use cw_parser::{
    AstCwtBlock, AstCwtRule, AstCwtRuleKey, CwtReferenceType, CwtSimpleValueType, CwtValue,
    CwtVisitor,
};

use crate::{
    ConversionError, CwtAnalysisData, CwtConverter, LocalisationRequirement, RuleOptions,
    SkipRootKey, SubtypeCondition, SubtypeDefinition, TypeDefinition, TypeOptions,
};

/// Specialized visitor for type definitions
pub struct TypeVisitor<'a> {
    data: &'a mut CwtAnalysisData,
    in_types_section: bool,
}

impl<'a> TypeVisitor<'a> {
    /// Create a new type visitor
    pub fn new(data: &'a mut CwtAnalysisData) -> Self {
        Self {
            data,
            in_types_section: false,
        }
    }

    /// Set whether we're in a types section
    pub fn set_in_types_section(&mut self, in_section: bool) {
        self.in_types_section = in_section;
    }

    /// Check if this visitor can handle the given rule
    fn can_handle_rule(&self, rule: &AstCwtRule) -> bool {
        match &rule.key {
            AstCwtRuleKey::Identifier(identifier) => {
                // Check for typed identifiers
                matches!(identifier.identifier_type, CwtReferenceType::Type)
            }
            AstCwtRuleKey::String(_) => {
                // String keys are only used for enum variant lists, not type definitions
                false
            }
        }
    }

    /// Process a type definition rule
    fn process_type_definition(&mut self, rule: &AstCwtRule) {
        let type_name = self.extract_type_name(rule);

        if let Some(name) = type_name {
            // Parse rule options
            let options = RuleOptions::from_rule(rule);

            // Convert the type definition
            let mut type_def = TypeDefinition {
                path: None,
                name_field: None,
                skip_root_key: None,
                subtypes: HashMap::new(),
                localisation: HashMap::new(),
                modifiers: HashMap::new(),
                rules: CwtConverter::convert_value(&rule.value),
                options: TypeOptions::default(),
            };

            // Apply cardinality constraints if present
            if let Some(cardinality) = &options.cardinality {
                type_def.rules =
                    CwtConverter::apply_cardinality_constraints(type_def.rules, cardinality);
            }

            // Extract additional type options from the block
            if let CwtValue::Block(block) = &rule.value {
                self.extract_type_options(&mut type_def, block);
            }

            // Store the type definition
            self.data.types.insert(name, type_def);
        } else {
            let key_name = match &rule.key {
                AstCwtRuleKey::Identifier(identifier) => identifier.name.raw_value(),
                AstCwtRuleKey::String(_) => {
                    panic!("String keys should not be used for type definitions")
                }
            };

            self.data
                .errors
                .push(ConversionError::InvalidTypeDefinition(format!(
                    "Could not extract type name from rule: {}",
                    key_name
                )));
        }
    }

    /// Extract the type name from a rule
    fn extract_type_name(&self, rule: &AstCwtRule) -> Option<String> {
        match &rule.key {
            AstCwtRuleKey::Identifier(identifier) => {
                if matches!(identifier.identifier_type, CwtReferenceType::Type) {
                    Some(identifier.name.raw_value().to_string())
                } else {
                    None
                }
            }
            AstCwtRuleKey::String(_) => {
                // String keys are only used for enum variant lists, not type definitions
                panic!("String keys should not be used for type definitions")
            }
        }
    }

    /// Extract type options from a type definition block
    fn extract_type_options(&mut self, type_def: &mut TypeDefinition, block: &AstCwtBlock) {
        for item in &block.items {
            if let cw_parser::cwt::AstCwtExpression::Rule(rule) = item {
                // Check if this is a subtype rule
                if let AstCwtRuleKey::Identifier(identifier) = &rule.key {
                    if matches!(identifier.identifier_type, CwtReferenceType::Subtype) {
                        let subtype_name = identifier.name.raw_value();
                        Self::extract_subtype_definition(type_def, subtype_name, rule);
                        continue;
                    }
                }

                let key = rule.key.name();

                match key {
                    "path" => {
                        if let CwtValue::String(s) = &rule.value {
                            type_def.path = Some(s.raw_value().to_string());
                        }
                    }
                    "name_field" => {
                        if let CwtValue::String(s) = &rule.value {
                            type_def.name_field = Some(s.raw_value().to_string());
                        }
                    }
                    "unique" => match &rule.value {
                        CwtValue::Simple(simple) => {
                            if simple.value_type == CwtSimpleValueType::Bool {
                                type_def.options.unique = true;
                            }
                        }
                        CwtValue::String(s) => {
                            if s.raw_value() == "yes" {
                                type_def.options.unique = true;
                            }
                        }
                        _ => {}
                    },
                    "type_per_file" => match &rule.value {
                        CwtValue::Simple(simple) => {
                            if simple.value_type == CwtSimpleValueType::Bool {
                                type_def.options.type_per_file = true;
                            }
                        }
                        CwtValue::String(s) => {
                            if s.raw_value() == "yes" {
                                type_def.options.type_per_file = true;
                            }
                        }
                        _ => {}
                    },
                    "path_strict" => match &rule.value {
                        CwtValue::Simple(simple) => {
                            if simple.value_type == CwtSimpleValueType::Bool {
                                type_def.options.path_strict = true;
                            }
                        }
                        CwtValue::String(s) => {
                            if s.raw_value() == "yes" {
                                type_def.options.path_strict = true;
                            }
                        }
                        _ => {}
                    },
                    "path_file" => {
                        if let CwtValue::String(s) = &rule.value {
                            type_def.options.path_file = Some(s.raw_value().to_string());
                        }
                    }
                    "path_extension" => {
                        if let CwtValue::String(s) = &rule.value {
                            type_def.options.path_extension = Some(s.raw_value().to_string());
                        }
                    }
                    "localisation" => {
                        if let CwtValue::Block(loc_block) = &rule.value {
                            Self::extract_localisation_requirements(type_def, loc_block);
                        }
                    }
                    "modifiers" => {
                        if let CwtValue::Block(mod_block) = &rule.value {
                            Self::extract_modifier_definitions(type_def, mod_block);
                        }
                    }
                    "skip_root_key" => {
                        Self::extract_skip_root_key(type_def, rule);
                    }
                    _ => {
                        // Regular field in type definition - handled by converter
                    }
                }
            }
        }
    }

    /// Extract localisation requirements from a type definition
    fn extract_localisation_requirements(type_def: &mut TypeDefinition, block: &AstCwtBlock) {
        for item in &block.items {
            if let cw_parser::cwt::AstCwtExpression::Rule(rule) = item {
                let key = rule.key.name();

                if let CwtValue::String(pattern) = &rule.value {
                    let requirement = LocalisationRequirement {
                        pattern: pattern.raw_value().to_string(),
                        required: false, // TODO: Parse from comments/options
                        primary: false,  // TODO: Parse from comments/options
                    };
                    type_def.localisation.insert(key.to_string(), requirement);
                }
            }
        }
    }

    /// Extract modifier definitions from a type definition
    fn extract_modifier_definitions(type_def: &mut TypeDefinition, block: &AstCwtBlock) {
        for item in &block.items {
            if let cw_parser::cwt::AstCwtExpression::Rule(rule) = item {
                let key = rule.key.name();

                if let CwtValue::String(scope) = &rule.value {
                    type_def
                        .modifiers
                        .insert(key.to_string(), scope.raw_value().to_string());
                }
            }
        }
    }

    /// Extract skip_root_key configuration
    fn extract_skip_root_key(type_def: &mut TypeDefinition, rule: &AstCwtRule) {
        match &rule.value {
            CwtValue::String(s) => {
                let value = s.raw_value();
                if value == "any" {
                    type_def.skip_root_key = Some(SkipRootKey::Any);
                } else {
                    type_def.skip_root_key = Some(SkipRootKey::Specific(value.to_string()));
                }
            }
            CwtValue::Block(block) => {
                // Handle more complex skip_root_key configurations
                let mut keys = Vec::new();
                for item in &block.items {
                    if let cw_parser::cwt::AstCwtExpression::String(s) = item {
                        keys.push(s.raw_value().to_string());
                    }
                }
                if !keys.is_empty() {
                    type_def.skip_root_key = Some(SkipRootKey::Multiple(keys));
                }
            }
            _ => {}
        }
    }

    /// Extract subtype definition
    fn extract_subtype_definition(
        type_def: &mut TypeDefinition,
        subtype_name: &str,
        _rule: &AstCwtRule,
    ) {
        let subtype_def = SubtypeDefinition {
            condition: SubtypeCondition::Expression(subtype_name.to_string()), // Convert the subtype rule to a condition
            properties: HashMap::new(), // Will be populated by processing the rule's value
            exclusive: false,           // Default to non-exclusive
            options: Vec::new(),        // No CWT options for now
            display_name: None,
            abbreviation: None,
        };

        type_def
            .subtypes
            .insert(subtype_name.to_string(), subtype_def);
    }
}

impl<'a> CwtVisitor<'a> for TypeVisitor<'a> {
    fn visit_rule(&mut self, rule: &AstCwtRule<'a>) {
        if self.can_handle_rule(rule) {
            self.process_type_definition(rule);
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
    fn test_type_visitor() {
        let mut data = CwtAnalysisData::new();
        let mut visitor = TypeVisitor::new(&mut data);

        let cwt_text = r#"
types = {
    type[test_type] = {
        path = "test_path"
        name_field = "test_name_field"
        unique = yes
    }
}
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();
        let types_rules = module.find_rule("types");
        let types_rules = types_rules.unwrap();

        visitor.visit_rule(types_rules);

        assert_eq!(data.types.len(), 1);
        assert_eq!(
            data.types.get("test_type").unwrap().path,
            Some("test_path".to_string())
        );
        assert_eq!(
            data.types.get("test_type").unwrap().name_field,
            Some("test_name_field".to_string())
        );
        assert_eq!(data.types.get("test_type").unwrap().options.unique, true);
    }

    #[test]
    fn test_complex_type_visitor() {
        let mut data = CwtAnalysisData::new();
        let mut visitor = TypeVisitor::new(&mut data);

        let cwt_text = r#"
types = {
    type[opinion_modifier] = {
        path = "game/common/opinion_modifiers"
        path_strict = yes
        type_per_file = yes
        
        subtype[triggered_opinion_modifier] = {
            trigger = { }
        }
        subtype[block_triggered] = {
            block_triggered = yes
        }
        
        localisation = {
            Name = "$"
            Description = "$_desc"
        }
        
        modifiers = {
            "$_modifier" = country
            "$_opinion_boost" = diplomacy
        }
        
        skip_root_key = any
    }
    
    type[static_modifier] = {
        path = "game/common/static_modifiers"
        path_extension = ".txt"
        unique = no
        
        subtype[planet] = {
            icon_frame = int
        }
        
        localisation = {
            Name = "$"
            Description = "$_desc"
        }
        
        modifiers = {
            "$_boost" = planet
        }
    }
}
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();
        let types_rules = module.find_rule("types");
        let types_rules = types_rules.unwrap();

        visitor.visit_rule(types_rules);

        // Check that we have 2 types
        assert_eq!(data.types.len(), 2);

        // Test opinion_modifier type
        let opinion_modifier = data.types.get("opinion_modifier").unwrap();
        assert_eq!(
            opinion_modifier.path,
            Some("game/common/opinion_modifiers".to_string())
        );
        assert_eq!(opinion_modifier.options.path_strict, true);
        assert_eq!(opinion_modifier.options.type_per_file, true);
        assert_eq!(opinion_modifier.skip_root_key, Some(SkipRootKey::Any));

        // Check subtypes
        assert_eq!(opinion_modifier.subtypes.len(), 2);
        assert!(
            opinion_modifier
                .subtypes
                .contains_key("triggered_opinion_modifier")
        );
        assert!(opinion_modifier.subtypes.contains_key("block_triggered"));

        // Check localisation
        assert_eq!(opinion_modifier.localisation.len(), 2);
        assert_eq!(
            opinion_modifier.localisation.get("Name").unwrap().pattern,
            "$"
        );
        assert_eq!(
            opinion_modifier
                .localisation
                .get("Description")
                .unwrap()
                .pattern,
            "$_desc"
        );

        // Check modifiers
        assert_eq!(opinion_modifier.modifiers.len(), 2);
        assert_eq!(
            opinion_modifier.modifiers.get("$_modifier").unwrap(),
            "country"
        );
        assert_eq!(
            opinion_modifier.modifiers.get("$_opinion_boost").unwrap(),
            "diplomacy"
        );

        // Test static_modifier type
        let static_modifier = data.types.get("static_modifier").unwrap();
        assert_eq!(
            static_modifier.path,
            Some("game/common/static_modifiers".to_string())
        );
        assert_eq!(
            static_modifier.options.path_extension,
            Some(".txt".to_string())
        );
        assert_eq!(static_modifier.options.unique, false);

        // Check subtypes
        assert_eq!(static_modifier.subtypes.len(), 1);
        assert!(static_modifier.subtypes.contains_key("planet"));

        // Check localisation
        assert_eq!(static_modifier.localisation.len(), 2);
        assert_eq!(
            static_modifier.localisation.get("Name").unwrap().pattern,
            "$"
        );
        assert_eq!(
            static_modifier
                .localisation
                .get("Description")
                .unwrap()
                .pattern,
            "$_desc"
        );

        // Check modifiers
        assert_eq!(static_modifier.modifiers.len(), 1);
        assert_eq!(static_modifier.modifiers.get("$_boost").unwrap(), "planet");
    }
}
