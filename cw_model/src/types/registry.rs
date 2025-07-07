use std::collections::{HashMap, HashSet};

use crate::types::{
    ArrayType, Cardinality, InferredType, ObjectType, PrimitiveType, PropertyDefinition,
    TypeInferenceConfig,
};

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
        self.merge_types_with_depth(existing, new, 0)
    }

    /// Merge two types into a single type with depth tracking to prevent stack overflow
    fn merge_types_with_depth(
        &self,
        existing: InferredType,
        new: InferredType,
        depth: usize,
    ) -> InferredType {
        // Check depth limit to prevent stack overflow
        if depth > self.config.max_depth {
            // Return Unknown when depth limit is reached to avoid stack overflow
            return InferredType::Unknown;
        }

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
                // Merge properties
                for (key, prop_def) in b.properties {
                    let existing_prop = a
                        .properties
                        .entry(key)
                        .or_insert(PropertyDefinition::simple(InferredType::Unknown));
                    let existing_clone = (*existing_prop.property_type).clone();
                    let merged = self.merge_types_with_depth(
                        existing_clone,
                        *prop_def.property_type,
                        depth + 1,
                    );
                    existing_prop.property_type = Box::new(merged);
                }

                // Merge subtypes
                for (key, subtype) in b.subtypes {
                    a.subtypes.insert(key, subtype);
                }

                // Merge other properties
                if a.localisation.is_none() {
                    a.localisation = b.localisation;
                }
                if a.modifiers.is_none() {
                    a.modifiers = b.modifiers;
                }
                a.extensible = a.extensible || b.extensible;

                InferredType::Object(a)
            }

            (InferredType::Array(a), InferredType::Array(b)) => {
                let merged =
                    self.merge_types_with_depth(*a.element_type, *b.element_type, depth + 1);
                InferredType::Array(ArrayType {
                    element_type: Box::new(merged),
                    cardinality: Cardinality::optional_repeating(), // Use a reasonable default
                })
            }

            // Handle same types
            (a, b) if std::mem::discriminant(&a) == std::mem::discriminant(&b) => {
                // For types that are the same variant but potentially different content
                match (a, b) {
                    (InferredType::Reference(ref_a), InferredType::Reference(ref_b)) => {
                        // If references are the same, keep one; otherwise create union
                        if ref_a == ref_b {
                            InferredType::Reference(ref_a)
                        } else {
                            InferredType::Union(vec![
                                InferredType::Reference(ref_a),
                                InferredType::Reference(ref_b),
                            ])
                        }
                    }
                    (InferredType::Constrained(c_a), InferredType::Constrained(c_b)) => {
                        // Merge the base types
                        let merged_base =
                            self.merge_types_with_depth(*c_a.base_type, *c_b.base_type, depth + 1);
                        InferredType::Constrained(crate::types::ConstrainedType {
                            base_type: Box::new(merged_base),
                            cardinality: c_a.cardinality.or(c_b.cardinality),
                            range: c_a.range.or(c_b.range),
                            options: [c_a.options, c_b.options].concat(),
                        })
                    }
                    (InferredType::Comparable(c_a), InferredType::Comparable(c_b)) => {
                        let merged_base = self.merge_types_with_depth(*c_a, *c_b, depth + 1);
                        InferredType::Comparable(Box::new(merged_base))
                    }
                    // For other same-discriminant cases, just return the first one
                    (a, _) => a,
                }
            }

            // Convert to union if types are incompatible
            (a, b) => {
                // Flatten unions to avoid nested Union types
                let mut union_types = Vec::new();

                // Add types from first argument
                match a {
                    InferredType::Union(types) => union_types.extend(types),
                    other => union_types.push(other),
                }

                // Add types from second argument
                match b {
                    InferredType::Union(types) => union_types.extend(types),
                    other => union_types.push(other),
                }

                // Remove duplicates (simple approach - just dedup without sorting)
                union_types.dedup();

                if union_types.len() == 1 {
                    union_types.into_iter().next().unwrap()
                } else {
                    InferredType::Union(union_types)
                }
            }
        }
    }
}
