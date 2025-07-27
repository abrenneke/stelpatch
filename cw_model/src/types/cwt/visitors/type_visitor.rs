//! Specialized visitor for CWT type definitions
//!
//! This visitor handles the processing of CWT type definitions, including nested
//! subtypes, localisation requirements, and type options.

use std::{collections::HashMap, sync::Arc};

use cw_parser::{
    AstCwtBlock, AstCwtExpression, AstCwtIdentifierOrString, AstCwtRule, CwtOperator,
    CwtReferenceType, CwtSimpleValueType, CwtValue, CwtVisitor,
};
use lasso::Spur;

use crate::{
    CaseInsensitiveInterner, ConversionError, CwtAnalysisData, CwtConverter, CwtOptions, CwtType,
    LocalisationRequirement, ModifierSpec, Property, RuleOptions, SeverityLevel, SkipRootKey,
    Subtype, TypeDefinition, TypeOptions,
};

/// Specialized visitor for type definitions
pub struct TypeVisitor<'a, 'interner> {
    data: &'a mut CwtAnalysisData,
    in_types_section: bool,
    interner: &'interner CaseInsensitiveInterner,
}

impl<'a, 'interner> TypeVisitor<'a, 'interner> {
    /// Create a new type visitor
    pub fn new(
        data: &'a mut CwtAnalysisData,
        interner: &'interner CaseInsensitiveInterner,
    ) -> Self {
        Self {
            data,
            in_types_section: false,
            interner,
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
    fn process_type_definition(&mut self, rule: &AstCwtRule, interner: &CaseInsensitiveInterner) {
        let type_name = self.extract_type_name(rule);

        if let Some(name) = type_name {
            // Parse rule options
            let options = RuleOptions::from_rule(rule, interner);

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
                rules: Arc::new(CwtType::Unknown),
                options: TypeOptions::default(),
                rule_options: options,
            };

            // Extract additional type options from the block
            if let CwtValue::Block(block) = &rule.value {
                self.extract_type_options(&mut type_def, block, interner);
            }

            // Store the type definition (merge with existing if present)
            self.data.insert_or_merge_type(name, type_def, interner);
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
    fn extract_type_name(&self, rule: &AstCwtRule) -> Option<Spur> {
        match &rule.key {
            AstCwtIdentifierOrString::Identifier(identifier) => {
                if matches!(identifier.identifier_type, CwtReferenceType::Type) {
                    Some(self.interner.get_or_intern(identifier.name.raw_value()))
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
    fn extract_type_options(
        &mut self,
        type_def: &mut TypeDefinition,
        block: &AstCwtBlock,
        interner: &CaseInsensitiveInterner,
    ) {
        let mut skip_root_key_rules = Vec::new();

        for item in &block.items {
            if let AstCwtExpression::Rule(rule) = item {
                // Check if this is a subtype rule
                if let AstCwtIdentifierOrString::Identifier(identifier) = &rule.key {
                    if matches!(identifier.identifier_type, CwtReferenceType::Subtype) {
                        let subtype_name = identifier.name.raw_value();
                        Self::extract_subtype_definition(type_def, subtype_name, rule, interner);
                        continue;
                    }
                }

                let key = rule.key.name();

                match key {
                    "path" => {
                        if let CwtValue::String(s) = &rule.value {
                            type_def.path = Some(interner.get_or_intern(s.raw_value().to_string()));
                        }
                    }
                    "name_field" => {
                        if let CwtValue::String(s) = &rule.value {
                            type_def.name_field =
                                Some(interner.get_or_intern(s.raw_value().to_string()));
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
                            type_def.options.path_file =
                                Some(interner.get_or_intern(s.raw_value()));
                        }
                    }
                    "path_extension" => {
                        if let CwtValue::String(s) = &rule.value {
                            type_def.options.path_extension =
                                Some(interner.get_or_intern(s.raw_value()));
                        }
                    }
                    "starts_with" => {
                        if let CwtValue::String(s) = &rule.value {
                            type_def.options.starts_with =
                                Some(interner.get_or_intern(s.raw_value()));
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
                            Self::extract_localisation_requirements(type_def, loc_block, interner);
                        }
                    }
                    "modifiers" => {
                        if let CwtValue::Block(mod_block) = &rule.value {
                            Self::extract_modifier_definitions(type_def, mod_block, interner);
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
            Self::extract_multiple_skip_root_keys(type_def, &skip_root_key_rules, self.interner);
        }
    }

    /// Extract multiple skip_root_key configurations and combine them appropriately
    fn extract_multiple_skip_root_keys(
        type_def: &mut TypeDefinition,
        rules: &[&AstCwtRule],
        interner: &CaseInsensitiveInterner,
    ) {
        let mut specific_keys: Vec<Spur> = Vec::new();
        let mut except_keys: Vec<Spur> = Vec::new();
        let mut multiple_keys: Vec<Spur> = Vec::new();
        let mut has_any = false;

        for rule in rules {
            match (&rule.operator, &rule.value) {
                (CwtOperator::NotEquals, CwtValue::String(s)) => {
                    // skip_root_key != tech_group
                    except_keys.push(interner.get_or_intern(s.raw_value().to_string()));
                }
                (CwtOperator::Equals, CwtValue::String(s)) => {
                    let value = s.raw_value();
                    if value == "any" {
                        has_any = true;
                    } else {
                        specific_keys.push(interner.get_or_intern(value.to_string()));
                    }
                }
                (CwtOperator::Equals, CwtValue::Block(block)) => {
                    // Handle block configurations
                    for item in &block.items {
                        if let cw_parser::cwt::AstCwtExpression::Value(v) = item {
                            match v {
                                CwtValue::String(s) => {
                                    multiple_keys
                                        .push(interner.get_or_intern(s.raw_value().to_string()));
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
    fn extract_localisation_requirements(
        type_def: &mut TypeDefinition,
        block: &AstCwtBlock,
        interner: &CaseInsensitiveInterner,
    ) {
        for item in &block.items {
            if let cw_parser::cwt::AstCwtExpression::Rule(rule) = item {
                let key = rule.key.name();

                // Check if this is a subtype rule within localisation
                if let AstCwtIdentifierOrString::Identifier(identifier) = &rule.key {
                    if matches!(identifier.identifier_type, CwtReferenceType::Subtype) {
                        let subtype_name = identifier.name.raw_value();
                        Self::extract_subtype_localisation(type_def, subtype_name, rule, interner);
                        continue;
                    }
                }

                if let CwtValue::String(pattern) = &rule.value {
                    let mut requirement = LocalisationRequirement::new(
                        interner.get_or_intern(pattern.raw_value().to_string()),
                    );

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

                    type_def
                        .localisation
                        .insert(interner.get_or_intern(key.to_string()), requirement);
                }
            }
        }
    }

    /// Extract subtype-specific localisation requirements
    fn extract_subtype_localisation(
        type_def: &mut TypeDefinition,
        subtype_name: &str,
        rule: &AstCwtRule,
        interner: &CaseInsensitiveInterner,
    ) {
        if let CwtValue::Block(subtype_block) = &rule.value {
            for item in &subtype_block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(loc_rule) = item {
                    let loc_key = loc_rule.key.name();

                    if let CwtValue::String(pattern) = &loc_rule.value {
                        let pattern_str = pattern.raw_value().to_string();

                        // Only add to existing base localisation requirements
                        if let Some(base_requirement) = type_def
                            .localisation
                            .get_mut(&interner.get_or_intern(loc_key.to_string()))
                        {
                            // Add to subtype-specific localisation for existing base requirement
                            let subtype_map = base_requirement
                                .subtypes
                                .entry(interner.get_or_intern(subtype_name.to_string()))
                                .or_insert_with(HashMap::new);

                            subtype_map.insert(
                                interner.get_or_intern(loc_key.to_string()),
                                interner.get_or_intern(pattern_str),
                            );

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
    fn extract_modifier_definitions(
        type_def: &mut TypeDefinition,
        block: &AstCwtBlock,
        interner: &CaseInsensitiveInterner,
    ) {
        for item in &block.items {
            if let cw_parser::cwt::AstCwtExpression::Rule(rule) = item {
                let key = rule.key.name();

                // Check if this is a subtype rule within modifiers
                if let AstCwtIdentifierOrString::Identifier(identifier) = &rule.key {
                    if matches!(identifier.identifier_type, CwtReferenceType::Subtype) {
                        let subtype_name = identifier.name.raw_value();
                        Self::extract_subtype_modifiers(type_def, subtype_name, rule, interner);
                        continue;
                    }
                }

                if let CwtValue::String(scope) = &rule.value {
                    type_def.modifiers.modifiers.insert(
                        interner.get_or_intern(key),
                        interner.get_or_intern(scope.raw_value()),
                    );
                }
            }
        }
    }

    /// Extract subtype-specific modifier definitions
    fn extract_subtype_modifiers(
        type_def: &mut TypeDefinition,
        subtype_name: &str,
        rule: &AstCwtRule,
        interner: &CaseInsensitiveInterner,
    ) {
        if let CwtValue::Block(subtype_block) = &rule.value {
            let mut subtype_modifiers: HashMap<Spur, Spur> = HashMap::new();

            for item in &subtype_block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(mod_rule) = item {
                    let mod_key = mod_rule.key.name();

                    if let CwtValue::String(scope) = &mod_rule.value {
                        subtype_modifiers.insert(
                            interner.get_or_intern(mod_key),
                            interner.get_or_intern(scope.raw_value()),
                        );
                    }
                }
            }

            if !subtype_modifiers.is_empty() {
                type_def
                    .modifiers
                    .subtypes
                    .insert(interner.get_or_intern(subtype_name), subtype_modifiers);
            }
        }
    }

    /// Extract subtype definition
    fn extract_subtype_definition(
        type_def: &mut TypeDefinition,
        subtype_name: &str,
        rule: &AstCwtRule,
        interner: &CaseInsensitiveInterner,
    ) {
        // Parse CWT options (metadata like display_name, starts_with, etc.)
        let subtype_options = CwtOptions::from_rule(rule, interner);

        // Extract properties from the rule value block
        let mut properties: HashMap<Spur, Property> = HashMap::new();

        if let CwtValue::Block(block) = &rule.value {
            // Look for property definitions that define this subtype's constraints
            let mut property_conditions: HashMap<Spur, CwtValue> = HashMap::new();

            for item in &block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(prop_rule) = item {
                    let prop_key = prop_rule.key.name();

                    // Extract options from the individual rule (e.g., cardinality constraints)
                    let prop_options = CwtOptions::from_rule(prop_rule, interner);

                    // Always create a Property object to store the rule with its options
                    // This ensures cardinality constraints are preserved even for block values
                    let property_type =
                        CwtConverter::convert_value(&prop_rule.value, None, interner);
                    let property = Property {
                        property_type,
                        options: prop_options.clone(),
                        documentation: None,
                    };
                    properties.insert(interner.get_or_intern(prop_key), property);

                    // For subtype condition matching, skip block values as they're not used for conditions
                    // But we still store them above for cardinality validation
                    if matches!(prop_rule.value, CwtValue::Block(_)) {
                        continue;
                    }

                    // Store the actual value for condition determination (non-block values only)
                    property_conditions
                        .insert(interner.get_or_intern(prop_key), prop_rule.value.clone());
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
            .insert(interner.get_or_intern(subtype_name), subtype_def);
    }
}

impl<'a, 'interner> CwtVisitor<'a> for TypeVisitor<'a, 'interner> {
    fn visit_rule(&mut self, rule: &AstCwtRule<'a>) {
        if self.can_handle_rule(rule) {
            self.process_type_definition(rule, self.interner);
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
        let interner = CaseInsensitiveInterner::new();
        let mut visitor = TypeVisitor::new(&mut data, &interner);

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
            data.types
                .get(&interner.get_or_intern("test_type"))
                .unwrap()
                .path,
            Some(interner.get_or_intern("test_path"))
        );
        assert_eq!(
            data.types
                .get(&interner.get_or_intern("test_type"))
                .unwrap()
                .name_field,
            Some(interner.get_or_intern("test_name_field"))
        );
        assert_eq!(
            data.types
                .get(&interner.get_or_intern("test_type"))
                .unwrap()
                .options
                .unique,
            true
        );
    }

    #[test]
    fn test_complex_type_visitor() {
        let mut data = CwtAnalysisData::new();
        let interner = CaseInsensitiveInterner::new();
        let mut visitor = TypeVisitor::new(&mut data, &interner);

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
        let opinion_modifier = data
            .types
            .get(&interner.get_or_intern("opinion_modifier"))
            .unwrap();
        assert_eq!(
            opinion_modifier.path,
            Some(interner.get_or_intern("game/common/opinion_modifiers"))
        );
        assert_eq!(opinion_modifier.options.path_strict, true);
        assert_eq!(opinion_modifier.options.type_per_file, true);
        assert_eq!(opinion_modifier.skip_root_key, Some(SkipRootKey::Any));

        // Check subtypes
        assert_eq!(opinion_modifier.subtypes.len(), 2);
        assert!(
            opinion_modifier
                .subtypes
                .contains_key(&interner.get_or_intern("triggered_opinion_modifier"))
        );
        assert!(
            opinion_modifier
                .subtypes
                .contains_key(&interner.get_or_intern("block_triggered"))
        );

        // Check localisation
        assert_eq!(opinion_modifier.localisation.len(), 2);
        assert_eq!(
            opinion_modifier
                .localisation
                .get(&interner.get_or_intern("Name"))
                .unwrap()
                .pattern,
            interner.get_or_intern("$")
        );
        assert_eq!(
            opinion_modifier
                .localisation
                .get(&interner.get_or_intern("Description"))
                .unwrap()
                .pattern,
            interner.get_or_intern("$_desc")
        );

        // Check modifiers
        assert_eq!(opinion_modifier.modifiers.modifiers.len(), 2);
        assert_eq!(
            *opinion_modifier
                .modifiers
                .modifiers
                .get(&interner.get_or_intern("$_modifier"))
                .unwrap(),
            interner.get_or_intern("country")
        );
        assert_eq!(
            *opinion_modifier
                .modifiers
                .modifiers
                .get(&interner.get_or_intern("$_opinion_boost"))
                .unwrap(),
            interner.get_or_intern("diplomacy")
        );

        // Test static_modifier type
        let static_modifier = data
            .types
            .get(&interner.get_or_intern("static_modifier"))
            .unwrap();
        assert_eq!(
            static_modifier.path,
            Some(interner.get_or_intern("game/common/static_modifiers"))
        );
        assert_eq!(
            static_modifier.options.path_extension,
            Some(interner.get_or_intern(".txt"))
        );
        assert_eq!(static_modifier.options.unique, false);

        // Check subtypes
        assert_eq!(static_modifier.subtypes.len(), 1);
        assert!(
            static_modifier
                .subtypes
                .contains_key(&interner.get_or_intern("planet"))
        );

        // Check localisation
        assert_eq!(static_modifier.localisation.len(), 2);
        assert_eq!(
            static_modifier
                .localisation
                .get(&interner.get_or_intern("Name"))
                .unwrap()
                .pattern,
            interner.get_or_intern("$")
        );
        assert_eq!(
            static_modifier
                .localisation
                .get(&interner.get_or_intern("Description"))
                .unwrap()
                .pattern,
            interner.get_or_intern("$_desc")
        );

        // Check modifiers
        assert_eq!(static_modifier.modifiers.modifiers.len(), 1);
        assert_eq!(
            *static_modifier
                .modifiers
                .modifiers
                .get(&interner.get_or_intern("$_boost"))
                .unwrap(),
            interner.get_or_intern("planet")
        );
    }

    #[test]
    fn test_enhanced_cwt_features() {
        let mut data = CwtAnalysisData::new();
        let interner = CaseInsensitiveInterner::new();
        let mut visitor = TypeVisitor::new(&mut data, &interner);

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
        let advanced_type = data
            .types
            .get(&interner.get_or_intern("advanced_type"))
            .unwrap();
        assert_eq!(
            advanced_type.path,
            Some(interner.get_or_intern("game/common/advanced_types"))
        );
        assert_eq!(advanced_type.options.path_strict, true);
        assert_eq!(advanced_type.options.type_per_file, true);
        assert_eq!(
            advanced_type.options.starts_with,
            Some(interner.get_or_intern("adv_"))
        );

        // Check type-level options (from comments)
        assert_eq!(advanced_type.options.severity, Some(SeverityLevel::Warning));
        assert_eq!(
            advanced_type.options.graph_related_types,
            vec![
                interner.get_or_intern("technology"),
                interner.get_or_intern("building"),
            ]
        );

        // Check subtypes
        assert_eq!(advanced_type.subtypes.len(), 2);
        let advanced_subtype = advanced_type
            .subtypes
            .get(&interner.get_or_intern("advanced"))
            .unwrap();
        let basic_subtype = advanced_type
            .subtypes
            .get(&interner.get_or_intern("basic"))
            .unwrap();

        // Check subtype options (comment-based options should be parsed)
        assert_eq!(
            advanced_subtype.options.display_name,
            Some(interner.get_or_intern("Advanced Subtype"))
        );
        assert_eq!(
            advanced_subtype.options.abbreviation,
            Some(interner.get_or_intern("ADV"))
        );

        // Check basic subtype options
        assert_eq!(
            basic_subtype.options.display_name,
            Some(interner.get_or_intern("Basic Subtype"))
        );
        assert_eq!(basic_subtype.options.abbreviation, None); // No abbreviation specified

        // Check localisation structure
        assert_eq!(advanced_type.localisation.len(), 2);
        let name_loc = advanced_type
            .localisation
            .get(&interner.get_or_intern("Name"))
            .unwrap();
        let desc_loc = advanced_type
            .localisation
            .get(&interner.get_or_intern("Description"))
            .unwrap();

        assert_eq!(name_loc.pattern, interner.get_or_intern("$"));
        // Comment-based options should be parsed
        assert_eq!(name_loc.required, true); // From ## required comment
        assert_eq!(name_loc.primary, true); // From ## primary comment

        assert_eq!(desc_loc.pattern, interner.get_or_intern("$_desc"));
        assert_eq!(desc_loc.required, false); // Marked as ## optional
        assert_eq!(desc_loc.primary, false);

        // Check subtype-specific localisation
        // Note: This would be fully implemented with proper nested parsing

        // Check modifiers structure
        assert_eq!(advanced_type.modifiers.modifiers.len(), 2);
        assert_eq!(
            *advanced_type
                .modifiers
                .modifiers
                .get(&interner.get_or_intern("$_base_modifier"))
                .unwrap(),
            interner.get_or_intern("country")
        );
        assert_eq!(
            *advanced_type
                .modifiers
                .modifiers
                .get(&interner.get_or_intern("$_power_modifier"))
                .unwrap(),
            interner.get_or_intern("fleet")
        );

        // Check subtype-specific modifiers
        // Note: This would be fully implemented with proper nested parsing

        // Check skip_root_key (complex structure)
        assert_eq!(
            advanced_type.skip_root_key,
            Some(SkipRootKey::Multiple(vec![
                interner.get_or_intern("tech_group"),
                interner.get_or_intern("any"),
                interner.get_or_intern("military"),
            ]))
        );
    }

    #[test]
    fn test_skip_root_key_variants() {
        let mut data = CwtAnalysisData::new();
        let interner = CaseInsensitiveInterner::new();
        let mut visitor = TypeVisitor::new(&mut data, &interner);

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
        let specific_skip = data
            .types
            .get(&interner.get_or_intern("specific_skip"))
            .unwrap();
        assert_eq!(
            specific_skip.skip_root_key,
            Some(SkipRootKey::Specific(
                interner.get_or_intern("specific_key")
            ))
        );

        // Test any skip
        let any_skip = data.types.get(&interner.get_or_intern("any_skip")).unwrap();
        assert_eq!(any_skip.skip_root_key, Some(SkipRootKey::Any));

        // Test multiple skip
        let multiple_skip = data
            .types
            .get(&interner.get_or_intern("multiple_skip"))
            .unwrap();
        assert_eq!(
            multiple_skip.skip_root_key,
            Some(SkipRootKey::Multiple(vec![
                interner.get_or_intern("level1"),
                interner.get_or_intern("level2"),
                interner.get_or_intern("level3"),
            ]))
        );
    }

    #[test]
    fn test_subtype_specific_localisation_modifiers() {
        let mut data = CwtAnalysisData::new();
        let interner = CaseInsensitiveInterner::new();
        let mut visitor = TypeVisitor::new(&mut data, &interner);

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

        let complex_type = data
            .types
            .get(&interner.get_or_intern("complex_type"))
            .unwrap();

        // Check subtypes
        assert_eq!(complex_type.subtypes.len(), 2);
        assert!(
            complex_type
                .subtypes
                .contains_key(&interner.get_or_intern("variant_a"))
        );
        assert!(
            complex_type
                .subtypes
                .contains_key(&interner.get_or_intern("variant_b"))
        );

        // Check base localisation
        assert_eq!(complex_type.localisation.len(), 2);
        assert!(
            complex_type
                .localisation
                .contains_key(&interner.get_or_intern("name"))
        );
        assert!(
            complex_type
                .localisation
                .contains_key(&interner.get_or_intern("description"))
        );

        // Check base modifiers
        assert_eq!(complex_type.modifiers.modifiers.len(), 2);
        assert_eq!(
            *complex_type
                .modifiers
                .modifiers
                .get(&interner.get_or_intern("$_base_power"))
                .unwrap(),
            interner.get_or_intern("country")
        );
        assert_eq!(
            *complex_type
                .modifiers
                .modifiers
                .get(&interner.get_or_intern("$_base_cost"))
                .unwrap(),
            interner.get_or_intern("economy")
        );

        // Note: Subtype-specific localisation and modifiers would be fully parsed
        // in a complete implementation with proper nested block parsing
    }

    #[test]
    fn test_subtype_rule_options() {
        let mut data = CwtAnalysisData::new();
        let interner = CaseInsensitiveInterner::new();
        let mut visitor = TypeVisitor::new(&mut data, &interner);

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

        let civic_or_origin = data
            .types
            .get(&interner.get_or_intern("civic_or_origin"))
            .unwrap();

        // Check subtypes
        assert_eq!(civic_or_origin.subtypes.len(), 2);
        assert!(
            civic_or_origin
                .subtypes
                .contains_key(&interner.get_or_intern("origin"))
        );
        assert!(
            civic_or_origin
                .subtypes
                .contains_key(&interner.get_or_intern("civic"))
        );

        // Check that the origin subtype has the is_origin property
        let origin_subtype = civic_or_origin
            .subtypes
            .get(&interner.get_or_intern("origin"))
            .unwrap();
        assert!(
            origin_subtype
                .condition_properties
                .contains_key(&interner.get_or_intern("is_origin"))
        );
        let origin_property = origin_subtype
            .condition_properties
            .get(&interner.get_or_intern("is_origin"))
            .unwrap();
        assert_eq!(origin_property.options.cardinality, None); // No cardinality specified

        // Check that the civic subtype has the is_origin property with cardinality
        let civic_subtype = civic_or_origin
            .subtypes
            .get(&interner.get_or_intern("civic"))
            .unwrap();
        assert!(
            civic_subtype
                .condition_properties
                .contains_key(&interner.get_or_intern("is_origin"))
        );
        let civic_property = civic_subtype
            .condition_properties
            .get(&interner.get_or_intern("is_origin"))
            .unwrap();

        // Check that the cardinality option was properly extracted
        assert!(civic_property.options.cardinality.is_some());
        let cardinality = civic_property.options.cardinality.as_ref().unwrap();
        assert_eq!(cardinality.min, Some(0));
        assert_eq!(cardinality.max, Some(1));
        assert_eq!(cardinality.soft, false);
    }
}
