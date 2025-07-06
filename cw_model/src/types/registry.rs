use std::collections::{HashMap, HashSet};

use crate::types::{InferredType, PrimitiveType, TypeInferenceConfig};

/// Registry that stores and manages all inferred types
#[derive(Debug, Clone)]
pub struct TypeRegistry {
    /// Types for each namespace - all modules in a namespace share the same type space
    pub namespace_types: HashMap<String, HashMap<String, InferredType>>,

    /// Configuration for type inference
    pub config: TypeInferenceConfig,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            namespace_types: HashMap::new(),
            config: TypeInferenceConfig::default(),
        }
    }

    pub fn with_config(config: TypeInferenceConfig) -> Self {
        Self {
            namespace_types: HashMap::new(),
            config,
        }
    }

    /// Add a type observation for a specific namespace and property.
    /// All modules within the same namespace share the same type space, so if multiple
    /// modules define the same property, their types will be merged.
    pub fn observe_type(&mut self, namespace: &str, property: &str, inferred_type: InferredType) {
        // Get the existing type, if any
        let existing_type = self
            .namespace_types
            .get(namespace)
            .and_then(|ns_map| ns_map.get(property))
            .cloned()
            .unwrap_or(InferredType::Unknown);

        // Merge the types
        let merged_type = self.merge_types(existing_type, inferred_type);

        // Store the merged type
        self.namespace_types
            .entry(namespace.to_string())
            .or_insert_with(HashMap::new)
            .insert(property.to_string(), merged_type);
    }

    /// Get the inferred type for a specific namespace and property
    pub fn get_type(&self, namespace: &str, property: &str) -> Option<&InferredType> {
        self.namespace_types.get(namespace)?.get(property)
    }

    /// Get all types for a specific namespace
    pub fn get_namespace_types(&self, namespace: &str) -> Option<&HashMap<String, InferredType>> {
        self.namespace_types.get(namespace)
    }

    /// Get all namespaces that have types
    pub fn get_namespaces(&self) -> Vec<&String> {
        self.namespace_types.keys().collect()
    }

    /// Merge two types into a single type
    pub fn merge_types(&self, existing: InferredType, new: InferredType) -> InferredType {
        match (existing, new) {
            (InferredType::Unknown, t) | (t, InferredType::Unknown) => t,

            (InferredType::Literal(a), InferredType::Literal(b)) if a == b => {
                InferredType::Literal(a)
            }
            (InferredType::Literal(a), InferredType::Literal(b)) => {
                let mut set = HashSet::new();
                set.insert(a);
                set.insert(b);
                InferredType::LiteralUnion(set)
            }

            (InferredType::LiteralUnion(mut set), InferredType::Literal(lit)) => {
                set.insert(lit);
                if set.len() > self.config.max_literals {
                    InferredType::Primitive(PrimitiveType::String)
                } else {
                    InferredType::LiteralUnion(set)
                }
            }

            (InferredType::Literal(lit), InferredType::LiteralUnion(mut set)) => {
                set.insert(lit);
                if set.len() > self.config.max_literals {
                    InferredType::Primitive(PrimitiveType::String)
                } else {
                    InferredType::LiteralUnion(set)
                }
            }

            (InferredType::LiteralUnion(mut a), InferredType::LiteralUnion(b)) => {
                a.extend(b);
                if a.len() > self.config.max_literals {
                    InferredType::Primitive(PrimitiveType::String)
                } else {
                    InferredType::LiteralUnion(a)
                }
            }

            (InferredType::Primitive(a), InferredType::Primitive(b)) if a == b => {
                InferredType::Primitive(a)
            }
            (InferredType::Primitive(a), InferredType::Primitive(b)) => {
                let mut set = HashSet::new();
                set.insert(a);
                set.insert(b);
                InferredType::PrimitiveUnion(set)
            }

            (InferredType::Object(mut a), InferredType::Object(b)) if self.config.merge_objects => {
                for (key, value) in b {
                    let existing = a.entry(key).or_insert(Box::new(InferredType::Unknown));
                    let existing_clone = (*existing).clone();
                    let merged = self.merge_types(*existing_clone, *value);
                    *existing = Box::new(merged);
                }
                InferredType::Object(a)
            }

            (InferredType::Array(a), InferredType::Array(b)) => {
                let merged = self.merge_types(*a, *b);
                InferredType::Array(Box::new(merged))
            }

            // Convert to union if types are incompatible
            (a, b) if a != b => {
                let mut union = Vec::new();
                union.push(a);
                union.push(b);
                InferredType::Union(union)
            }

            (a, _) => a,
        }
    }
}
