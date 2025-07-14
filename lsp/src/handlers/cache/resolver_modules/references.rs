use super::{ResolverUtils, SubtypeHandler};
use crate::handlers::cache::{EntityRestructurer, FullAnalysis};
use crate::handlers::scope::ScopeStack;
use cw_model::types::CwtAnalyzer;
use cw_model::{AliasDefinition, AliasName, CwtType, ReferenceType, SimpleType};
use std::collections::HashSet;
use std::sync::Arc;

pub struct ReferenceResolver {
    pub cwt_analyzer: Arc<CwtAnalyzer>,
    pub utils: Arc<ResolverUtils>,
    pub subtype_handler: Arc<SubtypeHandler>,
}

impl ReferenceResolver {
    pub fn new(
        cwt_analyzer: Arc<CwtAnalyzer>,
        utils: Arc<ResolverUtils>,
        subtype_handler: Arc<SubtypeHandler>,
    ) -> Self {
        Self {
            cwt_analyzer,
            utils,
            subtype_handler,
        }
    }

    /// Resolves references & nested types to concrete types
    pub fn resolve_reference_type(
        &self,
        ref_type: &ReferenceType,
        scope_stack: &ScopeStack,
    ) -> Arc<CwtType> {
        match ref_type {
            ReferenceType::Type { key } => {
                // Check if this is a subtype reference (contains a dot)
                if let Some(dot_pos) = key.find('.') {
                    let (base_type, subtype) = key.split_at(dot_pos);
                    let subtype = &subtype[1..]; // Remove the leading dot

                    // Get the base type definition
                    let type_def = self.cwt_analyzer.get_type(base_type);

                    if let Some(type_def) = type_def {
                        if let Some(path) = type_def.path.as_ref() {
                            // CWT paths are prefixed with "game/"
                            let path = path.trim_start_matches("game/");

                            // Get the CWT type for this namespace
                            if let Some(cwt_type) = self.cwt_analyzer.get_type(base_type) {
                                // Use subtype handler to filter entities by subtype
                                let filtered_keys = self
                                    .subtype_handler
                                    .get_entity_keys_in_namespace_for_subtype(
                                        path,
                                        &cwt_type.rules,
                                        subtype,
                                    );

                                if !filtered_keys.is_empty() {
                                    return Arc::new(CwtType::LiteralSet(
                                        filtered_keys.into_iter().collect(),
                                    ));
                                }
                            }
                        }
                    }

                    // If subtype filtering failed, return the original reference
                    return Arc::new(CwtType::Reference(ref_type.clone()));
                }

                // Handle regular type references (no subtype)
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
                            EntityRestructurer::get_namespace_entity_keys(&path)
                        {
                            found = Some(CwtType::LiteralSet(namespace_keys.into_iter().collect()));
                        }

                        // Also try the key directly in case it's already a full path
                        if found.is_none() {
                            if let Some(namespace_keys) =
                                EntityRestructurer::get_namespace_entity_keys(key)
                            {
                                found =
                                    Some(CwtType::LiteralSet(namespace_keys.into_iter().collect()));
                            }
                        }
                    }
                }

                if let Some(found) = found {
                    return Arc::new(found);
                }

                // If game data isn't available or namespace not found, return the original reference
                Arc::new(CwtType::Reference(ref_type.clone()))
            }
            ReferenceType::Alias { .. } => {
                // Invalid alias[] on RHS
                Arc::new(CwtType::Reference(ref_type.clone()))
            }
            ReferenceType::AliasName { .. } => {
                // Invalid alias_name on RHS
                Arc::new(CwtType::Reference(ref_type.clone()))
            }
            ReferenceType::AliasMatchLeft { .. } => {
                // alias_match_left[category] cannot be resolved statically because it depends
                // on the key being passed in at runtime. This is like TypeScript's T[P] where
                // we need to know P to resolve the type.
                // The resolution must happen during property navigation.
                Arc::new(CwtType::Reference(ref_type.clone()))
            }
            ReferenceType::SingleAlias { .. } => {
                // Invalid single_alias_name on RHS
                Arc::new(CwtType::Reference(ref_type.clone()))
            }
            ReferenceType::Enum { key } => {
                // Try to get the enum type from our analyzer
                if let Some(enum_def) = self.cwt_analyzer.get_enum(key) {
                    let mut values = enum_def.values.clone();

                    // Also include complex enum values if available
                    if let Some(full_analysis) = FullAnalysis::get() {
                        if let Some(complex_values) = full_analysis.complex_enums.get(key) {
                            values.extend(complex_values.clone());
                        }
                    }

                    Arc::new(CwtType::LiteralSet(values))
                } else {
                    Arc::new(CwtType::Reference(ref_type.clone()))
                }
            }
            ReferenceType::ValueSet { .. } => Arc::new(CwtType::Reference(ref_type.clone())),
            ReferenceType::Value { key } => {
                if let Some(full_analysis) = FullAnalysis::get() {
                    if let Some(dynamic_values) = full_analysis.dynamic_value_sets.get(key) {
                        return Arc::new(CwtType::LiteralSet(dynamic_values.clone()));
                    }
                }

                Arc::new(CwtType::Reference(ref_type.clone()))
            }
            ReferenceType::ComplexEnum { .. } => {
                // Invalid complex_enum on RHS
                Arc::new(CwtType::Reference(ref_type.clone()))
            }
            ReferenceType::Subtype { name } => {
                // For subtypes, we need to look up the subtype definition
                // This is typically used in contexts where we know the base type
                // but need to specialize based on the subtype
                // For now, return a descriptive literal that can be used for completion
                Arc::new(CwtType::Literal(format!("subtype:{}", name)))
            }
            ReferenceType::Scope { key } => {
                // If "any", then _any_ link or scope property is valid from the current scope
                if key == "any" {
                    let current_scope = &scope_stack.current_scope().scope_type;
                    let mut properties = self.get_scope_link_properties(current_scope);

                    // Add available scope properties (handles "unknown" scopes automatically)
                    properties.extend(scope_stack.available_scope_names());

                    Arc::new(CwtType::LiteralSet(properties.into_iter().collect()))
                } else {
                    let properties =
                        self.get_properties_for_scope_names(vec![key.clone()], scope_stack);
                    Arc::new(CwtType::LiteralSet(properties))
                }
            }
            ReferenceType::ScopeGroup { key } => {
                if let Some(scope_group) = self.cwt_analyzer.get_scope_group(key) {
                    let properties = self
                        .get_properties_for_scope_names(scope_group.members.clone(), scope_stack);
                    Arc::new(CwtType::LiteralSet(properties))
                } else {
                    Arc::new(CwtType::Reference(ref_type.clone()))
                }
            }
            ReferenceType::AliasKeysField { key } => {
                let mut properties = HashSet::new();
                if let Some(aliases_in_category) = self.cwt_analyzer.get_aliases_for_category(key) {
                    for alias_pattern in aliases_in_category {
                        match &alias_pattern.name {
                            AliasName::Static(name) => {
                                properties.insert(name.clone());
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
                Arc::new(CwtType::LiteralSet(properties))
            }
            // Right now, inline_script validates to a string,
            // but eventually it should validate to a union of all possible paths to
            // inline script files
            ReferenceType::InlineScript => Arc::new(CwtType::Simple(SimpleType::Scalar)),
            // For any remaining unhandled reference types, return the original
            _ => Arc::new(CwtType::Reference(ref_type.clone())),
        }
    }

    /// Resolve an AliasMatchLeft reference using a specific property name
    /// Returns (resolved_type, alias_definition_if_found)
    pub fn resolve_alias_match_left(
        &self,
        category: &str,
        property_name: &str,
    ) -> (CwtType, Option<AliasDefinition>) {
        // Look up the specific alias category:property_name and return its type
        if let Some(aliases_in_category) = self.cwt_analyzer.get_aliases_for_category(category) {
            for alias_pattern in aliases_in_category {
                if let Some(alias_def) = self.cwt_analyzer.get_alias(alias_pattern) {
                    match &alias_pattern.name {
                        AliasName::Static(name) => {
                            if name == property_name {
                                return (alias_def.to.clone(), Some(alias_def.clone()));
                            }
                        }
                        AliasName::TypeRef(type_name) => {
                            // Check if property_name is a valid key for this type
                            if let Some(namespace_keys) =
                                self.utils.get_namespace_keys_for_type_ref(type_name)
                            {
                                if namespace_keys.contains(&property_name.to_string()) {
                                    return (alias_def.to.clone(), Some(alias_def.clone()));
                                }
                            }
                        }
                        AliasName::Enum(enum_name) => {
                            // Check if property_name is a valid enum value
                            if let Some(enum_def) = self.cwt_analyzer.get_enum(enum_name) {
                                if enum_def.values.contains(property_name) {
                                    return (alias_def.to.clone(), Some(alias_def.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }

        // If no matching alias was found, return the original AliasMatchLeft
        (
            CwtType::Reference(ReferenceType::AliasMatchLeft {
                key: category.to_string(),
            }),
            None,
        )
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

    /// Get properties for multiple scope names (unified logic for Scope and ScopeGroup)
    fn get_properties_for_scope_names(
        &self,
        scope_names: Vec<String>,
        scope_stack: &ScopeStack,
    ) -> HashSet<String> {
        let mut all_properties = HashSet::new();
        let current_scope = &scope_stack.current_scope().scope_type;

        // If current scope is "unknown", treat it as a fallback that can navigate anywhere
        let is_unknown_scope = current_scope == "unknown";

        // For each scope name
        for scope_name in scope_names {
            // Resolve the scope name to get the canonical name (e.g., "country" -> "Country")
            if let Some(resolved_scope_name) = self.cwt_analyzer.resolve_scope_name(&scope_name) {
                // Add link properties that are valid from the current scope and have the specified output scope
                for (link_name, link_def) in self.cwt_analyzer.get_links() {
                    // If scope is unknown, allow all links as fallback, otherwise use normal validation
                    if is_unknown_scope
                        || link_def.can_be_used_from(current_scope, &self.cwt_analyzer)
                    {
                        if let Some(link_output_scope) =
                            self.cwt_analyzer.resolve_scope_name(&link_def.output_scope)
                        {
                            if link_output_scope == resolved_scope_name {
                                all_properties.insert(link_name.clone());
                            }
                        }
                    }
                }

                // Add scope properties that resolve to the specified scope type
                if is_unknown_scope {
                    // If scope is unknown, add all available scope properties as fallback
                    for scope_property in scope_stack.available_scope_names() {
                        all_properties.insert(scope_property);
                    }
                } else {
                    // Normal case: filter scope properties by target scope type
                    for scope_property in scope_stack.available_scope_names() {
                        if let Some(scope_context) = scope_stack.get_scope_by_name(&scope_property)
                        {
                            if let Some(scope_type) = self
                                .cwt_analyzer
                                .resolve_scope_name(&scope_context.scope_type)
                            {
                                if scope_type == resolved_scope_name {
                                    all_properties.insert(scope_property);
                                }
                            }
                        }
                    }
                }
            }
        }

        all_properties
    }
}
