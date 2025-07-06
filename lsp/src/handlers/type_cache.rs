use cw_games::stellaris::BaseGame;
use cw_model::{
    InferredType, LoadMode, PrimitiveType, TypeInferenceConfig, TypeInferenceEngine, TypeRegistry,
};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

/// Cache for Stellaris type information that's loaded once and shared across requests
pub struct TypeCache {
    registry: Arc<TypeRegistry>,
    namespace_types: HashMap<String, InferredType>,
}

static TYPE_CACHE: OnceLock<TypeCache> = OnceLock::new();

impl TypeCache {
    /// Initialize the type cache by loading Stellaris data
    pub fn initialize_in_background() {
        // This runs in a background task since it can take time
        std::thread::spawn(|| {
            let _ = Self::get_or_init_blocking();
        });
    }

    fn get() -> Option<&'static TypeCache> {
        TYPE_CACHE.get()
    }

    /// Get or initialize the global type cache (blocking version)
    fn get_or_init_blocking() -> &'static TypeCache {
        TYPE_CACHE.get_or_init(|| {
            eprintln!("Initializing type cache");

            // Load Stellaris base game data
            let game_mod = BaseGame::load_global_as_mod_definition(LoadMode::Parallel);

            eprintln!("Loaded Stellaris base game data");

            // Create a single type inference engine with configuration
            let config = TypeInferenceConfig {
                max_literals: 20,
                infer_booleans: true,
                merge_objects: true,
                prefer_arrays: false,
                max_depth: 100, // Reasonable depth limit to prevent stack overflow
            };
            let mut global_engine = TypeInferenceEngine::with_config(config);

            eprintln!("Processing namespaces");

            // Process all namespaces to build complete type information
            for (_namespace_name, namespace) in &game_mod.namespaces {
                let modules: Vec<_> = namespace.modules.values().collect();
                if !modules.is_empty() {
                    global_engine.process_modules(&modules);
                }
            }

            eprintln!("Building registry");

            let registry = global_engine.registry().clone();

            // Pre-compute entity types for quick lookups
            let mut namespace_types = HashMap::new();
            for namespace_name in game_mod.namespaces.keys() {
                if let Some(entity_type) = registry.get_type(namespace_name, "entity") {
                    namespace_types.insert(namespace_name.clone(), entity_type.clone());
                }
            }

            eprintln!("Built type cache");

            TypeCache {
                registry: Arc::new(registry),
                namespace_types,
            }
        })
    }

    /// Get type information for a specific namespace
    pub fn get_namespace_type(&self, namespace: &str) -> Option<&InferredType> {
        self.namespace_types.get(namespace)
    }

    /// Get the type registry for advanced queries
    #[allow(dead_code)]
    pub fn get_registry(&self) -> &TypeRegistry {
        &self.registry
    }

    /// Get type information for a specific property path in a namespace
    /// Path format: "property" or "property.nested.field"
    pub fn get_property_type(&self, namespace: &str, property_path: &str) -> Option<TypeInfo> {
        let namespace_type = self.get_namespace_type(namespace)?;

        let path_parts: Vec<&str> = property_path.split('.').collect();
        let mut current_type = namespace_type;
        let mut current_path = String::new();

        for (i, part) in path_parts.iter().enumerate() {
            if i > 0 {
                current_path.push('.');
            }
            current_path.push_str(part);

            match current_type {
                InferredType::Object(obj) => {
                    if let Some(boxed_type) = obj.get(*part) {
                        current_type = boxed_type.as_ref();
                    } else {
                        return Some(TypeInfo {
                            property_path: current_path,
                            type_description: format!("Unknown property '{}'", part),
                            inferred_type: None,
                        });
                    }
                }
                _ => {
                    return Some(TypeInfo {
                        property_path: current_path,
                        type_description: "Cannot access property on non-object type".to_string(),
                        inferred_type: None,
                    });
                }
            }
        }

        Some(TypeInfo {
            property_path: property_path.to_string(),
            type_description: format_type_description(current_type),
            inferred_type: Some(current_type.clone()),
        })
    }

    /// Check if the cache is ready
    pub fn is_initialized() -> bool {
        TYPE_CACHE.get().is_some()
    }
}

/// Information about a property's type
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub property_path: String,
    pub type_description: String,
    pub inferred_type: Option<InferredType>,
}

/// Format a type description for display in hover tooltips
fn format_type_description(inferred_type: &InferredType) -> String {
    format_type_description_with_depth(inferred_type, 0, 12)
}

