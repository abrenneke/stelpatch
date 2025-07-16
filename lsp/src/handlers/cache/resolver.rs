use crate::handlers::scope::ScopeStack;
use crate::handlers::scoped_type::{CwtTypeOrSpecial, PropertyNavigationResult, ScopedType};
use cw_model::types::{CwtAnalyzer, LinkDefinition, PatternProperty, PatternType};
use cw_model::{CwtType, ReferenceType};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::RwLock;

use crate::handlers::cache::resolver_modules::{
    PatternMatcher, PropertyNavigator, ReferenceResolver, ResolverUtils, ScopeHandler,
    SubtypeHandler, TypeResolverCache,
};

pub struct TypeResolver {
    cwt_analyzer: Arc<CwtAnalyzer>,
    cache: Arc<RwLock<TypeResolverCache>>,
    reference_resolver: ReferenceResolver,
    pattern_matcher: PatternMatcher,
    property_navigator: PropertyNavigator,
    scope_handler: ScopeHandler,
    subtype_handler: SubtypeHandler,
}

impl TypeResolver {
    pub fn new(cwt_analyzer: Arc<CwtAnalyzer>) -> Self {
        let utils = Arc::new(ResolverUtils::new(cwt_analyzer.clone()));
        let cache = Arc::new(RwLock::new(TypeResolverCache::new()));

        let subtype_handler_for_reference = Arc::new(SubtypeHandler::new(cwt_analyzer.clone()));

        Self {
            reference_resolver: ReferenceResolver::new(
                cwt_analyzer.clone(),
                utils.clone(),
                subtype_handler_for_reference,
            ),
            pattern_matcher: PatternMatcher::new(cwt_analyzer.clone(), utils.clone()),
            property_navigator: PropertyNavigator::new(cwt_analyzer.clone(), utils.clone()),
            scope_handler: ScopeHandler::new(cwt_analyzer.clone()),
            subtype_handler: SubtypeHandler::new(cwt_analyzer.clone()),
            cwt_analyzer,
            cache,
        }
    }

    /// Resolves references & nested types to concrete types
    pub fn resolve_type(&self, scoped_type: Arc<ScopedType>) -> Arc<ScopedType> {
        let cwt_type = scoped_type.cwt_type();

        match cwt_type {
            // For references, try to resolve to the actual type
            CwtTypeOrSpecial::CwtType(CwtType::Reference(ref_type)) => {
                let resolved_cwt_type = self.resolve_reference_type(
                    ref_type,
                    scoped_type.scope_stack(),
                    scoped_type.in_scripted_effect_block().cloned(),
                );
                let result = ScopedType::new_cwt_with_subtypes(
                    (*resolved_cwt_type).clone(),
                    scoped_type.scope_stack().clone(),
                    scoped_type.subtypes().clone(),
                    scoped_type.in_scripted_effect_block().cloned(),
                );

                Arc::new(result)
            }
            // For comparables, unwrap to the base type
            CwtTypeOrSpecial::CwtType(CwtType::Comparable(base_type)) => {
                let base_scoped = ScopedType::new_cwt_with_subtypes(
                    (**base_type).clone(),
                    scoped_type.scope_stack().clone(),
                    scoped_type.subtypes().clone(),
                    scoped_type.in_scripted_effect_block().cloned(),
                );
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
        self.scope_handler
            .is_valid_scope_or_link_property(property_name, scope_stack)
    }

    /// Get all available scope properties and link properties for the current scope
    pub fn get_available_scope_and_link_properties(&self, scope_stack: &ScopeStack) -> Vec<String> {
        self.scope_handler
            .get_available_scope_and_link_properties(scope_stack)
    }

    pub fn navigate_to_property(
        &self,
        scoped_type: Arc<ScopedType>,
        property_name: &str,
    ) -> PropertyNavigationResult {
        let resolved_type = self.resolve_type(scoped_type);

        self.property_navigator
            .navigate_to_property(resolved_type, property_name)
    }

    /// Check if a key matches any pattern property in a block
    pub fn key_matches_pattern<'a>(
        &self,
        key: &str,
        block_type: &'a cw_model::types::BlockType,
    ) -> Option<&'a PatternProperty> {
        self.pattern_matcher.key_matches_pattern(key, block_type)
    }

