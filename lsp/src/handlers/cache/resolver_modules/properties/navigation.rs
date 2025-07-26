use std::sync::Arc;

use cw_model::{BlockType, CwtAnalyzer, CwtType, PatternType, ReferenceType};
use lasso::Spur;

use crate::{
    handlers::{
        cache::{
            FullAnalysis, PatternMatcher, ReferenceResolver,
            resolver_modules::properties::{
                handlers::{
                    handle_pattern_property, handle_pattern_property_all_matches,
                    handle_regular_property, handle_subtype_pattern_property_all_matches,
                    handle_subtype_property,
                },
                links::is_link_property,
                scope_changes::apply_alias_scope_changes,
                subtypes::{get_all_subtype_pattern_properties, get_subtype_property},
            },
        },
        scope::{ScopeError, ScopeStack},
        scoped_type::{CwtTypeOrSpecial, PropertyNavigationResult, ScopedType},
        settings::Settings,
        utils::contains_scripted_argument,
    },
    interner::get_interner,
};

pub fn navigate_to_block_property(
    cwt_analyzer: Arc<CwtAnalyzer>,
    reference_resolver: Arc<ReferenceResolver>,
    pattern_matcher: Arc<PatternMatcher>,
    scoped_type: Arc<ScopedType>,
    block: &BlockType,
    property_name: Spur,
) -> PropertyNavigationResult {
    let interner = get_interner();
    // Collect ALL possible matches instead of returning early
    let mut successful_results: Vec<Arc<ScopedType>> = Vec::new();
    let mut scope_errors = Vec::new();

    // First, check if this property is a scope property (from, fromfrom, prev, etc.)
    if let Some(scope_context) = scoped_type.scope_stack().get_scope_by_name(property_name) {
        // This is a scope property - push that scope onto the current stack
        let mut new_scope_context = scoped_type.scope_stack().clone();
        match new_scope_context.push_scope(scope_context.clone()) {
            Ok(()) => {
                let result = ScopedType::new_with_subtypes(
                    scoped_type.cwt_type().clone(),
                    new_scope_context,
                    scoped_type.subtypes().clone(),
                    scoped_type.in_scripted_effect_block().cloned(),
                );
                let scope_result = PropertyNavigationResult::Success(Arc::new(result));
                collect_navigation_result(scope_result, &mut successful_results, &mut scope_errors);
            }
            Err(scope_error) => {
                let scope_result = PropertyNavigationResult::ScopeError(scope_error);
                collect_navigation_result(scope_result, &mut successful_results, &mut scope_errors);
            }
        }
    } else if !Settings::global().report_unknown_scopes
        && ScopeStack::get_all_scope_properties().contains(&interner.resolve(&property_name))
    {
        let mut new_scope_context = scoped_type.scope_stack().clone();
        match new_scope_context.push_scope_type(property_name) {
            Ok(()) => {
                let result = ScopedType::new_with_subtypes(
                    scoped_type.cwt_type().clone(),
                    new_scope_context,
                    scoped_type.subtypes().clone(),
                    scoped_type.in_scripted_effect_block().cloned(),
                );
                let scope_result = PropertyNavigationResult::Success(Arc::new(result));
                collect_navigation_result(scope_result, &mut successful_results, &mut scope_errors);
            }
            Err(scope_error) => {
                let scope_result = PropertyNavigationResult::ScopeError(scope_error);
                collect_navigation_result(scope_result, &mut successful_results, &mut scope_errors);
            }
        }
    }

    // Second, check regular properties
    if let Some(property) = block.properties.get(&property_name) {
        let result = handle_regular_property(
            cwt_analyzer.clone(),
            scoped_type.clone(),
            property,
            property_name,
        );
        collect_navigation_result(result, &mut successful_results, &mut scope_errors);
    }

    // Third, check if there's a subtype-specific property
    for subtype_name in scoped_type.subtypes() {
        if let Some(subtype_property) = get_subtype_property(block, *subtype_name, property_name) {
            let result = handle_subtype_property(
                cwt_analyzer.clone(),
                scoped_type.clone(),
                subtype_property,
            );
            collect_navigation_result(result, &mut successful_results, &mut scope_errors);
        }

        let matching_subtype_pattern_properties = get_all_subtype_pattern_properties(
            pattern_matcher.clone(),
            block,
            *subtype_name,
            property_name,
        );
        for subtype_pattern_property in matching_subtype_pattern_properties {
            let results = handle_subtype_pattern_property_all_matches(
                cwt_analyzer.clone(),
                reference_resolver.clone(),
                scoped_type.clone(),
                subtype_pattern_property,
                property_name,
            );
            for result in results {
                collect_navigation_result(result, &mut successful_results, &mut scope_errors);
            }
        }
    }

    // Fourth, check for special "scalar" key that matches any string
    if let Some(scalar_property) = block.properties.get(&interner.get_or_intern("scalar")) {
        let result = handle_regular_property(
            cwt_analyzer.clone(),
            scoped_type.clone(),
            scalar_property,
            property_name,
        );
        collect_navigation_result(result, &mut successful_results, &mut scope_errors);
    }

    if let Some(int_property) = block.properties.get(&interner.get_or_intern("int")) {
        if interner.resolve(&property_name).parse::<i32>().is_ok() {
            let result = handle_regular_property(
                cwt_analyzer.clone(),
                scoped_type.clone(),
                int_property,
                property_name,
            );
            collect_navigation_result(result, &mut successful_results, &mut scope_errors);
        }
    }

    if let Some(localisation_property) = block
        .properties
        .get(&interner.get_or_intern("localisation"))
    {
        if !Settings::global().validate_localisation {
            let result = handle_regular_property(
                cwt_analyzer.clone(),
                scoped_type.clone(),
                localisation_property,
                property_name,
            );
            collect_navigation_result(result, &mut successful_results, &mut scope_errors);
        }
    }

    // Fifth, check for special inline_script property
    if property_name == interner.get_or_intern("inline_script") {
        let inline_script_type = CwtTypeOrSpecial::CwtType(
            cwt_analyzer
                .get_type(interner.get_or_intern("$inline_script"))
                .unwrap()
                .rules
                .clone(),
        );
        let inline_script_scoped = ScopedType::new_with_subtypes(
            inline_script_type,
            scoped_type.scope_stack().clone(),
            scoped_type.subtypes().clone(),
            scoped_type.in_scripted_effect_block().cloned(),
        );
        successful_results.push(Arc::new(inline_script_scoped));
    }

    if interner
        .resolve(&property_name)
        .starts_with("event_target:")
    {
        let mut new_scope = scoped_type.scope_stack().branch();
        new_scope
            .push_scope_type(interner.get_or_intern("unknown"))
            .unwrap(); // We don't store what scope the event target is right now

        let result = ScopedType::new_with_subtypes(
            scoped_type.cwt_type().clone(),
            new_scope,
            scoped_type.subtypes().clone(),
            scoped_type.in_scripted_effect_block().cloned(),
        );

        successful_results.push(Arc::new(result));
    }

    // Sixth, check pattern properties - collect ALL matches, not just the first
    let matching_pattern_properties =
        pattern_matcher.key_matches_all_patterns(property_name, block);
    for pattern_property in matching_pattern_properties {
        let results = handle_pattern_property_all_matches(
            cwt_analyzer.clone(),
            reference_resolver.clone(),
            scoped_type.clone(),
            pattern_property,
            property_name,
        );
        for result in results {
            collect_navigation_result(result, &mut successful_results, &mut scope_errors);
        }
    }

    // Seventh, check the special scripted_effect_params enum
    if let Some(scripted_effect_name) = scoped_type.in_scripted_effect_block() {
        if let Some(full_analysis) = FullAnalysis::get() {
            if let Some(arguments) = full_analysis
                .scripted_effect_arguments
                .get(scripted_effect_name)
            {
                for pattern_property in &block.pattern_properties {
                    if let PatternType::Enum { key } = &pattern_property.pattern_type {
                        if *key == interner.get_or_intern("scripted_effect_params") {
                            if arguments.contains(&property_name) {
                                let result = handle_pattern_property(
                                    cwt_analyzer.clone(),
                                    reference_resolver.clone(),
                                    scoped_type.clone(),
                                    pattern_property,
                                    property_name,
                                );
                                collect_navigation_result(
                                    result,
                                    &mut successful_results,
                                    &mut scope_errors,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // If it's a scripted argument, this could be really anything
    if contains_scripted_argument(property_name) {
        let any_scoped = ScopedType::new_with_subtypes(
            CwtTypeOrSpecial::CwtType(Arc::new(CwtType::Any)),
            scoped_type.scope_stack().clone(),
            scoped_type.subtypes().clone(),
            scoped_type.in_scripted_effect_block().cloned(),
        );
        successful_results.push(Arc::new(any_scoped));
    }

    // Finally, check if this property is a link property (as fallback)
    let current_scope = &scoped_type.scope_stack().current_scope().scope_type;
    if let Some(link_def) = is_link_property(&cwt_analyzer, property_name, *current_scope) {
        // This is a link property - create a scoped type with the output scope
        let mut new_scope_context = scoped_type.scope_stack().clone();
        if new_scope_context
            .push_scope_type(link_def.output_scope)
            .is_ok()
        {
            let result = ScopedType::new_with_subtypes(
                scoped_type.cwt_type().clone(),
                new_scope_context,
                scoped_type.subtypes().clone(),
                scoped_type.in_scripted_effect_block().cloned(),
            );
            successful_results.push(Arc::new(result));
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
            let result = successful_results.into_iter().next().unwrap();
            PropertyNavigationResult::Success(result)
        }
        _ => {
            // Multiple results - create a scoped union of them to preserve all scope contexts
            let result_scoped = ScopedType::new_with_subtypes(
                CwtTypeOrSpecial::ScopedUnion(successful_results),
                scoped_type.scope_stack().clone(),
                scoped_type.subtypes().clone(),
                scoped_type.in_scripted_effect_block().cloned(),
            );
            PropertyNavigationResult::Success(Arc::new(result_scoped))
        }
    }
}

pub fn navigate_to_alias_property(
    cwt_analyzer: Arc<CwtAnalyzer>,
    reference_resolver: Arc<ReferenceResolver>,
    scoped_type: Arc<ScopedType>,
    key: Spur,
    property_name: Spur,
) -> PropertyNavigationResult {
    let interner = get_interner();
    // For alias_match_left[category], we need to look up ALL matching aliases
    // category:property_name and return all their types
    let all_alias_results = reference_resolver.resolve_all_alias_match_left(key, property_name);

    let mut successful_results: Vec<Arc<ScopedType>> = Vec::new();
    let mut scope_errors = Vec::new();
    let mut found_match = false;

    for (resolved_cwt_type, alias_def, scripted_effect_name) in all_alias_results {
        // Check if we found a matching alias
        if matches!(
            &*resolved_cwt_type,
            CwtType::Reference(ReferenceType::AliasMatchLeft { .. })
        ) {
            // This is the fallback case - no actual match found
            continue;
        }

        found_match = true;

        // We found a matching alias - check if it has scope changes
        if let Some(alias_def) = alias_def {
            if alias_def.changes_scope() {
                match apply_alias_scope_changes(
                    cwt_analyzer.clone(),
                    scoped_type.scope_stack(),
                    &alias_def,
                ) {
                    Ok(new_scope) => {
                        let property_scoped = ScopedType::new_cwt_with_subtypes(
                            resolved_cwt_type,
                            new_scope,
                            scoped_type.subtypes().clone(),
                            scripted_effect_name,
                        );
                        successful_results.push(Arc::new(property_scoped));
                    }
                    Err(error) => scope_errors.push(error),
                }
            } else {
                // No scope changes - use current scope
                let property_scoped = ScopedType::new_cwt_with_subtypes(
                    resolved_cwt_type,
                    scoped_type.scope_stack().clone(),
                    scoped_type.subtypes().clone(),
                    scripted_effect_name,
                );
                successful_results.push(Arc::new(property_scoped));
            }
        } else {
            // No alias definition found - use current scope
            let property_scoped = ScopedType::new_cwt_with_subtypes(
                resolved_cwt_type,
                scoped_type.scope_stack().clone(),
                scoped_type.subtypes().clone(),
                scripted_effect_name,
            );
            successful_results.push(Arc::new(property_scoped));
        }
    }

    if !found_match {
        // No matching alias was found - check if this property is a link property as fallback
        let current_scope = &scoped_type.scope_stack().current_scope().scope_type;
        if let Some(link_def) = is_link_property(&cwt_analyzer, property_name, *current_scope) {
            // This is a link property - create a scoped type with the output scope
            let mut new_scope_context = scoped_type.scope_stack().clone();
            new_scope_context
                .push_scope_type(link_def.output_scope)
                .unwrap();
            let result = ScopedType::new_with_subtypes(
                scoped_type.cwt_type().clone(),
                new_scope_context,
                scoped_type.subtypes().clone(),
                None,
            );
            return PropertyNavigationResult::Success(Arc::new(result));
        }
        return PropertyNavigationResult::NotFound;
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
            let result = successful_results.into_iter().next().unwrap();
            PropertyNavigationResult::Success(result)
        }
        _ => {
            // Multiple results - create a scoped union of them
            let result_scoped = ScopedType::new_with_subtypes(
                CwtTypeOrSpecial::ScopedUnion(successful_results),
                scoped_type.scope_stack().clone(),
                scoped_type.subtypes().clone(),
                scoped_type.in_scripted_effect_block().cloned(),
            );
            PropertyNavigationResult::Success(Arc::new(result_scoped))
        }
    }
}

/// Helper method to collect PropertyNavigationResult into vectors
pub fn collect_navigation_result(
    result: PropertyNavigationResult,
    successful_results: &mut Vec<Arc<ScopedType>>,
    scope_errors: &mut Vec<ScopeError>,
) {
    match result {
        PropertyNavigationResult::Success(result) => {
            successful_results.push(result);
        }
        PropertyNavigationResult::ScopeError(error) => {
            scope_errors.push(error);
        }
        PropertyNavigationResult::NotFound => {
            // No action needed for NotFound
        }
    }
}
