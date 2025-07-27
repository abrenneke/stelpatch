use std::sync::Arc;

use cw_model::{AliasName, CwtAnalyzer, ReferenceType};

use crate::{
    handlers::{
        cache::{
            PatternMatcher, ResolverUtils,
            resolver_modules::properties::links::get_scope_link_properties,
        },
        scoped_type::{CwtTypeOrSpecialRef, ScopedType},
    },
    interner::get_interner,
};

/// Get all available property names for a scoped type
pub fn get_available_properties(
    pattern_matcher: Arc<PatternMatcher>,
    cwt_analyzer: Arc<CwtAnalyzer>,
    utils: Arc<ResolverUtils>,
    scoped_type: Arc<ScopedType>,
) -> Vec<String> {
    let mut properties: Vec<String> = Vec::new();
    let interner = get_interner();

    match scoped_type.cwt_type_for_matching() {
        CwtTypeOrSpecialRef::Block(block) => {
            // Add subtype-specific properties first
            for subtype_name in scoped_type.subtypes() {
                if let Some(subtype_def) = block.subtypes.get(subtype_name) {
                    properties.extend(
                        subtype_def
                            .allowed_properties
                            .keys()
                            .map(|k| interner.resolve(&k).to_string()),
                    );
                }
            }

            // Add regular properties
            properties.extend(
                block
                    .properties
                    .keys()
                    .map(|k| interner.resolve(&k).to_string()),
            );

            // Add pattern properties (get completions)
            for pattern_property in &block.pattern_properties {
                let completions =
                    pattern_matcher.get_pattern_completions(&pattern_property.pattern_type);
                properties.extend(completions);
            }
        }
        CwtTypeOrSpecialRef::Reference(ReferenceType::AliasMatchLeft { key }) => {
            // For alias_match_left[category], return all possible alias names from that category
            if let Some(aliases_in_category) =
                cwt_analyzer.get_aliases_for_category(interner.get_or_intern(key))
            {
                for alias_pattern in aliases_in_category {
                    match &alias_pattern.name {
                        AliasName::Static(name) => {
                            properties.push(interner.resolve(name).to_string());
                        }
                        AliasName::TypeRef(type_name) => {
                            if let Some(namespace_keys) =
                                utils.get_namespace_keys_for_type_ref(*type_name)
                            {
                                properties.extend(
                                    namespace_keys
                                        .iter()
                                        .cloned()
                                        .map(|k| interner.resolve(&k).to_string()),
                                );
                            }
                        }
                        AliasName::Enum(enum_name) => {
                            if let Some(enum_def) = cwt_analyzer.get_enum(*enum_name) {
                                properties.extend(
                                    enum_def
                                        .values
                                        .iter()
                                        .cloned()
                                        .map(|k| interner.resolve(&k).to_string()),
                                );
                            }
                        }
                        AliasName::TypeRefWithPrefixSuffix(type_name, prefix, suffix) => {
                            if let Some(namespace_keys) =
                                utils.get_namespace_keys_for_type_ref(*type_name)
                            {
                                for key in namespace_keys.iter() {
                                    let property = match (prefix, suffix) {
                                        (Some(p), Some(s)) => interner.get_or_intern(format!(
                                            "{}{}{}",
                                            interner.resolve(p),
                                            interner.resolve(key),
                                            interner.resolve(s)
                                        )),
                                        (Some(p), None) => interner.get_or_intern(format!(
                                            "{}{}",
                                            interner.resolve(p),
                                            interner.resolve(key)
                                        )),
                                        (None, Some(s)) => interner.get_or_intern(format!(
                                            "{}{}",
                                            interner.resolve(key),
                                            interner.resolve(s)
                                        )),
                                        (None, None) => *key,
                                    };
                                    properties.push(interner.resolve(&property).to_string());
                                }
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
    properties.extend(
        scope_properties
            .iter()
            .map(|k| interner.resolve(k).to_string()),
    );

    // Add link properties based on the current scope
    let current_scope = &scoped_type.scope_stack().current_scope().scope_type;
    let link_properties = get_scope_link_properties(cwt_analyzer.clone(), *current_scope);
    properties.extend(
        link_properties
            .iter()
            .map(|k| interner.resolve(k).to_string()),
    );

    properties.sort();
    properties.dedup();
    properties
}
