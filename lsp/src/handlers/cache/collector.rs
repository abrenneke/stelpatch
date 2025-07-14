use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use cw_model::{ComplexEnumDefinition, CwtType, Entity, ReferenceType};
use rayon::prelude::*;

use crate::handlers::{
    cache::{
        Namespace, entity_restructurer::EntityRestructurer, game_data::GameDataCache,
        get_namespace_entity_type, resolver::TypeResolver,
    },
    scoped_type::{CwtTypeOrSpecial, PropertyNavigationResult, ScopedType},
};

pub struct DataCollector<'game_data, 'resolver> {
    value_sets: HashMap<String, HashSet<String>>,
    complex_enums: HashMap<String, HashSet<String>>,
    scripted_effect_arguments: HashMap<String, HashSet<String>>, // Also scripted triggers for convenience... might be wrong because clashes
    game_data: &'game_data GameDataCache,
    type_resolver: &'resolver TypeResolver,
}

impl<'game_data, 'resolver> DataCollector<'game_data, 'resolver> {
    pub fn new(
        game_data: &'game_data GameDataCache,
        type_resolver: &'resolver TypeResolver,
    ) -> Self {
        Self {
            value_sets: HashMap::new(),
            complex_enums: HashMap::new(),
            scripted_effect_arguments: HashMap::new(),
            game_data,
            type_resolver,
        }
    }

    pub fn value_sets(&self) -> &HashMap<String, HashSet<String>> {
        &self.value_sets
    }

    pub fn complex_enums(&self) -> &HashMap<String, HashSet<String>> {
        &self.complex_enums
    }

    pub fn scripted_effect_arguments(&self) -> &HashMap<String, HashSet<String>> {
        &self.scripted_effect_arguments
    }

    pub fn collect_all(&mut self) {
        // Collect value_sets from parallel processing
        let results: Vec<HashMap<String, HashSet<String>>> = self
            .game_data
            .get_namespaces()
            .par_iter()
            .filter_map(|(namespace, namespace_data)| {
                get_namespace_entity_type(namespace)
                    .and_then(|namespace_type| namespace_type.scoped_type)
                    .map(|scoped_type| {
                        self.collect_value_sets_from_namespace(namespace_data, scoped_type)
                    })
            })
            .collect();

        // Merge all results into the main value_sets HashMap
        for result in results {
            for (key, values) in result {
                self.value_sets.entry(key).or_default().extend(values);
            }
        }

        // Collect scripted effect arguments
        self.collect_scripted_effect_arguments();

        // Collect complex enums
        self.collect_complex_enums();
    }

    fn collect_value_sets_from_namespace(
        &self,
        namespace_data: &Namespace,
        scoped_type: Arc<ScopedType>,
    ) -> HashMap<String, HashSet<String>> {
        // Process entities in parallel within the namespace
        let results: Vec<HashMap<String, HashSet<String>>> = namespace_data
            .entities
            .par_iter()
            .map(|(_entity_name, entity)| {
                self.collect_value_sets_from_entity(entity, scoped_type.clone())
            })
            .collect();

        // Merge results from this namespace
        let mut namespace_value_sets: HashMap<String, HashSet<String>> = HashMap::new();
        for result in results {
            for (key, values) in result {
                namespace_value_sets.entry(key).or_default().extend(values);
            }
        }

        namespace_value_sets
    }

