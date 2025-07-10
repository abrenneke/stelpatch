use super::core::GameDataCache;
use cw_model::types::{CwtAnalyzer, TypeFingerprint};
use cw_model::{AliasName, BlockType, CwtOptions, CwtType, Property, ReferenceType, SimpleType};
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};

pub struct TypeResolver {
    cwt_analyzer: Arc<CwtAnalyzer>,
    cache: Arc<RwLock<TypeResolverCache>>,
}

pub struct TypeResolverCache {
    cache: HashMap<String, CwtType>,
    resolving: std::collections::HashSet<String>,
}

impl TypeResolver {
    pub fn new(cwt_analyzer: Arc<CwtAnalyzer>) -> Self {
        Self {
            cwt_analyzer,
            cache: Arc::new(RwLock::new(TypeResolverCache {
                cache: HashMap::new(),
                resolving: std::collections::HashSet::new(),
            })),
        }
    }

    /// Clear the cache (useful for debugging or memory management)
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.cache.clear();
        cache.resolving.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().unwrap();
        (cache.cache.len(), cache.resolving.len())
    }

    /// Resolve a type to its actual concrete type with fingerprint-based caching
    ///
    /// This method uses the type fingerprint system to cache resolved types,
    /// which provides better cache hit rates and more reliable deduplication
    /// compared to the previous custom cache key system.
    pub fn resolve_type(&self, cwt_type: &CwtType) -> CwtType {
        match cwt_type {
            // For references, try to resolve to the actual type
            CwtType::Reference(ref_type) => {
                let cache_key = cwt_type.fingerprint();

                // Check if we're already resolving this type (circular reference)
                if self.cache.read().unwrap().resolving.contains(&cache_key) {
                    // Return the original type to break the cycle
                    return cwt_type.clone();
                }

                // Check cache first
                if let Some(cached_result) = self.cache.read().unwrap().cache.get(&cache_key) {
                    return cached_result.clone();
                }

                // Mark as resolving
                self.cache
                    .write()
                    .unwrap()
                    .resolving
                    .insert(cache_key.clone());

                let result = self.resolve_reference_type(ref_type);

                // Remove from resolving set
                {
                    let mut cache = self.cache.write().unwrap();
                    cache.resolving.remove(&cache_key);
                }

                // Cache the result
                self.cache
                    .write()
                    .unwrap()
                    .cache
                    .insert(cache_key, result.clone());

                result
            }
            // For comparables, unwrap to the base type
            CwtType::Comparable(base_type) => self.resolve_type(base_type),
            // For blocks, resolve and expand patterns with caching
            CwtType::Block(block_type) => {
                let cache_key = cwt_type.fingerprint();

                // Check cache first
                if let Some(cached_result) = self.cache.read().unwrap().cache.get(&cache_key) {
                    return cached_result.clone();
                }

                // Check if we're already resolving this block (circular reference)
                if self.cache.read().unwrap().resolving.contains(&cache_key) {
                    return cwt_type.clone();
                }

                // Mark as resolving
                self.cache
                    .write()
                    .unwrap()
                    .resolving
                    .insert(cache_key.clone());

                let mut resolved_block = block_type.clone();
                self.expand_patterns_in_block(&mut resolved_block);
                let result = CwtType::Block(resolved_block);

                // Remove from resolving set
                {
                    let mut cache = self.cache.write().unwrap();
                    cache.resolving.remove(&cache_key);
                }

                // Cache the result
                self.cache
                    .write()
                    .unwrap()
                    .cache
                    .insert(cache_key, result.clone());

                result
            }
            // For all other types, return as-is
            _ => cwt_type.clone(),
        }
    }

    /// Check if two types are equivalent using their fingerprints
    /// This is more efficient than resolving both types and comparing them
    pub fn are_types_equivalent(&self, type1: &CwtType, type2: &CwtType) -> bool {
        type1.fingerprint() == type2.fingerprint()
    }

    /// Get the fingerprint hash for a type for efficient deduplication
    /// This can be used for storing types in hash sets or other data structures
    pub fn get_type_fingerprint_hash(&self, cwt_type: &CwtType) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        cwt_type.fingerprint().hash(&mut hasher);
        hasher.finish()
    }

    /// Deduplicate a collection of types using their fingerprints
    /// Returns a Vec with unique types, preserving order of first occurrence
    pub fn deduplicate_types(&self, types: Vec<CwtType>) -> Vec<CwtType> {
        let mut seen_fingerprints = std::collections::HashSet::new();
        let mut result = Vec::new();

        for cwt_type in types {
            let fingerprint = cwt_type.fingerprint();
            if seen_fingerprints.insert(fingerprint) {
                result.push(cwt_type);
            }
        }

        result
    }

    /// Create a union type from a collection of types, automatically deduplicating
    /// and flattening nested unions
    pub fn create_deduplicated_union(&self, types: Vec<CwtType>) -> CwtType {
        let mut flattened_types = Vec::new();

        // Flatten nested unions
        for cwt_type in types {
            match cwt_type {
                CwtType::Union(nested_types) => {
                    flattened_types.extend(nested_types);
                }
                _ => {
                    flattened_types.push(cwt_type);
                }
            }
        }

        // Deduplicate
        let unique_types = self.deduplicate_types(flattened_types);

        // Return appropriate type based on count
        match unique_types.len() {
            0 => CwtType::Unknown,
            1 => unique_types.into_iter().next().unwrap(),
            _ => CwtType::Union(unique_types),
        }
    }

    fn resolve_reference_type(&self, ref_type: &ReferenceType) -> CwtType {
        match ref_type {
            ReferenceType::Type { key } => {
                let type_def = self.cwt_analyzer.get_type(key);

                if let Some(type_def) = type_def {
                    if let Some(path) = type_def.path.as_ref() {
                        // CWT paths are prefixed with "game/"
                        let path = path.trim_start_matches("game/");

                        // For Type references, we want the union of all keys in that namespace
                        // This is what the user expects when they hover over "resource" - they want to see
                        // all the possible resource keys like "energy", "minerals", etc.
                        if let Some(game_data) = GameDataCache::get() {
                            if let Some(namespace_keys) = game_data.get_namespace_keys(&path) {
                                return CwtType::LiteralSet(
                                    namespace_keys.iter().cloned().collect(),
                                );
                            }

                            // Also try the key directly in case it's already a full path
                            if let Some(namespace_keys) = game_data.get_namespace_keys(key) {
                                return CwtType::LiteralSet(
                                    namespace_keys.iter().cloned().collect(),
                                );
                            }
                        }
                    }
                }

                // If game data isn't available or namespace not found, return the original reference
                CwtType::Reference(ref_type.clone())
            }
            ReferenceType::Alias { .. } => {
                // Invalid alias[] on RHS
                CwtType::Reference(ref_type.clone())
            }
            ReferenceType::AliasName { .. } => {
                // Invalid alias_name on RHS
                CwtType::Reference(ref_type.clone())
            }
            ReferenceType::AliasMatchLeft { key } => {
                // For alias_match_left, we want to represent the value types of aliases from this category
                let mut union_types = Vec::new();

                // Look for aliases that match the category (format: "category:name")
                for (alias_key, alias_def) in self.cwt_analyzer.get_aliases() {
                    if alias_key.category == *key {
                        // DON'T recursively resolve - just use the alias definition directly
                        union_types.push(alias_def.to.clone());
                    }
                }

                union_types.dedup_by(|a, b| a.fingerprint() == b.fingerprint());

                if !union_types.is_empty() {
                    if union_types.len() == 1 {
                        union_types.into_iter().next().unwrap()
                    } else {
                        CwtType::Union(union_types)
                    }
                } else {
                    CwtType::Reference(ref_type.clone())
                }
            }
            ReferenceType::SingleAlias { .. } => {
                // Invalid single_alias_name on RHS
                CwtType::Reference(ref_type.clone())
            }
            ReferenceType::Enum { key } => {
                // Try to get the enum type from our analyzer
                if let Some(enum_def) = self.cwt_analyzer.get_enum(key) {
                    CwtType::LiteralSet(enum_def.values.clone())
                } else {
                    CwtType::Reference(ref_type.clone())
                }
            }
            ReferenceType::ValueSet { key } => {
                // Try to get the value set type from our analyzer
                if let Some(value_set) = self.cwt_analyzer.get_value_set(key) {
                    CwtType::LiteralSet(value_set.clone())
                } else {
                    CwtType::Reference(ref_type.clone())
                }
            }
            ReferenceType::Value { key } => {
                // Try to resolve value references
                if let Some(resolved_type) = self.cwt_analyzer.get_value_set(key) {
                    CwtType::LiteralSet(resolved_type.clone())
                } else {
                    CwtType::Reference(ref_type.clone())
                }
            }
            ReferenceType::ComplexEnum { key } => {
                // Try to get the enum type from our analyzer
                if let Some(enum_def) = self.cwt_analyzer.get_enum(key) {
                    CwtType::LiteralSet(enum_def.values.clone())
                } else {
                    CwtType::Reference(ref_type.clone())
                }
            }
            ReferenceType::AliasKeysField { key } => {
                // Try to resolve alias keys field references
                if let Some(resolved_type) = self.cwt_analyzer.get_single_alias(key) {
                    self.resolve_type(resolved_type)
                } else {
                    CwtType::Reference(ref_type.clone())
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
            ReferenceType::StellarisNameFormat { .. } => CwtType::Simple(SimpleType::Localisation),
            ReferenceType::Scope { .. } => CwtType::Simple(SimpleType::ScopeField),
            ReferenceType::ScopeGroup { .. } => CwtType::Simple(SimpleType::ScopeField),
            // For any remaining unhandled reference types, return the original
            _ => CwtType::Reference(ref_type.clone()),
        }
    }

    /// Expand patterns in a block type
    /// This handles both enum patterns and alias patterns
    /// Note: This method is now cached in resolve_type for performance
    fn expand_patterns_in_block(&self, block_type: &mut BlockType) {
        // Early return if no patterns to expand
        if block_type.enum_patterns.is_empty() && block_type.alias_patterns.is_empty() {
            return;
        }

        let mut new_properties = HashMap::new();

        if !GameDataCache::is_initialized() {
            return;
        }

        let game_data = GameDataCache::get().unwrap();

        // Process each enum pattern
        for (enum_key, value_type) in &block_type.enum_patterns {
            if let Some(enum_def) = self.cwt_analyzer.get_enum(enum_key) {
                // Create a property for each enum value
                for enum_value in &enum_def.values {
                    let new_property = Property {
                        property_type: self.resolve_type(value_type),
                        options: CwtOptions::default(),
                        documentation: Some(format!("Enum value from {}", enum_key)),
                    };
                    new_properties.insert(enum_value.clone(), new_property);
                }
            }
        }

        // Process each alias pattern
        for (alias_pattern, value_type) in &block_type.alias_patterns {
            // Get all aliases from this category and create properties for them
            for (alias_key_full, _) in self.cwt_analyzer.get_aliases() {
                if alias_key_full.category == *alias_pattern {
                    match &alias_key_full.name {
                        // For alias[foo:x] = bar, we create a single property for each alias
                        AliasName::Static(name) => {
                            // DON'T resolve the value_type here - it causes O(n²) performance issues
                            // Just use the raw type and let it be resolved lazily when needed
                            let new_property = Property {
                                property_type: value_type.clone(),
                                options: CwtOptions::default(),
                                documentation: Some(format!(
                                    "Alias from {} category",
                                    alias_pattern
                                )),
                            };
                            new_properties.insert(name.to_string(), new_property);
                        }
                        // For alias[foo:<type_name>] = bar, we expand <type_name> to all types in the namespace
                        AliasName::TypeRef(name) => {
                            let type_def = self.cwt_analyzer.get_type(name);
                            if let Some(type_def) = type_def {
                                if let Some(path) = type_def.path.as_ref() {
                                    let path = path.trim_start_matches("game/");
                                    let all_types = game_data.get_namespace_keys(path);
                                    if let Some(all_types) = all_types {
                                        for type_key in all_types {
                                            let new_property = Property {
                                                property_type: value_type.clone(),
                                                options: CwtOptions::default(),
                                                documentation: Some(format!(
                                                    "Alias from {} category",
                                                    alias_pattern
                                                )),
                                            };
                                            new_properties.insert(type_key.clone(), new_property);
                                        }
                                    }
                                }
                            }
                        }
                        AliasName::Enum(name) => {
                            let all_enums = self.cwt_analyzer.get_enum(name);
                            if let Some(all_enums) = all_enums {
                                for enum_value in &all_enums.values {
                                    let new_property = Property {
                                        property_type: value_type.clone(),
                                        options: CwtOptions::default(),
                                        documentation: Some(format!("Enum value from {}", name)),
                                    };

                                    new_properties.insert(enum_value.clone(), new_property);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add all expanded properties
        block_type.properties.extend(new_properties);
    }
}
