use crate::handlers::scope::{ScopeError, ScopeStack};
use cw_model::CwtType;
use cw_model::types::{CwtAnalyzer, SubtypeCondition};
use std::collections::HashMap;
use std::sync::Arc;

pub struct SubtypeHandler {
    pub cwt_analyzer: Arc<CwtAnalyzer>,
}

impl SubtypeHandler {
    pub fn new(cwt_analyzer: Arc<CwtAnalyzer>) -> Self {
        Self { cwt_analyzer }
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

    /// Check if a subtype condition would be satisfied
    /// This checks basic conditions like property existence, equality, etc.
    pub fn would_subtype_condition_match(
        &self,
        condition: &SubtypeCondition,
        property_data: &HashMap<String, String>,
    ) -> bool {
        match condition {
            SubtypeCondition::PropertyEquals { key, value } => {
                // Check if the property exists and equals the expected value
                property_data
                    .get(key)
                    .map_or(false, |prop_value| prop_value == value)
            }
            SubtypeCondition::PropertyNotEquals { key, value } => {
                // Check if the property doesn't exist or doesn't equal the value
                property_data
                    .get(key)
                    .map_or(true, |prop_value| prop_value != value)
            }
            SubtypeCondition::PropertyExists { key } => {
                // Check if the property exists
                property_data.contains_key(key)
            }
            SubtypeCondition::PropertyNotExists { key } => {
                // Check if the property doesn't exist
                !property_data.contains_key(key)
            }
            SubtypeCondition::KeyStartsWith { prefix } => {
                // Check if any property key starts with the prefix
                property_data.keys().any(|key| key.starts_with(prefix))
            }
            SubtypeCondition::KeyMatches { filter } => {
                // For now, treat this as a simple string match
                // This could be extended to support regex or glob patterns
                property_data.keys().any(|key| key.contains(filter))
            }
            SubtypeCondition::Expression(_expr) => {
                // Complex expressions would require a full parser/evaluator
                // For now, return false - this could be extended in the future
                false
            }
        }
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
            _ => self.would_subtype_condition_match(condition, property_data),
        }
    }

    /// Determine the most likely subtype based on property data
    /// This is useful for auto-detecting subtypes in LSP contexts
    pub fn determine_likely_subtype(
        &self,
        cwt_type: &CwtType,
        property_data: &HashMap<String, String>,
    ) -> Option<String> {
        match cwt_type {
            CwtType::Block(block) => {
                // Check each subtype condition and return the first match
                for (subtype_name, subtype_def) in &block.subtypes {
                    if self.would_subtype_condition_match(&subtype_def.condition, property_data) {
                        return Some(subtype_name.clone());
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Get all matching subtypes for given property data
    /// Multiple subtypes could potentially match the same data
    pub fn get_matching_subtypes(
        &self,
        cwt_type: &CwtType,
        property_data: &HashMap<String, String>,
    ) -> Vec<String> {
        match cwt_type {
            CwtType::Block(block) => {
                let mut matching_subtypes = Vec::new();
                for (subtype_name, subtype_def) in &block.subtypes {
                    if self.would_subtype_condition_match(&subtype_def.condition, property_data) {
                        matching_subtypes.push(subtype_name.clone());
                    }
                }
                matching_subtypes
            }
            _ => Vec::new(),
        }
    }

    /// Get all subtype names and their conditions for a given type
    pub fn get_subtype_conditions<'b>(
        &self,
        cwt_type: &'b CwtType,
    ) -> Vec<(String, &'b SubtypeCondition)> {
        match cwt_type {
            CwtType::Block(block) => block
                .subtypes
                .iter()
                .map(|(name, subtype)| (name.clone(), &subtype.condition))
                .collect(),
            _ => Vec::new(),
        }
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
}
