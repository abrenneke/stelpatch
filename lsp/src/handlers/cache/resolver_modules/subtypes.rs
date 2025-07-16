use crate::handlers::cache::ORIGINAL_KEY_PROPERTY;
use crate::handlers::cache::entity_restructurer::EntityRestructurer;
use crate::handlers::scope::{ScopeError, ScopeStack};
use crate::handlers::scoped_type::{CwtTypeOrSpecial, ScopedType};
use cw_model::types::CwtAnalyzer;
use cw_model::{CwtType, Entity, TypeKeyFilter, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct SubtypeHandler {
    pub cwt_analyzer: Arc<CwtAnalyzer>,
}

impl SubtypeHandler {
    pub fn new(cwt_analyzer: Arc<CwtAnalyzer>) -> Self {
        Self { cwt_analyzer }
    }

    /// Directly evaluate if a subtype matches based on its condition_properties and options
    /// This replaces the condition derivation + matching approach
    fn does_subtype_match(
        &self,
        subtype_def: &cw_model::types::Subtype,
        property_data: &HashMap<String, String>,
    ) -> bool {
        // Handle CWT options that affect matching first (these take precedence)
        if let Some(starts_with) = &subtype_def.options.starts_with {
            // Check if any key in property_data starts with the prefix
            return property_data.keys().any(|key| key.starts_with(starts_with));
        } else if let Some(type_key_filter) = &subtype_def.options.type_key_filter {
            return match type_key_filter {
                TypeKeyFilter::Specific(key) => {
                    // Check if the entity has the specific key OR if the original key matches
                    property_data.contains_key(key)
                        || property_data
                            .get(ORIGINAL_KEY_PROPERTY)
                            .map_or(false, |original_key| original_key.contains(key))
                }
                TypeKeyFilter::Not(key) => {
                    // Check if the entity does NOT have the specific key AND the original key doesn't match
                    !property_data.contains_key(key)
                        && property_data
                            .get(ORIGINAL_KEY_PROPERTY)
                            .map_or(true, |original_key| !original_key.contains(key))
                }
                TypeKeyFilter::OneOf(keys) => {
                    // Check if the entity has any of the specified keys OR if the original key matches any
                    keys.iter().any(|key| property_data.contains_key(key))
                        || property_data
                            .get(ORIGINAL_KEY_PROPERTY)
                            .map_or(false, |original_key| {
                                keys.iter().any(|key| original_key.contains(key))
                            })
                }
            };
        }

        // Evaluate condition_properties directly
        let property_conditions: Vec<_> = subtype_def
            .condition_properties
            .iter()
            .filter_map(|(key, property)| {
                // Consider all properties including blocks for cardinality evaluation
                match &property.property_type {
                    CwtType::Literal(value) => Some((key.clone(), Some(value.clone()), property)),
                    CwtType::Simple(_) => Some((key.clone(), None, property)), // Exists condition
                    CwtType::Block(_) => Some((key.clone(), None, property)), // Block existence with cardinality
                    _ => Some((key.clone(), None, property)), // Default to exists condition
                }
            })
            .collect();

        // If no conditions, fallback to true (subtype always matches)
        if property_conditions.is_empty() {
            return true;
        }

        // Evaluate all conditions (they must all match - AND logic)
        property_conditions
            .iter()
            .all(|(key, expected_value, property)| {
                // Check cardinality constraints first
                if !self.property_satisfies_cardinality(
                    key,
                    property_data,
                    &property.options.cardinality,
                ) {
                    return false;
                }

                // Then check value conditions
                match expected_value {
                    Some(value) => {
                        // PropertyEquals: handle absent properties with cardinality constraints
                        if !property_data.contains_key(key) {
                            // Property is absent - check if cardinality allows this
                            if let Some(card) = &property.options.cardinality {
                                if card.min.unwrap_or(1) == 0 {
                                    // Cardinality allows absence - treat as matching the condition
                                    true
                                } else {
                                    // Property is required but absent - doesn't match
                                    false
                                }
                            } else {
                                // No cardinality constraint: property is required but absent
                                false
                            }
                        } else {
                            // Property is present - check if value matches
                            property_data
                                .get(key)
                                .map_or(false, |actual_value| actual_value == value)
                        }
                    }
                    None => {
                        // PropertyExists or block: check based on cardinality
                        if let Some(cardinality) = &property.options.cardinality {
                            if cardinality.max == Some(0) {
                                // Cardinality 0..0 means property must NOT exist
                                !property_data.contains_key(key)
                            } else {
                                // Other cardinalities: property should exist if min > 0
                                if cardinality.min.unwrap_or(0) > 0 {
                                    property_data.contains_key(key)
                                } else {
                                    // Optional property - always matches
                                    true
                                }
                            }
                        } else {
                            // No cardinality constraint: property should exist (default requirement)
                            property_data.contains_key(key)
                        }
                    }
                }
            })
    }

    /// Get all available subtypes for a given type
    pub fn get_available_subtypes(&self, cwt_type: &CwtType) -> Vec<String> {
        match cwt_type {
            CwtType::Block(block) => block.subtypes.keys().cloned().collect(),
            _ => Vec::new(),
        }
    }

    /// Check if a type has a specific subtype
    pub fn has_subtype(&self, cwt_type: &CwtType, subtype_name: &str) -> bool {
        match cwt_type {
            CwtType::Block(block) => block.subtypes.contains_key(subtype_name),
            _ => false,
        }
    }

    /// Get subtype definition for a given type and subtype name
    pub fn get_subtype_definition<'b>(
        &self,
        cwt_type: &'b CwtType,
        subtype_name: &str,
    ) -> Option<&'b cw_model::types::Subtype> {
        match cwt_type {
            CwtType::Block(block) => block.subtypes.get(subtype_name),
            _ => None,
        }
    }

    /// Check if a property satisfies cardinality constraints
    fn property_satisfies_cardinality(
        &self,
        property_key: &str,
        property_data: &HashMap<String, String>,
        cardinality: &Option<cw_model::types::Cardinality>,
    ) -> bool {
        let property_count = if property_data.contains_key(property_key) {
            1u32
        } else {
            0u32
        };

        match cardinality {
            Some(card) => {
                // Check minimum constraint
                if let Some(min) = card.min {
                    if property_count < min {
                        return false;
                    }
                }

                // Check maximum constraint
                if let Some(max) = card.max {
                    if property_count > max {
                        return false;
                    }
                }

                true
            }
            None => {
                // No cardinality constraint means property must be present (default requirement)
                property_count > 0
            }
        }
    }

    /// Extract cardinality for a specific property from a subtype definition
    fn get_property_cardinality_from_subtype(
        &self,
        subtype_def: &cw_model::types::Subtype,
        property_key: &str,
    ) -> Option<cw_model::types::Cardinality> {
        // Look in condition_properties first (CWT schema rules with cardinality)
        subtype_def
            .condition_properties
            .get(property_key)
            .and_then(|prop| prop.options.cardinality.clone())
            .or_else(|| {
                // Fallback to allowed_properties (game data properties)
                subtype_def
                    .allowed_properties
                    .get(property_key)
                    .and_then(|prop| prop.options.cardinality.clone())
            })
    }

    /// Determine all matching subtypes based on property data
    /// This is the main method for determining active subtypes
    pub fn determine_matching_subtypes(
        &self,
        scoped_type: Arc<ScopedType>,
        property_data: &HashMap<String, String>,
    ) -> HashSet<String> {
        match scoped_type.cwt_type() {
            CwtTypeOrSpecial::CwtType(CwtType::Block(block)) => {
                // Check each subtype condition and collect all matches
                let mut matching_subtypes = HashSet::new();

                for (subtype_name, subtype_def) in &block.subtypes {
                    if subtype_def.is_inverted {
                        continue; // Handled by the else below
                    }

                    // Directly evaluate if the subtype matches
                    if self.does_subtype_match(subtype_def, property_data) {
                        matching_subtypes.insert(subtype_name.clone());
                    } else {
                        matching_subtypes.insert(format!("!{}", subtype_name));
                    }
                }

                matching_subtypes
            }
            _ => HashSet::new(),
        }
    }

    /// Get entity keys from a namespace that match a specific subtype
    pub fn get_entity_keys_in_namespace_for_subtype(
        &self,
        namespace: &str,
        cwt_type: &CwtType,
        subtype_name: &str,
    ) -> Vec<String> {
        // Get the subtype definition
        let subtype_def = match self.get_subtype_definition(cwt_type, subtype_name) {
            Some(def) => def,
            None => return Vec::new(),
        };

        // Get all entities from the namespace
        let entities = match EntityRestructurer::get_namespace_entities(namespace) {
            Some(entities) => entities,
            None => return Vec::new(),
        };

        let mut matching_keys = Vec::new();

        // Check each entity against the subtype condition
        for (entity_key, entity) in entities {
            let property_data = self.extract_property_data_from_entity(&entity);

            // Directly evaluate if the subtype matches
            if self.does_subtype_match(subtype_def, &property_data) {
                matching_keys.push(entity_key);
            }
        }

        matching_keys
    }

    /// Extract property data from an entity for subtype matching
    fn extract_property_data_from_entity(&self, entity: &Entity) -> HashMap<String, String> {
        let mut property_data = HashMap::new();

        for (key, property_list) in &entity.properties.kv {
            // Take the first property value and convert it to string
            if let Some(first_property) = property_list.0.first() {
                let value_str = match &first_property.value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.clone(),
                    Value::Entity(_) => "{}".to_string(), // Special marker for entity values
                    Value::Color(_) => "color".to_string(), // Special marker for color values
                    Value::Maths(m) => m.clone(),         // Math expressions as strings
                };
                property_data.insert(key.clone(), value_str);
            }
        }

        property_data
    }

    /// Check if a subtype has scope changes
    pub fn subtype_has_scope_changes(&self, subtype_def: &cw_model::types::Subtype) -> bool {
        subtype_def.options.push_scope.is_some() || subtype_def.options.replace_scope.is_some()
    }

    /// Apply scope changes from subtype definition options
    pub fn apply_subtype_scope_changes(
        &self,
        scope_stack: &ScopeStack,
        subtype_def: &cw_model::types::Subtype,
    ) -> Result<ScopeStack, ScopeError> {
        let mut new_scope = scope_stack.branch();

        // Apply push_scope if present
        if let Some(push_scope) = &subtype_def.options.push_scope {
            if let Some(scope_name) = self.cwt_analyzer.resolve_scope_name(push_scope) {
                new_scope.push_scope_type(scope_name.to_string())?;
            }
        }

        // Apply replace_scope if present
        if let Some(replace_scope) = &subtype_def.options.replace_scope {
            let mut new_scopes = HashMap::new();

            for (key, value) in replace_scope {
                if let Some(scope_name) = self.cwt_analyzer.resolve_scope_name(value) {
                    new_scopes.insert(key.clone(), scope_name.to_string());
                }
            }

            new_scope.replace_scope_from_strings(new_scopes)?;
        }

        Ok(new_scope)
    }

    /// Apply scope changes from multiple subtype definitions
    pub fn apply_multiple_subtype_scope_changes(
        &self,
        scope_stack: &ScopeStack,
        cwt_type: &CwtType,
        active_subtypes: &HashSet<String>,
    ) -> Result<ScopeStack, ScopeError> {
        let mut new_scope = scope_stack.branch();

        match cwt_type {
            CwtType::Block(block) => {
                // Apply scope changes from all active subtypes
                for subtype_name in active_subtypes {
                    if let Some(subtype_def) = block.subtypes.get(subtype_name) {
                        new_scope = self.apply_subtype_scope_changes(&new_scope, subtype_def)?;
                    }
                }
            }
            _ => {}
        }

        Ok(new_scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cw_model::types::Cardinality;
    use std::collections::HashMap;

    #[test]
    fn test_property_satisfies_cardinality() {
        let handler = SubtypeHandler::new(Arc::new(CwtAnalyzer::new()));
        let mut property_data = HashMap::new();
        property_data.insert("test_key".to_string(), "test_value".to_string());

        // Test with no cardinality (default requirement)
        assert!(handler.property_satisfies_cardinality("test_key", &property_data, &None));
        assert!(!handler.property_satisfies_cardinality("missing_key", &property_data, &None));

        // Test with cardinality 0..1 (optional)
        let optional_cardinality = Some(Cardinality::optional());
        assert!(handler.property_satisfies_cardinality(
            "test_key",
            &property_data,
            &optional_cardinality
        ));
        assert!(handler.property_satisfies_cardinality(
            "missing_key",
            &property_data,
            &optional_cardinality
        ));

        // Test with cardinality 1..1 (required)
        let required_cardinality = Some(Cardinality::required());
        assert!(handler.property_satisfies_cardinality(
            "test_key",
            &property_data,
            &required_cardinality
        ));
        assert!(!handler.property_satisfies_cardinality(
            "missing_key",
            &property_data,
            &required_cardinality
        ));
    }

    #[test]
    fn test_does_subtype_match_civic_without_is_origin() {
        use cw_model::types::{Cardinality, Property, Subtype};
        use std::collections::HashMap;

        let handler = SubtypeHandler::new(Arc::new(CwtAnalyzer::new()));

        // Create a mock civic subtype definition similar to the real one
        // subtype[civic] = { ## cardinality = 0..1; is_origin = no }
        let mut civic_subtype = Subtype {
            condition_properties: HashMap::new(),
            allowed_properties: HashMap::new(),
            options: Default::default(),
            is_inverted: false,
        };
        let condition_property = Property::optional(CwtType::Literal("no".to_string()));
        civic_subtype
            .condition_properties
            .insert("is_origin".to_string(), condition_property);

        // Test case 1: Civic without is_origin property (should match)
        let civic_data_without_is_origin = HashMap::new();
        let result1 = handler.does_subtype_match(&civic_subtype, &civic_data_without_is_origin);
        println!("Civic without is_origin: {}", result1);
        assert!(
            result1,
            "Civic without is_origin property should match civic subtype"
        );

        // Test case 2: Civic with is_origin = no (should match)
        let mut civic_data_with_no = HashMap::new();
        civic_data_with_no.insert("is_origin".to_string(), "no".to_string());
        let result2 = handler.does_subtype_match(&civic_subtype, &civic_data_with_no);
        println!("Civic with is_origin=no: {}", result2);
        assert!(
            result2,
            "Civic with is_origin = no should match civic subtype"
        );

        // Test case 3: Civic with is_origin = yes (should NOT match)
        let mut civic_data_with_yes = HashMap::new();
        civic_data_with_yes.insert("is_origin".to_string(), "yes".to_string());
        let result3 = handler.does_subtype_match(&civic_subtype, &civic_data_with_yes);
        println!("Civic with is_origin=yes: {}", result3);
        assert!(
            !result3,
            "Civic with is_origin = yes should NOT match civic subtype"
        );

        // Now test origin subtype (required property)
        let mut origin_subtype = Subtype {
            condition_properties: HashMap::new(),
            allowed_properties: HashMap::new(),
            options: Default::default(),
            is_inverted: false,
        };
        let origin_condition_property = Property::required(CwtType::Literal("yes".to_string()));
        origin_subtype
            .condition_properties
            .insert("is_origin".to_string(), origin_condition_property);

        // Test case 4: Origin without is_origin property (should NOT match)
        let result4 = handler.does_subtype_match(&origin_subtype, &civic_data_without_is_origin);
        println!("Origin without is_origin: {}", result4);
        assert!(
            !result4,
            "Entity without is_origin property should NOT match origin subtype"
        );

        // Test case 5: Origin with is_origin = yes (should match)
        let result5 = handler.does_subtype_match(&origin_subtype, &civic_data_with_yes);
        println!("Origin with is_origin=yes: {}", result5);
        assert!(
            result5,
            "Entity with is_origin = yes should match origin subtype"
        );
    }
}
