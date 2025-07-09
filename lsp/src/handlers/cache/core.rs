use cw_games::stellaris::BaseGame;
use cw_model::CwtType;
use cw_model::types::CwtAnalyzer;
use cw_model::{GameMod, LoadMode};
use cw_parser::CwtModuleCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use crate::handlers::cache::resolver::TypeResolver;

use super::formatter::format_type_description_with_property_context;
use super::types::TypeInfo;

/// Cache for Stellaris type information that's loaded once and shared across requests
pub struct TypeCache {
    namespace_types: HashMap<String, CwtType>,
    cwt_analyzer: Arc<CwtAnalyzer>,
    resolver: TypeResolver,
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

    pub fn get() -> Option<&'static TypeCache> {
        TYPE_CACHE.get()
    }

    /// Get or initialize the global type cache (blocking version)
    fn get_or_init_blocking() -> &'static TypeCache {
        TYPE_CACHE.get_or_init(|| {
            eprintln!("Initializing type cache");

            // Load CWT files - these contain all the type definitions we need
            let cwt_analyzer = Self::load_cwt_files();

            eprintln!("Building cache from CWT types");

            // Pre-compute entity types for quick lookups
            let mut namespace_types = HashMap::new();
            for (type_name, type_def) in cwt_analyzer.get_types() {
                // Extract namespace from the path
                let namespace = if let Some(path) = &type_def.path {
                    // Remove the "game/common" prefix to get the namespace
                    // e.g., "game/common/ambient_objects" -> "ambient_objects"
                    // e.g., "game/common/buildings/districts" -> "buildings/districts"
                    if path.starts_with("game/") {
                        path.strip_prefix("game/").unwrap_or(type_name).to_string()
                    } else {
                        path.clone()
                    }
                } else {
                    // Fallback to type name if no path
                    type_name.clone()
                };

                // Store the type rules for this namespace
                namespace_types.insert(namespace, type_def.rules.clone());
            }

            eprintln!(
                "Built type cache with {} CWT types",
                cwt_analyzer.get_types().len()
            );

            let cwt_analyzer = Arc::new(cwt_analyzer);

            TypeCache {
                namespace_types,
                cwt_analyzer: cwt_analyzer.clone(),
                resolver: TypeResolver::new(cwt_analyzer.clone()),
            }
        })
    }

    /// Load CWT files from the hardcoded path
    fn load_cwt_files() -> CwtAnalyzer {
        eprintln!("Loading CWT files from hardcoded path");

        let cwt_path = r"D:\dev\github\cwtools-stellaris-config\config";
        let dir_path = Path::new(cwt_path);

        let mut cwt_analyzer = CwtAnalyzer::new();

        if !dir_path.exists() {
            eprintln!("Warning: CWT directory '{}' does not exist", cwt_path);
            return cwt_analyzer;
        }

        // Find all .cwt files in the directory recursively
        let mut cwt_files = Vec::new();
        fn visit_dir(dir: &Path, cwt_files: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
            if dir.is_dir() {
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        visit_dir(&path, cwt_files)?;
                    } else if path.is_file() && path.extension().map_or(false, |ext| ext == "cwt") {
                        cwt_files.push(path);
                    }
                }
            }
            Ok(())
        }

        if let Err(e) = visit_dir(dir_path, &mut cwt_files) {
            eprintln!("Error reading directory {}: {}", dir_path.display(), e);
        }

        eprintln!("Found {} CWT files", cwt_files.len());

        // Parse and convert each CWT file
        for cwt_file in &cwt_files {
            if let Ok(content) = fs::read_to_string(cwt_file) {
                let module = CwtModuleCell::from_input(content);

                if let Ok(module_ref) = module.borrow_dependent().as_ref() {
                    if let Err(errors) = cwt_analyzer.convert_module(module_ref) {
                        eprintln!(
                            "Errors converting {}: {} errors",
                            cwt_file.display(),
                            errors.len()
                        );
                    }
                } else {
                    eprintln!("Failed to parse CWT file: {}", cwt_file.display());
                }
            }
        }

        let stats = cwt_analyzer.get_stats();
        eprintln!(
            "CWT Analysis complete: {} types, {} enums, {} aliases",
            stats.types_count, stats.enums_count, stats.aliases_count
        );

        cwt_analyzer
    }

    /// Get type information for a specific namespace
    pub fn get_namespace_type(&self, namespace: &str) -> Option<&CwtType> {
        self.namespace_types.get(namespace)
    }

    /// Get type information for a specific property path in a namespace
    /// Path format: "property" or "property.nested.field"
    pub fn get_property_type(&self, namespace: &str, property_path: &str) -> Option<TypeInfo> {
        // First try to get from namespace types (game data)
        if let Some(namespace_type) = self.get_namespace_type(namespace) {
            let path_parts: Vec<&str> = property_path.split('.').collect();
            let mut current_type = namespace_type.clone();
            let mut current_path = String::new();

            for (i, part) in path_parts.iter().enumerate() {
                if i > 0 {
                    current_path.push('.');
                }
                current_path.push_str(part);

                // Resolve the current type to its actual type
                current_type = self.resolver.resolve_type(&current_type);

                match &current_type {
                    CwtType::Block(block) => {
                        if let Some(property_def) = block.properties.get(*part) {
                            current_type = property_def.property_type.clone();
                        } else {
                            return Some(TypeInfo {
                                property_path: current_path,
                                type_description: format!("Unknown property '{}'", part),
                                cwt_type: None,
                                documentation: None,
                                source_info: Some(format!(
                                    "Property not found in {} entity",
                                    namespace
                                )),
                            });
                        }
                    }
                    CwtType::Reference(_reference) => {
                        // For references, resolve to the actual type and continue traversal
                        let resolved_type = self.resolver.resolve_type(&current_type);

                        // If we couldn't resolve the reference, return info about the reference itself
                        if matches!(resolved_type, CwtType::Reference(_)) {
                            return Some(TypeInfo {
                                property_path: current_path.clone(),
                                type_description: format_type_description_with_property_context(
                                    &resolved_type,
                                    0,
                                    30,
                                    &self.cwt_analyzer,
                                    &self.resolver,
                                    Some(part), // Pass the current property name
                                ),
                                cwt_type: Some(resolved_type),
                                documentation: None,
                                source_info: Some(format!(
                                    "Reference in {} entity at path '{}'",
                                    namespace, current_path
                                )),
                            });
                        }

                        // Continue with the resolved type
                        current_type = resolved_type;

                        // If this is the last part of the path, we're done
                        if i == path_parts.len() - 1 {
                            return Some(TypeInfo {
                                property_path: property_path.to_string(),
                                type_description: format_type_description_with_property_context(
                                    &current_type,
                                    0,
                                    30,
                                    &self.cwt_analyzer,
                                    &self.resolver,
                                    path_parts.last().map(|s| *s), // Pass the last part as property name
                                ),
                                cwt_type: Some(current_type.clone()),
                                documentation: None,
                                source_info: Some(format!(
                                    "Resolved reference in {} entity",
                                    namespace
                                )),
                            });
                        }

                        // Continue to next iteration to handle the resolved type
                        continue;
                    }
                    _ => {
                        return Some(TypeInfo {
                            property_path: current_path,
                            type_description: format!(
                                "Cannot access property '{}' on non-block type {:?}",
                                part, current_type
                            ),
                            cwt_type: None,
                            documentation: None,
                            source_info: Some("Property access on non-block type".to_string()),
                        });
                    }
                }
            }

            // IMPORTANT: Resolve the final property type to expand any patterns it may have
            let resolved_final_type = self.resolver.resolve_type(&current_type);

            return Some(TypeInfo {
                property_path: property_path.to_string(),
                type_description: format_type_description_with_property_context(
                    &resolved_final_type,
                    0,
                    30,
                    &self.cwt_analyzer,
                    &self.resolver,
                    path_parts.last().map(|s| *s), // Pass the last part as property name
                ),
                cwt_type: Some(resolved_final_type),
                documentation: None,
                source_info: Some(format!("From {} entity definition", namespace)),
            });
        }

        // If not found in namespace types, try CWT type definitions
        if let Some(type_def) = self.cwt_analyzer.get_type(namespace) {
            let path_parts: Vec<&str> = property_path.split('.').collect();
            return Some(TypeInfo {
                property_path: property_path.to_string(),
                type_description: format_type_description_with_property_context(
                    &type_def.rules,
                    0,
                    30,
                    &self.cwt_analyzer,
                    &self.resolver,
                    path_parts.last().map(|s| *s), // Pass the last part as property name
                ),
                cwt_type: Some(type_def.rules.clone()),
                documentation: None,
                source_info: Some(format!("CWT type definition: {}", namespace)),
            });
        }

        None
    }

    /// Check if the cache is ready
    pub fn is_initialized() -> bool {
        TYPE_CACHE.get().is_some()
    }

    /// Get the CWT analyzer
    pub fn get_cwt_analyzer(&self) -> &Arc<CwtAnalyzer> {
        &self.cwt_analyzer
    }

    /// Resolve a type to its actual concrete type
    pub fn resolve_type(&self, cwt_type: &CwtType) -> CwtType {
        self.resolver.resolve_type(cwt_type)
    }
}

