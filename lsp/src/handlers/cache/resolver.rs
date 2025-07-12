use crate::handlers::cache::{FullAnalysis, GameDataCache};
use crate::handlers::scope::ScopeStack;
use crate::handlers::scoped_type::{
    CwtTypeOrSpecial, PropertyNavigationResult, ScopeAwareProperty, ScopedType,
};
use cw_model::types::{CwtAnalyzer, LinkDefinition, PatternProperty, PatternType, TypeFingerprint};
use cw_model::{AliasName, BlockType, CwtOptions, CwtType, ReferenceType, SimpleType};
use std::sync::Arc;
use std::sync::RwLock;

pub struct TypeResolver {
    cwt_analyzer: Arc<CwtAnalyzer>,
    cache: Arc<RwLock<TypeResolverCache>>,
}

pub struct TypeResolverCache {
    resolving: std::collections::HashSet<String>,
}

impl TypeResolver {
    pub fn new(cwt_analyzer: Arc<CwtAnalyzer>) -> Self {
        Self {
            cwt_analyzer,
            cache: Arc::new(RwLock::new(TypeResolverCache {
                resolving: std::collections::HashSet::new(),
            })),
        }
    }

    /// Resolves references & nested types to concrete types
    pub fn resolve_type(&self, scoped_type: &ScopedType) -> ScopedType {
        let cwt_type = scoped_type.cwt_type();
        let cache_key = cwt_type.fingerprint();

        match cwt_type {
            // For references, try to resolve to the actual type
            CwtTypeOrSpecial::CwtType(CwtType::Reference(ref_type)) => {
                // Check if we're already resolving this type (circular reference)
                if self.cache.read().unwrap().resolving.contains(&cache_key) {
                    // Return the original scoped type to break the cycle
                    return scoped_type.clone();
                }

                // For scoped types, we don't cache as heavily since scope context matters
                // Mark as resolving
                self.cache
                    .write()
                    .unwrap()
                    .resolving
                    .insert(cache_key.clone());

                let resolved_cwt_type = self.resolve_reference_type(ref_type);
                let result =
                    ScopedType::new_cwt(resolved_cwt_type, scoped_type.scope_stack().clone());

                // Remove from resolving set
                {
                    let mut cache = self.cache.write().unwrap();
                    cache.resolving.remove(&cache_key);
                }

                result
            }
            // For comparables, unwrap to the base type
            CwtTypeOrSpecial::CwtType(CwtType::Comparable(base_type)) => {
                let base_scoped =
                    ScopedType::new_cwt((**base_type).clone(), scoped_type.scope_stack().clone());
                self.resolve_type(&base_scoped)
            }
            // For blocks, resolve and convert patterns to pattern properties
            CwtTypeOrSpecial::CwtType(CwtType::Block(block_type)) => {
                // Check if we're already resolving this block (circular reference)
                if self.cache.read().unwrap().resolving.contains(&cache_key) {
                    return scoped_type.clone();
                }

                // Mark as resolving
                self.cache
                    .write()
                    .unwrap()
                    .resolving
                    .insert(cache_key.clone());

                let mut resolved_block = block_type.clone();
                self.convert_patterns_to_pattern_properties(&mut resolved_block);
                let resolved_cwt_type = CwtType::Block(resolved_block);
                let result =
                    ScopedType::new_cwt(resolved_cwt_type, scoped_type.scope_stack().clone());

                // Remove from resolving set
                {
                    let mut cache = self.cache.write().unwrap();
                    cache.resolving.remove(&cache_key);
                }

                result
            }
            // For all other types, return as-is with same scope
            _ => scoped_type.clone(),
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

    pub fn navigate_to_property(
        &self,
        scoped_type: &ScopedType,
        property_name: &str,
    ) -> PropertyNavigationResult {
        let resolved_type = self.resolve_type(scoped_type);

        // First, check if this property is a scope property (from, fromfrom, etc.)
        if let Some(scope_context) = resolved_type.scope_stack().get_scope_by_name(property_name) {
            // This is a scope property - push that scope onto the current stack
            let mut new_scope_context = resolved_type.scope_stack().clone();
            new_scope_context.push_scope(scope_context.clone()).unwrap();
            let result = ScopedType::new(resolved_type.cwt_type().clone(), new_scope_context);
            return PropertyNavigationResult::Success(result);
        }

        // Second, check if this property is a link property
        let current_scope = &resolved_type.scope_stack().current_scope().scope_type;

        if let Some(link_def) = self.is_link_property(property_name, current_scope) {
            // This is a link property - create a scoped type with the output scope
            let mut new_scope_context = resolved_type.scope_stack().clone();
            new_scope_context
                .push_scope_type(&link_def.output_scope)
                .unwrap();
            let result = ScopedType::new(resolved_type.cwt_type().clone(), new_scope_context);
            return PropertyNavigationResult::Success(result);
        }

        // If not a link property, handle as regular property
        match resolved_type.cwt_type() {
            CwtTypeOrSpecial::CwtType(CwtType::Block(block)) => {
                // Check regular properties first
                if let Some(property) = block.properties.get(property_name) {
                    // Check if this property changes scope
                    if property.changes_scope() {
                        match property
                            .apply_scope_changes(resolved_type.scope_stack(), &self.cwt_analyzer)
                        {
                            Ok(new_scope) => {
                                let property_scoped =
                                    ScopedType::new_cwt(property.property_type.clone(), new_scope);
                                let resolved_property = self.resolve_type(&property_scoped);
                                PropertyNavigationResult::Success(resolved_property)
                            }
                            Err(error) => PropertyNavigationResult::ScopeError(error),
                        }
                    } else {
                        // Same scope context
                        let property_scoped = ScopedType::new_cwt(
                            property.property_type.clone(),
                            resolved_type.scope_stack().clone(),
                        );
                        let resolved_property = self.resolve_type(&property_scoped);
                        PropertyNavigationResult::Success(resolved_property)
                    }
                } else {
                    // Check pattern properties
                    if let Some(pattern_property) = self.key_matches_pattern(property_name, block) {
                        // Check if the pattern property's value type is an AliasMatchLeft that needs resolution
                        let resolved_value_type = match &pattern_property.value_type {
                            CwtType::Reference(ReferenceType::AliasMatchLeft { key }) => {
                                // Resolve the AliasMatchLeft using the property name
                                self.resolve_alias_match_left(key, property_name)
                            }
                            _ => pattern_property.value_type.clone(),
                        };

                        if pattern_property.changes_scope() {
                            match pattern_property.apply_scope_changes(
                                resolved_type.scope_stack(),
                                &self.cwt_analyzer,
                            ) {
                                Ok(new_scope) => {
                                    let property_scoped =
                                        ScopedType::new_cwt(resolved_value_type, new_scope);
                                    let resolved_property = self.resolve_type(&property_scoped);
                                    PropertyNavigationResult::Success(resolved_property)
                                }
                                Err(error) => PropertyNavigationResult::ScopeError(error),
                            }
                        } else {
                            // Same scope context
                            let property_scoped = ScopedType::new_cwt(
                                resolved_value_type,
                                resolved_type.scope_stack().clone(),
                            );
                            let resolved_property = self.resolve_type(&property_scoped);
                            PropertyNavigationResult::Success(resolved_property)
                        }
                    } else {
                        PropertyNavigationResult::NotFound
                    }
                }
            }
            CwtTypeOrSpecial::CwtType(CwtType::Reference(ReferenceType::AliasMatchLeft {
                key,
            })) => {
                // For alias_match_left[category], we need to look up the specific alias
                // category:property_name and return its type
                let resolved_cwt_type = self.resolve_alias_match_left(key, property_name);

                // Check if we found a matching alias
                if matches!(
                    resolved_cwt_type,
                    CwtType::Reference(ReferenceType::AliasMatchLeft { .. })
                ) {
                    // No matching alias was found
                    PropertyNavigationResult::NotFound
                } else {
                    // We found a matching alias - return the resolved type
                    let property_scoped =
                        ScopedType::new_cwt(resolved_cwt_type, resolved_type.scope_stack().clone());
                    let resolved_property = self.resolve_type(&property_scoped);
                    PropertyNavigationResult::Success(resolved_property)
                }
            }
            _ => PropertyNavigationResult::NotFound,
        }
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
                if let Some(aliases_in_category) =
                    self.cwt_analyzer.get_aliases_for_category(category)
                {
                    for alias_pattern in aliases_in_category {
                        match &alias_pattern.name {
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
                                                game_data.get_namespace_entity_keys(&path)
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
                if let Some(aliases_in_category) =
                    self.cwt_analyzer.get_aliases_for_category(category)
                {
                    for alias_pattern in aliases_in_category {
                        match &alias_pattern.name {
                            AliasName::Static(name) => {
                                completions.push(name.clone());
                            }
                            AliasName::TypeRef(type_name) => {
                                if let Some(type_def) = self.cwt_analyzer.get_type(type_name) {
                                    if let Some(path) = type_def.path.as_ref() {
                                        let path = path.trim_start_matches("game/");
                                        if let Some(game_data) = GameDataCache::get() {
                                            if let Some(namespace_keys) =
                                                game_data.get_namespace_entity_keys(&path)
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
                            if let Some(namespace_keys) = game_data.get_namespace_entity_keys(&path)
                            {
                                return CwtType::LiteralSet(
                                    namespace_keys.iter().cloned().collect(),
                                );
                            }

                            // Also try the key directly in case it's already a full path
                            if let Some(namespace_keys) = game_data.get_namespace_entity_keys(key) {
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
            ReferenceType::AliasMatchLeft { .. } => {
                // alias_match_left[category] cannot be resolved statically because it depends
                // on the key being passed in at runtime. This is like TypeScript's T[P] where
                // we need to know P to resolve the type.
                // The resolution must happen during property navigation.
                CwtType::Reference(ref_type.clone())
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
            ReferenceType::ValueSet { .. } => CwtType::Reference(ref_type.clone()),
            ReferenceType::Value { key } => {
                if let Some(full_analysis) = FullAnalysis::get() {
                    if let Some(dynamic_values) = full_analysis.dynamic_value_sets.get(key) {
                        return CwtType::LiteralSet(dynamic_values.clone());
                    }
                }

                CwtType::Reference(ref_type.clone())
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
                    resolved_type.clone()
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

    /// Resolve an AliasMatchLeft reference using a specific property name
    fn resolve_alias_match_left(&self, category: &str, property_name: &str) -> CwtType {
        // Look up the specific alias category:property_name and return its type
        if let Some(aliases_in_category) = self.cwt_analyzer.get_aliases_for_category(category) {
            for alias_pattern in aliases_in_category {
                if let Some(alias_def) = self.cwt_analyzer.get_alias(alias_pattern) {
                    match &alias_pattern.name {
                        AliasName::Static(name) => {
                            if name == property_name {
                                return alias_def.to.clone();
                            }
                        }
                        AliasName::TypeRef(type_name) => {
                            // Check if property_name is a valid key for this type
                            if let Some(type_def) = self.cwt_analyzer.get_type(type_name) {
                                if let Some(path) = type_def.path.as_ref() {
                                    let path = path.trim_start_matches("game/");
                                    if let Some(game_data) = GameDataCache::get() {
                                        if let Some(namespace_keys) =
                                            game_data.get_namespace_entity_keys(&path)
                                        {
                                            if namespace_keys.contains(&property_name.to_string()) {
                                                return alias_def.to.clone();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        AliasName::Enum(enum_name) => {
                            // Check if property_name is a valid enum value
                            if let Some(enum_def) = self.cwt_analyzer.get_enum(enum_name) {
                                if enum_def.values.contains(property_name) {
                                    return alias_def.to.clone();
                                }
                            }
                        }
                    }
                }
            }
        }

        // If no matching alias was found, return the original AliasMatchLeft
        CwtType::Reference(ReferenceType::AliasMatchLeft {
            key: category.to_string(),
        })
    }

    /// Get all available link properties for the current scope
    pub fn get_scope_link_properties(&self, scope: &str) -> Vec<String> {
        let mut link_properties = Vec::new();

        for (link_name, link_def) in self.cwt_analyzer.get_links() {
            if link_def.can_be_used_from(scope, &self.cwt_analyzer) {
                link_properties.push(link_name.clone());
            }
        }

        link_properties
    }

    /// Check if a property name is a link property for the current scope
    pub fn is_link_property(&self, property_name: &str, scope: &str) -> Option<&LinkDefinition> {
        if let Some(link_def) = self.cwt_analyzer.get_link(property_name) {
            if link_def.can_be_used_from(scope, &self.cwt_analyzer) {
                return Some(link_def);
            }
        }
        None
    }

    /// Get all available property names for a scoped type
    pub fn get_available_properties(&self, scoped_type: &ScopedType) -> Vec<String> {
        let resolved_type = self.resolve_type(scoped_type);
        let mut properties = Vec::new();

        match resolved_type.cwt_type() {
            CwtTypeOrSpecial::CwtType(CwtType::Block(block)) => {
                // Add regular properties
                properties.extend(block.properties.keys().cloned());

                // Add pattern properties (get completions)
                for pattern_property in &block.pattern_properties {
                    let completions = self.get_pattern_completions(&pattern_property.pattern_type);
                    properties.extend(completions);
                }
            }
            CwtTypeOrSpecial::CwtType(CwtType::Reference(ReferenceType::AliasMatchLeft {
                key,
            })) => {
                // For alias_match_left[category], return all possible alias names from that category
                if let Some(aliases_in_category) = self.cwt_analyzer.get_aliases_for_category(key) {
                    for alias_pattern in aliases_in_category {
                        match &alias_pattern.name {
                            AliasName::Static(name) => {
                                properties.push(name.clone());
                            }
                            AliasName::TypeRef(type_name) => {
                                if let Some(type_def) = self.cwt_analyzer.get_type(type_name) {
                                    if let Some(path) = type_def.path.as_ref() {
                                        let path = path.trim_start_matches("game/");
                                        if let Some(game_data) = GameDataCache::get() {
                                            if let Some(namespace_keys) =
                                                game_data.get_namespace_entity_keys(&path)
                                            {
                                                properties.extend(namespace_keys.iter().cloned());
                                            }
                                        }
                                    }
                                }
                            }
                            AliasName::Enum(enum_name) => {
                                if let Some(enum_def) = self.cwt_analyzer.get_enum(enum_name) {
                                    properties.extend(enum_def.values.iter().cloned());
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Add scope properties (from, fromfrom, etc.) based on the current scope stack
        let scope_properties = scoped_type.scope_stack().available_scope_names();
        properties.extend(scope_properties);

        // Add link properties based on the current scope
        let current_scope = &scoped_type.scope_stack().current_scope().scope_type;
        let link_properties = self.get_scope_link_properties(current_scope);
        properties.extend(link_properties);

        properties.sort();
        properties.dedup();
        properties
    }
}
