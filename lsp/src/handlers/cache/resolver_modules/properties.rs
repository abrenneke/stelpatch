use crate::handlers::scope::{ScopeError, ScopeStack};
use crate::handlers::scoped_type::{
    CwtTypeOrSpecial, PropertyNavigationResult, ScopeAwareProperty, ScopedType,
};
use cw_model::{
    AliasDefinition, AliasName, BlockType, CwtAnalyzer, CwtType, LinkDefinition, PatternProperty,
    Property, ReferenceType,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::ResolverUtils;
use super::patterns::PatternMatcher;
use super::references::ReferenceResolver;
use super::subtypes::SubtypeHandler;

pub struct PropertyNavigator {
    pub cwt_analyzer: Arc<CwtAnalyzer>,
    pub utils: Arc<ResolverUtils>,
    pub reference_resolver: Arc<ReferenceResolver>,
    pub pattern_matcher: Arc<PatternMatcher>,
}

impl PropertyNavigator {
    pub fn new(cwt_analyzer: Arc<CwtAnalyzer>, utils: Arc<ResolverUtils>) -> Self {
        Self {
            reference_resolver: Arc::new(ReferenceResolver::new(
                cwt_analyzer.clone(),
                utils.clone(),
                Arc::new(SubtypeHandler::new(cwt_analyzer.clone())),
            )),
            pattern_matcher: Arc::new(PatternMatcher::new(cwt_analyzer.clone(), utils.clone())),
            utils,
            cwt_analyzer,
        }
    }

    /// Navigate to a property from a given scoped type
    pub fn navigate_to_property(
        &self,
        scoped_type: Arc<ScopedType>,
        property_name: &str,
    ) -> PropertyNavigationResult {
        // First, check if this property is a scope property (from, fromfrom, etc.)
        if let Some(scope_context) = scoped_type.scope_stack().get_scope_by_name(property_name) {
            // This is a scope property - push that scope onto the current stack
            let mut new_scope_context = scoped_type.scope_stack().clone();
            new_scope_context.push_scope(scope_context.clone()).unwrap();
            let result = ScopedType::new_with_subtypes(
                scoped_type.cwt_type().clone(),
                new_scope_context,
                scoped_type.subtypes().clone(),
            );
            return PropertyNavigationResult::Success(Arc::new(result));
        }

        // Second, check if this property is a link property
        let current_scope = &scoped_type.scope_stack().current_scope().scope_type;

        if let Some(link_def) = self.is_link_property(property_name, current_scope) {
            // This is a link property - create a scoped type with the output scope
            let mut new_scope_context = scoped_type.scope_stack().clone();
            new_scope_context
                .push_scope_type(&link_def.output_scope)
                .unwrap();
            let result = ScopedType::new_with_subtypes(
                scoped_type.cwt_type().clone(),
                new_scope_context,
                scoped_type.subtypes().clone(),
            );
            return PropertyNavigationResult::Success(Arc::new(result));
        }

        // If not a link property, handle as regular property
        match scoped_type.cwt_type() {
            CwtTypeOrSpecial::CwtType(CwtType::Block(block)) => {
                // First, check regular properties
                if let Some(property) = block.properties.get(property_name) {
                    return self.handle_regular_property(scoped_type.clone(), property);
                }

                // Second, check if there's a subtype-specific property
                for subtype_name in scoped_type.subtypes() {
                    if let Some(subtype_property) =
                        self.get_subtype_property(block, subtype_name, property_name)
                    {
                        return self.handle_subtype_property(
                            scoped_type.clone(),
                            subtype_property,
                            property_name,
                        );
                    }
                }

                // Finally, check pattern properties
                if let Some(pattern_property) = self
                    .pattern_matcher
                    .key_matches_pattern(property_name, block)
                {
                    return self.handle_pattern_property(
                        scoped_type.clone(),
                        pattern_property,
                        property_name,
                    );
                } else {
                    PropertyNavigationResult::NotFound
                }
            }
            CwtTypeOrSpecial::CwtType(CwtType::Reference(ReferenceType::AliasMatchLeft {
                key,
            })) => {
                // For alias_match_left[category], we need to look up the specific alias
                // category:property_name and return its type
                let (resolved_cwt_type, alias_def) = self
                    .reference_resolver
                    .resolve_alias_match_left(key, property_name);

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
                                .apply_alias_scope_changes(scoped_type.scope_stack(), &alias_def)
                            {
                                Ok(new_scope) => {
                                    let property_scoped = ScopedType::new_cwt_with_subtypes(
                                        resolved_cwt_type,
                                        new_scope,
                                        scoped_type.subtypes().clone(),
                                    );
                                    PropertyNavigationResult::Success(Arc::new(property_scoped))
                                }
                                Err(error) => PropertyNavigationResult::ScopeError(error),
                            }
                        } else {
                            // No scope changes - use current scope
                            let property_scoped = ScopedType::new_cwt_with_subtypes(
                                resolved_cwt_type,
                                scoped_type.scope_stack().clone(),
                                scoped_type.subtypes().clone(),
                            );
                            PropertyNavigationResult::Success(Arc::new(property_scoped))
                        }
                    } else {
                        // No alias definition found - use current scope
                        let property_scoped = ScopedType::new_cwt_with_subtypes(
                            resolved_cwt_type,
                            scoped_type.scope_stack().clone(),
                            scoped_type.subtypes().clone(),
                        );
                        PropertyNavigationResult::Success(Arc::new(property_scoped))
                    }
                }
            }
            _ => PropertyNavigationResult::NotFound,
        }
    }

    /// Handle navigation to a regular property
    fn handle_regular_property(
        &self,
        scoped_type: Arc<ScopedType>,
        property: &Property,
    ) -> PropertyNavigationResult {
        // Check if this property changes scope
        if property.changes_scope() {
            match property.apply_scope_changes(scoped_type.scope_stack(), &self.cwt_analyzer) {
                Ok(new_scope) => {
                    let property_scoped = ScopedType::new_cwt_with_subtypes(
                        property.property_type.clone(),
                        new_scope,
                        scoped_type.subtypes().clone(),
                    );
                    PropertyNavigationResult::Success(Arc::new(property_scoped))
                }
                Err(error) => PropertyNavigationResult::ScopeError(error),
            }
        } else {
            // Same scope context - but we may need to determine the subtype for the property type
            let property_scoped = if let CwtType::Block(property_block) = &property.property_type {
                // If the property type is a block with subtypes, we might need to determine the subtype
                let subtypes = if !property_block.subtypes.is_empty() {
                    // For now, we don't have the actual property data to determine subtypes
                    // This would need to be integrated with the LSP context where we have actual document data
                    HashSet::new()
                } else {
                    scoped_type.subtypes().clone()
                };
                ScopedType::new_cwt_with_subtypes(
                    property.property_type.clone(),
                    scoped_type.scope_stack().clone(),
                    subtypes,
                )
            } else {
                ScopedType::new_cwt_with_subtypes(
                    property.property_type.clone(),
                    scoped_type.scope_stack().clone(),
                    scoped_type.subtypes().clone(),
                )
            };
            PropertyNavigationResult::Success(Arc::new(property_scoped))
        }
    }

    /// Handle navigation to a pattern property
    fn handle_pattern_property(
        &self,
        scoped_type: Arc<ScopedType>,
        pattern_property: &PatternProperty,
        property_name: &str,
    ) -> PropertyNavigationResult {
        // Check if the pattern property's value type is an AliasMatchLeft that needs resolution
        let (resolved_value_type, alias_def) = match &pattern_property.value_type {
            CwtType::Reference(ReferenceType::AliasMatchLeft { key }) => {
                // Resolve the AliasMatchLeft using the property name
                self.reference_resolver
                    .resolve_alias_match_left(key, property_name)
            }
            _ => (pattern_property.value_type.clone(), None),
        };

        // Apply scope changes - first from alias definition, then from pattern property
        let mut current_scope = scoped_type.scope_stack().clone();

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
            match pattern_property.apply_scope_changes(&current_scope, &self.cwt_analyzer) {
                Ok(new_scope) => current_scope = new_scope,
                Err(error) => return PropertyNavigationResult::ScopeError(error),
            }
        }

        let property_scoped = ScopedType::new_cwt_with_subtypes(
            resolved_value_type,
            current_scope,
            scoped_type.subtypes().clone(),
        );
        PropertyNavigationResult::Success(Arc::new(property_scoped))
    }

    /// Handle navigation to a subtype-specific property
    fn handle_subtype_property(
        &self,
        scoped_type: Arc<ScopedType>,
        subtype_property: &Property,
        _property_name: &str,
    ) -> PropertyNavigationResult {
        // Check if this property changes scope
        if subtype_property.changes_scope() {
            match subtype_property
                .apply_scope_changes(scoped_type.scope_stack(), &self.cwt_analyzer)
            {
                Ok(new_scope) => {
                    let property_scoped = ScopedType::new_cwt_with_subtypes(
                        subtype_property.property_type.clone(),
                        new_scope,
                        scoped_type.subtypes().clone(),
                    );
                    PropertyNavigationResult::Success(Arc::new(property_scoped))
                }
                Err(error) => PropertyNavigationResult::ScopeError(error),
            }
        } else {
            // Same scope context
            let property_scoped = ScopedType::new_cwt_with_subtypes(
                subtype_property.property_type.clone(),
                scoped_type.scope_stack().clone(),
                scoped_type.subtypes().clone(),
            );
            PropertyNavigationResult::Success(Arc::new(property_scoped))
        }
    }

    /// Get subtype-specific property from a block type
    fn get_subtype_property<'b>(
        &self,
        block_type: &'b BlockType,
        subtype_name: &str,
        property_name: &str,
    ) -> Option<&'b Property> {
        // Check if there's a subtype definition for this block type
        if let Some(subtype_def) = block_type.subtypes.get(subtype_name) {
            // Check condition_properties first (CWT schema), then allowed_properties (game data)
            if let Some(prop) = subtype_def.condition_properties.get(property_name) {
                return Some(prop);
            }
            return subtype_def.allowed_properties.get(property_name);
        }
        None
    }

    /// Get all available property names for a scoped type
    pub fn get_available_properties(&self, scoped_type: Arc<ScopedType>) -> Vec<String> {
        let mut properties = Vec::new();

        match scoped_type.cwt_type() {
            CwtTypeOrSpecial::CwtType(CwtType::Block(block)) => {
                // Add subtype-specific properties first
                for subtype_name in scoped_type.subtypes() {
                    if let Some(subtype_def) = block.subtypes.get(subtype_name) {
                        properties.extend(subtype_def.allowed_properties.keys().cloned());
                    }
                }

                // Add regular properties
                properties.extend(block.properties.keys().cloned());

                // Add pattern properties (get completions)
                for pattern_property in &block.pattern_properties {
                    let completions = self
                        .pattern_matcher
                        .get_pattern_completions(&pattern_property.pattern_type);
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
                                    self.utils.get_namespace_keys_for_type_ref(type_name)
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

    /// Check if a property name is a link property for the current scope
    fn is_link_property(&self, property_name: &str, scope: &str) -> Option<&LinkDefinition> {
        if let Some(link_def) = self.cwt_analyzer.get_link(property_name) {
            if link_def.can_be_used_from(scope, &self.cwt_analyzer) {
                return Some(link_def);
            }
        }
        None
    }

    /// Get all available link properties for the current scope
    fn get_scope_link_properties(&self, scope: &str) -> Vec<String> {
        let mut link_properties = Vec::new();

        for (link_name, link_def) in self.cwt_analyzer.get_links() {
            if link_def.can_be_used_from(scope, &self.cwt_analyzer) {
                link_properties.push(link_name.clone());
            }
        }

        link_properties
    }

    /// Apply scope changes from alias definition options
    fn apply_alias_scope_changes(
        &self,
        scope_stack: &ScopeStack,
        alias_def: &AliasDefinition,
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
}
