use cw_model::types::CwtAnalyzer;
use cw_model::{
    BlockType, CwtType, Entity, LowerCaseHashMap, Property, ReferenceType, SimpleType,
    TypeDefinition, TypeKeyFilter, entity_from_ast,
};
use cw_parser::CwtModuleCell;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use crate::handlers::cache::resolver::TypeResolver;
use crate::handlers::scoped_type::{CwtTypeOrSpecialRef, PropertyNavigationResult, ScopedType};

use super::types::TypeInfo;

/// Cache for Stellaris type information that's loaded once and shared across requests
pub struct TypeCache {
    namespace_types: HashMap<String, Vec<Arc<ScopedType>>>,
    cwt_analyzer: Arc<CwtAnalyzer>,
    resolver: TypeResolver,
}

static TYPE_CACHE: OnceLock<Arc<TypeCache>> = OnceLock::new();

impl TypeCache {
    /// Initialize the type cache by loading Stellaris data
    pub fn initialize_in_background() {
        // This runs in a background task since it can take time
        std::thread::spawn(|| {
            let _ = Self::get_or_init_blocking();
        });
    }

    pub fn get() -> Option<&'static Arc<TypeCache>> {
        TYPE_CACHE.get()
    }

    /// Get or initialize the global type cache (blocking version)
    fn get_or_init_blocking() -> &'static Arc<TypeCache> {
        TYPE_CACHE.get_or_init(|| {
            eprintln!("Initializing type cache");

            // Load CWT files - these contain all the type definitions we need
            let mut cwt_analyzer = Self::load_cwt_files();

            eprintln!("Building cache from CWT types");

            // Pre-compute entity types for quick lookups
            let mut namespace_types: HashMap<String, Vec<Arc<ScopedType>>> = HashMap::new();
            for (type_name, type_def) in cwt_analyzer.get_types() {
                // Extract namespace from the path
                let namespace = if let Some(path) = &type_def.path {
                    // Remove the "game/common" prefix to get the namespace
                    // e.g., "game/common/ambient_objects" -> "ambient_objects"
                    // e.g., "game/common/buildings/districts" -> "buildings/districts"
                    if path.starts_with("game/") {
                        path.strip_prefix("game/").unwrap_or(type_name).to_string()
                    } else {
                        path.to_string()
                    }
                } else {
                    eprintln!("Type has no path: {}", type_name);
                    continue;
                };

                let mut scoped_type =
                    ScopedType::new_cwt(type_def.rules.clone(), Default::default(), None);

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
                namespace_types
                    .entry(namespace)
                    .or_default()
                    .push(Arc::new(scoped_type));
            }

            // Modifiers are loaded separately so artificially add a modifier type
            cwt_analyzer.add_type(
                "modifier",
                TypeDefinition {
                    path: Some("game/modifiers".to_string()),
                    name_field: None,
                    skip_root_key: None,
                    localisation: LowerCaseHashMap::new(),
                    rules: Arc::new(CwtType::Unknown),
                    subtypes: LowerCaseHashMap::new(),
                    options: Default::default(),
                    rule_options: Default::default(),
                    modifiers: Default::default(),
                },
            );

            let mut inline_script_block = BlockType {
                type_name: "$inline_script".to_string(),
                properties: LowerCaseHashMap::new(),
                subtypes: LowerCaseHashMap::new(),
                subtype_properties: LowerCaseHashMap::new(),
                subtype_pattern_properties: LowerCaseHashMap::new(),
                pattern_properties: vec![],
                localisation: None,
                modifiers: Default::default(),
                additional_flags: Default::default(),
            };

            inline_script_block.properties.insert(
                "script".to_string(),
                Property {
                    property_type: Arc::new(CwtType::Reference(ReferenceType::InlineScript)),
                    documentation: None,
                    options: Default::default(),
                },
            );

            inline_script_block.properties.insert(
                "scalar".to_string(),
                Property {
                    property_type: Arc::new(CwtType::Any),
                    documentation: None,
                    options: Default::default(),
                },
            );

            // inline_script is special, it can appear anywhere and is not defined in the cwt files
            cwt_analyzer.add_type(
                "$inline_script",
                TypeDefinition {
                    path: Some("game/$inline_scripts".to_string()),
                    name_field: None,
                    skip_root_key: None,
                    subtypes: LowerCaseHashMap::new(),
                    localisation: LowerCaseHashMap::new(),
                    rules: Arc::new(CwtType::Union(vec![
                        // inline_script = {}
                        Arc::new(CwtType::Block(inline_script_block)),
                        // inline_script = "path/to/script"
                        Arc::new(CwtType::Simple(SimpleType::Scalar)),
                    ])),
                    options: Default::default(),
                    rule_options: Default::default(),
                    modifiers: Default::default(),
                },
            );

            eprintln!(
                "Built type cache with {} CWT types",
                cwt_analyzer.get_types().len()
            );

            let cwt_analyzer = Arc::new(cwt_analyzer);

            Arc::new(TypeCache {
                namespace_types,
                cwt_analyzer: cwt_analyzer.clone(),
                resolver: TypeResolver::new(cwt_analyzer.clone()),
            })
        })
    }

    /// Load CWT files from a path relative to the executable
    fn load_cwt_files() -> CwtAnalyzer {
        eprintln!("Loading CWT files from relative path");

        // First try to load from relative path (for bundled extension)
        let cwt_path = if let Ok(exe_path) = env::current_exe() {
            // Get the directory containing the executable (server/)
            if let Some(exe_dir) = exe_path.parent() {
                // Get the parent directory (extension root)
                if let Some(ext_root) = exe_dir.parent() {
                    // Join with config directory
                    let relative_config = ext_root.join("config");
                    eprintln!("Trying relative config path: {}", relative_config.display());
                    if relative_config.exists() {
                        Some(relative_config)
                    } else {
                        eprintln!(
                            "Relative config path doesn't exist, falling back to hardcoded path"
                        );
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Fall back to hardcoded path if relative path doesn't work
        let dir_path = cwt_path.unwrap_or_else(|| {
            eprintln!("Using hardcoded config path");
            Path::new(r"D:\dev\github\cwtools-stellaris-config\config").to_path_buf()
        });

        let mut cwt_analyzer = CwtAnalyzer::new();

        if !dir_path.exists() {
            eprintln!(
                "Warning: CWT directory '{}' does not exist",
                dir_path.display()
            );
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

        if let Err(e) = visit_dir(&dir_path, &mut cwt_files) {
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

    pub fn get_actual_namespace(namespace: &str) -> &str {
        if namespace.starts_with("gfx/portraits/portraits") {
            return "gfx/portraits/portraits";
        }

        if namespace.starts_with("gfx/") {
            return "gfx";
        }

        namespace
    }

    /// Get type information for a specific namespace
    pub fn get_namespace_types(&self, namespace: &str) -> Option<Vec<Arc<ScopedType>>> {
        let namespace = Self::get_actual_namespace(namespace);

        let mut all_types = vec![];

        if let Some(types) = self.namespace_types.get(namespace) {
            all_types.extend(types.clone());
        }

        if namespace.starts_with("gfx/models") {
            if let Some(types) = self.namespace_types.get("gfx") {
                all_types.extend(types.clone());
            }
        }

        if all_types.is_empty() {
            None
        } else {
            Some(all_types)
        }
    }

    pub fn get_namespace_type(
        &self,
        namespace: &str,
        file_path: Option<&str>,
    ) -> Option<Arc<ScopedType>> {
        let namespace = Self::get_actual_namespace(namespace);

        if let Some(namespace_types) = self.get_namespace_types(namespace) {
            if namespace_types.is_empty() {
                return None;
            }

            let mut union_types: Vec<Arc<CwtType>> = vec![];

            // If the type def has a file_path... try to match the file_path to the type def,
            // this takes precence over the union type
            for scoped_type in &namespace_types {
                if let CwtTypeOrSpecialRef::Block(block) = scoped_type.cwt_type_for_matching() {
                    if let Some(type_def) = self.cwt_analyzer.get_type(&block.type_name) {
                        if let Some(path_file) = type_def.options.path_file.as_ref() {
                            // path_file == file_path
                            if let Some(file_path) = file_path {
                                if path_file.contains(file_path) {
                                    if let CwtTypeOrSpecialRef::Block(block) =
                                        scoped_type.cwt_type_for_matching()
                                    {
                                        // path_file always wins
                                        return Some(Arc::new(ScopedType::new_cwt(
                                            Arc::new(CwtType::Block(block.clone())),
                                            Default::default(),
                                            None,
                                        )));
                                    }
                                }
                            }
                        } else {
                            // path_file exists, but is not the current file
                            // so we can ignore it
                            continue;
                        }

                        // namespace contains path
                        if let Some(path) = type_def.path.as_ref() {
                            if namespace.contains(path.trim_start_matches("game/")) {
                                if let CwtTypeOrSpecialRef::Block(block) =
                                    scoped_type.cwt_type_for_matching()
                                {
                                    union_types.push(Arc::new(CwtType::Block(block.clone())));
                                }
                            }
                        }
                    }
                }
            }

            match union_types.len() {
                0 => {}
                1 => {
                    return Some(Arc::new(ScopedType::new_cwt(
                        union_types[0].clone(),
                        Default::default(),
                        None,
                    )));
                }
                _ => {
                    return Some(Arc::new(ScopedType::new_cwt(
                        Arc::new(CwtType::Union(union_types)),
                        Default::default(),
                        None,
                    )));
                }
            }

            for scoped_type in &namespace_types {
                if let CwtTypeOrSpecialRef::Block(block) = scoped_type.cwt_type_for_matching() {
                    union_types.push(Arc::new(CwtType::Block(block.clone())));
                }
            }

            match union_types.len() {
                0 => {
                    return None;
                }
                1 => {
                    return Some(Arc::new(ScopedType::new_cwt(
                        union_types[0].clone(),
                        Default::default(),
                        None,
                    )));
                }
                _ => {
                    return Some(Arc::new(ScopedType::new_cwt(
                        Arc::new(CwtType::Union(union_types)),
                        Default::default(),
                        None,
                    )));
                }
            }
        }

        None
    }

    /// Filters union type members based on type_key_filter conditions.
    ///
    /// For union types, this method examines each member's type_key_filter
    /// and only includes members whose filter conditions are satisfied by
    /// the provided properties. Returns the original type if not a union.
    pub fn filter_union_types_by_properties(
        &self,
        scoped_type: Arc<ScopedType>,
        entity: &Entity,
    ) -> Arc<ScopedType> {
        match scoped_type.cwt_type_for_matching() {
            CwtTypeOrSpecialRef::Union(union_types) => {
                let mut filtered_types: Vec<Arc<CwtType>> = vec![];
                for t in union_types {
                    match &**t {
                        CwtType::Block(block) => {
                            if let Some(type_def) = self.cwt_analyzer.get_type(&block.type_name) {
                                if let Some(type_key_filter) =
                                    type_def.rule_options.type_key_filter.as_ref()
                                {
                                    match type_key_filter {
                                        TypeKeyFilter::Specific(key) => {
                                            if entity.properties.kv.contains_key(&key.to_string()) {
                                                filtered_types
                                                    .push(Arc::new(CwtType::Block(block.clone())));
                                            }
                                        }
                                        TypeKeyFilter::OneOf(keys) => {
                                            if keys.iter().any(|key| {
                                                entity.properties.kv.contains_key(&key.to_string())
                                            }) {
                                                filtered_types
                                                    .push(Arc::new(CwtType::Block(block.clone())));
                                            }
                                        }
                                        TypeKeyFilter::Not(key) => {
                                            if !entity.properties.kv.contains_key(&key.to_string())
                                            {
                                                filtered_types
                                                    .push(Arc::new(CwtType::Block(block.clone())));
                                            }
                                        }
                                    }
                                } else {
                                    filtered_types.push(Arc::new(CwtType::Block(block.clone())));
                                }
                            }
                        }
                        _ => {
                            filtered_types.push(t.clone());
                        }
                    }
                }

                match filtered_types.len() {
                    0 => {
                        return Arc::new(ScopedType::new_cwt(
                            Arc::new(CwtType::Unknown),
                            Default::default(),
                            None,
                        ));
                    }
                    1 => {
                        return Arc::new(ScopedType::new_cwt(
                            filtered_types[0].clone(),
                            Default::default(),
                            None,
                        ));
                    }
                    _ => {
                        return Arc::new(ScopedType::new_cwt(
                            Arc::new(CwtType::Union(filtered_types)),
                            Default::default(),
                            None,
                        ));
                    }
                }
            }
            CwtTypeOrSpecialRef::ScopedUnion(scoped_union_types) => {
                let mut filtered_types: Vec<Arc<ScopedType>> = vec![];
                for scoped_t in scoped_union_types {
                    match scoped_t.cwt_type_for_matching() {
                        CwtTypeOrSpecialRef::Block(block) => {
                            if let Some(type_def) = self.cwt_analyzer.get_type(&block.type_name) {
                                if let Some(type_key_filter) =
                                    type_def.rule_options.type_key_filter.as_ref()
                                {
                                    match type_key_filter {
                                        TypeKeyFilter::Specific(key) => {
                                            if entity.properties.kv.contains_key(&key.to_string()) {
                                                filtered_types.push(scoped_t.clone());
                                            }
                                        }
                                        TypeKeyFilter::OneOf(keys) => {
                                            if keys.iter().any(|key| {
                                                entity.properties.kv.contains_key(&key.to_string())
                                            }) {
                                                filtered_types.push(scoped_t.clone());
                                            }
                                        }
                                        TypeKeyFilter::Not(key) => {
                                            if !entity.properties.kv.contains_key(&key.to_string())
                                            {
                                                filtered_types.push(scoped_t.clone());
                                            }
                                        }
                                    }
                                } else {
                                    filtered_types.push(scoped_t.clone());
                                }
                            }
                        }
                        _ => {
                            // For non-block types in scoped union, include them
                            filtered_types.push(scoped_t.clone());
                        }
                    }
                }

                match filtered_types.len() {
                    0 => {
                        return Arc::new(ScopedType::new_cwt(
                            Arc::new(CwtType::Unknown),
                            Default::default(),
                            None,
                        ));
                    }
                    1 => {
                        return filtered_types[0].clone();
                    }
                    _ => {
                        return Arc::new(ScopedType::new_scoped_union(
                            filtered_types,
                            scoped_type.scope_stack().clone(),
                            scoped_type.in_scripted_effect_block().cloned(),
                        ));
                    }
                }
            }
            _ => scoped_type,
        }
    }

    /// Filters union type members based on type_key_filter conditions using the entity key.
    ///
    /// For union types, this method examines each member's type_key_filter
    /// and only includes members whose filter conditions are satisfied by
    /// the provided entity key. Returns the original type if not a union.
    pub fn filter_union_types_by_key(
        &self,
        scoped_type: Arc<ScopedType>,
        entity_key: &str,
    ) -> Arc<ScopedType> {
        match scoped_type.cwt_type_for_matching() {
            CwtTypeOrSpecialRef::Union(union_types) => {
                let mut filtered_types: Vec<Arc<CwtType>> = vec![];
                for t in union_types {
                    match &**t {
                        CwtType::Block(block) => {
                            if let Some(type_def) = self.cwt_analyzer.get_type(&block.type_name) {
                                if let Some(type_key_filter) =
                                    type_def.rule_options.type_key_filter.as_ref()
                                {
                                    match type_key_filter {
                                        TypeKeyFilter::Specific(key) => {
                                            if entity_key == key {
                                                filtered_types
                                                    .push(Arc::new(CwtType::Block(block.clone())));
                                            }
                                        }
                                        TypeKeyFilter::OneOf(keys) => {
                                            if keys.iter().any(|key| entity_key == key) {
                                                filtered_types
                                                    .push(Arc::new(CwtType::Block(block.clone())));
                                            }
                                        }
                                        TypeKeyFilter::Not(key) => {
                                            if entity_key != key {
                                                filtered_types
                                                    .push(Arc::new(CwtType::Block(block.clone())));
                                            }
                                        }
                                    }
                                } else {
                                    // No type_key_filter means this type applies to all keys
                                    filtered_types.push(Arc::new(CwtType::Block(block.clone())));
                                }
                            }
                        }
                        _ => {
                            // Non-block types don't have type_key_filter, include them
                            filtered_types.push(t.clone());
                        }
                    }
                }

                match filtered_types.len() {
                    0 => {
                        return Arc::new(ScopedType::new_cwt(
                            Arc::new(CwtType::Unknown),
                            Default::default(),
                            None,
                        ));
                    }
                    1 => {
                        return Arc::new(ScopedType::new_cwt(
                            filtered_types[0].clone(),
                            Default::default(),
                            None,
                        ));
                    }
                    _ => {
                        return Arc::new(ScopedType::new_cwt(
                            Arc::new(CwtType::Union(filtered_types)),
                            Default::default(),
                            None,
                        ));
                    }
                }
            }
            CwtTypeOrSpecialRef::ScopedUnion(scoped_union_types) => {
                let mut filtered_types: Vec<Arc<ScopedType>> = vec![];
                for scoped_t in scoped_union_types {
                    match scoped_t.cwt_type_for_matching() {
                        CwtTypeOrSpecialRef::Block(block) => {
                            if let Some(type_def) = self.cwt_analyzer.get_type(&block.type_name) {
                                if let Some(type_key_filter) =
                                    type_def.rule_options.type_key_filter.as_ref()
                                {
                                    match type_key_filter {
                                        TypeKeyFilter::Specific(key) => {
                                            if entity_key == key {
                                                filtered_types.push(scoped_t.clone());
                                            }
                                        }
                                        TypeKeyFilter::OneOf(keys) => {
                                            if keys.iter().any(|key| entity_key == key) {
                                                filtered_types.push(scoped_t.clone());
                                            }
                                        }
                                        TypeKeyFilter::Not(key) => {
                                            if entity_key != key {
                                                filtered_types.push(scoped_t.clone());
                                            }
                                        }
                                    }
                                } else {
                                    // No type_key_filter means this type applies to all keys
                                    filtered_types.push(scoped_t.clone());
                                }
                            }
                        }
                        _ => {
                            // For non-block types in scoped union, include them
                            filtered_types.push(scoped_t.clone());
                        }
                    }
                }

                match filtered_types.len() {
                    0 => {
                        return Arc::new(ScopedType::new_cwt(
                            Arc::new(CwtType::Unknown),
                            Default::default(),
                            None,
                        ));
                    }
                    1 => {
                        return filtered_types[0].clone();
                    }
                    _ => {
                        return Arc::new(ScopedType::new_scoped_union(
                            filtered_types,
                            scoped_type.scope_stack().clone(),
                            scoped_type.in_scripted_effect_block().cloned(),
                        ));
                    }
                }
            }
            _ => scoped_type,
        }
    }

    /// Get type information for a property by navigating through an AST entity
    /// This method does full AST navigation with subtype narrowing, similar to validate_entity_value.
    ///
    /// Unlike `get_property_type`, this method:
    /// - Analyzes the actual AST entity to extract property data
    /// - Applies subtype narrowing based on the entity's properties
    /// - Provides more accurate type information for properties that depend on subtypes
    ///
    /// Use this method when you have access to the actual AST entity and need precise type information.
    /// Use `get_property_type` for simple string-based property lookups without AST context.
    pub fn get_property_type_from_ast(
        &self,
        namespace: &str,
        entity: &cw_parser::AstEntity<'_>,
        property_path: &str,
        file_path: Option<&str>,
    ) -> Option<TypeInfo> {
        // Get the base namespace type
        let namespace_type = self.get_namespace_type(namespace, file_path)?;

        let model_entity = entity_from_ast(entity);

        // Apply subtype narrowing to the namespace type
        let narrowed_namespace_type =
            self.narrow_type_with_subtypes(namespace_type.clone(), &model_entity); // Start with the first type

        // Navigate to the property with the narrowed type
        let path_parts: Vec<&str> = property_path.split('.').collect();
        let mut current_type = narrowed_namespace_type;
        let mut current_path = String::new();

        for (i, part) in path_parts.iter().enumerate() {
            if i > 0 {
                current_path.push('.');
            }
            current_path.push_str(part);

            // Resolve the current type to its actual type
            current_type = self.resolver.resolve_type(current_type);

            match &current_type.cwt_type_for_matching() {
                CwtTypeOrSpecialRef::Block(_) => {
                    match self.resolver.navigate_to_property(current_type, part) {
                        PropertyNavigationResult::Success(scoped_type) => {
                            current_type = scoped_type;
                        }
                        PropertyNavigationResult::ScopeError(e) => {
                            return Some(TypeInfo {
                                property_path: current_path,
                                scoped_type: None,
                                documentation: None,
                                source_info: Some(format!("Scope error: {}", e)),
                            });
                        }
                        PropertyNavigationResult::NotFound => {
                            return Some(TypeInfo {
                                property_path: current_path,
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
                CwtTypeOrSpecialRef::Reference(_) => {
                    // Handle reference types using the resolver
                    match self.resolver.navigate_to_property(current_type, part) {
                        PropertyNavigationResult::Success(scoped_type) => {
                            current_type = scoped_type;
                        }
                        PropertyNavigationResult::ScopeError(e) => {
                            return Some(TypeInfo {
                                property_path: current_path,
                                scoped_type: None,
                                documentation: None,
                                source_info: Some(format!("Scope error: {}", e)),
                            });
                        }
                        PropertyNavigationResult::NotFound => {
                            return Some(TypeInfo {
                                property_path: current_path,
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
                CwtTypeOrSpecialRef::Union(_) | CwtTypeOrSpecialRef::ScopedUnion(_) => {
                    // Let the resolver handle union types - it has the proper logic for this
                    match self.resolver.navigate_to_property(current_type, part) {
                        PropertyNavigationResult::Success(scoped_type) => {
                            current_type = scoped_type;
                        }
                        PropertyNavigationResult::ScopeError(e) => {
                            return Some(TypeInfo {
                                property_path: current_path,
                                scoped_type: None,
                                documentation: None,
                                source_info: Some(format!("Scope error: {}", e)),
                            });
                        }
                        PropertyNavigationResult::NotFound => {
                            return Some(TypeInfo {
                                property_path: current_path,
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
                _ => {
                    return Some(TypeInfo {
                        property_path: current_path,
                        scoped_type: None,
                        documentation: None,
                        source_info: Some("Property access on non-block type".to_string()),
                    });
                }
            }
        }

        Some(TypeInfo {
            property_path: property_path.to_string(),
            scoped_type: Some(current_type.clone()),
            documentation: None,
            source_info: None,
        })
    }

    /// Narrow a type with subtypes based on property data
    fn narrow_type_with_subtypes(
        &self,
        base_type: Arc<ScopedType>,
        entity: &Entity,
    ) -> Arc<ScopedType> {
        if let CwtTypeOrSpecialRef::Block(block_type) = base_type.cwt_type_for_matching() {
            if !block_type.subtypes.is_empty() {
                // Try to determine the matching subtypes
                let detected_subtypes = self
                    .resolver
                    .determine_matching_subtypes(base_type.clone(), entity);

                if !detected_subtypes.is_empty() {
                    // Create a new scoped type with the detected subtypes
                    return Arc::new(base_type.with_subtypes(detected_subtypes));
                }
            }
        }

        base_type
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

    pub fn resolve_type(&self, scoped_type: Arc<ScopedType>) -> Arc<ScopedType> {
        self.resolver.resolve_type(scoped_type)
    }
}
