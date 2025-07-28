use crate::handlers::cache::resolver_modules::properties::links::is_link_property;
use crate::handlers::cache::resolver_modules::properties::navigation::collect_navigation_result;
use crate::handlers::scoped_type::{
    CwtTypeOrSpecial, CwtTypeOrSpecialRef, PropertyNavigationResult, ScopedType,
};
use crate::interner::get_interner;
use cw_model::{CwtAnalyzer, ReferenceType};
use lasso::Spur;
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
        let subtype_handler = Arc::new(SubtypeHandler::new(cwt_analyzer.clone()));

        Self {
            reference_resolver: Arc::new(ReferenceResolver::new(
                cwt_analyzer.clone(),
                utils.clone(),
                subtype_handler.clone(),
            )),
            pattern_matcher: Arc::new(PatternMatcher::new(
                cwt_analyzer.clone(),
                utils.clone(),
                subtype_handler,
            )),
            utils,
            cwt_analyzer,
        }
    }

    /// Navigate to a property from a given scoped type
    /// Supports complex properties like "root.owner" which are treated as chained navigation
    pub fn navigate_to_property(
        &self,
        scoped_type: Arc<ScopedType>,
        property_name: Spur,
    ) -> PropertyNavigationResult {
        let interner = get_interner();

        let property_path = interner.resolve(&property_name);

        // Handle complex properties (containing dots) by navigating through each part
        if property_path.contains('.') {
            return self.navigate_to_complex_property(scoped_type, &property_path);
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
                    get_interner().get_or_intern(key),
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
                    is_link_property(&self.cwt_analyzer, property_name, *current_scope)
                {
                    // This is a link property - create a scoped type with the output scope
                    let mut new_scope_context = scoped_type.scope_stack().clone();
                    new_scope_context
                        .push_scope_type(link_def.output_scope)
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
            match self.navigate_to_property(current_scoped_type, get_interner().get_or_intern(part))
            {
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

    /// Get the documentation for a property if it exists
    pub fn get_property_documentation(
        &self,
        scoped_type: Arc<ScopedType>,
        property_name: Spur,
    ) -> Option<String> {
        let interner = get_interner();

        eprintln!(
            "DEBUG: get_property_documentation called for property: {}",
            interner.resolve(&property_name)
        );

        // Handle complex properties (containing dots) by navigating to the last part
        let property_path = interner.resolve(&property_name);
        let final_property_name = if property_path.contains('.') {
            let parts: Vec<&str> = property_path.split('.').collect();
            if let Some(last_part) = parts.last() {
                interner.get_or_intern(last_part)
            } else {
                property_name
            }
        } else {
            property_name
        };

        eprintln!(
            "DEBUG: final_property_name: {}",
            interner.resolve(&final_property_name)
        );

        // Handle regular properties based on the current type
        match scoped_type.cwt_type_for_matching() {
            CwtTypeOrSpecialRef::Block(block) => {
                eprintln!(
                    "DEBUG: Found block type with {} properties",
                    block.properties.len()
                );

                // Debug: Show what type of block this is
                if let Some(type_name) = &block.type_name {
                    eprintln!("DEBUG: Block type name: {}", interner.resolve(type_name));
                } else {
                    eprintln!("DEBUG: Block has no type_name");
                }

                // First, check regular properties
                if let Some(property) = block.properties.get(&final_property_name) {
                    eprintln!("DEBUG: Found property in regular properties");
                    if let Some(doc_spur) = property.documentation {
                        let doc_text = interner.resolve(&doc_spur).to_string();
                        eprintln!("DEBUG: Found documentation: {}", doc_text);
                        return Some(doc_text);
                    } else {
                        eprintln!("DEBUG: Property found but no documentation");
                    }
                } else {
                    eprintln!("DEBUG: Property not found in regular properties");
                }

                // Check if there's a subtype-specific property
                eprintln!("DEBUG: Checking {} subtypes", scoped_type.subtypes().len());
                for subtype_name in scoped_type.subtypes() {
                    eprintln!(
                        "DEBUG: Checking subtype: {}",
                        interner.resolve(subtype_name)
                    );
                    if let Some(subtype) = block.subtypes.get(subtype_name) {
                        eprintln!(
                            "DEBUG: Found subtype with {} allowed properties",
                            subtype.allowed_properties.len()
                        );
                        if let Some(property) = subtype.allowed_properties.get(&final_property_name)
                        {
                            eprintln!("DEBUG: Found property in subtype allowed properties");
                            if let Some(doc_spur) = property.documentation {
                                let doc_text = interner.resolve(&doc_spur).to_string();
                                eprintln!("DEBUG: Found subtype documentation: {}", doc_text);
                                return Some(doc_text);
                            } else {
                                eprintln!("DEBUG: Subtype property found but no documentation");
                            }
                        } else {
                            eprintln!("DEBUG: Property not found in subtype allowed properties");
                        }
                    } else {
                        eprintln!("DEBUG: Subtype not found in block");
                    }
                }

                eprintln!("DEBUG: No documentation found in any location");
                None
            }
            _ => {
                eprintln!("DEBUG: Not a block type");
                None
            }
        }
    }
}
