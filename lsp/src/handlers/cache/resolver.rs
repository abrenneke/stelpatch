use crate::handlers::cache::{EntityRestructurer, FullAnalysis, GameDataCache};
use crate::handlers::scope::{ScopeError, ScopeStack};
use crate::handlers::scoped_type::{
    CwtTypeOrSpecial, PropertyNavigationResult, ScopeAwareProperty, ScopedType,
};
use cw_model::types::{CwtAnalyzer, LinkDefinition, PatternProperty, PatternType};
use cw_model::{AliasDefinition, AliasName, BlockType, CwtType, ReferenceType};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::RwLock;

pub struct TypeResolver {
    cwt_analyzer: Arc<CwtAnalyzer>,
    cache: Arc<RwLock<TypeResolverCache>>,
}

pub struct TypeResolverCache {
    resolved_references: HashMap<String, Arc<CwtType>>,
    namespace_keys: HashMap<String, Option<Arc<HashSet<String>>>>,
    alias_match_left: HashMap<String, (CwtType, Option<AliasDefinition>)>,
    pattern_type_matches: HashMap<String, bool>,
}

impl TypeResolver {
    pub fn new(cwt_analyzer: Arc<CwtAnalyzer>) -> Self {
        Self {
            cwt_analyzer,
            cache: Arc::new(RwLock::new(TypeResolverCache {
                resolved_references: HashMap::new(),
                namespace_keys: HashMap::new(),
                alias_match_left: HashMap::new(),
                pattern_type_matches: HashMap::new(),
            })),
        }
    }

    /// Get namespace keys for a TypeRef alias name
    fn get_namespace_keys_for_type_ref(&self, type_name: &str) -> Option<Arc<HashSet<String>>> {
        if let Some(cached_result) = self.cache.read().unwrap().namespace_keys.get(type_name) {
            return cached_result.clone();
        }

        if let Some(type_def) = self.cwt_analyzer.get_type(type_name) {
            if let Some(path) = type_def.path.as_ref() {
                let path = path.trim_start_matches("game/");
                if let Some(namespace_keys) =
                    EntityRestructurer::get_namespace_entity_keys_set(&path)
                {
                    let result = Some(namespace_keys);
                    self.cache
                        .write()
                        .unwrap()
                        .namespace_keys
                        .insert(type_name.to_string(), result.clone());
                    return result;
                }
            }
        }
        let result = None;
        self.cache
            .write()
            .unwrap()
            .namespace_keys
            .insert(type_name.to_string(), result.clone());
        result
    }

    /// Resolves references & nested types to concrete types
    pub fn resolve_type(&self, scoped_type: Arc<ScopedType>) -> Arc<ScopedType> {
        let cwt_type = scoped_type.cwt_type();

        match cwt_type {
            // For references, try to resolve to the actual type
            CwtTypeOrSpecial::CwtType(CwtType::Reference(ref_type)) => {
                let resolved_cwt_type =
                    self.resolve_reference_type(ref_type, scoped_type.scope_stack());
                let result = ScopedType::new_cwt(
                    (*resolved_cwt_type).clone(),
                    scoped_type.scope_stack().clone(),
                );

                Arc::new(result)
            }
            // For comparables, unwrap to the base type
            CwtTypeOrSpecial::CwtType(CwtType::Comparable(base_type)) => {
                let base_scoped =
                    ScopedType::new_cwt((**base_type).clone(), scoped_type.scope_stack().clone());
                self.resolve_type(Arc::new(base_scoped))
            }
            // For all other types, return as-is with same scope
            _ => scoped_type,
        }
    }

    /// Check if a property name is a valid scope property or link property
    /// Returns Some(description) if valid, None if invalid
    pub fn is_valid_scope_or_link_property(
        &self,
        property_name: &str,
        scope_stack: &ScopeStack,
    ) -> Option<String> {
        // First, check if this property is a scope property (from, fromfrom, etc.)
        if let Some(scope_context) = scope_stack.get_scope_by_name(property_name) {
            return Some(format!("scope property ({})", scope_context.scope_type));
        }

        // Second, check if this property is a link property
        let current_scope = &scope_stack.current_scope().scope_type;
        if let Some(link_def) = self.is_link_property(property_name, current_scope) {
            return Some(format!("link property ({})", link_def.output_scope));
        }

        None
    }

    /// Get all available scope properties and link properties for the current scope
    pub fn get_available_scope_and_link_properties(&self, scope_stack: &ScopeStack) -> Vec<String> {
        let mut properties = Vec::new();

        // Add scope properties (from, fromfrom, etc.) based on the current scope stack
        let scope_properties = scope_stack.available_scope_names();
        properties.extend(scope_properties);

        // Add link properties based on the current scope
        let current_scope = &scope_stack.current_scope().scope_type;
        let link_properties = self.get_scope_link_properties(current_scope);
        properties.extend(link_properties);

        properties.sort();
        properties.dedup();
        properties
    }