/// Cache for actual game data keys from namespaces (e.g., "energy", "minerals" from resources namespace)
pub struct GameDataCache {
    /// Maps namespace -> set of keys defined in that namespace
    namespace_keys: HashMap<String, Vec<String>>,
    base_game: &'static GameMod,
}

static GAME_DATA_CACHE: OnceLock<GameDataCache> = OnceLock::new();

impl GameDataCache {
    /// Initialize the game data cache by loading Stellaris base game data
    pub fn initialize_in_background() {
        // This runs in a background task since it can take time
        std::thread::spawn(|| {
            let _ = Self::get_or_init_blocking();
        });
    }

    pub fn get() -> Option<&'static GameDataCache> {
        GAME_DATA_CACHE.get()
    }

    /// Get or initialize the global game data cache (blocking version)
    fn get_or_init_blocking() -> &'static GameDataCache {
        GAME_DATA_CACHE.get_or_init(|| {
            eprintln!("Initializing game data cache");

            // Load base game data
            let base_game = BaseGame::load_global_as_mod_definition(LoadMode::Parallel);

            eprintln!(
                "Building namespace keys cache from {} namespaces",
                base_game.namespaces.len()
            );

            // Extract keys from each namespace
            let mut namespace_keys = HashMap::new();
            for (namespace_name, namespace) in &base_game.namespaces {
                let mut keys: Vec<String> = namespace.properties.kv.keys().cloned().collect();
                keys.sort(); // Sort for consistent ordering
                namespace_keys.insert(namespace_name.clone(), keys);
            }

            eprintln!(
                "Built game data cache with {} namespaces",
                namespace_keys.len()
            );

            GameDataCache {
                namespace_keys,
                base_game,
            }
        })
    }

    /// Get all keys defined in a namespace
    pub fn get_namespace_keys(&self, namespace: &str) -> Option<&Vec<String>> {
        self.namespace_keys.get(namespace)
    }

    /// Check if the game data cache is initialized
    pub fn is_initialized() -> bool {
        GAME_DATA_CACHE.get().is_some()
    }
}