/// Format a type description with depth control and max lines
fn format_type_description_with_depth(
    inferred_type: &InferredType,
    depth: usize,
    max_lines: usize,
) -> String {
    if depth > 4 {
        return "...".to_string();
    }

    match inferred_type {
        InferredType::Literal(lit) => format!("\"{}\"", lit),
        InferredType::LiteralUnion(literals) => {
            let mut sorted: Vec<_> = literals.iter().collect();
            sorted.sort();
            if sorted.len() <= 8 {
                sorted
                    .iter()
                    .map(|s| format!("\"{}\"", s))
                    .collect::<Vec<_>>()
                    .join(" | ")
            } else {
                format!(
                    "{} | /* ... ({} more) */",
                    sorted
                        .iter()
                        .take(5)
                        .map(|s| format!("\"{}\"", s))
                        .collect::<Vec<_>>()
                        .join(" | "),
                    literals.len() - 5
                )
            }
        }
        InferredType::Primitive(prim) => match prim {
            PrimitiveType::String => "string".to_string(),
            PrimitiveType::Number => "number".to_string(),
            PrimitiveType::Boolean => "boolean".to_string(),
            PrimitiveType::Color => "color".to_string(),
            PrimitiveType::Maths => "maths".to_string(),
        },
        InferredType::PrimitiveUnion(prims) => prims
            .iter()
            .map(|p| match p {
                PrimitiveType::String => "string".to_string(),
                PrimitiveType::Number => "number".to_string(),
                PrimitiveType::Boolean => "boolean".to_string(),
                PrimitiveType::Color => "color".to_string(),
                PrimitiveType::Maths => "maths".to_string(),
            })
            .collect::<Vec<_>>()
            .join(" | "),
        InferredType::Object(obj) => {
            if obj.is_empty() {
                return "Entity: {}".to_string();
            }

            let mut properties: Vec<_> = obj.iter().collect();
            properties.sort_by_key(|(k, _)| k.as_str());

            let mut lines = vec!["Entity:".to_string()];
            let mut line_count = 0;
            let mut properties_processed = 0;

            for (key, value_type) in properties {
                if line_count >= max_lines {
                    lines.push(format!(
                        "  # ... ({} more properties)",
                        obj.len() - properties_processed
                    ));
                    break;
                }

                let formatted_value = format_type_description_with_depth(
                    value_type,
                    depth + 1,
                    max_lines - line_count,
                );

                // Handle multi-line types (nested objects)
                if formatted_value.contains('\n') {
                    // For nested objects, show them with proper indentation
                    lines.push(format!("  {}:", key));
                    let nested_lines: Vec<&str> = formatted_value.lines().collect();
                    let mut lines_added = 1; // Count the "key:" line

                    for line in nested_lines {
                        if line.starts_with("Entity:") {
                            // Skip the "Entity:" line for nested objects
                            continue;
                        }
                        if line_count + lines_added >= max_lines {
                            lines.push("    # ... (truncated)".to_string());
                            break;
                        }
                        lines.push(format!("    {}", line));
                        lines_added += 1;
                    }
                    line_count += lines_added;
                } else {
                    lines.push(format!("  {}: {}", key, formatted_value));
                    line_count += 1;
                }
                properties_processed += 1;
            }

            lines.join("\n")
        }
        InferredType::Array(element_type) => {
            let element_desc =
                format_type_description_with_depth(element_type, depth + 1, max_lines);
            if element_desc.contains('\n') {
                format!("array[object]")
            } else {
                format!("array[{}]", element_desc)
            }
        }
        InferredType::Union(types) => {
            if types.len() <= 3 {
                types
                    .iter()
                    .map(|t| format_type_description_with_depth(t, depth + 1, max_lines))
                    .collect::<Vec<_>>()
                    .join(" | ")
            } else {
                format!(
                    "{} | /* ... ({} more types) */",
                    types
                        .iter()
                        .take(2)
                        .map(|t| format_type_description_with_depth(t, depth + 1, max_lines))
                        .collect::<Vec<_>>()
                        .join(" | "),
                    types.len() - 2
                )
            }
        }
        InferredType::Unknown => "unknown".to_string(),
    }
}

/// Get type information for a namespace entity (top-level entity structure)
pub async fn get_namespace_entity_type(namespace: &str) -> Option<TypeInfo> {
    if !TypeCache::is_initialized() {
        return Some(TypeInfo {
            property_path: "entity".to_string(),
            type_description: "Loading type information...".to_string(),
            inferred_type: None,
        });
    }

    let cache = TypeCache::get().unwrap();
    if let Some(namespace_type) = cache.get_namespace_type(namespace) {
        Some(TypeInfo {
            property_path: "entity".to_string(),
            type_description: format_type_description(namespace_type),
            inferred_type: Some(namespace_type.clone()),
        })
    } else {
        Some(TypeInfo {
            property_path: "entity".to_string(),
            type_description: "No type information available for this namespace".to_string(),
            inferred_type: None,
        })
    }
}

/// Get type information for a property within a namespace entity
/// The property_path should be just the property path without the entity name
pub async fn get_entity_property_type(namespace: &str, property_path: &str) -> Option<TypeInfo> {
    if !TypeCache::is_initialized() {
        return Some(TypeInfo {
            property_path: property_path.to_string(),
            type_description: "Loading type information...".to_string(),
            inferred_type: None,
        });
    }

    let cache = TypeCache::get().unwrap();
    cache.get_property_type(namespace, property_path)
}
