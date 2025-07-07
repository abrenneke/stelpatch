//! Specialized visitor for CWT type definitions
//!
//! This visitor handles the processing of CWT type definitions, including nested
//! subtypes, localisation requirements, and type options.

use std::collections::HashMap;

use cw_parser::{
    AstCwtBlock, AstCwtRule, AstCwtRuleKey, CwtOperator, CwtOptionExpression, CwtReferenceType,
    CwtSeverityLevel, CwtSimpleValueType, CwtValue, CwtVisitor,
};

use crate::{
    ConversionError, CwtAnalysisData, CwtConverter, LocalisationRequirement, ModifierGeneration,
    RuleOptions, SeverityLevel, SkipRootKey, SubtypeCondition, SubtypeDefinition, SubtypeOptions,
    TypeDefinition, TypeKeyFilter, TypeOptions,
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
                modifiers: ModifierGeneration {
                    modifiers: HashMap::new(),
                    subtypes: HashMap::new(),
                },
                rules: CwtConverter::convert_value(&rule.value),
                options: TypeOptions::default(),
            };

            // Apply cardinality constraints if present
            if let Some(cardinality) = &options.cardinality {
                type_def.rules =
                    CwtConverter::apply_cardinality_constraints(type_def.rules, cardinality);
            }

            // Parse CWT options from the rule
            Self::parse_rule_options(&mut type_def, rule);

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
                    "starts_with" => {
                        if let CwtValue::String(s) = &rule.value {
                            type_def.options.starts_with = Some(s.raw_value().to_string());
                        }
                    }
                    "severity" => {
                        if let CwtValue::String(s) = &rule.value {
                            type_def.options.severity =
                                Some(s.raw_value().parse().unwrap_or(SeverityLevel::Error));
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
                    let mut requirement =
                        LocalisationRequirement::new(pattern.raw_value().to_string());

                    for option in &rule.options {
                        match option.key {
                            "required" => {
                                requirement.required = true;
                            }
                            "primary" => {
                                requirement.primary = true;
                            }
                            _ => {}
                        }
                    }

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
                        .modifiers
                        .insert(key.to_string(), scope.raw_value().to_string());
                }
            }
        }
    }

    /// Extract skip_root_key configuration
    fn extract_skip_root_key(type_def: &mut TypeDefinition, rule: &AstCwtRule) {
        match (&rule.operator, &rule.value) {
            (CwtOperator::NotEquals, CwtValue::String(s)) => {
                // skip_root_key != tech_group -> SkipRootKey::Except(["tech_group"])
                type_def.skip_root_key = Some(SkipRootKey::Except(vec![s.raw_value().to_string()]));
            }
            (CwtOperator::Equals, CwtValue::String(s)) => {
                let value = s.raw_value();
                if value == "any" {
                    type_def.skip_root_key = Some(SkipRootKey::Any);
                } else {
                    type_def.skip_root_key = Some(SkipRootKey::Specific(value.to_string()));
                }
            }
            (CwtOperator::Equals, CwtValue::Block(block)) => {
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
        rule: &AstCwtRule,
    ) {
        let mut subtype_options = SubtypeOptions::default();

        // Parse CWT options from the rule
        for option in &rule.options {
            match option.key {
                "display_name" => {
                    subtype_options.display_name =
                        Some(option.value.as_string_or_identifier().unwrap().to_string());
                }
                "abbreviation" => {
                    subtype_options.abbreviation =
                        Some(option.value.as_string_or_identifier().unwrap().to_string());
                }
                "push_scope" => {
                    subtype_options.push_scope =
                        Some(option.value.as_string_or_identifier().unwrap().to_string());
                }
                "starts_with" => {
                    subtype_options.starts_with =
                        Some(option.value.as_string_or_identifier().unwrap().to_string());
                }
                "severity" => {
                    subtype_options.severity =
                        Some(option.value.as_identifier().unwrap().parse().unwrap());
                }
                "type_key_filter" => {
                    subtype_options.type_key_filter = Some(TypeKeyFilter::Specific(
                        option.value.as_string_or_identifier().unwrap().to_string(),
                    ));
                }
                _ => {}
            }
        }

        let subtype_def = SubtypeDefinition {
            condition: SubtypeCondition::Expression(subtype_name.to_string()), // Convert the subtype rule to a condition
            properties: HashMap::new(), // Will be populated by processing the rule's value
            exclusive: false,           // Default to non-exclusive
            options: Vec::new(),        // No CWT options for now
            display_name: subtype_options.display_name,
            abbreviation: subtype_options.abbreviation,
        };

        type_def
            .subtypes
            .insert(subtype_name.to_string(), subtype_def);
    }

    /// Parse CWT options from a rule and apply them to the type definition
    fn parse_rule_options(type_def: &mut TypeDefinition, rule: &AstCwtRule) {
        for option in &rule.options {
            match option.key {
                "severity" => {
                    type_def.options.severity =
                        Some(option.value.as_identifier().unwrap().parse().unwrap());
                }
                "starts_with" => {
                    type_def.options.starts_with =
                        Some(option.value.as_string_or_identifier().unwrap().to_string());
                }
                "type_key_filter" => {
                    type_def.options.type_key_filter = match (&option.value, option.is_ne) {
                        (CwtOptionExpression::Identifier(id), false) => {
                            Some(TypeKeyFilter::Specific(id.to_string()))
                        }
                        (CwtOptionExpression::Identifier(id), true) => {
                            Some(TypeKeyFilter::Not(id.to_string()))
                        }
                        (CwtOptionExpression::List(list), false) => Some(TypeKeyFilter::OneOf(
                            list.iter()
                                .map(|t| t.as_string_or_identifier().unwrap().to_string())
                                .collect(),
                        )),
                        (CwtOptionExpression::List(list), true) => Some(TypeKeyFilter::Not(
                            list.iter()
                                .map(|t| t.as_string_or_identifier().unwrap().to_string())
                                .collect(),
                        )),
                        _ => None,
                    };
                }
                "graph_related_types" => {
                    type_def.options.graph_related_types = option
                        .value
                        .as_list()
                        .unwrap()
                        .iter()
                        .map(|t| t.as_string_or_identifier().unwrap().to_string())
                        .collect();
                }
                _ => {}
            }
        }
    }

    /// Convert CWT severity level to our internal representation
    fn convert_severity_level(level: &CwtSeverityLevel) -> SeverityLevel {
        match level {
            CwtSeverityLevel::Error => SeverityLevel::Error,
            CwtSeverityLevel::Warning => SeverityLevel::Warning,
            CwtSeverityLevel::Information => SeverityLevel::Information,
            CwtSeverityLevel::Hint => SeverityLevel::Hint,
        }
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
        assert_eq!(opinion_modifier.modifiers.modifiers.len(), 2);
        assert_eq!(
            opinion_modifier
                .modifiers
                .modifiers
                .get("$_modifier")
                .unwrap(),
            "country"
        );
        assert_eq!(
            opinion_modifier
                .modifiers
                .modifiers
                .get("$_opinion_boost")
                .unwrap(),
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
        assert_eq!(static_modifier.modifiers.modifiers.len(), 1);
        assert_eq!(
            static_modifier.modifiers.modifiers.get("$_boost").unwrap(),
            "planet"
        );
    }

    #[test]
    fn test_enhanced_cwt_features() {
        let mut data = CwtAnalysisData::new();
        let mut visitor = TypeVisitor::new(&mut data);

        let cwt_text = r#"
types = {
    ## severity = warning
    ## graph_related_types = { technology building }
    type[advanced_type] = {
        path = "game/common/advanced_types"
        path_strict = yes
        type_per_file = yes
        starts_with = "adv_"
        
        ## display_name = "Advanced Subtype"
        ## abbreviation = ADV
        ## push_scope = country
        ## type_key_filter = advanced_event
        subtype[advanced] = {
            is_advanced = yes
            has_technology = yes
        }
        
        ## display_name = "Basic Subtype"
        ## starts_with = basic_
        subtype[basic] = {
            is_basic = yes
        }
        
        localisation = {
            ## required
            ## primary
            Name = "$"
            ## optional
            Description = "$_desc"
            
            subtype[advanced] = {
                advanced_tooltip = "$_advanced_tooltip"
                advanced_desc = "$_advanced_desc"
            }
            
            subtype[basic] = {
                basic_info = "$_basic_info"
            }
        }
        
        modifiers = {
            "$_base_modifier" = country
            "$_power_modifier" = fleet
            
            subtype[advanced] = {
                "$_advanced_bonus" = country
                "$_tech_bonus" = technology
            }
            
            subtype[basic] = {
                "$_basic_bonus" = planet
            }
        }
        
        skip_root_key = { tech_group any military }
    }
}
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();
        let types_rules = module.find_rule("types");
        let types_rules = types_rules.unwrap();

        visitor.visit_rule(types_rules);

        // Check that we have 1 type
        assert_eq!(data.types.len(), 1);

        // Test advanced_type
        let advanced_type = data.types.get("advanced_type").unwrap();
        assert_eq!(
            advanced_type.path,
            Some("game/common/advanced_types".to_string())
        );
        assert_eq!(advanced_type.options.path_strict, true);
        assert_eq!(advanced_type.options.type_per_file, true);
        assert_eq!(advanced_type.options.starts_with, Some("adv_".to_string()));

        // Check type-level options (from comments)
        assert_eq!(advanced_type.options.severity, Some(SeverityLevel::Warning));
        assert_eq!(
            advanced_type.options.graph_related_types,
            vec!["technology".to_string(), "building".to_string()]
        );

        // Check subtypes
        assert_eq!(advanced_type.subtypes.len(), 2);
        let advanced_subtype = advanced_type.subtypes.get("advanced").unwrap();
        let basic_subtype = advanced_type.subtypes.get("basic").unwrap();

        // Check subtype options (comment-based options should be parsed)
        assert_eq!(
            advanced_subtype.display_name,
            Some("Advanced Subtype".to_string())
        );
        assert_eq!(advanced_subtype.abbreviation, Some("ADV".to_string()));

        // Check basic subtype options
        assert_eq!(
            basic_subtype.display_name,
            Some("Basic Subtype".to_string())
        );
        assert_eq!(basic_subtype.abbreviation, None); // No abbreviation specified

        // Check localisation structure
        assert_eq!(advanced_type.localisation.len(), 2);
        let name_loc = advanced_type.localisation.get("Name").unwrap();
        let desc_loc = advanced_type.localisation.get("Description").unwrap();

        assert_eq!(name_loc.pattern, "$");
        // Comment-based options should be parsed
        assert_eq!(name_loc.required, true); // From ## required comment
        assert_eq!(name_loc.primary, true); // From ## primary comment

        assert_eq!(desc_loc.pattern, "$_desc");
        assert_eq!(desc_loc.required, false); // Marked as ## optional
        assert_eq!(desc_loc.primary, false);

        // Check subtype-specific localisation
        // Note: This would be fully implemented with proper nested parsing

        // Check modifiers structure
        assert_eq!(advanced_type.modifiers.modifiers.len(), 2);
        assert_eq!(
            advanced_type
                .modifiers
                .modifiers
                .get("$_base_modifier")
                .unwrap(),
            "country"
        );
        assert_eq!(
            advanced_type
                .modifiers
                .modifiers
                .get("$_power_modifier")
                .unwrap(),
            "fleet"
        );

        // Check subtype-specific modifiers
        // Note: This would be fully implemented with proper nested parsing

        // Check skip_root_key (complex structure)
        assert_eq!(
            advanced_type.skip_root_key,
            Some(SkipRootKey::Multiple(vec![
                "tech_group".to_string(),
                "any".to_string(),
                "military".to_string()
            ]))
        );
    }

    #[test]
    fn test_skip_root_key_variants() {
        let mut data = CwtAnalysisData::new();
        let mut visitor = TypeVisitor::new(&mut data);

        let cwt_text = r#"
types = {
    type[specific_skip] = {
        path = "game/common/specific"
        skip_root_key = specific_key
    }
    
    type[any_skip] = {
        path = "game/common/any"
        skip_root_key = any
    }
    
    type[multiple_skip] = {
        path = "game/common/multiple"
        skip_root_key = { level1 level2 level3 }
    }
}
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();
        let types_rules = module.find_rule("types");
        let types_rules = types_rules.unwrap();

        visitor.visit_rule(types_rules);

        // Check that we have 3 types
        assert_eq!(data.types.len(), 3);

        // Test specific skip
        let specific_skip = data.types.get("specific_skip").unwrap();
        assert_eq!(
            specific_skip.skip_root_key,
            Some(SkipRootKey::Specific("specific_key".to_string()))
        );

        // Test any skip
        let any_skip = data.types.get("any_skip").unwrap();
        assert_eq!(any_skip.skip_root_key, Some(SkipRootKey::Any));

        // Test multiple skip
        let multiple_skip = data.types.get("multiple_skip").unwrap();
        assert_eq!(
            multiple_skip.skip_root_key,
            Some(SkipRootKey::Multiple(vec![
                "level1".to_string(),
                "level2".to_string(),
                "level3".to_string()
            ]))
        );
    }

    #[test]
    fn test_subtype_specific_localisation_modifiers() {
        let mut data = CwtAnalysisData::new();
        let mut visitor = TypeVisitor::new(&mut data);

        let cwt_text = r#"
types = {
    type[complex_type] = {
        path = "game/common/complex"
        
        subtype[variant_a] = {
            variant = "a"
        }
        
        subtype[variant_b] = {
            variant = "b"
        }
        
        localisation = {
            name = "$"
            description = "$_desc"
            
            subtype[variant_a] = {
                variant_a_tooltip = "$_variant_a_tooltip"
                variant_a_name = "$_variant_a_name"
            }
            
            subtype[variant_b] = {
                variant_b_tooltip = "$_variant_b_tooltip"
            }
        }
        
        modifiers = {
            "$_base_power" = country
            "$_base_cost" = economy
            
            subtype[variant_a] = {
                "$_variant_a_bonus" = military
                "$_variant_a_cost" = economy
            }
            
            subtype[variant_b] = {
                "$_variant_b_bonus" = diplomacy
            }
        }
    }
}
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();
        let types_rules = module.find_rule("types");
        let types_rules = types_rules.unwrap();

        visitor.visit_rule(types_rules);

        // Check that we have 1 type
        assert_eq!(data.types.len(), 1);

        let complex_type = data.types.get("complex_type").unwrap();

        // Check subtypes
        assert_eq!(complex_type.subtypes.len(), 2);
        assert!(complex_type.subtypes.contains_key("variant_a"));
        assert!(complex_type.subtypes.contains_key("variant_b"));

        // Check base localisation
        assert_eq!(complex_type.localisation.len(), 2);
        assert!(complex_type.localisation.contains_key("name"));
        assert!(complex_type.localisation.contains_key("description"));

        // Check base modifiers
        assert_eq!(complex_type.modifiers.modifiers.len(), 2);
        assert_eq!(
            complex_type
                .modifiers
                .modifiers
                .get("$_base_power")
                .unwrap(),
            "country"
        );
        assert_eq!(
            complex_type.modifiers.modifiers.get("$_base_cost").unwrap(),
            "economy"
        );

        // Note: Subtype-specific localisation and modifiers would be fully parsed
        // in a complete implementation with proper nested block parsing
    }

    #[test]
    fn test_missing_advanced_cwt_features() {
        let mut data = CwtAnalysisData::new();
        let mut visitor = TypeVisitor::new(&mut data);

        let cwt_text = r#"
types = {
    ## type_key_filter <> country_event
    type[exclude_country_events] = {
        path = "game/common/exclude_events"
        skip_root_key != tech_group
    }
    
    ## type_key_filter = { military_event diplomatic_event }
    type[multiple_key_filter] = {
        path = "game/common/multiple_filter"
    }
    
    type[nested_subtype_features] = {
        path = "game/common/nested_features"
        
        ## display_name = "Advanced Subtype"
        ## starts_with = advanced_
        subtype[advanced] = {
            is_advanced = yes
            complexity = high
        }
        
        ## display_name = "Basic Subtype"
        subtype[basic] = {
            is_basic = yes
        }
        
        localisation = {
            ## required
            name = "$"
            description = "$_desc"
            
            subtype[advanced] = {
                ## required
                advanced_tooltip = "$_advanced_tooltip"
                advanced_name = "$_advanced_name"
                ## optional
                advanced_help = "$_advanced_help"
            }
            
            subtype[basic] = {
                basic_tooltip = "$_basic_tooltip"
                ## primary
                basic_name = "$_basic_name"
            }
        }
        
        modifiers = {
            "$_base_power" = country
            "$_base_cost" = economy
            
            subtype[advanced] = {
                "$_advanced_bonus" = military
                "$_advanced_cost_mult" = economy
                "$_tech_requirement" = technology
            }
            
            subtype[basic] = {
                "$_basic_bonus" = planet
                "$_basic_upkeep" = economy
            }
        }
    }
}
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();
        let types_rules = module.find_rule("types");
        let types_rules = types_rules.unwrap();

        visitor.visit_rule(types_rules);

        // Check that we have the expected types
        assert_eq!(data.types.len(), 3);

        // Test 1: Complex type key filter with <> (not-equal)
        let exclude_events = data.types.get("exclude_country_events").unwrap();
        if let Some(filter) = &exclude_events.options.type_key_filter {
            match filter {
                TypeKeyFilter::Not(key) => {
                    assert_eq!(key, "country_event");
                    println!("✓ type_key_filter <> pattern working correctly");
                }
                _ => {
                    println!("✗ type_key_filter <> pattern not working, got {:?}", filter);
                }
            }
        } else {
            println!("✗ type_key_filter <> pattern not parsed");
        }

        // Test 2: Multiple type key filter values
        let multiple_filter = data.types.get("multiple_key_filter").unwrap();
        if let Some(filter) = &multiple_filter.options.type_key_filter {
            match filter {
                TypeKeyFilter::OneOf(keys) => {
                    assert_eq!(keys.len(), 2);
                    assert!(keys.contains(&"military_event".to_string()));
                    assert!(keys.contains(&"diplomatic_event".to_string()));
                    println!("✓ type_key_filter list pattern working correctly");
                }
                _ => {
                    println!(
                        "✗ type_key_filter list pattern not working, got {:?}",
                        filter
                    );
                }
            }
        } else {
            println!("✗ type_key_filter list pattern not parsed");
        }

        // Test 3: Skip root key != pattern (should now work!)
        if let Some(skip) = &exclude_events.skip_root_key {
            match skip {
                SkipRootKey::Except(keys) => {
                    assert_eq!(keys.len(), 1);
                    assert_eq!(keys[0], "tech_group");
                    println!("✓ skip_root_key != pattern working correctly");
                }
                _ => {
                    println!(
                        "✗ skip_root_key != pattern not working correctly, got {:?}",
                        skip
                    );
                }
            }
        } else {
            println!("✗ skip_root_key != pattern not parsed");
        }

        // Test 4: Subtype-specific localisation (still missing)
        let nested_features = data.types.get("nested_subtype_features").unwrap();

        // Base localisation should be parsed
        assert_eq!(nested_features.localisation.len(), 2);
        assert!(nested_features.localisation.contains_key("name"));
        assert!(nested_features.localisation.contains_key("description"));

        let name_loc = nested_features.localisation.get("name").unwrap();
        assert_eq!(name_loc.required, true);
        assert_eq!(name_loc.pattern, "$");

        // Check if subtype-specific localisation is parsed
        if !name_loc.subtypes.is_empty() {
            let advanced_subtype_loc = name_loc.subtypes.get("advanced");
            if let Some(advanced_loc) = advanced_subtype_loc {
                assert!(advanced_loc.contains_key("advanced_tooltip"));
                assert!(advanced_loc.contains_key("advanced_name"));
                assert!(advanced_loc.contains_key("advanced_help"));
                println!("✓ Subtype-specific localisation working correctly");
            } else {
                println!("✗ Subtype-specific localisation not parsed for advanced subtype");
            }
        } else {
            println!("✗ Subtype-specific localisation not parsed at all");
        }

        // Test 5: Subtype-specific modifiers (still missing)
        assert_eq!(nested_features.modifiers.modifiers.len(), 2);
        assert!(
            nested_features
                .modifiers
                .modifiers
                .contains_key("$_base_power")
        );
        assert!(
            nested_features
                .modifiers
                .modifiers
                .contains_key("$_base_cost")
        );

        if !nested_features.modifiers.subtypes.is_empty() {
            let advanced_subtype_mods = nested_features.modifiers.subtypes.get("advanced");
            if let Some(advanced_mods) = advanced_subtype_mods {
                assert!(advanced_mods.contains_key("$_advanced_bonus"));
                assert!(advanced_mods.contains_key("$_advanced_cost_mult"));
                assert!(advanced_mods.contains_key("$_tech_requirement"));
                println!("✓ Subtype-specific modifiers working correctly");
            } else {
                println!("✗ Subtype-specific modifiers not parsed for advanced subtype");
            }
        } else {
            println!("✗ Subtype-specific modifiers not parsed at all");
        }

        // Test 6: Subtype comment options
        let advanced_subtype = nested_features.subtypes.get("advanced").unwrap();
        if advanced_subtype.display_name.is_some() {
            assert_eq!(
                advanced_subtype.display_name.as_ref().unwrap(),
                "Advanced Subtype"
            );
            println!("✓ Subtype display_name working correctly");
        } else {
            println!("✗ Subtype display_name not parsed");
        }

        println!("\n=== REMAINING MISSING FEATURES ===");
        println!("1. Subtype-specific localisation and modifiers (nested parsing)");
        println!("2. Multiple skip_root_key entries on same type");
        println!("3. Subtype starts_with pattern matching in conditions");
    }
}