    /// Check if a key matches a specific pattern type
    pub fn key_matches_pattern_type(&self, key: &str, pattern_type: &PatternType) -> bool {
        self.pattern_matcher
            .key_matches_pattern_type(key, pattern_type)
    }

    /// Get all possible completions for a pattern type
    pub fn get_pattern_completions(&self, pattern_type: &PatternType) -> Vec<String> {
        self.pattern_matcher.get_pattern_completions(pattern_type)
    }

    fn resolve_reference_type(
        &self,
        ref_type: &ReferenceType,
        scope_stack: &ScopeStack,
        in_scripted_effect_block: Option<String>,
    ) -> Arc<CwtType> {
        let cache_key = format!(
            "{}-{}-{}",
            ref_type.id(),
            scope_stack.to_string(),
            in_scripted_effect_block.clone().unwrap_or_default()
        );

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

        let result = self
            .reference_resolver
            .resolve_reference_type(ref_type, scope_stack);
        self.cache
            .write()
            .unwrap()
            .resolved_references
            .insert(cache_key, result.clone());
        result
    }

    pub fn get_all_scope_properties(&self) -> Vec<String> {
        self.scope_handler.get_all_scope_properties()
    }

    pub fn get_all_link_properties(&self) -> Vec<String> {
        self.scope_handler.get_all_link_properties()
    }

    /// Get all available link properties for the current scope
    pub fn get_scope_link_properties(&self, scope: &str) -> Vec<String> {
        self.scope_handler.get_scope_link_properties(scope)
    }

    /// Check if a property name is a link property for the current scope
    pub fn is_link_property(&self, property_name: &str, scope: &str) -> Option<&LinkDefinition> {
        self.scope_handler.is_link_property(property_name, scope)
    }

    /// Get all available property names for a scoped type
    pub fn get_available_properties(&self, scoped_type: Arc<ScopedType>) -> Vec<String> {
        self.property_navigator
            .get_available_properties(scoped_type)
    }

    /// Get all available subtypes for a given type
    pub fn get_available_subtypes(&self, cwt_type: &CwtType) -> Vec<String> {
        self.subtype_handler.get_available_subtypes(cwt_type)
    }

    /// Check if a type has a specific subtype
    pub fn has_subtype(&self, cwt_type: &CwtType, subtype_name: &str) -> bool {
        self.subtype_handler.has_subtype(cwt_type, subtype_name)
    }

    /// Get subtype definition for a given type and subtype name
    pub fn get_subtype_definition<'a>(
        &self,
        cwt_type: &'a CwtType,
        subtype_name: &str,
    ) -> Option<&'a cw_model::types::Subtype> {
        self.subtype_handler
            .get_subtype_definition(cwt_type, subtype_name)
    }

    /// Check if a subtype condition would be satisfied for a specific property key
    pub fn would_subtype_condition_match_for_key(
        &self,
        condition: &cw_model::types::SubtypeCondition,
        property_data: &HashMap<String, String>,
        accessing_key: &str,
    ) -> bool {
        self.subtype_handler.would_subtype_condition_match_for_key(
            condition,
            property_data,
            accessing_key,
        )
    }

    /// Determine all matching subtypes based on property data
    pub fn determine_matching_subtypes(
        &self,
        scoped_type: Arc<ScopedType>,
        property_data: &HashMap<String, String>,
    ) -> HashSet<String> {
        self.subtype_handler
            .determine_matching_subtypes(scoped_type, property_data)
    }

    /// Create a new scoped type with a specific subtype
    pub fn create_scoped_type_with_subtype(
        &self,
        cwt_type: CwtType,
        scope_stack: ScopeStack,
        subtype_name: Option<String>,
        scripted_effect_name: Option<String>,
    ) -> Arc<ScopedType> {
        Arc::new(ScopedType::new_cwt_with_subtype(
            cwt_type,
            scope_stack,
            subtype_name,
            scripted_effect_name,
        ))
    }

    /// Get all enum definitions from the CWT analyzer
    pub fn get_enums(&self) -> &HashMap<String, cw_model::types::EnumDefinition> {
        self.cwt_analyzer.get_enums()
    }
}
