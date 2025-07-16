use crate::handlers::cache::FullAnalysis;
use crate::handlers::scope::{ScopeError, ScopeStack};
use crate::handlers::scoped_type::{
    CwtTypeOrSpecial, PropertyNavigationResult, ScopeAwareProperty, ScopedType,
};
use crate::handlers::utils::contains_scripted_argument;
use cw_model::{
    AliasDefinition, AliasName, BlockType, CwtAnalyzer, CwtType, LinkDefinition, PatternProperty,
    PatternType, Property, ReferenceType,
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
    /// Supports complex properties like "root.owner" which are treated as chained navigation
    pub fn navigate_to_property(
        &self,
        scoped_type: Arc<ScopedType>,
        property_name: &str,
    ) -> PropertyNavigationResult {
        // Handle complex properties (containing dots) by navigating through each part
        if property_name.contains('.') {
            return self.navigate_to_complex_property(scoped_type, property_name);
        }

        // First, check if this property is a scope property (from, fromfrom, etc.)
        if let Some(scope_context) = scoped_type.scope_stack().get_scope_by_name(property_name) {
            // This is a scope property - push that scope onto the current stack
            let mut new_scope_context = scoped_type.scope_stack().clone();
            new_scope_context.push_scope(scope_context.clone()).unwrap();
            let result = ScopedType::new_with_subtypes(
                scoped_type.cwt_type().clone(),
                new_scope_context,
                scoped_type.subtypes().clone(),
                scoped_type.in_scripted_effect_block().cloned(),
            );
            return PropertyNavigationResult::Success(Arc::new(result));
        }

        // Handle regular properties based on the current type
        match scoped_type.cwt_type() {
            CwtTypeOrSpecial::CwtType(CwtType::Block(block)) => {
                // Collect ALL possible matches instead of returning early
                let mut successful_results = Vec::new();
                let mut scope_errors = Vec::new();

                // First, check regular properties
                if let Some(property) = block.properties.get(property_name) {
                    match self.handle_regular_property(scoped_type.clone(), property) {
                        PropertyNavigationResult::Success(result) => {
                            successful_results.push(result.cwt_type().clone());
                        }
                        PropertyNavigationResult::ScopeError(error) => {
                            scope_errors.push(error);
                        }
                        PropertyNavigationResult::NotFound => {
                            // This shouldn't happen for regular properties, but handle it
                        }
                    }
                }

                // Second, check if there's a subtype-specific property
                for subtype_name in scoped_type.subtypes() {
                    if let Some(subtype_property) =
                        self.get_subtype_property(block, subtype_name, property_name)
                    {
                        match self.handle_subtype_property(
                            scoped_type.clone(),
                            subtype_property,
                            property_name,
                        ) {
                            PropertyNavigationResult::Success(result) => {
                                successful_results.push(result.cwt_type().clone());
                            }
                            PropertyNavigationResult::ScopeError(error) => {
                                scope_errors.push(error);
                            }
                            PropertyNavigationResult::NotFound => {
                                // This shouldn't happen for subtype properties, but handle it
                            }
                        }
                    }
                }

                // Third, check for special "scalar" key that matches any string
                if let Some(scalar_property) = block.properties.get("scalar") {
                    match self.handle_regular_property(scoped_type.clone(), scalar_property) {
                        PropertyNavigationResult::Success(result) => {
                            successful_results.push(result.cwt_type().clone());
                        }
                        PropertyNavigationResult::ScopeError(error) => {
                            scope_errors.push(error);
                        }
                        PropertyNavigationResult::NotFound => {
                            // This shouldn't happen for scalar properties, but handle it
                        }
                    }
                }

                if let Some(int_property) = block.properties.get("int") {
                    if property_name.parse::<i32>().is_ok() {
                        match self.handle_regular_property(scoped_type.clone(), int_property) {
                            PropertyNavigationResult::Success(result) => {
                                successful_results.push(result.cwt_type().clone());
                            }
                            PropertyNavigationResult::ScopeError(error) => {
                                scope_errors.push(error);
                            }
                            PropertyNavigationResult::NotFound => {
                                // This shouldn't happen for int properties, but handle it
                            }
                        }
                    }
                }

                // Fourth, check for special inline_script property
                if property_name == "inline_script" {
                    successful_results.push(CwtTypeOrSpecial::CwtType(
                        self.cwt_analyzer
                            .get_type("$inline_script")
                            .unwrap()
                            .rules
                            .clone(),
                    ));
                }

                // Fifth, check pattern properties
                if let Some(pattern_property) = self
                    .pattern_matcher
                    .key_matches_pattern(property_name, block)
                {
                    match self.handle_pattern_property(
                        scoped_type.clone(),
                        pattern_property,
                        property_name,
                    ) {
                        PropertyNavigationResult::Success(result) => {
                            successful_results.push(result.cwt_type().clone());
                        }
                        PropertyNavigationResult::ScopeError(error) => {
                            scope_errors.push(error);
                        }
                        PropertyNavigationResult::NotFound => {
                            // This shouldn't happen for pattern properties, but handle it
                        }
                    }
                }

                // Sixth, check the special scripted_effect_params enum
                if let Some(scripted_effect_name) = scoped_type.in_scripted_effect_block() {
                    if let Some(full_analysis) = FullAnalysis::get() {
                        if let Some(arguments) = full_analysis
                            .scripted_effect_arguments
                            .get(scripted_effect_name)
                        {
                            for pattern_property in &block.pattern_properties {
                                if let PatternType::Enum { key } = &pattern_property.pattern_type {
                                    if key == "scripted_effect_params" {
                                        if arguments.contains(property_name) {
                                            match self.handle_pattern_property(
                                                scoped_type.clone(),
                                                pattern_property,
                                                property_name,
                                            ) {
                                                PropertyNavigationResult::Success(result) => {
                                                    successful_results
                                                        .push(result.cwt_type().clone());
                                                }
                                                PropertyNavigationResult::ScopeError(error) => {
                                                    scope_errors.push(error);
                                                }
                                                PropertyNavigationResult::NotFound => {
                                                    // This shouldn't happen for scripted_effect_params, but handle it
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // If it's a scripted argument, this could be really anything
                if contains_scripted_argument(property_name) {
                    successful_results.push(CwtTypeOrSpecial::CwtType(CwtType::Any));
                }

                // Finally, check if this property is a link property (as fallback)
                let current_scope = &scoped_type.scope_stack().current_scope().scope_type;
                if let Some(_link_def) = self.is_link_property(property_name, current_scope) {
                    // For link properties, we maintain the current type but note scope change
                    // Since we're collecting multiple results, we can't easily apply scope changes here
                    // The type remains the same, but the consumer should be aware this is a link
                    match scoped_type.cwt_type() {
                        CwtTypeOrSpecial::CwtType(cwt_type) => {
                            successful_results.push(CwtTypeOrSpecial::CwtType(cwt_type.clone()));
                        }
                        scoped_union => {
                            successful_results.push(scoped_union.clone());
                        }
                    }
                }

                // Combine all successful results
                match successful_results.len() {
                    0 => {
                        // No successful results - if we have scope errors, return the first one
                        if let Some(error) = scope_errors.into_iter().next() {
                            PropertyNavigationResult::ScopeError(error)
                        } else {
                            PropertyNavigationResult::NotFound
                        }
                    }
                    1 => {
                        // Single result - return it directly
                        let result_type = successful_results.into_iter().next().unwrap();
                        let result_scoped = ScopedType::new_with_subtypes(
                            result_type,
                            scoped_type.scope_stack().clone(),
                            scoped_type.subtypes().clone(),
                            scoped_type.in_scripted_effect_block().cloned(),
                        );
                        PropertyNavigationResult::Success(Arc::new(result_scoped))
                    }
                    _ => {
                        // Multiple results - create a union of them
                        let union_type = CwtType::Union(
                            successful_results
                                .into_iter()
                                .map(|t| match t {
                                    CwtTypeOrSpecial::CwtType(t) => t,
                                    CwtTypeOrSpecial::ScopedUnion(_) => CwtType::Any, // Fallback for scoped unions
                                })
                                .collect(),
                        );
                        let result_scoped = ScopedType::new_with_subtypes(
                            CwtTypeOrSpecial::CwtType(union_type),
                            scoped_type.scope_stack().clone(),
                            scoped_type.subtypes().clone(),
                            scoped_type.in_scripted_effect_block().cloned(),
                        );
                        PropertyNavigationResult::Success(Arc::new(result_scoped))
                    }
                }
            }
            CwtTypeOrSpecial::CwtType(CwtType::Reference(ReferenceType::AliasMatchLeft {
                key,
            })) => {
                // For alias_match_left[category], we need to look up the specific alias
                // category:property_name and return its type
                let (resolved_cwt_type, alias_def, scripted_effect_name) = self
                    .reference_resolver
                    .resolve_alias_match_left(key, property_name);

                // Check if we found a matching alias
                if matches!(
                    resolved_cwt_type,
                    CwtType::Reference(ReferenceType::AliasMatchLeft { .. })
                ) {
                    // No matching alias was found - check if this property is a link property as fallback
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
                            scripted_effect_name,
                        );
                        return PropertyNavigationResult::Success(Arc::new(result));
                    }
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
                                        scripted_effect_name,
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
                                scripted_effect_name,
                            );
                            PropertyNavigationResult::Success(Arc::new(property_scoped))
                        }
                    } else {
                        // No alias definition found - use current scope
                        let property_scoped = ScopedType::new_cwt_with_subtypes(
                            resolved_cwt_type,
                            scoped_type.scope_stack().clone(),
                            scoped_type.subtypes().clone(),
                            scripted_effect_name,
                        );

                        PropertyNavigationResult::Success(Arc::new(property_scoped))
                    }
                }
            }
            CwtTypeOrSpecial::CwtType(CwtType::Union(union)) => {
                // For unions, try each type in the union and if there are multiple matches, make
                // a union of the results
                let mut successful_results = Vec::new();
                let mut scope_errors = Vec::new();

                for union_type in union {
                    // Create a temporary scoped type with this union member type
                    let temp_scoped_type = Arc::new(ScopedType::new_cwt_with_subtypes(
                        union_type.clone(),
                        scoped_type.scope_stack().clone(),
                        scoped_type.subtypes().clone(),
                        scoped_type.in_scripted_effect_block().cloned(),
                    ));

                    // Try to navigate to the property with this type
                    match self.navigate_to_property(temp_scoped_type, property_name) {
                        PropertyNavigationResult::Success(result) => {
                            successful_results.push(result.cwt_type().clone());
                        }
                        PropertyNavigationResult::ScopeError(error) => {
                            scope_errors.push(error);
                        }
                        PropertyNavigationResult::NotFound => {
                            // Continue to next union member
                        }
                    }
                }

                match successful_results.len() {
                    0 => {
                        // No successful results - if we have scope errors, return the first one
                        if let Some(error) = scope_errors.into_iter().next() {
                            PropertyNavigationResult::ScopeError(error)
                        } else {
                            PropertyNavigationResult::NotFound
                        }
                    }
                    1 => {
                        // Single result - return it directly
                        let result_type = successful_results.into_iter().next().unwrap();
                        let result_scoped = ScopedType::new_with_subtypes(
                            result_type,
                            scoped_type.scope_stack().clone(),
                            scoped_type.subtypes().clone(),
                            scoped_type.in_scripted_effect_block().cloned(),
                        );
                        PropertyNavigationResult::Success(Arc::new(result_scoped))
                    }
                    _ => {
                        // Multiple results - create a union of them
                        let union_type = CwtType::Union(
                            successful_results
                                .into_iter()
                                .map(|t| match t {
                                    CwtTypeOrSpecial::CwtType(t) => t,
                                    CwtTypeOrSpecial::ScopedUnion(_) => unreachable!(), // probably?
                                })
                                .collect(),
                        );
                        let result_scoped = ScopedType::new_with_subtypes(
                            CwtTypeOrSpecial::CwtType(union_type),
                            scoped_type.scope_stack().clone(),
                            scoped_type.subtypes().clone(),
                            scoped_type.in_scripted_effect_block().cloned(),
                        );
                        PropertyNavigationResult::Success(Arc::new(result_scoped))
                    }
                }
            }
            _ => {
                // For other types, check if this property is a link property as fallback
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
                        scoped_type.in_scripted_effect_block().cloned(),
                    );
                    return PropertyNavigationResult::Success(Arc::new(result));
                }
                PropertyNavigationResult::NotFound
            }
        }
    }

    /// Navigate to a complex property (containing dots) by navigating through each part sequentially
    fn navigate_to_complex_property(
        &self,
        mut current_scoped_type: Arc<ScopedType>,
        property_path: &str,
    ) -> PropertyNavigationResult {
        let parts: Vec<&str> = property_path.split('.').collect();

        for part in parts {
            match self.navigate_to_property(current_scoped_type, part) {
                PropertyNavigationResult::Success(result) => {
                    current_scoped_type = result;
                }
                PropertyNavigationResult::ScopeError(error) => {
                    return PropertyNavigationResult::ScopeError(error);
                }
                PropertyNavigationResult::NotFound => {
                    return PropertyNavigationResult::NotFound;
                }
            }
        }

        PropertyNavigationResult::Success(current_scoped_type)
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
                        scoped_type.in_scripted_effect_block().cloned(),
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
                    scoped_type.in_scripted_effect_block().cloned(),
                )
            } else {
                ScopedType::new_cwt_with_subtypes(
                    property.property_type.clone(),
                    scoped_type.scope_stack().clone(),
                    scoped_type.subtypes().clone(),
                    scoped_type.in_scripted_effect_block().cloned(),
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
        let (resolved_value_type, alias_def, scripted_effect_block) =
            match &pattern_property.value_type {
                CwtType::Reference(ReferenceType::AliasMatchLeft { key }) => {
                    // Resolve the AliasMatchLeft using the property name
                    let result = self
                        .reference_resolver
                        .resolve_alias_match_left(key, property_name);

                    result
                }
                _ => (pattern_property.value_type.clone(), None, None),
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
            scripted_effect_block,
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
                        scoped_type.in_scripted_effect_block().cloned(),
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
                scoped_type.in_scripted_effect_block().cloned(),
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
            // If current scope is "unknown", treat it as a fallback that can navigate anywhere
            if scope == "unknown" || link_def.can_be_used_from(scope, &self.cwt_analyzer) {
                return Some(link_def);
            }
        }
        None
    }

    /// Get all available link properties for the current scope
    fn get_scope_link_properties(&self, scope: &str) -> Vec<String> {
        let mut link_properties = Vec::new();

        // If current scope is "unknown", treat it as a fallback that can navigate anywhere
        let is_unknown_scope = scope == "unknown";

        for (link_name, link_def) in self.cwt_analyzer.get_links() {
            // If scope is unknown, allow all links as fallback, otherwise use normal validation
            if is_unknown_scope || link_def.can_be_used_from(scope, &self.cwt_analyzer) {
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
