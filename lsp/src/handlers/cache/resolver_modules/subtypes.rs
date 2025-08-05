use crate::handlers::cache::ORIGINAL_KEY_PROPERTY;
use crate::handlers::cache::entity_restructurer::EntityRestructurer;
use crate::handlers::scope::{ScopeError, ScopeStack};
use crate::handlers::scoped_type::{CwtTypeOrSpecialRef, ScopedType};
use crate::interner::get_interner;
use cw_model::types::CwtAnalyzer;
use cw_model::{CwtType, Entity, SpurMap, Subtype, TypeKeyFilter, Value};
use lasso::Spur;
use std::collections::HashSet;
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
    fn does_subtype_match(&self, subtype_def: &Subtype, entity: &Entity) -> bool {
        let interner = get_interner();

        // Handle CWT options that affect matching first (these take precedence)
        if let Some(starts_with) = &subtype_def.options.starts_with {
            // Check if any key in entity starts with the prefix
            return entity.properties.kv.keys().any(|key| {
                interner
                    .resolve(&key)
                    .starts_with(interner.resolve(starts_with))
            });
        } else if let Some(type_key_filter) = &subtype_def.options.type_key_filter {
            return match type_key_filter {
                TypeKeyFilter::Specific(key) => {
                    // Check if the entity has the specific key OR if the original key matches
                    let has_property = entity.properties.kv.contains_key(key);
                    let original_key_matches = entity
                        .properties
                        .kv
                        .get(&interner.get_or_intern(ORIGINAL_KEY_PROPERTY))
                        .and_then(|prop_list| prop_list.0.first())
                        .and_then(|prop| match &prop.value {
                            Value::String(s) => {
                                Some(interner.resolve(s).contains(interner.resolve(key)))
                            }
                            _ => None,
                        })
                        .unwrap_or(false);

                    has_property || original_key_matches
                }
                TypeKeyFilter::Not(key) => {
                    // Check if the entity does NOT have the specific key AND the original key doesn't match
                    !entity.properties.kv.contains_key(key)
                        && entity
                            .properties
                            .kv
                            .get(&interner.get_or_intern(ORIGINAL_KEY_PROPERTY))
                            .and_then(|prop_list| prop_list.0.first())
                            .and_then(|prop| match &prop.value {
                                Value::String(s) => {
                                    Some(!interner.resolve(s).contains(interner.resolve(key)))
                                }
                                _ => Some(true),
                            })
                            .unwrap_or(true)
                }
                TypeKeyFilter::OneOf(keys) => {
                    // Check if the entity has any of the specified keys OR if the original key matches any
                    keys.iter()
                        .any(|key| entity.properties.kv.contains_key(key))
                        || entity
                            .properties
                            .kv
                            .get(&interner.get_or_intern(ORIGINAL_KEY_PROPERTY))
                            .and_then(|prop_list| prop_list.0.first())
                            .and_then(|prop| match &prop.value {
                                Value::String(s) => Some(keys.iter().any(|key| {
                                    interner.resolve(s).contains(interner.resolve(key))
                                })),
                                _ => None,
                            })
                            .unwrap_or(false)
                }
            };
        }

        // If no conditions, fallback to true (subtype always matches)
        if subtype_def.condition_properties.is_empty() {
            return true;
        }

        // Evaluate all conditions (they must all match - AND logic)
        subtype_def
            .condition_properties
            .iter()
            .all(|(key, property)| self.does_property_match_condition(key, property, entity))
    }

    /// Check if a property in an entity matches a specific condition
    fn does_property_match_condition(
        &self,
        property_key: Spur,
        condition_property: &cw_model::types::Property,
        entity: &Entity,
    ) -> bool {
        // Check cardinality constraints first
        let property_count = if entity.properties.kv.contains_key(&property_key) {
            1u32
        } else {
            0u32
        };

        if !self.satisfies_cardinality_constraint(
            property_count,
            &condition_property.options.cardinality,
        ) {
            return false;
        }

        // Get the actual property from the entity
        let actual_property = entity.properties.kv.get(&property_key);

        let res = match &*condition_property.property_type {
            CwtType::Literal(expected_value) => {
                // For literal conditions, check exact value match
                match actual_property {
                    Some(property_list) => {
                        if let Some(first_property) = property_list.0.first() {
                            match &first_property.value {
                                Value::String(s) => s == expected_value,
                                Value::Number(n) => n == expected_value,
                                _ => false,
                            }
                        } else {
                            // Property exists but has no values - check if cardinality allows absence
                            if let Some(card) = &condition_property.options.cardinality {
                                card.min.unwrap_or(1) == 0
                            } else {
                                false
                            }
                        }
                    }
                    None => {
                        // Property is absent - check if cardinality allows this
                        if let Some(card) = &condition_property.options.cardinality {
                            card.min.unwrap_or(1) == 0
                        } else {
                            false
                        }
                    }
                }
            }
            CwtType::Simple(_) => {
                // For simple types, just check existence based on cardinality
                match &condition_property.options.cardinality {
                    Some(cardinality) => {
                        if cardinality.max == Some(0) {
                            // Cardinality 0..0 means property must NOT exist
                            actual_property.is_none()
                        } else if cardinality.min.unwrap_or(0) > 0 {
                            // Property is required
                            actual_property.is_some()
                        } else {
                            // Optional property - always matches
                            true
                        }
                    }
                    None => {
                        // No cardinality constraint: property should exist (default requirement)
                        actual_property.is_some()
                    }
                }
            }
            CwtType::Block(expected_block) => {
                // For block conditions, recursively validate the structure
                match actual_property {
                    Some(property_list) => {
                        if let Some(first_property) = property_list.0.first() {
                            match &first_property.value {
                                Value::Entity(actual_entity) => self
                                    .does_entity_match_block_structure(
                                        actual_entity,
                                        expected_block,
                                    ),
                                _ => false, // Not a block/entity value
                            }
                        } else {
                            // Property exists but has no values - check if cardinality allows absence
                            if let Some(card) = &condition_property.options.cardinality {
                                card.min.unwrap_or(1) == 0
                            } else {
                                false
                            }
                        }
                    }
                    None => {
                        // Property is absent - check if cardinality allows this
                        if let Some(card) = &condition_property.options.cardinality {
                            card.min.unwrap_or(1) == 0
                        } else {
                            false
                        }
                    }
                }
            }
            CwtType::Union(union_types) => {
                // For union types, check if any of the union members matches
                // This handles cases like: station = shipclass_research_station | shipclass_mining_station
                union_types.iter().any(|union_type| {
                    // Create a temporary property with this union member's type
                    let temp_property = cw_model::types::Property {
                        property_type: union_type.clone(),
                        options: condition_property.options.clone(),
                        documentation: condition_property.documentation,
                    };

                    // Recursively check if this union member matches
                    self.does_property_match_condition(property_key, &temp_property, entity)
                })
            }
            _ => {
                // For other types, fall back to existence check
                actual_property.is_some()
            }
        };

        res
    }

    /// Check if an entity matches the structure defined in a CWT block
    fn does_entity_match_block_structure(
        &self,
        entity: &Entity,
        expected_block: &cw_model::types::BlockType,
    ) -> bool {
        // Check if all required properties in the expected block are satisfied
        expected_block
            .properties
            .iter()
            .all(|(key, property)| self.does_property_match_condition(key, property, entity))
    }

    /// Check if a property count satisfies cardinality constraints
    fn satisfies_cardinality_constraint(
        &self,
        property_count: u32,
        cardinality: &Option<cw_model::types::Cardinality>,
    ) -> bool {
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

    /// Get all available subtypes for a given type
    pub fn get_available_subtypes(&self, cwt_type: &CwtType) -> Vec<Spur> {
        match cwt_type {
            CwtType::Block(block) => block.subtypes.keys().collect(),
            _ => Vec::new(),
        }
    }

    /// Check if a type has a specific subtype
    pub fn has_subtype(&self, cwt_type: &CwtType, subtype_name: Spur) -> bool {
        match cwt_type {
            CwtType::Block(block) => block.subtypes.contains_key(&subtype_name),
            _ => false,
        }
    }

    /// Get subtype definition for a given type and subtype name
    pub fn get_subtype_definition<'b>(
        &self,
        cwt_type: &'b CwtType,
        subtype_name: Spur,
    ) -> Option<&'b cw_model::types::Subtype> {
        match cwt_type {
            CwtType::Block(block) => block.subtypes.get(&subtype_name),
            _ => None,
        }
    }

    /// Determine all matching subtypes based on entity structure
    /// This is the main method for determining active subtypes
    pub fn determine_matching_subtypes(
        &self,
        scoped_type: Arc<ScopedType>,
        entity: &Entity,
    ) -> HashSet<Spur> {
        let interner = get_interner();

        match scoped_type.cwt_type_for_matching() {
            CwtTypeOrSpecialRef::Block(block) => {
                // Check each subtype condition and collect all matches
                let mut matching_subtypes = HashSet::new();

                for (subtype_name, subtype_def) in &block.subtypes {
                    if subtype_def.is_inverted {
                        continue; // Handled by the else below
                    }

                    // Directly evaluate if the subtype matches
                    let matches = self.does_subtype_match(subtype_def, entity);

                    if matches {
                        matching_subtypes.insert(subtype_name.clone());
                    } else {
                        matching_subtypes.insert(
                            interner.get_or_intern(format!("!{}", interner.resolve(&subtype_name))),
                        );
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
        namespace: Spur,
        cwt_type: &CwtType,
        subtype_name: Spur,
    ) -> Vec<Spur> {
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
            // Directly evaluate if the subtype matches
            if self.does_subtype_match(subtype_def, &entity) {
                matching_keys.push(entity_key);
            }
        }

        matching_keys
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
            if let Some(scope_name) = self.cwt_analyzer.resolve_scope_name(*push_scope) {
                new_scope.push_scope_type(scope_name)?;
            }
        }

        // Apply replace_scope if present
        if let Some(replace_scope) = &subtype_def.options.replace_scope {
            let mut new_scopes = SpurMap::new();

            for (key, value) in replace_scope {
                if let Some(scope_name) = self.cwt_analyzer.resolve_scope_name(*value) {
                    new_scopes.insert(key.clone(), scope_name);
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
        active_subtypes: &HashSet<Spur>,
    ) -> Result<ScopeStack, ScopeError> {
        let mut new_scope = scope_stack.branch();

        match cwt_type {
            CwtType::Block(block) => {
                // Apply scope changes from all active subtypes
                for subtype_name in active_subtypes {
                    if let Some(subtype_def) = block.subtypes.get(&subtype_name) {
                        new_scope = self.apply_subtype_scope_changes(&new_scope, subtype_def)?;
                    }
                }
            }
            _ => {}
        }

        Ok(new_scope)
    }
}
