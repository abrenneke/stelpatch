use std::collections::{HashMap, HashSet};

use cw_model::{ComplexEnumDefinition, CwtType, Entity};
use lasso::Spur;

use crate::{
    handlers::cache::{EntityRestructurer, resolver::TypeResolver},
    interner::get_interner,
};

pub struct ComplexEnumCollector<'resolver> {
    complex_enums: HashMap<Spur, HashSet<Spur>>,
    type_resolver: &'resolver TypeResolver,
}

impl<'resolver> ComplexEnumCollector<'resolver> {
    pub fn new(type_resolver: &'resolver TypeResolver) -> Self {
        Self {
            complex_enums: HashMap::new(),
            type_resolver,
        }
    }

    pub fn collect(mut self) -> HashMap<Spur, HashSet<Spur>> {
        // Get all enum definitions from the CwtAnalyzer
        let enum_definitions = self.type_resolver.get_enums();

        // Process each complex enum
        for (enum_name, enum_def) in enum_definitions {
            if let Some(complex_def) = &enum_def.complex {
                let values = self.extract_complex_enum_values(complex_def, *enum_name);
                if !values.is_empty() {
                    let set = self.complex_enums.entry(*enum_name).or_default();
                    set.extend(values);
                }
            }
        }

        // Postprocess: convert all values to lowercase
        // TODO
        // for (_key, value_set) in &mut self.complex_enums {
        //     let lowercase_values: HashSet<Spur> =
        //         value_set.iter().map(|v| v.to_lowercase()).collect();
        //     *value_set = lowercase_values;
        // }

        self.complex_enums
    }

