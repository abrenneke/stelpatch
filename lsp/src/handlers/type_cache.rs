use cw_model::types::CwtAnalyzer;
use cw_model::{CwtType, ReferenceType, SimpleType};
use cw_parser::CwtModuleCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, OnceLock};

/// Cache for Stellaris type information that's loaded once and shared across requests
pub struct TypeCache {
    namespace_types: HashMap<String, CwtType>,
    cwt_analyzer: Arc<CwtAnalyzer>,
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

                eprintln!(
                    "Mapping type '{}' with path '{:?}' to namespace '{}'",
                    type_name, type_def.path, namespace
                );

                // Store the type rules for this namespace
                namespace_types.insert(namespace, type_def.rules.clone());
            }

            eprintln!(
                "Built type cache with {} CWT types",
                cwt_analyzer.get_types().len()
            );

            TypeCache {
                namespace_types,
                cwt_analyzer: Arc::new(cwt_analyzer),
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

    /// Get the CWT analyzer for direct access to CWT definitions
    pub fn get_cwt_analyzer(&self) -> &CwtAnalyzer {
        &self.cwt_analyzer
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
                current_type = self.resolve_type(&current_type);

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
                        let resolved_type = self.resolve_type(&current_type);

                        // If we couldn't resolve the reference, return info about the reference itself
                        if matches!(resolved_type, CwtType::Reference(_)) {
                            return Some(TypeInfo {
                                property_path: current_path.clone(),
                                type_description: format_type_description_with_context(
                                    &resolved_type,
                                    0,
                                    15,
                                    Some(&self.cwt_analyzer),
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
                                type_description: format_type_description_with_context(
                                    &current_type,
                                    0,
                                    15,
                                    Some(&self.cwt_analyzer),
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

            return Some(TypeInfo {
                property_path: property_path.to_string(),
                type_description: format_type_description_with_context(
                    &current_type,
                    0,
                    15,
                    Some(&self.cwt_analyzer),
                ),
                cwt_type: Some(current_type.clone()),
                documentation: None,
                source_info: Some(format!("From {} entity definition", namespace)),
            });
        }

        // If not found in namespace types, try CWT type definitions
        if let Some(type_def) = self.cwt_analyzer.get_type(namespace) {
            return Some(TypeInfo {
                property_path: property_path.to_string(),
                type_description: format_type_description_with_context(
                    &type_def.rules,
                    0,
                    15,
                    Some(&self.cwt_analyzer),
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

    /// Resolve a type to its actual concrete type
    /// This handles references and other indirect types
    pub fn resolve_type(&self, cwt_type: &CwtType) -> CwtType {
        match cwt_type {
            // For references, try to resolve to the actual type
            CwtType::Reference(ref_type) => {
                match ref_type {
                    ReferenceType::Type { key } => {
                        // Try to find the referenced type in our analyzer
                        if let Some(resolved_type) = self.cwt_analyzer.get_type(key) {
                            self.resolve_type(&resolved_type.rules)
                        } else if let Some(resolved_type) = self.cwt_analyzer.get_single_alias(key)
                        {
                            self.resolve_type(resolved_type)
                        } else {
                            // If we can't resolve it, return the original reference
                            cwt_type.clone()
                        }
                    }
                    ReferenceType::Alias { key } => {
                        // Try to resolve alias references
                        if let Some(alias) = self.cwt_analyzer.get_alias(key) {
                            self.resolve_type(&alias.rules)
                        } else if let Some(resolved_type) = self.cwt_analyzer.get_single_alias(key)
                        {
                            self.resolve_type(resolved_type)
                        } else {
                            cwt_type.clone()
                        }
                    }
                    ReferenceType::AliasName { key } => {
                        // Try to resolve alias_name references
                        if let Some(resolved_type) = self.cwt_analyzer.get_single_alias(key) {
                            self.resolve_type(resolved_type)
                        } else {
                            cwt_type.clone()
                        }
                    }
                    ReferenceType::AliasMatchLeft { key } => {
                        // Try to resolve alias_match_left references
                        if let Some(resolved_type) = self.cwt_analyzer.get_single_alias(key) {
                            self.resolve_type(resolved_type)
                        } else {
                            cwt_type.clone()
                        }
                    }
                    ReferenceType::Enum { key } => {
                        // Try to get the enum type from our analyzer
                        if let Some(enum_def) = self.cwt_analyzer.get_enum(key) {
                            CwtType::LiteralSet(enum_def.values.clone())
                        } else {
                            cwt_type.clone()
                        }
                    }
                    ReferenceType::ValueSet { key } => {
                        // Try to get the value set type from our analyzer
                        if let Some(value_set) = self.cwt_analyzer.get_value_set(key) {
                            CwtType::LiteralSet(value_set.clone())
                        } else {
                            cwt_type.clone()
                        }
                    }
                    ReferenceType::SingleAlias { key } => {
                        // Try to resolve single alias references
                        if let Some(resolved_type) = self.cwt_analyzer.get_single_alias(key) {
                            self.resolve_type(resolved_type)
                        } else {
                            cwt_type.clone()
                        }
                    }
                    ReferenceType::Value { key } => {
                        // Try to resolve value references
                        if let Some(resolved_type) = self.cwt_analyzer.get_value_set(key) {
                            CwtType::LiteralSet(resolved_type.clone())
                        } else {
                            cwt_type.clone()
                        }
                    }
                    ReferenceType::ComplexEnum { key } => {
                        // Try to get the enum type from our analyzer
                        if let Some(enum_def) = self.cwt_analyzer.get_enum(key) {
                            CwtType::LiteralSet(enum_def.values.clone())
                        } else {
                            cwt_type.clone()
                        }
                    }
                    ReferenceType::AliasKeysField { key } => {
                        // Try to resolve alias keys field references
                        if let Some(resolved_type) = self.cwt_analyzer.get_single_alias(key) {
                            self.resolve_type(resolved_type)
                        } else {
                            cwt_type.clone()
                        }
                    }
                    ReferenceType::Subtype { name } => {
                        // For subtypes, we can't resolve them without more context
                        // Return a descriptive type instead
                        CwtType::Literal(format!("subtype:{}", name))
                    }
                    // For primitive-like references, return appropriate simple types
                    ReferenceType::Colour { .. } => CwtType::Simple(SimpleType::Color),
                    ReferenceType::Icon { .. } => CwtType::Simple(SimpleType::Icon),
                    ReferenceType::Filepath { .. } => CwtType::Simple(SimpleType::Filepath),
                    ReferenceType::StellarisNameFormat { .. } => {
                        CwtType::Simple(SimpleType::Localisation)
                    }
                    ReferenceType::Scope { .. } => CwtType::Simple(SimpleType::ScopeField),
                    ReferenceType::ScopeGroup { .. } => CwtType::Simple(SimpleType::ScopeField),
                    // For any remaining unhandled reference types, return the original
                    _ => cwt_type.clone(),
                }
            }
            // For comparables, unwrap to the base type
            CwtType::Comparable(base_type) => self.resolve_type(base_type),
            // For all other types, return as-is
            _ => cwt_type.clone(),
        }
    }
}

/// Information about a property's type
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub property_path: String,
    pub type_description: String,
    pub cwt_type: Option<CwtType>,
    pub documentation: Option<String>,
    pub source_info: Option<String>,
}

/// Format a type description for display in hover tooltips
fn format_type_description(cwt_type: &CwtType) -> String {
    format_type_description_with_context(cwt_type, 0, 15, None)
}

/// Format a type description with depth control, max lines, and optional CWT context
fn format_type_description_with_context(
    cwt_type: &CwtType,
    depth: usize,
    max_lines: usize,
    cwt_context: Option<&CwtAnalyzer>,
) -> String {
    if depth > 5 {
        return "...".to_string();
    }

    match cwt_type {
        CwtType::Literal(lit) => format!("\"{}\"", lit),
        CwtType::LiteralSet(literals) => {
            let mut sorted: Vec<_> = literals.iter().collect();
            sorted.sort();
            if sorted.len() <= 6 {
                sorted
                    .iter()
                    .map(|s| format!("\"{}\"", s))
                    .collect::<Vec<_>>()
                    .join(" | ")
            } else {
                format!(
                    "{} | /* ... +{} more */",
                    sorted
                        .iter()
                        .take(4)
                        .map(|s| format!("\"{}\"", s))
                        .collect::<Vec<_>>()
                        .join(" | "),
                    literals.len() - 4
                )
            }
        }
        CwtType::Simple(simple) => match simple {
            SimpleType::Bool => "boolean".to_string(),
            SimpleType::Int => "integer".to_string(),
            SimpleType::Float => "float".to_string(),
            SimpleType::Scalar => "scalar (0.0-1.0)".to_string(),
            SimpleType::PercentageField => "percentage".to_string(),
            SimpleType::Localisation => "localisation key".to_string(),
            SimpleType::LocalisationSynced => "synced localisation key".to_string(),
            SimpleType::LocalisationInline => "inline localisation".to_string(),
            SimpleType::DateField => "date (YYYY.MM.DD)".to_string(),
            SimpleType::VariableField => "variable reference".to_string(),
            SimpleType::IntVariableField => "integer variable reference".to_string(),
            SimpleType::ValueField => "value reference".to_string(),
            SimpleType::IntValueField => "integer value reference".to_string(),
            SimpleType::ScopeField => "scope reference".to_string(),
            SimpleType::Filepath => "file path".to_string(),
            SimpleType::Icon => "icon reference".to_string(),
            SimpleType::Color => "color (rgb/hsv/hex)".to_string(),
            SimpleType::Maths => "mathematical expression".to_string(),
        },
        CwtType::Reference(ref_type) => {
            match ref_type {
                ReferenceType::Type { key } => {
                    format!("→ {}", key)
                }
                ReferenceType::Enum { key } => {
                    // Try to get actual enum values from CWT context
                    if let Some(cwt) = cwt_context {
                        if let Some(enum_def) = cwt.get_enum(key) {
                            let values: Vec<_> = enum_def.values.iter().take(5).collect();
                            if values.len() < enum_def.values.len() {
                                format!(
                                    "enum {} ({}... +{} more)",
                                    key,
                                    values
                                        .iter()
                                        .map(|v| format!("\"{}\"", v))
                                        .collect::<Vec<_>>()
                                        .join(" | "),
                                    enum_def.values.len() - values.len()
                                )
                            } else {
                                format!(
                                    "enum {} ({})",
                                    key,
                                    values
                                        .iter()
                                        .map(|v| format!("\"{}\"", v))
                                        .collect::<Vec<_>>()
                                        .join(" | ")
                                )
                            }
                        } else {
                            format!("enum {}", key)
                        }
                    } else {
                        format!("enum {}", key)
                    }
                }
                ReferenceType::Scope { key } => {
                    format!("scope {}", key)
                }
                ReferenceType::Value { key } => {
                    format!("value {}", key)
                }
                ReferenceType::ValueSet { key } => {
                    // Try to get actual value set from CWT context
                    if let Some(cwt) = cwt_context {
                        if let Some(value_set) = cwt.get_value_set(key) {
                            let values: Vec<_> = value_set.iter().take(5).collect();
                            if values.len() < value_set.len() {
                                format!(
                                    "value_set {} ({}... +{} more)",
                                    key,
                                    values
                                        .iter()
                                        .map(|v| format!("\"{}\"", v))
                                        .collect::<Vec<_>>()
                                        .join(" | "),
                                    value_set.len() - values.len()
                                )
                            } else {
                                format!(
                                    "value_set {} ({})",
                                    key,
                                    values
                                        .iter()
                                        .map(|v| format!("\"{}\"", v))
                                        .collect::<Vec<_>>()
                                        .join(" | ")
                                )
                            }
                        } else {
                            format!("value_set {}", key)
                        }
                    } else {
                        format!("value_set {}", key)
                    }
                }
                ReferenceType::Alias { key } => {
                    format!("alias {}", key)
                }
                ReferenceType::AliasName { key } => {
                    format!("alias_name {}", key)
                }
                ReferenceType::AliasMatchLeft { key } => {
                    format!("alias_match_left {}", key)
                }
                ReferenceType::SingleAlias { key } => {
                    format!("single_alias {}", key)
                }
                ReferenceType::ComplexEnum { key } => {
                    // Try to get actual enum values from CWT context
                    if let Some(cwt) = cwt_context {
                        if let Some(enum_def) = cwt.get_enum(key) {
                            let values: Vec<_> = enum_def.values.iter().take(5).collect();
                            if values.len() < enum_def.values.len() {
                                format!(
                                    "complex_enum {} ({}... +{} more)",
                                    key,
                                    values
                                        .iter()
                                        .map(|v| format!("\"{}\"", v))
                                        .collect::<Vec<_>>()
                                        .join(" | "),
                                    enum_def.values.len() - values.len()
                                )
                            } else {
                                format!(
                                    "complex_enum {} ({})",
                                    key,
                                    values
                                        .iter()
                                        .map(|v| format!("\"{}\"", v))
                                        .collect::<Vec<_>>()
                                        .join(" | ")
                                )
                            }
                        } else {
                            format!("complex_enum {}", key)
                        }
                    } else {
                        format!("complex_enum {}", key)
                    }
                }
                ReferenceType::ScopeGroup { key } => {
                    format!("scope_group {}", key)
                }
                ReferenceType::Colour { format } => {
                    format!("colour ({})", format)
                }
                ReferenceType::Icon { path } => {
                    format!("icon ({})", path)
                }
                ReferenceType::Filepath { path } => {
                    format!("filepath ({})", path)
                }
                ReferenceType::Subtype { name } => {
                    format!("subtype {}", name)
                }
                ReferenceType::StellarisNameFormat { key } => {
                    format!("name_format {}", key)
                }
                ReferenceType::AliasKeysField { key } => {
                    format!("alias_keys_field {}", key)
                }
                _ => format!("reference {:?}", ref_type),
            }
        }
        CwtType::Comparable(comparable) => {
            format!(
                "comparable[{}]",
                format_type_description_with_context(comparable, depth + 1, max_lines, cwt_context)
            )
        }
        CwtType::Block(block) => {
            if block.properties.is_empty() {
                return "Entity: {}".to_string();
            }

            let mut properties: Vec<_> = block.properties.iter().collect();
            properties.sort_by_key(|(k, _)| k.as_str());

            let mut lines = vec!["Entity:".to_string()];
            let mut line_count = 1;
            let mut properties_shown = 0;

            for (key, property_def) in properties {
                if line_count >= max_lines {
                    lines.push(format!(
                        "  # ... +{} more properties",
                        block.properties.len() - properties_shown
                    ));
                    break;
                }

                let formatted_value = format_type_description_with_context(
                    &property_def.property_type,
                    depth + 1,
                    max_lines - line_count,
                    cwt_context,
                );

                // Handle multi-line types (nested blocks)
                if formatted_value.contains('\n') {
                    lines.push(format!("  {}:", key));
                    let nested_lines: Vec<&str> = formatted_value.lines().collect();
                    let mut lines_added = 1;

                    for line in nested_lines {
                        if line.starts_with("Entity:") {
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
                properties_shown += 1;
            }

            lines.join("\n")
        }
        CwtType::Array(array_type) => {
            let element_desc = format_type_description_with_context(
                &array_type.element_type,
                depth + 1,
                max_lines,
                cwt_context,
            );
            if element_desc.contains('\n') {
                format!(
                    "array[{}]",
                    if let CwtType::Block(_) = array_type.element_type.as_ref() {
                        "Entity"
                    } else {
                        "object"
                    }
                )
            } else {
                format!("array[{}]", element_desc)
            }
        }
        CwtType::Union(types) => {
            if types.len() <= 3 {
                types
                    .iter()
                    .map(|t| {
                        format_type_description_with_context(t, depth + 1, max_lines, cwt_context)
                    })
                    .collect::<Vec<_>>()
                    .join(" | ")
            } else {
                format!(
                    "{} | /* ... +{} more types */",
                    types
                        .iter()
                        .take(2)
                        .map(|t| format_type_description_with_context(
                            t,
                            depth + 1,
                            max_lines,
                            cwt_context
                        ))
                        .collect::<Vec<_>>()
                        .join(" | "),
                    types.len() - 2
                )
            }
        }
        CwtType::Unknown => "unknown".to_string(),
    }
}

/// Format a type description with depth control and max lines
fn format_type_description_with_depth(
    cwt_type: &CwtType,
    depth: usize,
    max_lines: usize,
) -> String {
    format_type_description_with_context(cwt_type, depth, max_lines, None)
}

/// Get type information for a namespace entity (top-level entity structure)
pub async fn get_namespace_entity_type(namespace: &str) -> Option<TypeInfo> {
    if !TypeCache::is_initialized() {
        return Some(TypeInfo {
            property_path: "entity".to_string(),
            type_description: "Loading type information...".to_string(),
            cwt_type: None,
            documentation: None,
            source_info: Some("Type system initializing".to_string()),
        });
    }

    let cache = TypeCache::get().unwrap();
    if let Some(namespace_type) = cache.get_namespace_type(namespace) {
        Some(TypeInfo {
            property_path: "entity".to_string(),
            type_description: format_type_description_with_context(
                namespace_type,
                0,
                15,
                Some(&cache.cwt_analyzer),
            ),
            cwt_type: Some(namespace_type.clone()),
            documentation: None,
            source_info: Some(format!("Entity structure for {} namespace", namespace)),
        })
    } else {
        Some(TypeInfo {
            property_path: "entity".to_string(),
            type_description: "No type information available for this namespace".to_string(),
            cwt_type: None,
            documentation: None,
            source_info: Some(format!("Namespace {} not found in type system", namespace)),
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
            cwt_type: None,
            documentation: None,
            source_info: Some("Type system initializing".to_string()),
        });
    }

    let cache = TypeCache::get().unwrap();
    cache.get_property_type(namespace, property_path)
}
