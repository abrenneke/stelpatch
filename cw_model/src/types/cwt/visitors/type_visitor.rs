//! Specialized visitor for CWT type definitions
//!
//! This visitor handles the processing of CWT type definitions, including nested
//! subtypes, localisation requirements, and type options.

use std::collections::HashMap;

use cw_parser::{
    AstCwtBlock, AstCwtExpression, AstCwtIdentifierOrString, AstCwtRule, CwtOperator,
    CwtReferenceType, CwtSimpleValueType, CwtValue, CwtVisitor,
};

use crate::{
    ConversionError, CwtAnalysisData, CwtConverter, CwtOptions, CwtType, LocalisationRequirement,
    ModifierSpec, Property, RuleOptions, SeverityLevel, SkipRootKey, Subtype, TypeDefinition,
    TypeOptions,
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
            AstCwtIdentifierOrString::Identifier(identifier) => {
                // Check for typed identifiers
                matches!(identifier.identifier_type, CwtReferenceType::Type)
            }
            AstCwtIdentifierOrString::String(_) => {
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
                modifiers: ModifierSpec {
                    modifiers: HashMap::new(),
                    subtypes: HashMap::new(),
                },
                rules: CwtType::Unknown,
                options: TypeOptions::default(),
                rule_options: options,
            };

            // Extract additional type options from the block
            if let CwtValue::Block(block) = &rule.value {
                self.extract_type_options(&mut type_def, block);
            }

            // Store the type definition (merge with existing if present)
            self.data.insert_or_merge_type(name, type_def);
        } else {
            let key_name = match &rule.key {
                AstCwtIdentifierOrString::Identifier(identifier) => identifier.name.raw_value(),
                AstCwtIdentifierOrString::String(_) => {
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
            AstCwtIdentifierOrString::Identifier(identifier) => {
                if matches!(identifier.identifier_type, CwtReferenceType::Type) {
                    Some(identifier.name.raw_value().to_string())
                } else {
                    None
                }
            }
            AstCwtIdentifierOrString::String(_) => {
                // String keys are only used for enum variant lists, not type definitions
                panic!("String keys should not be used for type definitions")
            }
        }
    }

    /// Extract type options from a type definition block
    fn extract_type_options(&mut self, type_def: &mut TypeDefinition, block: &AstCwtBlock) {
        let mut skip_root_key_rules = Vec::new();

        for item in &block.items {
            if let AstCwtExpression::Rule(rule) = item {
                // Check if this is a subtype rule
                if let AstCwtIdentifierOrString::Identifier(identifier) = &rule.key {
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
                        // Collect all skip_root_key rules for processing together
                        skip_root_key_rules.push(rule);
                    }
                    _ => {
                        // Regular field in type definition - handled by converter
                    }
                }
            }
        }

        // Process all collected skip_root_key rules together
        if !skip_root_key_rules.is_empty() {
            Self::extract_multiple_skip_root_keys(type_def, &skip_root_key_rules);
        }
    }

    /// Extract multiple skip_root_key configurations and combine them appropriately
    fn extract_multiple_skip_root_keys(type_def: &mut TypeDefinition, rules: &[&AstCwtRule]) {
        let mut specific_keys = Vec::new();
        let mut except_keys = Vec::new();
        let mut multiple_keys = Vec::new();
        let mut has_any = false;

        for rule in rules {
            match (&rule.operator, &rule.value) {
                (CwtOperator::NotEquals, CwtValue::String(s)) => {
                    // skip_root_key != tech_group
                    except_keys.push(s.raw_value().to_string());
                }
                (CwtOperator::Equals, CwtValue::String(s)) => {
                    let value = s.raw_value();
                    if value == "any" {
                        has_any = true;
                    } else {
                        specific_keys.push(value.to_string());
                    }
                }
                (CwtOperator::Equals, CwtValue::Block(block)) => {
                    // Handle block configurations
                    for item in &block.items {
                        if let cw_parser::cwt::AstCwtExpression::Value(v) = item {
                            match v {
                                CwtValue::String(s) => {
                                    multiple_keys.push(s.raw_value().to_string());
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Combine the results into the appropriate SkipRootKey variant
        if has_any {
            // "any" takes precedence
            type_def.skip_root_key = Some(SkipRootKey::Any);
        } else if !except_keys.is_empty() {
            // Except keys take precedence over specific keys
            type_def.skip_root_key = Some(SkipRootKey::Except(except_keys));
        } else if !multiple_keys.is_empty() {
            // Block-based multiple keys
            type_def.skip_root_key = Some(SkipRootKey::Multiple(multiple_keys));
        } else if specific_keys.len() == 1 {
            // Single specific key
            type_def.skip_root_key = Some(SkipRootKey::Specific(
                specific_keys.into_iter().next().unwrap(),
            ));
        } else if specific_keys.len() > 1 {
            // Multiple specific keys become Multiple
            type_def.skip_root_key = Some(SkipRootKey::Multiple(specific_keys));
        }
    }

    /// Extract localisation requirements from a type definition
    fn extract_localisation_requirements(type_def: &mut TypeDefinition, block: &AstCwtBlock) {
        for item in &block.items {
            if let cw_parser::cwt::AstCwtExpression::Rule(rule) = item {
                let key = rule.key.name();

                // Check if this is a subtype rule within localisation
                if let AstCwtIdentifierOrString::Identifier(identifier) = &rule.key {
                    if matches!(identifier.identifier_type, CwtReferenceType::Subtype) {
                        let subtype_name = identifier.name.raw_value();
                        Self::extract_subtype_localisation(type_def, subtype_name, rule);
                        continue;
                    }
                }

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

    /// Extract subtype-specific localisation requirements
    fn extract_subtype_localisation(
        type_def: &mut TypeDefinition,
        subtype_name: &str,
        rule: &AstCwtRule,
    ) {
        if let CwtValue::Block(subtype_block) = &rule.value {
            for item in &subtype_block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(loc_rule) = item {
                    let loc_key = loc_rule.key.name();

                    if let CwtValue::String(pattern) = &loc_rule.value {
                        let pattern_str = pattern.raw_value().to_string();

                        // Only add to existing base localisation requirements
                        if let Some(base_requirement) =
                            type_def.localisation.get_mut(&loc_key.to_string())
                        {
                            // Add to subtype-specific localisation for existing base requirement
                            let subtype_map = base_requirement
                                .subtypes
                                .entry(subtype_name.to_string())
                                .or_insert_with(HashMap::new);

                            subtype_map.insert(loc_key.to_string(), pattern_str);

                            // Check for required/primary flags on subtype localisation
                            for option in &loc_rule.options {
                                match option.key {
                                    "required" => {
                                        // Mark this subtype localisation as required
                                        // This could be stored in a more complex structure if needed
                                    }
                                    "primary" => {
                                        // Mark this subtype localisation as primary
                                        // This could be stored in a more complex structure if needed
                                    }
                                    _ => {}
                                }
                            }
                        }
                        // If no base requirement exists, this is a subtype-only localisation
                        // We don't add it to the base localisation map
                    }
                }
            }
        }
    }

    /// Extract modifier definitions from a type definition
    fn extract_modifier_definitions(type_def: &mut TypeDefinition, block: &AstCwtBlock) {
        for item in &block.items {
            if let cw_parser::cwt::AstCwtExpression::Rule(rule) = item {
                let key = rule.key.name();

                // Check if this is a subtype rule within modifiers
                if let AstCwtIdentifierOrString::Identifier(identifier) = &rule.key {
                    if matches!(identifier.identifier_type, CwtReferenceType::Subtype) {
                        let subtype_name = identifier.name.raw_value();
                        Self::extract_subtype_modifiers(type_def, subtype_name, rule);
                        continue;
                    }
                }

                if let CwtValue::String(scope) = &rule.value {
                    type_def
                        .modifiers
                        .modifiers
                        .insert(key.to_string(), scope.raw_value().to_string());
                }
            }
        }
    }

    /// Extract subtype-specific modifier definitions
    fn extract_subtype_modifiers(
        type_def: &mut TypeDefinition,
        subtype_name: &str,
        rule: &AstCwtRule,
    ) {
        if let CwtValue::Block(subtype_block) = &rule.value {
            let mut subtype_modifiers = HashMap::new();

            for item in &subtype_block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(mod_rule) = item {
                    let mod_key = mod_rule.key.name();

                    if let CwtValue::String(scope) = &mod_rule.value {
                        subtype_modifiers
                            .insert(mod_key.to_string(), scope.raw_value().to_string());
                    }
                }
            }

            if !subtype_modifiers.is_empty() {
                type_def
                    .modifiers
                    .subtypes
                    .insert(subtype_name.to_string(), subtype_modifiers);
            }
        }
    }

    /// Extract subtype definition
    fn extract_subtype_definition(
        type_def: &mut TypeDefinition,
        subtype_name: &str,
        rule: &AstCwtRule,
    ) {
        // Parse CWT options (metadata like display_name, starts_with, etc.)
        let subtype_options = CwtOptions::from_rule(rule);

        // Extract properties from the rule value block
        let mut properties = HashMap::new();

        if let CwtValue::Block(block) = &rule.value {
            // Look for property definitions that define this subtype's constraints
            let mut property_conditions = HashMap::new();

            for item in &block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(prop_rule) = item {
                    let prop_key = prop_rule.key.name();

                    // Extract options from the individual rule (e.g., cardinality constraints)
                    let prop_options = CwtOptions::from_rule(prop_rule);

                    // Always create a Property object to store the rule with its options
                    // This ensures cardinality constraints are preserved even for block values
                    let property_type = CwtConverter::convert_value(&prop_rule.value, None);
                    let property = Property {
                        property_type,
                        options: prop_options.clone(),
                        documentation: None,
                    };
                    properties.insert(prop_key.to_string(), property);

                    // For subtype condition matching, skip block values as they're not used for conditions
                    // But we still store them above for cardinality validation
                    if matches!(prop_rule.value, CwtValue::Block(_)) {
                        continue;
                    }

                    // Store the actual value for condition determination (non-block values only)
                    property_conditions.insert(prop_key.to_string(), prop_rule.value.clone());
                }
            }

            // Note: We no longer create a single "condition" - instead, the condition logic
            // will be derived dynamically from the properties stored above when needed for matching
        }

        let subtype_def = Subtype {
            condition_properties: properties, // Use the properties we collected with their options
            allowed_properties: HashMap::new(),
            allowed_pattern_properties: Vec::new(),
            options: subtype_options,
            is_inverted: false,
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

    use crate::TypeKeyFilter;

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
            advanced_subtype.options.display_name,
            Some("Advanced Subtype".to_string())
        );
        assert_eq!(
            advanced_subtype.options.abbreviation,
            Some("ADV".to_string())
        );

        // Check basic subtype options
        assert_eq!(
            basic_subtype.options.display_name,
            Some("Basic Subtype".to_string())
        );
        assert_eq!(basic_subtype.options.abbreviation, None); // No abbreviation specified

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
    fn test_subtype_rule_options() {
        let mut data = CwtAnalysisData::new();
        let mut visitor = TypeVisitor::new(&mut data);

        let cwt_text = r#"
types = {
    type[civic_or_origin] = {
        path = "game/common/governments/civics"
        localisation = {
            ## required
            Name = "$"
            ## required
            Description = "$_desc"
        }
        subtype[origin] = {
            is_origin = yes
        }
        subtype[civic] = {
            ## cardinality = 0..1
            is_origin = no
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

        let civic_or_origin = data.types.get("civic_or_origin").unwrap();

        // Check subtypes
        assert_eq!(civic_or_origin.subtypes.len(), 2);
        assert!(civic_or_origin.subtypes.contains_key("origin"));
        assert!(civic_or_origin.subtypes.contains_key("civic"));

        // Check that the origin subtype has the is_origin property
        let origin_subtype = civic_or_origin.subtypes.get("origin").unwrap();
        assert!(
            origin_subtype
                .condition_properties
                .contains_key("is_origin")
        );
        let origin_property = origin_subtype
            .condition_properties
            .get("is_origin")
            .unwrap();
        assert_eq!(origin_property.options.cardinality, None); // No cardinality specified

        // Check that the civic subtype has the is_origin property with cardinality
        let civic_subtype = civic_or_origin.subtypes.get("civic").unwrap();
        assert!(civic_subtype.condition_properties.contains_key("is_origin"));
        let civic_property = civic_subtype.condition_properties.get("is_origin").unwrap();

        // Check that the cardinality option was properly extracted
        assert!(civic_property.options.cardinality.is_some());
        let cardinality = civic_property.options.cardinality.as_ref().unwrap();
        assert_eq!(cardinality.min, Some(0));
        assert_eq!(cardinality.max, Some(1));
        assert_eq!(cardinality.soft, false);
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
        skip_root_key != military_group
    }
    
    ## type_key_filter = { military_event diplomatic_event }
    type[multiple_key_filter] = {
        path = "game/common/multiple_filter"
        skip_root_key = first_level
        skip_root_key = second_level
    }
    
    type[nested_subtype_features] = {
        path = "game/common/nested_features"
        
        ## display_name = "Advanced Subtype"
        ## starts_with = advanced_
        subtype[advanced] = {
            is_advanced = yes
            level = int
        }
        
        localisation = {
            ## display_name = "Localisation for Advanced"
            ## starts_with = adv_
            subtype[advanced] = {
                ## required
                name = "advanced_$"
                ## primary
                description = "advanced_$_desc"
            }
            
            ## required
            base_name = "$"
        }
        
        modifiers = {
            ## display_name = "Modifiers for Advanced"
            ## starts_with = adv_
            subtype[advanced] = {
                advanced_modifier = country
                special_effect = planet
            }
            
            base_modifier = country
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
                }
                _ => {
                    panic!(
                        "Expected TypeKeyFilter::Not(country_event), got {:?}",
                        filter
                    );
                }
            }
        } else {
            panic!("Expected type_key_filter <> country_event to be parsed");
        }

        // Test 2: Multiple type key filter values
        let multiple_filter = data.types.get("multiple_key_filter").unwrap();
        if let Some(filter) = &multiple_filter.options.type_key_filter {
            match filter {
                TypeKeyFilter::OneOf(keys) => {
                    assert_eq!(keys.len(), 2);
                    assert!(keys.contains(&"military_event".to_string()));
                    assert!(keys.contains(&"diplomatic_event".to_string()));
                }
                _ => {
                    panic!(
                        "Expected TypeKeyFilter::OneOf(military_event, diplomatic_event), got {:?}",
                        filter
                    );
                }
            }
        } else {
            panic!("Expected type_key_filter list pattern to be parsed");
        }

        // Test 3: Skip root key != pattern (should now work!)
        if let Some(skip) = &exclude_events.skip_root_key {
            match skip {
                SkipRootKey::Except(keys) => {
                    assert_eq!(keys.len(), 2);
                    assert_eq!(keys[0], "tech_group");
                    assert_eq!(keys[1], "military_group");
                }
                _ => {
                    panic!(
                        "Expected SkipRootKey::Except(tech_group, military_group), got {:?}",
                        skip
                    );
                }
            }
        } else {
            panic!("Expected skip_root_key != pattern to be parsed");
        }
    }
}
