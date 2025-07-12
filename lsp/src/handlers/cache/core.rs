use cw_model::CwtType;
use cw_model::types::CwtAnalyzer;
use cw_parser::CwtModuleCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use crate::handlers::cache::resolver::TypeResolver;
use crate::handlers::scoped_type::{CwtTypeOrSpecial, PropertyNavigationResult, ScopedType};

use super::formatter::TypeFormatter;
use super::types::TypeInfo;

/// Cache for Stellaris type information that's loaded once and shared across requests
pub struct TypeCache {
    namespace_types: HashMap<String, ScopedType>,
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

                let mut scoped_type =
                    ScopedType::new_cwt(type_def.rules.clone(), Default::default());

                if let Some(push_scope) = type_def.rule_options.push_scope.as_ref() {
                    if let Some(scope_name) = cwt_analyzer.resolve_scope_name(push_scope) {
                        scoped_type
                            .scope_stack_mut()
                            .push_scope_type(scope_name.to_string())
                            .unwrap();
                    }
                }

                if let Some(replace_scope) = type_def.rule_options.replace_scope.as_ref() {
                    let mut new_scopes = HashMap::new();
                    for (key, value) in replace_scope {
                        if let Some(scope_name) = cwt_analyzer.resolve_scope_name(value) {
                            new_scopes.insert(key.clone(), scope_name.to_string());
                        }
                    }

                    scoped_type
                        .scope_stack_mut()
                        .replace_scope_from_strings(new_scopes)
                        .unwrap();
                }

                // Store the type rules for this namespace
                namespace_types.insert(namespace, scoped_type);
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
    pub fn get_namespace_type(&self, namespace: &str) -> Option<&ScopedType> {
        self.namespace_types.get(namespace)
    }

    /// Get type information for a specific property path in a namespace
    /// Path format: "property" or "property.nested.field"
    pub fn get_property_type(&self, namespace: &str, property_path: &str) -> Option<TypeInfo> {
        let formatter = TypeFormatter::new(&self.resolver, 30);

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

                match &current_type.cwt_type() {
                    CwtTypeOrSpecial::CwtType(CwtType::Block(_)) => {
                        match self.resolver.navigate_to_property(&current_type, part) {
                            PropertyNavigationResult::Success(scoped_type) => {
                                current_type = scoped_type;
                            }
                            PropertyNavigationResult::ScopeError(e) => {
                                return Some(TypeInfo {
                                    property_path: current_path,
                                    type_description: format!("Scope error: {}", e),
                                    scoped_type: None,
                                    documentation: None,
                                    source_info: Some(format!("Scope error: {}", e)),
                                });
                            }
                            PropertyNavigationResult::NotFound => {
                                return Some(TypeInfo {
                                    property_path: current_path,
                                    type_description: format!("Unknown property '{}'", part),
                                    scoped_type: None,
                                    documentation: None,
                                    source_info: Some(format!(
                                        "Property not found in {} entity",
                                        namespace
                                    )),
                                });
                            }
                        }
                    }
                    CwtTypeOrSpecial::CwtType(CwtType::Reference(_)) => {
                        // Handle reference types (like alias_match_left) using the resolver
                        match self.resolver.navigate_to_property(&current_type, part) {
                            PropertyNavigationResult::Success(scoped_type) => {
                                current_type = scoped_type;
                            }
                            PropertyNavigationResult::ScopeError(e) => {
                                return Some(TypeInfo {
                                    property_path: current_path,
                                    type_description: format!("Scope error: {}", e),
                                    scoped_type: None,
                                    documentation: None,
                                    source_info: Some(format!("Scope error: {}", e)),
                                });
                            }
                            PropertyNavigationResult::NotFound => {
                                return Some(TypeInfo {
                                    property_path: current_path,
                                    type_description: format!("Unknown property '{}'", part),
                                    scoped_type: None,
                                    documentation: None,
                                    source_info: Some(format!(
                                        "Property not found in {} entity",
                                        namespace
                                    )),
                                });
                            }
                        }
                    }
                    CwtTypeOrSpecial::CwtType(CwtType::Union(_u)) => {
                        // let mut potential_types = vec![];
                        // for member in u {
                        //     self.extract_all_union_types_with_property(
                        //         current_type.child(member.clone()),
                        //         part,
                        //         &mut potential_types,
                        //     );
                        // }

                        // potential_types.dedup_by(|a, b| a.fingerprint() == b.fingerprint());

                        // current_type = ScopedType::new_scoped_union(
                        //     potential_types,
                        //     current_type.scope_context().clone(),
                        // );

                        continue;
                    }

                    _ => {
                        return Some(TypeInfo {
                            property_path: current_path,
                            type_description: format!(
                                "Cannot access property '{}' on non-block type {:?}",
                                part, current_type
                            ),
                            scoped_type: None,
                            documentation: None,
                            source_info: Some("Property access on non-block type".to_string()),
                        });
                    }
                }
            }

            return Some(TypeInfo {
                property_path: property_path.to_string(),
                type_description: formatter.format_type(
                    &current_type,
                    path_parts.last().map(|s| *s), // Pass the last part as property name
                ),
                scoped_type: Some(current_type),
                documentation: None,
                source_info: Some(format!("From {} entity definition", namespace)),
            });
        }

        // If not found in namespace types, try CWT type definitions
        if let Some(type_def) = self.cwt_analyzer.get_type(namespace) {
            let path_parts: Vec<&str> = property_path.split('.').collect();
            let scoped_type = ScopedType::new_cwt(type_def.rules.clone(), Default::default());
            return Some(TypeInfo {
                property_path: property_path.to_string(),
                type_description: formatter.format_type(
                    &scoped_type,
                    path_parts.last().map(|s| *s), // Pass the last part as property name
                ),
                scoped_type: Some(scoped_type),
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

    pub fn get_resolver(&self) -> &TypeResolver {
        &self.resolver
    }

    pub fn resolve_type(&self, scoped_type: &ScopedType) -> ScopedType {
        self.resolver.resolve_type(scoped_type)
    }
}
