use std::{collections::HashSet, sync::Arc};

use cw_model::{AliasDefinition, CwtAnalyzer, CwtType, PatternProperty, Property, ReferenceType};

use crate::handlers::{
    cache::{
        ReferenceResolver, resolver_modules::properties::scope_changes::apply_alias_scope_changes,
    },
    scoped_type::{PropertyNavigationResult, ScopeAwareProperty, ScopedType},
};

/// Handle navigation to a regular property
pub fn handle_regular_property(
    cwt_analyzer: Arc<CwtAnalyzer>,
    scoped_type: Arc<ScopedType>,
    property: &Property,
) -> PropertyNavigationResult {
    // Check if this property changes scope
    if property.changes_scope() {
        match property.apply_scope_changes(scoped_type.scope_stack(), &cwt_analyzer) {
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
        let property_scoped = if let CwtType::Block(property_block) = &*property.property_type {
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
pub fn handle_pattern_property(
    cwt_analyzer: Arc<CwtAnalyzer>,
    reference_resolver: Arc<ReferenceResolver>,
    scoped_type: Arc<ScopedType>,
    pattern_property: &PatternProperty,
    property_name: &str,
) -> PropertyNavigationResult {
    // Check if the pattern property's value type is an AliasMatchLeft that needs resolution
    let (resolved_value_type, alias_def, scripted_effect_block) =
        match &*pattern_property.value_type {
            CwtType::Reference(ReferenceType::AliasMatchLeft { key }) => {
                // Resolve the AliasMatchLeft using the property name
                let result = reference_resolver.resolve_alias_match_left(key, property_name);

                result
            }
            _ => (pattern_property.value_type.clone(), None, None),
        };

    // Apply scope changes - first from alias definition, then from pattern property
    let mut current_scope = scoped_type.scope_stack().clone();

    // Apply alias scope changes if present
    if let Some(alias_def) = alias_def {
        if alias_def.changes_scope() {
            match apply_alias_scope_changes(cwt_analyzer.clone(), &current_scope, &alias_def) {
                Ok(new_scope) => current_scope = new_scope,
                Err(error) => {
                    return PropertyNavigationResult::ScopeError(error);
                }
            }
        }
    }

    // Apply pattern property scope changes if present
    if pattern_property.changes_scope() {
        match pattern_property.apply_scope_changes(&current_scope, &cwt_analyzer) {
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

/// Handle navigation to multiple pattern properties with potential AliasMatchLeft resolution
pub fn handle_pattern_property_all_matches(
    cwt_analyzer: Arc<CwtAnalyzer>,
    reference_resolver: Arc<ReferenceResolver>,
    scoped_type: Arc<ScopedType>,
    pattern_property: &PatternProperty,
    property_name: &str,
) -> Vec<PropertyNavigationResult> {
    // Check if the pattern property's value type is an AliasMatchLeft that needs resolution
    match &*pattern_property.value_type {
        CwtType::Reference(ReferenceType::AliasMatchLeft { key }) => {
            // Resolve ALL AliasMatchLeft matches using the property name
            let all_results = reference_resolver.resolve_all_alias_match_left(key, property_name);

            let mut property_results = Vec::new();

            for (resolved_value_type, alias_def, scripted_effect_block) in all_results {
                // Apply scope changes - first from alias definition, then from pattern property
                let mut current_scope = scoped_type.scope_stack().clone();

                // Apply alias scope changes if present
                if let Some(alias_def) = alias_def {
                    if alias_def.changes_scope() {
                        match apply_alias_scope_changes(
                            cwt_analyzer.clone(),
                            &current_scope,
                            &alias_def,
                        ) {
                            Ok(new_scope) => current_scope = new_scope,
                            Err(error) => {
                                property_results.push(PropertyNavigationResult::ScopeError(error));
                                continue;
                            }
                        }
                    }
                }

                // Apply pattern property scope changes if present
                if pattern_property.changes_scope() {
                    match pattern_property.apply_scope_changes(&current_scope, &cwt_analyzer) {
                        Ok(new_scope) => current_scope = new_scope,
                        Err(error) => {
                            property_results.push(PropertyNavigationResult::ScopeError(error));
                            continue;
                        }
                    }
                }

                let property_scoped = ScopedType::new_cwt_with_subtypes(
                    resolved_value_type,
                    current_scope,
                    scoped_type.subtypes().clone(),
                    scripted_effect_block,
                );

                property_results.push(PropertyNavigationResult::Success(Arc::new(property_scoped)));
            }

            property_results
        }
        _ => {
            // No AliasMatchLeft - use the original handler
            vec![handle_pattern_property(
                cwt_analyzer,
                reference_resolver,
                scoped_type,
                pattern_property,
                property_name,
            )]
        }
    }
}

/// Handle navigation to a subtype-specific property
pub fn handle_subtype_property(
    cwt_analyzer: Arc<CwtAnalyzer>,
    scoped_type: Arc<ScopedType>,
    subtype_property: &Property,
) -> PropertyNavigationResult {
    // Check if this property changes scope
    if subtype_property.changes_scope() {
        match subtype_property.apply_scope_changes(scoped_type.scope_stack(), &cwt_analyzer) {
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

/// Handle navigation to a subtype-specific pattern property
pub fn handle_subtype_pattern_property(
    cwt_analyzer: Arc<CwtAnalyzer>,
    reference_resolver: Arc<ReferenceResolver>,
    scoped_type: Arc<ScopedType>,
    subtype_pattern_property: &PatternProperty,
    property_name: &str,
) -> PropertyNavigationResult {
    // Check if the pattern property's value type is an AliasMatchLeft that needs resolution
    let (resolved_value_type, alias_def, scripted_effect_block) =
        match &*subtype_pattern_property.value_type {
            CwtType::Reference(ReferenceType::AliasMatchLeft { key }) => {
                // Resolve the AliasMatchLeft using the property name
                let result = reference_resolver.resolve_alias_match_left(key, property_name);

                result
            }
            _ => (
                subtype_pattern_property.value_type.clone(),
                None::<AliasDefinition>,
                None::<String>,
            ),
        };

    // Apply scope changes - first from alias definition, then from pattern property
    let mut current_scope = scoped_type.scope_stack().clone();

    // Apply alias scope changes if present
    if let Some(alias_def) = alias_def {
        if alias_def.changes_scope() {
            match apply_alias_scope_changes(cwt_analyzer.clone(), &current_scope, &alias_def) {
                Ok(new_scope) => current_scope = new_scope,
                Err(error) => {
                    return PropertyNavigationResult::ScopeError(error);
                }
            }
        }
    }

    // Apply pattern property scope changes if present
    if subtype_pattern_property.changes_scope() {
        match subtype_pattern_property.apply_scope_changes(&current_scope, &cwt_analyzer) {
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

/// Handle navigation to multiple subtype pattern properties with potential AliasMatchLeft resolution
pub fn handle_subtype_pattern_property_all_matches(
    cwt_analyzer: Arc<CwtAnalyzer>,
    reference_resolver: Arc<ReferenceResolver>,
    scoped_type: Arc<ScopedType>,
    subtype_pattern_property: &PatternProperty,
    property_name: &str,
) -> Vec<PropertyNavigationResult> {
    // Check if the pattern property's value type is an AliasMatchLeft that needs resolution
    match &*subtype_pattern_property.value_type {
        CwtType::Reference(ReferenceType::AliasMatchLeft { key }) => {
            // Resolve ALL AliasMatchLeft matches using the property name
            let all_results = reference_resolver.resolve_all_alias_match_left(key, property_name);

            let mut property_results = Vec::new();

            for (resolved_value_type, alias_def, scripted_effect_block) in all_results {
                // Apply scope changes - first from alias definition, then from pattern property
                let mut current_scope = scoped_type.scope_stack().clone();

                // Apply alias scope changes if present
                if let Some(alias_def) = alias_def {
                    if alias_def.changes_scope() {
                        match apply_alias_scope_changes(
                            cwt_analyzer.clone(),
                            &current_scope,
                            &alias_def,
                        ) {
                            Ok(new_scope) => current_scope = new_scope,
                            Err(error) => {
                                property_results.push(PropertyNavigationResult::ScopeError(error));
                                continue;
                            }
                        }
                    }
                }

                // Apply pattern property scope changes if present
                if subtype_pattern_property.changes_scope() {
                    match subtype_pattern_property
                        .apply_scope_changes(&current_scope, &cwt_analyzer)
                    {
                        Ok(new_scope) => current_scope = new_scope,
                        Err(error) => {
                            property_results.push(PropertyNavigationResult::ScopeError(error));
                            continue;
                        }
                    }
                }

                let property_scoped = ScopedType::new_cwt_with_subtypes(
                    resolved_value_type,
                    current_scope,
                    scoped_type.subtypes().clone(),
                    scripted_effect_block,
                );

                property_results.push(PropertyNavigationResult::Success(Arc::new(property_scoped)));
            }

            property_results
        }
        _ => {
            // No AliasMatchLeft - use the original handler
            vec![handle_subtype_pattern_property(
                cwt_analyzer,
                reference_resolver,
                scoped_type,
                subtype_pattern_property,
                property_name,
            )]
        }
    }
}
