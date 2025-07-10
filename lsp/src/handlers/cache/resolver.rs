use super::core::GameDataCache;
use cw_model::types::{CwtAnalyzer, PatternProperty, PatternType, TypeFingerprint};
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
            // For blocks, resolve and convert patterns to pattern properties with caching
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
                self.convert_patterns_to_pattern_properties(&mut resolved_block);
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

    /// Convert patterns to pattern properties instead of expanding them
    /// This preserves cardinality constraints while allowing pattern matching
    fn convert_patterns_to_pattern_properties(&self, block_type: &mut BlockType) {
        // Convert enum patterns to pattern properties
        for (enum_key, value_type) in &block_type.enum_patterns {
            let pattern_property = PatternProperty {
                pattern_type: PatternType::Enum {
                    key: enum_key.clone(),
                },
                value_type: value_type.clone(),
                options: CwtOptions::default(),
                documentation: Some(format!("Enum pattern for {}", enum_key)),
            };
            block_type.pattern_properties.push(pattern_property);
        }

        // Convert alias patterns to pattern properties
        for (alias_pattern, value_type) in &block_type.alias_patterns {
            let pattern_property = PatternProperty {
                pattern_type: PatternType::AliasName {
                    category: alias_pattern.clone(),
                },
                value_type: value_type.clone(),
                options: CwtOptions::default(),
                documentation: Some(format!("Alias pattern for {} category", alias_pattern)),
            };
            block_type.pattern_properties.push(pattern_property);
        }

        // Clear the old pattern maps since we've converted them
        block_type.enum_patterns.clear();
        block_type.alias_patterns.clear();
    }

    /// Check if a key matches any pattern property in a block
    pub fn key_matches_pattern<'a>(
        &self,
        key: &str,
        block_type: &'a BlockType,
    ) -> Option<&'a PatternProperty> {
        for pattern_property in &block_type.pattern_properties {
            if self.key_matches_pattern_type(key, &pattern_property.pattern_type) {
                return Some(pattern_property);
            }
        }
        None
    }

    /// Check if a key matches a specific pattern type
    pub fn key_matches_pattern_type(&self, key: &str, pattern_type: &PatternType) -> bool {
        match pattern_type {
            PatternType::AliasName { category } => {
                // Check if the key matches any alias name from this category
                for (alias_key, _) in self.cwt_analyzer.get_aliases() {
                    if alias_key.category == *category {
                        match &alias_key.name {
                            AliasName::Static(name) => {
                                if name == key {
                                    return true;
                                }
                            }
                            AliasName::TypeRef(type_name) => {
                                // Check if key matches any type from this namespace
                                if let Some(type_def) = self.cwt_analyzer.get_type(type_name) {
                                    if let Some(path) = type_def.path.as_ref() {
                                        let path = path.trim_start_matches("game/");
                                        if let Some(game_data) = GameDataCache::get() {
                                            if let Some(namespace_keys) =
                                                game_data.get_namespace_keys(&path)
                                            {
                                                if namespace_keys.contains(&key.to_string()) {
                                                    return true;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            AliasName::Enum(enum_name) => {
                                // Check if key matches any enum value
                                if let Some(enum_def) = self.cwt_analyzer.get_enum(enum_name) {
                                    if enum_def.values.contains(key) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
                false
            }
            PatternType::Enum { key: enum_key } => {
                // Check if the key matches any enum value
                if let Some(enum_def) = self.cwt_analyzer.get_enum(enum_key) {
                    enum_def.values.contains(key)
                } else {
                    false
                }
            }
        }
    }

    /// Get all possible completions for a pattern type
    pub fn get_pattern_completions(&self, pattern_type: &PatternType) -> Vec<String> {
        match pattern_type {
            PatternType::AliasName { category } => {
                let mut completions = Vec::new();
                for (alias_key, _) in self.cwt_analyzer.get_aliases() {
                    if alias_key.category == *category {
                        match &alias_key.name {
                            AliasName::Static(name) => {
                                completions.push(name.clone());
                            }
                            AliasName::TypeRef(type_name) => {
                                if let Some(type_def) = self.cwt_analyzer.get_type(type_name) {
                                    if let Some(path) = type_def.path.as_ref() {
                                        let path = path.trim_start_matches("game/");
                                        if let Some(game_data) = GameDataCache::get() {
                                            if let Some(namespace_keys) =
                                                game_data.get_namespace_keys(&path)
                                            {
                                                completions.extend(namespace_keys.iter().cloned());
                                            }
                                        }
                                    }
                                }
                            }
                            AliasName::Enum(enum_name) => {
                                if let Some(enum_def) = self.cwt_analyzer.get_enum(enum_name) {
                                    completions.extend(enum_def.values.iter().cloned());
                                }
                            }
                        }
                    }
                }
                completions
            }
            PatternType::Enum { key } => {
                if let Some(enum_def) = self.cwt_analyzer.get_enum(key) {
                    enum_def.values.iter().cloned().collect()
                } else {
                    Vec::new()
                }
            }
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
}
