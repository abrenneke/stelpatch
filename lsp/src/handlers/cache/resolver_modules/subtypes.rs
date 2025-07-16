use crate::handlers::cache::ORIGINAL_KEY_PROPERTY;
use crate::handlers::cache::entity_restructurer::EntityRestructurer;
use crate::handlers::scope::{ScopeError, ScopeStack};
use crate::handlers::scoped_type::{CwtTypeOrSpecial, ScopedType};
use cw_model::types::{CwtAnalyzer, SubtypeCondition};
use cw_model::{CwtType, Entity, Value};
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
                cw_model::types::TypeKeyFilter::Specific(key) => {
                    // Check if the original key matches the filter
                    if let Some(original_key) = property_data.get(ORIGINAL_KEY_PROPERTY) {
                        original_key.contains(key)
                    } else {
                        false
                    }
                }
                cw_model::types::TypeKeyFilter::Not(key) => {
                    // Check if the original key does NOT match the filter
                    if let Some(original_key) = property_data.get(ORIGINAL_KEY_PROPERTY) {
                        !original_key.contains(key)
                    } else {
                        true // If no original key, it doesn't match the excluded pattern
                    }
                }
                cw_model::types::TypeKeyFilter::OneOf(keys) => {
                    // Check if the original key matches any of the filters
                    if let Some(original_key) = property_data.get(ORIGINAL_KEY_PROPERTY) {
                        keys.iter().any(|key| original_key.contains(key))
                    } else {
                        false
                    }
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
                        // PropertyEquals: check if property exists and has the expected value
                        property_data
                            .get(key)
                            .map_or(false, |actual_value| actual_value == value)
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

    /// Check if a subtype condition would be satisfied with cardinality constraints
    /// This is the cardinality-aware version of would_subtype_condition_match
    pub fn would_subtype_condition_match_with_cardinality(
        &self,
        condition: &SubtypeCondition,
        property_data: &HashMap<String, String>,
        cardinality: &Option<cw_model::types::Cardinality>,
    ) -> bool {
        match condition {
            SubtypeCondition::PropertyEquals { key, value } => {
                // First check if the property satisfies cardinality constraints
                if !self.property_satisfies_cardinality(key, property_data, cardinality) {
                    return false;
                }

                // For PropertyEquals with cardinality that allows absence (min = 0),
                // absence should count as matching the condition
                if !property_data.contains_key(key) {
                    // Property is absent - check if cardinality allows this
                    if let Some(card) = cardinality {
                        if card.min == Some(0) {
                            // Cardinality allows absence - treat as matching the condition
                            return true;
                        }
                    }
                    // Property is required but absent - doesn't match
                    return false;
                }

                // Property is present - check if value matches
                property_data
                    .get(key)
                    .map_or(false, |prop_value| prop_value == value)
            }
            SubtypeCondition::PropertyNotEquals { key, value } => {
                // First check if the property satisfies cardinality constraints
                if !self.property_satisfies_cardinality(key, property_data, cardinality) {
                    return false;
                }

                // For PropertyNotEquals, if cardinality allows absence and property is absent,
                // then absence counts as "not equal" to any specific value
                if !property_data.contains_key(key) {
                    // Property is absent - this counts as "not equal" to the value
                    return true;
                }

                // Property is present - check if it doesn't equal the value
                property_data
                    .get(key)
                    .map_or(true, |prop_value| prop_value != value)
            }
            SubtypeCondition::PropertyExists { key } => {
                // For PropertyExists, the property must actually exist
                // But we also need to respect cardinality constraints
                if !property_data.contains_key(key) {
                    // Property doesn't exist
                    return false;
                }

                // Property exists, now check if it satisfies cardinality constraints
                self.property_satisfies_cardinality(key, property_data, cardinality)
            }
            SubtypeCondition::PropertyNotExists { key } => {
                // For PropertyNotExists, the property should not exist
                // But we also need to respect cardinality constraints
                if let Some(card) = cardinality {
                    if card.min.unwrap_or(0) > 0 {
                        // If cardinality requires the property to be present, this condition can't be satisfied
                        return false;
                    }
                }
                !property_data.contains_key(key)
            }
            SubtypeCondition::KeyStartsWith { prefix } => {
                // For key-based conditions, cardinality doesn't apply directly
                // Fall back to original logic
                property_data.keys().any(|key| key.starts_with(prefix))
            }
            SubtypeCondition::KeyMatches { filter } => {
                // For key-based conditions, cardinality doesn't apply directly
                // Fall back to original logic
                property_data.keys().any(|key| key.contains(filter))
            }
            SubtypeCondition::Expression(_expr) => {
                // Complex expressions would require a full parser/evaluator
                // For now, return false - this could be extended in the future
                false
            }
        }
    }

    /// Check if a subtype condition would be satisfied using property-specific cardinality
    /// This method extracts the cardinality from the subtype definition's properties
    pub fn would_subtype_condition_match_with_subtype(
        &self,
        condition: &SubtypeCondition,
        property_data: &HashMap<String, String>,
        subtype_def: &cw_model::types::Subtype,
    ) -> bool {
        // Special case for KeyMatches - we need to check if the original key matches the filter
        // e.g.
        // ## type_key_filter = "utility_component_template"
        // subtype[utility_component_template] = {}
        if let SubtypeCondition::KeyMatches { filter } = condition {
            if let Some(original_key) = property_data.get(ORIGINAL_KEY_PROPERTY) {
                if original_key.contains(filter) {
                    return true;
                }
            }
        }

        // Extract the property key from the condition
        let property_key = match condition {
            SubtypeCondition::PropertyEquals { key, .. } => Some(key),
            SubtypeCondition::PropertyNotEquals { key, .. } => Some(key),
            SubtypeCondition::PropertyExists { key } => Some(key),
            SubtypeCondition::PropertyNotExists { key } => Some(key),
            SubtypeCondition::KeyStartsWith { .. } => None,
            SubtypeCondition::KeyMatches { .. } => None,
            SubtypeCondition::Expression(_) => None,
        };

        // Get the cardinality for this specific property
        let cardinality = if let Some(key) = property_key {
            self.get_property_cardinality_from_subtype(subtype_def, key)
        } else {
            None
        };

        // Call the cardinality-aware version with the property-specific cardinality
        self.would_subtype_condition_match_with_cardinality(condition, property_data, &cardinality)
    }

    /// Check if a subtype condition would be satisfied for a specific property key
    /// This is useful for checking conditions that depend on the property key being accessed
    pub fn would_subtype_condition_match_for_key(
        &self,
        condition: &SubtypeCondition,
        property_data: &HashMap<String, String>,
        accessing_key: &str,
    ) -> bool {
        match condition {
            SubtypeCondition::KeyStartsWith { prefix } => {
                // Check if the key being accessed starts with the prefix
                accessing_key.starts_with(prefix)
            }
            SubtypeCondition::KeyMatches { filter } => {
                // Check if the key being accessed matches the filter
                accessing_key.contains(filter)
            }
            // For other conditions, fall back to the regular check
            _ => {
                self.would_subtype_condition_match_with_cardinality(condition, property_data, &None)
            }
        }
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

    /// Get all subtype names and their condition descriptions for a given type
    pub fn get_subtype_conditions(&self, cwt_type: &CwtType) -> Vec<(String, String)> {
        match cwt_type {
            CwtType::Block(block) => block
                .subtypes
                .iter()
                .map(|(name, subtype)| {
                    let description = self.describe_subtype_conditions(subtype, name);
                    (name.clone(), description)
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Create a human-readable description of subtype conditions
    fn describe_subtype_conditions(
        &self,
        subtype_def: &cw_model::types::Subtype,
        subtype_name: &str,
    ) -> String {
        // Handle CWT options first
        if let Some(starts_with) = &subtype_def.options.starts_with {
            return format!("key starts with '{}'", starts_with);
        } else if let Some(type_key_filter) = &subtype_def.options.type_key_filter {
            return match type_key_filter {
                cw_model::types::TypeKeyFilter::Specific(key) => {
                    format!("key matches '{}'", key)
                }
                cw_model::types::TypeKeyFilter::Not(key) => {
                    format!("key does not match '{}'", key)
                }
                cw_model::types::TypeKeyFilter::OneOf(keys) => {
                    format!("key matches any of [{}]", keys.join(", "))
                }
            };
        }

        // Describe condition_properties
        let property_conditions: Vec<String> = subtype_def
            .condition_properties
            .iter()
            .filter_map(|(key, property)| {
                match &property.property_type {
                    CwtType::Literal(value) => Some(format!("{} = {}", key, value)),
                    CwtType::Simple(_) => Some(format!("{} exists", key)),
                    CwtType::Block(_) => {
                        // Include block properties with their cardinality constraints
                        if let Some(cardinality) = &property.options.cardinality {
                            if cardinality.max == Some(0) {
                                Some(format!("{} must not exist (cardinality 0..0)", key))
                            } else if cardinality.min == Some(0) {
                                Some(format!(
                                    "{} is optional (cardinality 0..{})",
                                    key,
                                    cardinality.max.map_or("∞".to_string(), |m| m.to_string())
                                ))
                            } else {
                                Some(format!(
                                    "{} required (cardinality {}..{})",
                                    key,
                                    cardinality.min.unwrap_or(0),
                                    cardinality.max.map_or("∞".to_string(), |m| m.to_string())
                                ))
                            }
                        } else {
                            Some(format!("{} block exists", key))
                        }
                    }
                    _ => Some(format!("{} exists", key)),
                }
            })
            .collect();

        if property_conditions.is_empty() {
            format!("always matches ({})", subtype_name)
        } else {
            property_conditions.join(" AND ")
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
    use cw_model::types::{Cardinality, SubtypeCondition};
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
    fn test_cardinality_aware_property_equals() {
        let handler = SubtypeHandler::new(Arc::new(CwtAnalyzer::new()));
        let condition = SubtypeCondition::PropertyEquals {
            key: "is_origin".to_string(),
            value: "yes".to_string(),
        };

        let mut property_data = HashMap::new();
        property_data.insert("is_origin".to_string(), "yes".to_string());

        // Test with no cardinality (default requirement)
        assert!(handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &property_data,
            &None
        ));

        // Test with cardinality 0..1 (optional)
        let optional_cardinality = Some(Cardinality::optional());
        assert!(handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &property_data,
            &optional_cardinality
        ));

        // Test with missing property
        let empty_data = HashMap::new();
        assert!(!handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &empty_data,
            &None
        ));
        assert!(handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &empty_data,
            &optional_cardinality
        ));

        // Test with wrong value
        let mut wrong_data = HashMap::new();
        wrong_data.insert("is_origin".to_string(), "no".to_string());
        assert!(!handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &wrong_data,
            &None
        ));
        assert!(!handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &wrong_data,
            &optional_cardinality
        ));
    }

    #[test]
    fn test_cardinality_aware_property_not_equals() {
        let handler = SubtypeHandler::new(Arc::new(CwtAnalyzer::new()));
        let condition = SubtypeCondition::PropertyNotEquals {
            key: "is_origin".to_string(),
            value: "yes".to_string(),
        };

        // Test with cardinality 0..1 (optional) - missing property should match "not equals yes"
        let optional_cardinality = Some(Cardinality::optional());
        let empty_data = HashMap::new();
        assert!(handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &empty_data,
            &optional_cardinality
        ));

        // Test with property set to "no"
        let mut property_data = HashMap::new();
        property_data.insert("is_origin".to_string(), "no".to_string());
        assert!(handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &property_data,
            &optional_cardinality
        ));

        // Test with property set to "yes" (should not match)
        let mut wrong_data = HashMap::new();
        wrong_data.insert("is_origin".to_string(), "yes".to_string());
        assert!(!handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &wrong_data,
            &optional_cardinality
        ));
    }

    #[test]
    fn test_civic_or_origin_subtype_scenario() {
        let handler = SubtypeHandler::new(Arc::new(CwtAnalyzer::new()));

        // Simulate the civic_or_origin scenario from the user's example
        let origin_condition = SubtypeCondition::PropertyEquals {
            key: "is_origin".to_string(),
            value: "yes".to_string(),
        };

        let civic_condition = SubtypeCondition::PropertyNotEquals {
            key: "is_origin".to_string(),
            value: "yes".to_string(),
        };

        let no_cardinality = None; // Origin subtype has no cardinality (required)
        let optional_cardinality = Some(Cardinality::optional()); // Civic subtype has cardinality 0..1

        // Test case 1: is_origin = yes -> should match origin, not civic
        let mut origin_data = HashMap::new();
        origin_data.insert("is_origin".to_string(), "yes".to_string());

        assert!(handler.would_subtype_condition_match_with_cardinality(
            &origin_condition,
            &origin_data,
            &no_cardinality
        ));
        assert!(!handler.would_subtype_condition_match_with_cardinality(
            &civic_condition,
            &origin_data,
            &optional_cardinality
        ));

        // Test case 2: is_origin = no -> should match civic, not origin
        let mut civic_data = HashMap::new();
        civic_data.insert("is_origin".to_string(), "no".to_string());

        assert!(!handler.would_subtype_condition_match_with_cardinality(
            &origin_condition,
            &civic_data,
            &no_cardinality
        ));
        assert!(handler.would_subtype_condition_match_with_cardinality(
            &civic_condition,
            &civic_data,
            &optional_cardinality
        ));

        // Test case 3: is_origin absent -> should match civic (due to cardinality 0..1), not origin
        let empty_data = HashMap::new();

        assert!(!handler.would_subtype_condition_match_with_cardinality(
            &origin_condition,
            &empty_data,
            &no_cardinality
        ));
        assert!(handler.would_subtype_condition_match_with_cardinality(
            &civic_condition,
            &empty_data,
            &optional_cardinality
        ));
    }

    #[test]
    fn test_property_exists_with_cardinality() {
        let handler = SubtypeHandler::new(Arc::new(CwtAnalyzer::new()));
        let condition = SubtypeCondition::PropertyExists {
            key: "test_key".to_string(),
        };

        let mut property_data = HashMap::new();
        property_data.insert("test_key".to_string(), "test_value".to_string());

        // Test with no cardinality (default requirement)
        assert!(handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &property_data,
            &None
        ));

        // Test with cardinality 0..1 (optional)
        let optional_cardinality = Some(Cardinality::optional());
        assert!(handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &property_data,
            &optional_cardinality
        ));

        // Test with missing property
        let empty_data = HashMap::new();
        assert!(!handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &empty_data,
            &None
        ));
        assert!(!handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &empty_data,
            &optional_cardinality
        ));
    }

    #[test]
    fn test_property_not_exists_with_cardinality() {
        let handler = SubtypeHandler::new(Arc::new(CwtAnalyzer::new()));
        let condition = SubtypeCondition::PropertyNotExists {
            key: "test_key".to_string(),
        };

        // Test with cardinality 0..1 (optional) - property absence should match
        let optional_cardinality = Some(Cardinality::optional());
        let empty_data = HashMap::new();
        assert!(handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &empty_data,
            &optional_cardinality
        ));

        // Test with required cardinality - property absence should not match PropertyNotExists
        let required_cardinality = Some(Cardinality::required());
        assert!(!handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &empty_data,
            &required_cardinality
        ));

        // Test with property present - should not match PropertyNotExists
        let mut property_data = HashMap::new();
        property_data.insert("test_key".to_string(), "test_value".to_string());
        assert!(!handler.would_subtype_condition_match_with_cardinality(
            &condition,
            &property_data,
            &optional_cardinality
        ));
    }

    #[test]
    fn test_cardinality_zero_minimum_handling() {
        let handler = SubtypeHandler::new(Arc::new(CwtAnalyzer::new()));

        // Test PropertyNotEquals with cardinality 0..1 - absent property should match
        let civic_condition = SubtypeCondition::PropertyNotEquals {
            key: "is_origin".to_string(),
            value: "yes".to_string(),
        };
        let optional_cardinality = Some(Cardinality::optional()); // 0..1

        // Case 1: Property is absent - should match PropertyNotEquals (absent ≠ "yes")
        let empty_data = HashMap::new();
        assert!(
            handler.would_subtype_condition_match_with_cardinality(
                &civic_condition,
                &empty_data,
                &optional_cardinality
            ),
            "Absent property should match PropertyNotEquals with cardinality 0..1"
        );

        // Case 2: Property is "no" - should match PropertyNotEquals ("no" ≠ "yes")
        let mut civic_data = HashMap::new();
        civic_data.insert("is_origin".to_string(), "no".to_string());
        assert!(
            handler.would_subtype_condition_match_with_cardinality(
                &civic_condition,
                &civic_data,
                &optional_cardinality
            ),
            "Property 'no' should match PropertyNotEquals 'yes'"
        );

        // Case 3: Property is "yes" - should NOT match PropertyNotEquals ("yes" == "yes")
        let mut origin_data = HashMap::new();
        origin_data.insert("is_origin".to_string(), "yes".to_string());
        assert!(
            !handler.would_subtype_condition_match_with_cardinality(
                &civic_condition,
                &origin_data,
                &optional_cardinality
            ),
            "Property 'yes' should NOT match PropertyNotEquals 'yes'"
        );

        // Test PropertyEquals with cardinality 0..1 - absent property should NOT match
        let origin_condition = SubtypeCondition::PropertyEquals {
            key: "is_origin".to_string(),
            value: "yes".to_string(),
        };

        // Case 4: Property is absent - should match PropertyEquals with cardinality 0..1
        assert!(
            handler.would_subtype_condition_match_with_cardinality(
                &origin_condition,
                &empty_data,
                &optional_cardinality
            ),
            "Absent property should match PropertyEquals with cardinality 0..1"
        );

        // Case 5: Property is "yes" - should match PropertyEquals ("yes" == "yes")
        assert!(
            handler.would_subtype_condition_match_with_cardinality(
                &origin_condition,
                &origin_data,
                &optional_cardinality
            ),
            "Property 'yes' should match PropertyEquals 'yes'"
        );
    }

    #[test]
    fn test_cardinality_zero_max_forbidden() {
        let handler = SubtypeHandler::new(Arc::new(CwtAnalyzer::new()));
        let mut property_data = HashMap::new();
        property_data.insert("forbidden_key".to_string(), "some_value".to_string());

        // Test with cardinality 0..0 (forbidden - must not be present)
        let forbidden_cardinality = Some(Cardinality {
            min: Some(0),
            max: Some(0),
            soft: false,
        });

        // Property is present - should fail cardinality check (violates max = 0)
        assert!(
            !handler.property_satisfies_cardinality(
                "forbidden_key",
                &property_data,
                &forbidden_cardinality
            ),
            "Property present should fail 0..0 cardinality (forbidden)"
        );

        // Property is absent - should pass cardinality check
        assert!(
            handler.property_satisfies_cardinality(
                "missing_key",
                &property_data,
                &forbidden_cardinality
            ),
            "Property absent should pass 0..0 cardinality (forbidden)"
        );

        // Test with condition matching - PropertyExists should fail if property is forbidden
        let exists_condition = SubtypeCondition::PropertyExists {
            key: "forbidden_key".to_string(),
        };

        // Property exists but is forbidden by cardinality - should fail
        assert!(
            !handler.would_subtype_condition_match_with_cardinality(
                &exists_condition,
                &property_data,
                &forbidden_cardinality
            ),
            "PropertyExists should fail for forbidden property even if present"
        );

        // Test PropertyNotExists with forbidden cardinality - should pass for absent property
        let not_exists_condition = SubtypeCondition::PropertyNotExists {
            key: "forbidden_key".to_string(),
        };

        let empty_data = HashMap::new();
        assert!(
            handler.would_subtype_condition_match_with_cardinality(
                &not_exists_condition,
                &empty_data,
                &forbidden_cardinality
            ),
            "PropertyNotExists should pass for absent forbidden property"
        );

        // PropertyNotExists should fail if property exists (even with forbidden cardinality)
        assert!(
            !handler.would_subtype_condition_match_with_cardinality(
                &not_exists_condition,
                &property_data,
                &forbidden_cardinality
            ),
            "PropertyNotExists should fail if forbidden property is present"
        );
    }
}