    fn extract_complex_enum_values(
        &self,
        complex_def: &ComplexEnumDefinition,
        enum_name: Spur,
    ) -> HashSet<Spur> {
        let mut values: HashSet<Spur> = HashSet::new();

        let interner = get_interner();

        // Get the namespace for the specified path
        let path = interner
            .resolve(&complex_def.path)
            .trim_start_matches("game/");

        // Check if this is a flat list extraction pattern (name = { enum_name })
        // This happens when enum_name is in additional_flags, meaning extract all values directly
        let is_flat_list_pattern = match &*complex_def.name_structure {
            CwtType::Block(block_type) => {
                // Check if additional_flags contains enum_name literal
                block_type
                    .additional_flags
                    .iter()
                    .any(|flag| matches!(&**flag, CwtType::Literal(lit) if *lit == interner.get_or_intern("enum_name")))
            }
            CwtType::Literal(lit) if *lit == interner.get_or_intern("enum_name") => true,
            _ => false,
        };

        // For flat list patterns, try to get flat string values (for cases like component_tags)
        if is_flat_list_pattern {
            if let Some(namespace_values) =
                EntityRestructurer::get_namespace_values(interner.get_or_intern(path))
            {
                values.extend(namespace_values.into_iter());
                return values;
            }
        }

        // Fall back to structured entity extraction
        if let Some(entities) =
            EntityRestructurer::get_all_namespace_entities(interner.get_or_intern(path))
        {
            for (_entity_name, entity) in &entities {
                if let Some(extracted_values) = self.extract_values_from_entity(
                    entity,
                    &complex_def.name_structure,
                    complex_def.start_from_root,
                    enum_name,
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
        enum_name: Spur,
    ) -> Option<HashSet<Spur>> {
        let mut values = HashSet::new();
        let interner = get_interner();

        // If start_from_root is true, we start from the entity itself
        // Otherwise, we start from the first level properties
        if start_from_root {
            self.extract_values_recursive(entity, name_structure, &mut values, enum_name);
        } else {
            // Process each top-level property by matching against name_structure
            if let CwtType::Block(block_type) = name_structure {
                for (property_name, property_value) in &entity.properties.kv {
                    if let Some(expected_property) = block_type.properties.get(property_name) {
                        for value in &property_value.0 {
                            match &*expected_property.property_type {
                                CwtType::Literal(literal)
                                    if *literal == interner.get_or_intern("enum_name") =>
                                {
                                    // Handle direct string values for enum_name
                                    if let Some(string_value) = value.value.as_string() {
                                        values.insert(string_value.clone());
                                    }
                                }
                                _ => {
                                    // Handle nested entities for other types
                                    if let Some(nested_entity) = value.value.as_entity() {
                                        // Pass the inner structure instead of the entire name_structure
                                        self.extract_values_recursive(
                                            nested_entity,
                                            &expected_property.property_type,
                                            &mut values,
                                            enum_name,
                                        );
                                    }
                                }
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
        values: &mut HashSet<Spur>,
        enum_name: Spur,
    ) {
        let interner = get_interner();

        match name_structure {
            CwtType::Block(block_type) => {
                // Check if enum_name is in additional_flags - if so, extract from entity.items
                let has_enum_name_flag = block_type
                    .additional_flags
                    .iter()
                    .any(|flag| matches!(&**flag, CwtType::Literal(lit) if *lit == interner.get_or_intern("enum_name")));

                // Check if any expected property is enum_name - if so, extract from entity.properties
                let has_enum_name_property = block_type
                    .properties
                    .keys()
                    .any(|key| *key == interner.get_or_intern("enum_name"));

                if has_enum_name_flag {
                    // Extract all string items as enum values
                    for item in &entity.items {
                        if let Some(string_value) = item.as_string() {
                            values.insert(string_value.clone());
                        }
                    }
                }

                if has_enum_name_property {
                    // Extract all keys from the current entity as enum values
                    for (key, _) in &entity.properties.kv {
                        values.insert(key.clone());
                    }
                }

                if !has_enum_name_flag && !has_enum_name_property {
                    // Process each property in the block structure normally
                    for (property_name, property_type) in &block_type.properties {
                        if let Some(property_value) = entity.properties.kv.get(property_name) {
                            for value in &property_value.0 {
                                match &*property_type.property_type {
                                    CwtType::Literal(literal)
                                        if *literal == interner.get_or_intern("enum_name") =>
                                    {
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
                                                enum_name,
                                            );
                                        }
                                    }
                                    CwtType::Array(array_type) => {
                                        // Recurse into the array element type
                                        self.extract_values_recursive(
                                            entity,
                                            &array_type.element_type,
                                            values,
                                            enum_name,
                                        );
                                    }
                                    CwtType::Union(union_types) => {
                                        // Process all union members
                                        for union_type in union_types {
                                            match &**union_type {
                                                CwtType::Literal(literal)
                                                    if *literal
                                                        == interner.get_or_intern("enum_name") =>
                                                {
                                                    // Extract all string values from the property value entities
                                                    for value in &property_value.0 {
                                                        if let Some(property_entity) =
                                                            value.value.as_entity()
                                                        {
                                                            for item in &property_entity.items {
                                                                if let Some(string_value) =
                                                                    item.as_string()
                                                                {
                                                                    values.insert(
                                                                        string_value.clone(),
                                                                    );
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                _ => {
                                                    // Recurse into other union member types
                                                    self.extract_values_recursive(
                                                        entity, union_type, values, enum_name,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        // For scalar matches, we can match any key
                                        if *property_name == interner.get_or_intern("scalar") {
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
            }
            CwtType::Literal(literal) if *literal == interner.get_or_intern("enum_name") => {
                // Direct enum name extraction - extract all keys as potential enum names
                for (key, _) in &entity.properties.kv {
                    values.insert(key.clone());
                }
            }
            CwtType::Array(array_type) => {
                // For arrays, check if the element type is enum_name
                if let CwtType::Literal(literal) = &**array_type.element_type {
                    if *literal == interner.get_or_intern("enum_name") {
                        // Extract all string values from entity items
                        for item in &entity.items {
                            if let Some(string_value) = item.as_string() {
                                values.insert(string_value.clone());
                            }
                        }
                    }
                } else {
                    // For other element types, recurse into the element type
                    self.extract_values_recursive(
                        entity,
                        &array_type.element_type,
                        values,
                        enum_name,
                    );
                }
            }
            CwtType::Union(union_types) => {
                // Process all union members
                for union_type in union_types {
                    match &**union_type {
                        CwtType::Literal(literal)
                            if *literal == interner.get_or_intern("enum_name") =>
                        {
                            // Extract all string values from entity items
                            for item in &entity.items {
                                if let Some(string_value) = item.as_string() {
                                    values.insert(string_value.clone());
                                }
                            }
                        }
                        _ => {
                            // Recurse into other union member types
                            self.extract_values_recursive(entity, union_type, values, enum_name);
                        }
                    }
                }
            }
            _ => {
                // For other types, we don't extract values
            }
        }
    }
}
