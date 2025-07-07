use crate::types::{InferredType, PrimitiveType, TypeRegistry};

/// Generator that outputs type definitions in various formats
pub struct TypeGenerator<'a> {
    registry: &'a TypeRegistry,
}

impl<'a> TypeGenerator<'a> {
    pub fn new(registry: &'a TypeRegistry) -> Self {
        Self { registry }
    }

    /// Generate TypeScript-like type definitions
    pub fn generate_typescript(&self) -> String {
        let mut output = String::new();

        for (namespace, types) in &self.registry.namespace_types {
            output.push_str(&format!("// Types for namespace: {}\n", namespace));
            output.push_str(&format!(
                "export namespace {} {{\n",
                self.sanitize_name(namespace)
            ));

            for (property, inferred_type) in types {
                let type_def = self.typescript_type_definition(inferred_type);
                output.push_str(&format!(
                    "  export type {} = {};\n",
                    self.sanitize_name(property),
                    type_def
                ));
            }

            output.push_str("}\n\n");
        }

        output
    }

    /// Generate JSON Schema
    pub fn generate_json_schema(&self) -> serde_json::Value {
        let mut schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {}
        });

        let properties = schema["properties"].as_object_mut().unwrap();

        for (namespace, types) in &self.registry.namespace_types {
            let mut namespace_schema = serde_json::json!({
                "type": "object",
                "properties": {}
            });

            let namespace_properties = namespace_schema["properties"].as_object_mut().unwrap();

            for (property, inferred_type) in types {
                namespace_properties.insert(
                    property.clone(),
                    self.json_schema_type_definition(inferred_type),
                );
            }

            properties.insert(namespace.clone(), namespace_schema);
        }

        schema
    }

    /// Convert InferredType to TypeScript type string
    fn typescript_type_definition(&self, inferred_type: &InferredType) -> String {
        match inferred_type {
            InferredType::Literal(lit) => format!("'{}'", lit),
            InferredType::LiteralUnion(literals) => {
                let mut sorted_literals: Vec<_> = literals.iter().collect();
                sorted_literals.sort();
                sorted_literals
                    .iter()
                    .map(|s| format!("'{}'", s))
                    .collect::<Vec<_>>()
                    .join(" | ")
            }
            InferredType::Primitive(ptype) => self.primitive_to_typescript(ptype),
            InferredType::PrimitiveUnion(primitives) => {
                let mut sorted_primitives: Vec<_> = primitives.iter().collect();
                sorted_primitives.sort();
                sorted_primitives
                    .iter()
                    .map(|p| self.primitive_to_typescript(p))
                    .collect::<Vec<_>>()
                    .join(" | ")
            }
            InferredType::Reference(ref_type) => {
                format!("/* {} */", format!("{:?}", ref_type))
            }
            InferredType::Object(obj) => {
                let mut fields = Vec::new();
                for (key, prop_def) in &obj.properties {
                    fields.push(format!(
                        "  {}: {}",
                        self.sanitize_name(key),
                        self.typescript_type_definition(&prop_def.property_type)
                    ));
                }
                format!("{{\n{}\n}}", fields.join(";\n"))
            }
            InferredType::Array(array_type) => {
                format!(
                    "Array<{}>",
                    self.typescript_type_definition(&array_type.element_type)
                )
            }
            InferredType::Union(types) => types
                .iter()
                .map(|t| self.typescript_type_definition(t))
                .collect::<Vec<_>>()
                .join(" | "),
            InferredType::Constrained(constrained) => {
                self.typescript_type_definition(&constrained.base_type)
            }
            InferredType::Comparable(base_type) => self.typescript_type_definition(base_type),
            InferredType::Unknown => "unknown".to_string(),
        }
    }

    fn primitive_to_typescript(&self, ptype: &PrimitiveType) -> String {
        match ptype {
            PrimitiveType::String => "string".to_string(),
            PrimitiveType::Boolean => "boolean".to_string(),
            PrimitiveType::Integer => "number".to_string(),
            PrimitiveType::Float => "number".to_string(),
            PrimitiveType::Scalar => "number".to_string(),
            PrimitiveType::PercentageField => "number".to_string(),
            PrimitiveType::Localisation => "string".to_string(),
            PrimitiveType::LocalisationSynced => "string".to_string(),
            PrimitiveType::LocalisationInline => "string".to_string(),
            PrimitiveType::DateField => "string".to_string(),
            PrimitiveType::VariableField => "string".to_string(),
            PrimitiveType::IntVariableField => "number".to_string(),
            PrimitiveType::ValueField => "string".to_string(),
            PrimitiveType::IntValueField => "number".to_string(),
            PrimitiveType::ScopeField => "string".to_string(),
            PrimitiveType::Filepath => "string".to_string(),
            PrimitiveType::Icon => "string".to_string(),
            PrimitiveType::Color => "Color".to_string(),
            PrimitiveType::Maths => "string".to_string(),
        }
    }

    /// Convert InferredType to JSON Schema
    pub fn json_schema_type_definition(&self, inferred_type: &InferredType) -> serde_json::Value {
        match inferred_type {
            InferredType::Literal(lit) => serde_json::json!({
                "type": "string",
                "enum": [lit]
            }),
            InferredType::LiteralUnion(literals) => {
                let mut sorted_literals: Vec<_> = literals.iter().collect();
                sorted_literals.sort();
                serde_json::json!({
                    "type": "string",
                    "enum": sorted_literals
                })
            }
            InferredType::Primitive(ptype) => self.primitive_to_json_schema(ptype),
            InferredType::PrimitiveUnion(primitives) => {
                let types: Vec<_> = primitives
                    .iter()
                    .map(|p| self.primitive_to_json_schema(p))
                    .collect();
                serde_json::json!({
                    "anyOf": types
                })
            }
            InferredType::Reference(ref_type) => {
                serde_json::json!({
                    "type": "string",
                    "description": format!("Reference: {:?}", ref_type)
                })
            }
            InferredType::Object(obj) => {
                let mut properties = serde_json::Map::new();
                for (key, prop_def) in &obj.properties {
                    properties.insert(
                        key.clone(),
                        self.json_schema_type_definition(&prop_def.property_type),
                    );
                }
                serde_json::json!({
                    "type": "object",
                    "properties": properties
                })
            }
            InferredType::Array(array_type) => serde_json::json!({
                "type": "array",
                "items": self.json_schema_type_definition(&array_type.element_type)
            }),
            InferredType::Union(types) => serde_json::json!({
                "anyOf": types.iter().map(|t| self.json_schema_type_definition(t)).collect::<Vec<_>>()
            }),
            InferredType::Constrained(constrained) => {
                let mut schema = self.json_schema_type_definition(&constrained.base_type);

                // Add constraints if they exist
                if let Some(range) = &constrained.range {
                    if let Some(obj) = schema.as_object_mut() {
                        // Add range constraints for numeric types
                        match &range.min {
                            crate::types::RangeBound::Integer(min) => {
                                obj.insert("minimum".to_string(), serde_json::json!(min));
                            }
                            crate::types::RangeBound::Float(min) => {
                                obj.insert("minimum".to_string(), serde_json::json!(min));
                            }
                            _ => {}
                        }

                        match &range.max {
                            crate::types::RangeBound::Integer(max) => {
                                obj.insert("maximum".to_string(), serde_json::json!(max));
                            }
                            crate::types::RangeBound::Float(max) => {
                                obj.insert("maximum".to_string(), serde_json::json!(max));
                            }
                            _ => {}
                        }
                    }
                }

                schema
            }
            InferredType::Comparable(base_type) => self.json_schema_type_definition(base_type),
            InferredType::Unknown => serde_json::json!({}),
        }
    }

    fn primitive_to_json_schema(&self, ptype: &PrimitiveType) -> serde_json::Value {
        match ptype {
            PrimitiveType::String => serde_json::json!({"type": "string"}),
            PrimitiveType::Boolean => serde_json::json!({"type": "boolean"}),
            PrimitiveType::Integer => serde_json::json!({"type": "integer"}),
            PrimitiveType::Float => serde_json::json!({"type": "number"}),
            PrimitiveType::Scalar => serde_json::json!({"type": "number"}),
            PrimitiveType::PercentageField => serde_json::json!({"type": "number"}),
            PrimitiveType::Localisation => serde_json::json!({"type": "string"}),
            PrimitiveType::LocalisationSynced => serde_json::json!({"type": "string"}),
            PrimitiveType::LocalisationInline => serde_json::json!({"type": "string"}),
            PrimitiveType::DateField => serde_json::json!({"type": "string"}),
            PrimitiveType::VariableField => serde_json::json!({"type": "string"}),
            PrimitiveType::IntVariableField => serde_json::json!({"type": "integer"}),
            PrimitiveType::ValueField => serde_json::json!({"type": "string"}),
            PrimitiveType::IntValueField => serde_json::json!({"type": "integer"}),
            PrimitiveType::ScopeField => serde_json::json!({"type": "string"}),
            PrimitiveType::Filepath => serde_json::json!({"type": "string"}),
            PrimitiveType::Icon => serde_json::json!({"type": "string"}),
            PrimitiveType::Color => serde_json::json!({"type": "object"}),
            PrimitiveType::Maths => serde_json::json!({"type": "string"}),
        }
    }

    /// Sanitize names for use in type definitions
    fn sanitize_name(&self, name: &str) -> String {
        name.replace("/", "_").replace("-", "_").replace(".", "_")
    }
}
