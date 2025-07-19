use std::collections::{HashMap, HashSet};

use cw_model::Entity;

use crate::handlers::cache::EntityRestructurer;

pub struct ScriptedEffectArgumentCollector {
    scripted_effect_arguments: HashMap<String, HashSet<String>>,
}

impl ScriptedEffectArgumentCollector {
    pub fn new() -> Self {
        Self {
            scripted_effect_arguments: HashMap::new(),
        }
    }

    pub fn collect(mut self) -> HashMap<String, HashSet<String>> {
        // Only collect from scripted_effects namespace using EntityRestructurer
        if let Some(scripted_effects_entities) =
            EntityRestructurer::get_all_namespace_entities("common/scripted_effects")
        {
            for (effect_name, entity) in scripted_effects_entities {
                let arguments = self.extract_arguments_from_entity(&entity);
                if !arguments.is_empty() {
                    self.scripted_effect_arguments
                        .insert(effect_name, arguments);
                }
            }
        }

        if let Some(scripted_triggers_entities) =
            EntityRestructurer::get_all_namespace_entities("common/scripted_triggers")
        {
            for (trigger_name, entity) in scripted_triggers_entities {
                let arguments = self.extract_arguments_from_entity(&entity);
                if !arguments.is_empty() {
                    self.scripted_effect_arguments
                        .insert(trigger_name, arguments);
                }
            }
        }

        self.scripted_effect_arguments
    }

    fn extract_arguments_from_entity(&self, entity: &Entity) -> HashSet<String> {
        let mut arguments = HashSet::new();
        self.extract_arguments_recursive(entity, &mut arguments);
        arguments
    }

    fn extract_arguments_recursive(&self, entity: &Entity, arguments: &mut HashSet<String>) {
        // Extract arguments from all string values in the entity
        for (_key, property_value) in &entity.properties.kv {
            for value in &property_value.0 {
                if let Some(string_value) = value.value.as_string() {
                    self.extract_arguments_from_string(string_value, arguments);
                } else if let Some(nested_entity) = value.value.as_entity() {
                    self.extract_arguments_recursive(nested_entity, arguments);
                }
            }
        }

        // Also check items (for arrays)
        for item in &entity.items {
            if let Some(string_value) = item.as_string() {
                self.extract_arguments_from_string(string_value, arguments);
            } else if let Some(nested_entity) = item.as_entity() {
                self.extract_arguments_recursive(nested_entity, arguments);
            }
        }

        // Also check conditional blocks
        for (condition, conditional_block) in &entity.conditional_blocks {
            arguments.insert(condition.clone());

            // Extract arguments from conditional block properties
            for (_key, property_value) in &conditional_block.properties.kv {
                for value in &property_value.0 {
                    if let Some(string_value) = value.value.as_string() {
                        self.extract_arguments_from_string(string_value, arguments);
                    } else if let Some(nested_entity) = value.value.as_entity() {
                        self.extract_arguments_recursive(nested_entity, arguments);
                    }
                }
            }

            // Extract arguments from conditional block items
            for item in &conditional_block.items {
                if let Some(string_value) = item.as_string() {
                    self.extract_arguments_from_string(string_value, arguments);
                } else if let Some(nested_entity) = item.as_entity() {
                    self.extract_arguments_recursive(nested_entity, arguments);
                }
            }
        }
    }

    fn extract_arguments_from_string(&self, string_value: &str, arguments: &mut HashSet<String>) {
        // Find all occurrences of $...$ patterns
        let mut chars = string_value.char_indices().peekable();
        while let Some((start_idx, ch)) = chars.next() {
            if ch == '$' {
                let mut end_idx = start_idx + 1;
                let mut found_end = false;

                // Find the closing $
                while let Some((idx, ch)) = chars.next() {
                    if ch == '$' {
                        end_idx = idx;
                        found_end = true;
                        break;
                    }
                }

                if found_end && end_idx > start_idx + 1 {
                    // Extract the content between $ signs
                    let content = &string_value[start_idx + 1..end_idx];
                    if !content.is_empty() {
                        // Handle fallback syntax: $VARIABLE|fallback$ -> extract just VARIABLE
                        let arg_name = if let Some(pipe_pos) = content.find('|') {
                            &content[..pipe_pos]
                        } else {
                            content
                        };

                        if !arg_name.is_empty() {
                            arguments.insert(arg_name.to_string());
                        }
                    }
                }
            }
        }
    }
}
