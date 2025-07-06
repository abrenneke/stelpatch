use cw_games::stellaris::BaseGame;
use cw_model::{InferredType, LoadMode, TypeInferenceConfig, TypeInferenceEngine, TypeRegistry};
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
    match inferred_type {
        InferredType::Literal(lit) => format!("Literal: `{}`", lit),
        InferredType::LiteralUnion(literals) => {
            let mut sorted: Vec<_> = literals.iter().collect();
            sorted.sort();
            if sorted.len() <= 5 {
                format!(
                    "Union: {}",
                    sorted
                        .iter()
                        .map(|s| format!("`{}`", s))
                        .collect::<Vec<_>>()
                        .join(" | ")
                )
            } else {
                format!(
                    "Union: {} options including {}",
                    literals.len(),
                    sorted
                        .iter()
                        .take(3)
                        .map(|s| format!("`{}`", s))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
        InferredType::Primitive(prim) => format!("Type: {:?}", prim),
        InferredType::PrimitiveUnion(prims) => {
            format!(
                "Union: {}",
                prims
                    .iter()
                    .map(|p| format!("{:?}", p))
                    .collect::<Vec<_>>()
                    .join(" | ")
            )
        }
        InferredType::Object(obj) => {
            if obj.is_empty() {
                "Object: {}".to_string()
            } else {
                let property_count = obj.len();
                let sample_props: Vec<_> = obj.keys().take(3).cloned().collect();
                if property_count <= 3 {
                    format!("Object: {{ {} }}", sample_props.join(", "))
                } else {
                    format!(
                        "Object: {} properties including {{ {} }}",
                        property_count,
                        sample_props.join(", ")
                    )
                }
            }
        }
        InferredType::Array(element_type) => {
            format!("Array<{}>", format_type_description(element_type))
        }
        InferredType::Union(types) => {
            if types.len() <= 3 {
                format!(
                    "Union: {}",
                    types
                        .iter()
                        .map(format_type_description)
                        .collect::<Vec<_>>()
                        .join(" | ")
                )
            } else {
                format!("Union: {} types", types.len())
            }
        }
        InferredType::Unknown => "Unknown (too deeply nested)".to_string(),
    }
}

/// Get type information for a property path
/// This is the main public interface for the LSP hover functionality
pub async fn get_type_info(namespace: &str, property_path: &str) -> Option<TypeInfo> {
    if !TypeCache::is_initialized() {
        // If cache isn't ready, return basic info
        return Some(TypeInfo {
            property_path: property_path.to_string(),
            type_description: "Loading type information...".to_string(),
            inferred_type: None,
        });
    }

    let cache = TypeCache::get().unwrap();
    cache.get_property_type(namespace, property_path)
}
