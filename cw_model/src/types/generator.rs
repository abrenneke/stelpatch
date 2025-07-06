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
            InferredType::Primitive(PrimitiveType::String) => "string".to_string(),
            InferredType::Primitive(PrimitiveType::Number) => "number".to_string(),
            InferredType::Primitive(PrimitiveType::Boolean) => "boolean".to_string(),
            InferredType::Primitive(PrimitiveType::Color) => "Color".to_string(),
            InferredType::Primitive(PrimitiveType::Maths) => "string".to_string(),
            InferredType::PrimitiveUnion(primitives) => {
                let mut sorted_primitives: Vec<_> = primitives.iter().collect();
                sorted_primitives.sort();
                sorted_primitives
                    .iter()
                    .map(|p| match p {
                        PrimitiveType::String => "string",
                        PrimitiveType::Number => "number",
                        PrimitiveType::Boolean => "boolean",
                        PrimitiveType::Color => "Color",
                        PrimitiveType::Maths => "string",
                    })
                    .collect::<Vec<_>>()
                    .join(" | ")
            }
            InferredType::Object(obj) => {
                let mut fields = Vec::new();
                for (key, value) in obj {
                    fields.push(format!(
                        "  {}: {}",
                        self.sanitize_name(key),
                        self.typescript_type_definition(value)
                    ));
                }
                format!("{{\n{}\n}}", fields.join(";\n"))
            }
            InferredType::Array(element_type) => {
                format!("Array<{}>", self.typescript_type_definition(element_type))
            }
            InferredType::Union(types) => types
                .iter()
                .map(|t| self.typescript_type_definition(t))
                .collect::<Vec<_>>()
                .join(" | "),
            InferredType::Unknown => "unknown".to_string(),
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
            InferredType::Primitive(PrimitiveType::String) => serde_json::json!({"type": "string"}),
            InferredType::Primitive(PrimitiveType::Number) => serde_json::json!({"type": "number"}),
            InferredType::Primitive(PrimitiveType::Boolean) => {
                serde_json::json!({"type": "boolean"})
            }
            InferredType::Primitive(PrimitiveType::Color) => serde_json::json!({"type": "object"}),
            InferredType::Primitive(PrimitiveType::Maths) => serde_json::json!({"type": "string"}),
            InferredType::PrimitiveUnion(primitives) => {
                let types: Vec<_> = primitives
                    .iter()
                    .map(|p| match p {
                        PrimitiveType::String => "string",
                        PrimitiveType::Number => "number",
                        PrimitiveType::Boolean => "boolean",
                        PrimitiveType::Color => "object",
                        PrimitiveType::Maths => "string",
                    })
                    .collect();
                serde_json::json!({
                    "anyOf": types.iter().map(|t| serde_json::json!({"type": t})).collect::<Vec<_>>()
                })
            }
            InferredType::Object(obj) => {
                let mut properties = serde_json::Map::new();
                for (key, value) in obj {
                    properties.insert(key.clone(), self.json_schema_type_definition(value));
                }
                serde_json::json!({
                    "type": "object",
                    "properties": properties
                })
            }
            InferredType::Array(element_type) => serde_json::json!({
                "type": "array",
                "items": self.json_schema_type_definition(element_type)
            }),
            InferredType::Union(types) => serde_json::json!({
                "anyOf": types.iter().map(|t| self.json_schema_type_definition(t)).collect::<Vec<_>>()
            }),
            InferredType::Unknown => serde_json::json!({}),
        }
    }

    /// Sanitize names for use in type definitions
    fn sanitize_name(&self, name: &str) -> String {
        name.replace("/", "_").replace("-", "_").replace(".", "_")
    }
}