    fn collect_value_sets_from_entity(
        &self,
        entity: &Entity,
        scoped_type: Arc<ScopedType>,
    ) -> HashMap<String, HashSet<String>> {
        let mut entity_value_sets = HashMap::new();

        for (property_name, property_value) in entity.properties.kv.iter() {
            let property_type = self
                .type_resolver
                .navigate_to_property(scoped_type.clone(), property_name);

            if let PropertyNavigationResult::Success(property_type) = property_type {
                match property_type.cwt_type() {
                    CwtTypeOrSpecial::CwtType(CwtType::Reference(ReferenceType::ValueSet {
                        key,
                    })) => {
                        let mut values = HashSet::new();
                        for value in property_value.0.iter() {
                            if let Some(value) = value.value.as_string() {
                                values.insert(value.clone());
                            }
                        }
                        if !values.is_empty() {
                            entity_value_sets.insert(key.clone(), values);
                        }
                    }
                    CwtTypeOrSpecial::CwtType(CwtType::Block(_)) => {
                        for value in property_value.0.iter() {
                            if let Some(value) = value.value.as_entity() {
                                let nested_results = self
                                    .collect_value_sets_from_entity(value, property_type.clone());
                                for (key, values) in nested_results {
                                    entity_value_sets.entry(key).or_default().extend(values);
                                }
                            }
                        }
                    }
                    CwtTypeOrSpecial::CwtType(CwtType::Union(union_types)) => {
                        // Process all union members that are blocks
                        for union_type in union_types {
                            match union_type {
                                CwtType::Block(_) => {
                                    // Create a scoped type for this union member
                                    let union_member_type = Arc::new(ScopedType::new_cwt(
                                        union_type.clone(),
                                        property_type.scope_stack().clone(),
                                        property_type.in_scripted_effect_block().cloned(),
                                    ));

                                    for value in property_value.0.iter() {
                                        if let Some(value) = value.value.as_entity() {
                                            let nested_results = self
                                                .collect_value_sets_from_entity(
                                                    value,
                                                    union_member_type.clone(),
                                                );
                                            for (key, values) in nested_results {
                                                entity_value_sets
                                                    .entry(key)
                                                    .or_default()
                                                    .extend(values);
                                            }
                                        }
                                    }
                                }
                                CwtType::Reference(ReferenceType::ValueSet { key }) => {
                                    // Handle value sets within unions
                                    let mut values = HashSet::new();
                                    for value in property_value.0.iter() {
                                        if let Some(value) = value.value.as_string() {
                                            values.insert(value.clone());
                                        }
                                    }
                                    if !values.is_empty() {
                                        entity_value_sets.insert(key.clone(), values);
                                    }
                                }
                                _ => {
                                    // Skip other union member types
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        entity_value_sets
    }

    fn collect_complex_enums(&mut self) {
        // Get all enum definitions from the CwtAnalyzer
        let enum_definitions = self.type_resolver.get_enums();

        // Process each complex enum
        for (enum_name, enum_def) in enum_definitions {
            if let Some(complex_def) = &enum_def.complex {
                let values = self.extract_complex_enum_values(complex_def);
                if !values.is_empty() {
                    self.complex_enums.insert(enum_name.to_string(), values);
                }
            }
        }
    }

    fn collect_scripted_effect_arguments(&mut self) {
        // Only collect from scripted_effects namespace
        if let Some(scripted_effects_namespace) = self
            .game_data
            .get_namespaces()
            .get("common/scripted_effects")
        {
            for (effect_name, entity) in &scripted_effects_namespace.entities {
                let arguments = self.extract_arguments_from_entity(entity);
                if !arguments.is_empty() {
                    self.scripted_effect_arguments
                        .insert(effect_name.clone(), arguments);
                }
            }
        }

        if let Some(scripted_triggers_namespace) = self
            .game_data
            .get_namespaces()
            .get("common/scripted_triggers")
        {
            for (trigger_name, entity) in &scripted_triggers_namespace.entities {
                let arguments = self.extract_arguments_from_entity(entity);
                if !arguments.is_empty() {
                    self.scripted_effect_arguments
                        .insert(trigger_name.clone(), arguments);
                }
            }
        }
    }

    fn extract_complex_enum_values(&self, complex_def: &ComplexEnumDefinition) -> HashSet<String> {
        let mut values = HashSet::new();

        // Get the namespace for the specified path
        let path = complex_def.path.trim_start_matches("game/");

        // Use EntityRestructurer to get entities, which handles special loading rules
        if let Some(entities) = EntityRestructurer::get_all_namespace_entities(path) {
            for (_entity_name, entity) in &entities {
                if let Some(extracted_values) = self.extract_values_from_entity(
                    entity,
                    &complex_def.name_structure,
                    complex_def.start_from_root,
                ) {
                    values.extend(extracted_values);
                }
            }
        }

        values
    }

    fn extract_values_from_entity(
        &self,
        entity: &Entity,
        name_structure: &CwtType,
        start_from_root: bool,
    ) -> Option<HashSet<String>> {
        let mut values = HashSet::new();

        // If start_from_root is true, we start from the entity itself
        // Otherwise, we start from the first level properties
        if start_from_root {
            self.extract_values_recursive(entity, name_structure, &mut values);
        } else {
            // Process each top-level property by matching against name_structure
            if let CwtType::Block(block_type) = name_structure {
                for (property_name, property_value) in &entity.properties.kv {
                    if let Some(expected_property) = block_type.properties.get(property_name) {
                        for value in &property_value.0 {
                            if let Some(nested_entity) = value.value.as_entity() {
                                // Pass the inner structure instead of the entire name_structure
                                self.extract_values_recursive(
                                    nested_entity,
                                    &expected_property.property_type,
                                    &mut values,
                                );
                            }
                        }
                    }
                }
            }
        }

        if values.is_empty() {
            None
        } else {
            Some(values)
        }
    }

    fn extract_values_recursive(
        &self,
        entity: &Entity,
        name_structure: &CwtType,
        values: &mut HashSet<String>,
    ) {
        match name_structure {
            CwtType::Block(block_type) => {
                // Process each property in the block structure
                for (property_name, property_type) in &block_type.properties {
                    if let Some(property_value) = entity.properties.kv.get(property_name) {
                        for value in &property_value.0 {
                            match &property_type.property_type {
                                CwtType::Literal(literal) if literal == "enum_name" => {
                                    // This is the special marker for enum name extraction
                                    if let Some(string_value) = value.value.as_string() {
                                        values.insert(string_value.clone());
                                    }
                                }
                                CwtType::Block(_) => {
                                    // Recurse into nested blocks
                                    if let Some(nested_entity) = value.value.as_entity() {
                                        self.extract_values_recursive(
                                            nested_entity,
                                            &property_type.property_type,
                                            values,
                                        );
                                    }
                                }
                                CwtType::Array(array_type) => {
                                    // Recurse into the array element type
                                    self.extract_values_recursive(
                                        entity,
                                        &array_type.element_type,
                                        values,
                                    );
                                }
                                CwtType::Union(union_types) => {
                                    // Process all union members
                                    for union_type in union_types {
                                        match union_type {
                                            CwtType::Literal(literal) if literal == "enum_name" => {
                                                // Extract all string values from the property value entities
                                                for value in &property_value.0 {
                                                    if let Some(property_entity) =
                                                        value.value.as_entity()
                                                    {
                                                        for item in &property_entity.items {
                                                            if let Some(string_value) =
                                                                item.as_string()
                                                            {
                                                                values.insert(string_value.clone());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            _ => {
                                                // Recurse into other union member types
                                                self.extract_values_recursive(
                                                    entity, union_type, values,
                                                );
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    // For scalar matches, we can match any key
                                    if property_name == "scalar" {
                                        // Match any property in the entity
                                        for (key, _) in &entity.properties.kv {
                                            values.insert(key.clone());
                                        }
                                    }
                                    // For any other type, if it's a string value, extract it
                                    // This handles cases where enum_name is not a literal but a reference
                                    if let Some(string_value) = value.value.as_string() {
                                        values.insert(string_value.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            CwtType::Literal(literal) if literal == "enum_name" => {
                // Direct enum name extraction - extract all keys as potential enum names
                for (key, _) in &entity.properties.kv {
                    values.insert(key.clone());
                }
            }
            CwtType::Array(array_type) => {
                // For arrays, check if the element type is enum_name
                if let CwtType::Literal(literal) = &*array_type.element_type {
                    if literal == "enum_name" {
                        // Extract all string values from entity items
                        for item in &entity.items {
                            if let Some(string_value) = item.as_string() {
                                values.insert(string_value.clone());
                            }
                        }
                    }
                } else {
                    // For other element types, recurse into the element type
                    self.extract_values_recursive(entity, &array_type.element_type, values);
                }
            }
            CwtType::Union(union_types) => {
                // Process all union members
                for union_type in union_types {
                    match union_type {
                        CwtType::Literal(literal) if literal == "enum_name" => {
                            // Extract all string values from entity items
                            for item in &entity.items {
                                if let Some(string_value) = item.as_string() {
                                    values.insert(string_value.clone());
                                }
                            }
                        }
                        _ => {
                            // Recurse into other union member types
                            self.extract_values_recursive(entity, union_type, values);
                        }
                    }
                }
            }
            _ => {
                // For other types, we don't extract values
            }
        }
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
