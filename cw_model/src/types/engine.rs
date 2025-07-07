use std::collections::HashMap;

use crate::types::{
    ArrayType, Cardinality, InferredType, ObjectType, PrimitiveType, PropertyDefinition,
    TypeInferenceConfig, TypeRegistry,
};
use crate::{Module, Value};

/// Engine that processes modules and entities to infer types
pub struct TypeInferenceEngine {
    registry: TypeRegistry,
}

impl TypeInferenceEngine {
    pub fn new() -> Self {
        Self {
            registry: TypeRegistry::new(),
        }
    }

    pub fn with_config(config: TypeInferenceConfig) -> Self {
        Self {
            registry: TypeRegistry::with_config(config),
        }
    }

    /// Process a single module to infer types
    pub fn process_module(&mut self, module: &Module) {
        // Process module-level properties - merge all values for the same namespace
        // into a single type that represents the structure of all entities
        let mut all_entities = Vec::new();

        for (_key, property_list) in &module.properties.kv {
            for property in property_list.iter() {
                // Each top-level key represents an entity instance
                // We want to merge all these entities into one type
                all_entities.push(self.infer_from_value(&property.value, 0));
            }
        }

        // Process module-level values
        for value in &module.values {
            all_entities.push(self.infer_from_value(value, 0));
        }

        // Merge all entities into a single type for this namespace
        if !all_entities.is_empty() {
            let merged_type = all_entities
                .into_iter()
                .reduce(|acc, t| self.registry.merge_types(acc, t))
                .unwrap();

            // Use a generic name for the merged type
            self.registry
                .observe_type(&module.namespace, "entity", merged_type);
        }
    }

    /// Process multiple modules to build a complete type registry
    pub fn process_modules(&mut self, modules: &[&Module]) {
        for module in modules {
            self.process_module(module);
        }
    }

    /// Infer type from a single value with depth tracking to prevent stack overflow
    fn infer_from_value(&self, value: &Value, depth: usize) -> InferredType {
        // Check depth limit to prevent stack overflow
        if depth > self.registry.config.max_depth {
            return InferredType::Unknown;
        }

        match value {
            Value::String(s) => {
                if self.registry.config.infer_booleans {
                    match s.to_lowercase().as_str() {
                        "yes" | "true" | "no" | "false" => InferredType::Literal(s.clone()),
                        _ => InferredType::Literal(s.clone()),
                    }
                } else {
                    InferredType::Literal(s.clone())
                }
            }

            Value::Number(n) => {
                // Try to determine if it's an integer or float by parsing the string
                if n.contains('.') || n.contains('e') || n.contains('E') {
                    InferredType::Primitive(PrimitiveType::Float)
                } else {
                    InferredType::Primitive(PrimitiveType::Integer)
                }
            }

            Value::Entity(entity) => {
                let mut properties = HashMap::new();

                // Process entity properties
                for (key, property_list) in &entity.properties.kv {
                    let mut types = Vec::new();

                    for property in property_list.iter() {
                        types.push(self.infer_from_value(&property.value, depth + 1));
                    }

                    let merged_type = if types.len() == 1 {
                        types.into_iter().next().unwrap()
                    } else if self.registry.config.prefer_arrays && types.len() > 1 {
                        // Create array type
                        let element_type = types
                            .into_iter()
                            .reduce(|acc, t| self.registry.merge_types(acc, t))
                            .unwrap();
                        InferredType::Array(ArrayType {
                            element_type: Box::new(element_type),
                            cardinality: Cardinality::optional_repeating(),
                        })
                    } else {
                        // Merge all types
                        types
                            .into_iter()
                            .reduce(|acc, t| self.registry.merge_types(acc, t))
                            .unwrap()
                    };

                    properties.insert(key.clone(), PropertyDefinition::simple(merged_type));
                }

                // Process entity items (array-like values)
                if !entity.items.is_empty() {
                    let mut item_types = Vec::new();
                    for item in &entity.items {
                        item_types.push(self.infer_from_value(item, depth + 1));
                    }

                    let merged_item_type = item_types
                        .into_iter()
                        .reduce(|acc, t| self.registry.merge_types(acc, t))
                        .unwrap();

                    // Add items as a special property
                    properties.insert(
                        "_items".to_string(),
                        PropertyDefinition::simple(InferredType::Array(ArrayType {
                            element_type: Box::new(merged_item_type),
                            cardinality: Cardinality::optional_repeating(),
                        })),
                    );
                }

                InferredType::Object(ObjectType {
                    properties,
                    subtypes: HashMap::new(),
                    extensible: true,
                    localisation: None,
                    modifiers: None,
                })
            }

            Value::Color(_) => InferredType::Primitive(PrimitiveType::Color),

            Value::Maths(_) => InferredType::Primitive(PrimitiveType::Maths),
        }
    }

    /// Get the type registry
    pub fn registry(&self) -> &TypeRegistry {
        &self.registry
    }

    /// Get a mutable reference to the type registry
    pub fn registry_mut(&mut self) -> &mut TypeRegistry {
        &mut self.registry
    }
}
