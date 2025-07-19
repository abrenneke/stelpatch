use crate::handlers::cache::resolver_modules::properties::links::is_link_property;
use crate::handlers::cache::resolver_modules::properties::navigation::collect_navigation_result;
use crate::handlers::scoped_type::{
    CwtTypeOrSpecial, CwtTypeOrSpecialRef, PropertyNavigationResult, ScopedType,
};
use cw_model::{CwtAnalyzer, ReferenceType};
use std::sync::Arc;

use super::ResolverUtils;
use super::patterns::PatternMatcher;
use super::references::ReferenceResolver;
use super::subtypes::SubtypeHandler;

mod discovery;
mod handlers;
mod links;
mod navigation;
mod scope_changes;
mod subtypes;

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
        match scoped_type.cwt_type_for_matching() {
            CwtTypeOrSpecialRef::Block(block) => navigation::navigate_to_block_property(
                self.cwt_analyzer.clone(),
                self.reference_resolver.clone(),
                self.pattern_matcher.clone(),
                scoped_type.clone(),
                block,
                property_name,
            ),
            CwtTypeOrSpecialRef::Reference(ReferenceType::AliasMatchLeft { key }) => {
                navigation::navigate_to_alias_property(
                    self.cwt_analyzer.clone(),
                    self.reference_resolver.clone(),
                    scoped_type.clone(),
                    key,
                    property_name,
                )
            }
            CwtTypeOrSpecialRef::Union(union) => {
                // For unions, try each type in the union and if there are multiple matches, make
                // a union of the results
                let mut successful_results: Vec<Arc<ScopedType>> = Vec::new();
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
                    let result = self.navigate_to_property(temp_scoped_type, property_name);
                    collect_navigation_result(result, &mut successful_results, &mut scope_errors);
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
            CwtTypeOrSpecialRef::ScopedUnion(scoped_unions) => {
                // For scoped unions, try each scoped type in the union and if there are multiple matches,
                // make a union of the results
                let mut successful_results: Vec<Arc<ScopedType>> = Vec::new();
                let mut scope_errors = Vec::new();

                for scoped_union_type in scoped_unions {
                    // Try to navigate to the property with this scoped type
                    let result =
                        self.navigate_to_property(scoped_union_type.clone(), property_name);
                    collect_navigation_result(result, &mut successful_results, &mut scope_errors);
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
            _ => {
                // For other types, check if this property is a link property as fallback
                let current_scope = &scoped_type.scope_stack().current_scope().scope_type;
                if let Some(link_def) =
                    is_link_property(&self.cwt_analyzer, property_name, current_scope)
                {
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

    pub fn get_available_properties(&self, scoped_type: Arc<ScopedType>) -> Vec<String> {
        discovery::get_available_properties(
            self.pattern_matcher.clone(),
            self.cwt_analyzer.clone(),
            self.utils.clone(),
            scoped_type,
        )
    }
}