    pub fn navigate_to_property(
        &self,
        scoped_type: Arc<ScopedType>,
        property_name: &str,
    ) -> PropertyNavigationResult {
        let resolved_type = self.resolve_type(scoped_type);

        // First, check if this property is a scope property (from, fromfrom, etc.)
        if let Some(scope_context) = resolved_type.scope_stack().get_scope_by_name(property_name) {
            // This is a scope property - push that scope onto the current stack
            let mut new_scope_context = resolved_type.scope_stack().clone();
            new_scope_context.push_scope(scope_context.clone()).unwrap();
            let result = ScopedType::new(resolved_type.cwt_type().clone(), new_scope_context);
            return PropertyNavigationResult::Success(Arc::new(result));
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
            return PropertyNavigationResult::Success(Arc::new(result));
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
                                let resolved_property =
                                    self.resolve_type(Arc::new(property_scoped));
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
                        let resolved_property = self.resolve_type(Arc::new(property_scoped));
                        PropertyNavigationResult::Success(resolved_property)
                    }
                } else {
                    // Check pattern properties
                    if let Some(pattern_property) = self.key_matches_pattern(property_name, block) {
                        // Check if the pattern property's value type is an AliasMatchLeft that needs resolution
                        let (resolved_value_type, alias_def) = match &pattern_property.value_type {
                            CwtType::Reference(ReferenceType::AliasMatchLeft { key }) => {
                                // Resolve the AliasMatchLeft using the property name
                                self.resolve_alias_match_left(key, property_name)
                            }
                            _ => (pattern_property.value_type.clone(), None),
                        };

                        // Apply scope changes - first from alias definition, then from pattern property
                        let mut current_scope = resolved_type.scope_stack().clone();

                        // Apply alias scope changes if present
                        if let Some(alias_def) = alias_def {
                            if alias_def.changes_scope() {
                                match self.apply_alias_scope_changes(&current_scope, &alias_def) {
                                    Ok(new_scope) => current_scope = new_scope,
                                    Err(error) => {
                                        return PropertyNavigationResult::ScopeError(error);
                                    }
                                }
                            }
                        }

                        // Apply pattern property scope changes if present
                        if pattern_property.changes_scope() {
                            match pattern_property
                                .apply_scope_changes(&current_scope, &self.cwt_analyzer)
                            {
                                Ok(new_scope) => current_scope = new_scope,
                                Err(error) => return PropertyNavigationResult::ScopeError(error),
                            }
                        }

                        let property_scoped =
                            ScopedType::new_cwt(resolved_value_type, current_scope);
                        let resolved_property = self.resolve_type(Arc::new(property_scoped));
                        PropertyNavigationResult::Success(resolved_property)
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
                let (resolved_cwt_type, alias_def) =
                    self.resolve_alias_match_left(key, property_name);

                // Check if we found a matching alias
                if matches!(
                    resolved_cwt_type,
                    CwtType::Reference(ReferenceType::AliasMatchLeft { .. })
                ) {
                    // No matching alias was found
                    PropertyNavigationResult::NotFound
                } else {
                    // We found a matching alias - check if it has scope changes
                    if let Some(alias_def) = alias_def {
                        if alias_def.changes_scope() {
                            match self
                                .apply_alias_scope_changes(resolved_type.scope_stack(), &alias_def)
                            {
                                Ok(new_scope) => {
                                    let property_scoped =
                                        ScopedType::new_cwt(resolved_cwt_type, new_scope);
                                    let resolved_property =
                                        self.resolve_type(Arc::new(property_scoped));
                                    PropertyNavigationResult::Success(resolved_property)
                                }
                                Err(error) => PropertyNavigationResult::ScopeError(error),
                            }
                        } else {
                            // No scope changes - use current scope
                            let property_scoped = ScopedType::new_cwt(
                                resolved_cwt_type,
                                resolved_type.scope_stack().clone(),
                            );
                            let resolved_property = self.resolve_type(Arc::new(property_scoped));
                            PropertyNavigationResult::Success(resolved_property)
                        }
                    } else {
                        // No alias definition found - use current scope
                        let property_scoped = ScopedType::new_cwt(
                            resolved_cwt_type,
                            resolved_type.scope_stack().clone(),
                        );
                        let resolved_property = self.resolve_type(Arc::new(property_scoped));
                        PropertyNavigationResult::Success(resolved_property)
                    }
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
        let composite_key = format!("{}:{}", key, pattern_type.id());

        if let Some(cached_result) = self
            .cache
            .read()
            .unwrap()
            .pattern_type_matches
            .get(&composite_key)
        {
            return *cached_result;
        }

        let result = match pattern_type {
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
                                if let Some(namespace_keys) =
                                    self.get_namespace_keys_for_type_ref(type_name)
                                {
                                    if namespace_keys.contains(&key.to_string()) {
                                        return true;
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
        };

        self.cache
            .write()
            .unwrap()
            .pattern_type_matches
            .insert(composite_key, result);
        result
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
                                if let Some(namespace_keys) =
                                    self.get_namespace_keys_for_type_ref(type_name)
                                {
                                    completions.extend(namespace_keys.iter().cloned());
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

    fn resolve_reference_type(
        &self,
        ref_type: &ReferenceType,
        scope_stack: &ScopeStack,
    ) -> Arc<CwtType> {
        let cache_key = ref_type.id();

        // Check if we already have this reference type cached
        if let Some(cached_result) = self
            .cache
            .read()
            .unwrap()
            .resolved_references
            .get(&cache_key)
        {
            match cached_result.as_ref() {
                // Don't cache unresolved references
                CwtType::Reference(_) => {}
                _ => return cached_result.clone(),
            }
        }

        let result = match ref_type {
            ReferenceType::Type { key } => {
                let type_def = self.cwt_analyzer.get_type(key);

                let mut found = None;

                if let Some(type_def) = type_def {
                    if let Some(path) = type_def.path.as_ref() {
                        // CWT paths are prefixed with "game/"
                        let path = path.trim_start_matches("game/");

                        // For Type references, we want the union of all keys in that namespace
                        // This is what the user expects when they hover over "resource" - they want to see
                        // all the possible resource keys like "energy", "minerals", etc.
                        if let Some(namespace_keys) =
                            crate::handlers::cache::EntityRestructurer::get_namespace_entity_keys(
                                &path,
                            )
                        {
                            found = Some(CwtType::LiteralSet(namespace_keys.into_iter().collect()));
                        }

                        // Also try the key directly in case it's already a full path
                        if found.is_none() {
                            if let Some(namespace_keys) = crate::handlers::cache::EntityRestructurer::get_namespace_entity_keys(key) {
                                found = Some(CwtType::LiteralSet(
                                    namespace_keys.into_iter().collect(),
                                ));
                            }
                        }
                    }
                }

                if let Some(found) = found {
                    let result = Arc::new(found);
                    self.cache
                        .write()
                        .unwrap()
                        .resolved_references
                        .insert(cache_key.clone(), result.clone());
                    return result;
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
                        let result = CwtType::LiteralSet(dynamic_values.clone());
                        let result = Arc::new(result);
                        self.cache
                            .write()
                            .unwrap()
                            .resolved_references
                            .insert(cache_key.clone(), result.clone());
                        return result;
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
            ReferenceType::Scope { key } => {
                // If "any", then _any_ link or scope property is valid from the current scope
                if key == "any" {
                    let current_scope = &scope_stack.current_scope().scope_type;
                    let mut properties = self.get_scope_link_properties(current_scope);
                    properties.extend(scope_stack.available_scope_names());
                    CwtType::LiteralSet(properties.into_iter().collect())
                } else {
                    // Otherwise, it's link properties that resolve to the specified scope type
                    // plus scope properties that actually resolve to the specified scope type
                    let mut properties = Vec::new();
                    let current_scope = &scope_stack.current_scope().scope_type;

                    // Resolve the scope name to get the canonical name (e.g., "country" -> "Country")
                    if let Some(resolved_scope_name) = self.cwt_analyzer.resolve_scope_name(key) {
                        // Add link properties that are valid from the current scope and have the specified output scope
                        for (link_name, link_def) in self.cwt_analyzer.get_links() {
                            if link_def.can_be_used_from(current_scope, &self.cwt_analyzer) {
                                if let Some(link_output_scope) =
                                    self.cwt_analyzer.resolve_scope_name(&link_def.output_scope)
                                {
                                    if link_output_scope == resolved_scope_name {
                                        properties.push(link_name.clone());
                                    }
                                }
                            }
                        }

                        // Add scope properties that resolve to the specified scope type
                        for scope_property in scope_stack.available_scope_names() {
                            if let Some(scope_context) =
                                scope_stack.get_scope_by_name(&scope_property)
                            {
                                if let Some(scope_type) = self
                                    .cwt_analyzer
                                    .resolve_scope_name(&scope_context.scope_type)
                                {
                                    if scope_type == resolved_scope_name {
                                        properties.push(scope_property);
                                    }
                                }
                            }
                        }
                    }

                    CwtType::LiteralSet(properties.into_iter().collect())
                }
            }
            // For any remaining unhandled reference types, return the original
            _ => CwtType::Reference(ref_type.clone()),
        };

        let result = Arc::new(result);
        self.cache
            .write()
            .unwrap()
            .resolved_references
            .insert(cache_key, result.clone());
        result
    }

    /// Resolve an AliasMatchLeft reference using a specific property name
    /// Returns (resolved_type, alias_definition_if_found)
    fn resolve_alias_match_left(
        &self,
        category: &str,
        property_name: &str,
    ) -> (CwtType, Option<AliasDefinition>) {
        let composite_key = format!("{}:{}", category, property_name);

        // Check if we already have this alias match left cached
        if let Some(cached_result) = self
            .cache
            .read()
            .unwrap()
            .alias_match_left
            .get(&composite_key)
        {
            return cached_result.clone();
        }

        // Look up the specific alias category:property_name and return its type
        if let Some(aliases_in_category) = self.cwt_analyzer.get_aliases_for_category(category) {
            for alias_pattern in aliases_in_category {
                if let Some(alias_def) = self.cwt_analyzer.get_alias(alias_pattern) {
                    match &alias_pattern.name {
                        AliasName::Static(name) => {
                            if name == property_name {
                                let result = alias_def.to.clone();
                                self.cache.write().unwrap().alias_match_left.insert(
                                    composite_key.clone(),
                                    (result.clone(), Some(alias_def.clone())),
                                );
                                return (result, Some(alias_def.clone()));
                            }
                        }
                        AliasName::TypeRef(type_name) => {
                            // Check if property_name is a valid key for this type
                            if let Some(namespace_keys) =
                                self.get_namespace_keys_for_type_ref(type_name)
                            {
                                if namespace_keys.contains(&property_name.to_string()) {
                                    let result = alias_def.to.clone();
                                    self.cache.write().unwrap().alias_match_left.insert(
                                        composite_key.clone(),
                                        (result.clone(), Some(alias_def.clone())),
                                    );
                                    return (result, Some(alias_def.clone()));
                                }
                            }
                        }
                        AliasName::Enum(enum_name) => {
                            // Check if property_name is a valid enum value
                            if let Some(enum_def) = self.cwt_analyzer.get_enum(enum_name) {
                                if enum_def.values.contains(property_name) {
                                    let result = alias_def.to.clone();
                                    self.cache.write().unwrap().alias_match_left.insert(
                                        composite_key.clone(),
                                        (result.clone(), Some(alias_def.clone())),
                                    );
                                    return (result, Some(alias_def.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }

        // If no matching alias was found, return the original AliasMatchLeft
        let result = CwtType::Reference(ReferenceType::AliasMatchLeft {
            key: category.to_string(),
        });
        self.cache
            .write()
            .unwrap()
            .alias_match_left
            .insert(composite_key.clone(), (result.clone(), None));
        (result, None)
    }

    /// Apply scope changes from alias definition options
    fn apply_alias_scope_changes(
        &self,
        scope_stack: &ScopeStack,
        alias_def: &cw_model::types::AliasDefinition,
    ) -> Result<ScopeStack, ScopeError> {
        let mut new_scope = scope_stack.branch();

        // Apply push_scope if present
        if let Some(push_scope) = &alias_def.options.push_scope {
            if let Some(scope_name) = self.cwt_analyzer.resolve_scope_name(push_scope) {
                new_scope.push_scope_type(scope_name)?;
            }
        }

        // Apply replace_scope if present
        if let Some(replace_scope) = &alias_def.options.replace_scope {
            let mut new_scopes = HashMap::new();

            for (key, value) in replace_scope {
                if let Some(scope_name) = self.cwt_analyzer.resolve_scope_name(value) {
                    new_scopes.insert(key.clone(), scope_name.to_string());
                }
            }

            new_scope.replace_scope_from_strings(new_scopes)?;
        }

        Ok(new_scope)
    }

    pub fn get_all_scope_properties(&self) -> Vec<String> {
        ScopeStack::get_all_scope_properties()
    }

    pub fn get_all_link_properties(&self) -> Vec<String> {
        self.cwt_analyzer.get_links().keys().cloned().collect()
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
    pub fn get_available_properties(&self, scoped_type: Arc<ScopedType>) -> Vec<String> {
        let resolved_type = self.resolve_type(scoped_type.clone());
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
                                if let Some(namespace_keys) =
                                    self.get_namespace_keys_for_type_ref(type_name)
                                {
                                    properties.extend(namespace_keys.iter().cloned());
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
