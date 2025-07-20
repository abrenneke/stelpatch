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
        _scope_stack: &ScopeStack,
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
                                } else {
                                    eprintln!(
                                        "No filtered keys found for: {}.{}, path: {}",
                                        base_type, subtype, path
                                    );
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
                        let namespace_keys = EntityRestructurer::get_namespace_entity_keys(&path);
                        found = Some(CwtType::LiteralSet(namespace_keys.into_iter().collect()));
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
            ReferenceType::Scope { .. } => {
                // Scope references need dynamic validation because values can contain
                // dotted paths like "prev.from" that can't be statically enumerated
                Arc::new(CwtType::Reference(ref_type.clone()))
            }
            ReferenceType::ScopeGroup { .. } => {
                // ScopeGroup references need dynamic validation because values can contain
                // dotted paths like "prev.from" that can't be statically enumerated
                Arc::new(CwtType::Reference(ref_type.clone()))
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

            ReferenceType::TypeWithAffix {
                key,
                prefix,
                suffix,
            } => {
                // First, resolve the base type the same way as ReferenceType::Type
                let base_ref = ReferenceType::Type { key: key.clone() };
                let base_type = self.resolve_reference_type(&base_ref, _scope_stack);

                // If we got a LiteralSet back, apply prefix and suffix to each element
                match base_type.as_ref() {
                    CwtType::LiteralSet(keys) => {
                        let affixed_keys: HashSet<String> = keys
                            .iter()
                            .map(|k| {
                                format!(
                                    "{}{}{}",
                                    prefix.as_deref().unwrap_or(""),
                                    k,
                                    suffix.as_deref().unwrap_or("")
                                )
                            })
                            .collect();
                        Arc::new(CwtType::LiteralSet(affixed_keys))
                    }
                    _ => {
                        // If we couldn't resolve to a literal set, return the original reference
                        Arc::new(CwtType::Reference(ref_type.clone()))
                    }
                }
            }
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
    ) -> (Arc<CwtType>, Option<AliasDefinition>, Option<String>) {
        // Look up the specific alias category:property_name and return its type
        if let Some(aliases_in_category) = self.cwt_analyzer.get_aliases_for_category(category) {
            for alias_pattern in aliases_in_category {
                if let Some(alias_def) = self.cwt_analyzer.get_alias(alias_pattern) {
                    match &alias_pattern.name {
                        AliasName::Static(name) => {
                            if name == property_name {
                                return (alias_def.to.clone(), Some(alias_def.clone()), None);
                            }
                        }
                        AliasName::TypeRef(type_name) => {
                            // Check if property_name is a valid key for this type
                            if let Some(namespace_keys) =
                                self.utils.get_namespace_keys_for_type_ref(type_name)
                            {
                                if namespace_keys.contains(&property_name.to_string()) {
                                    // Special case for scripted_effect - we need to know the name
                                    // of the scripted effect to set the scoped type context
                                    if type_name == "scripted_effect"
                                        || type_name == "scripted_trigger"
                                    {
                                        return (
                                            alias_def.to.clone(),
                                            Some(alias_def.clone()),
                                            Some(property_name.to_string()),
                                        );
                                    } else {
                                        return (
                                            alias_def.to.clone(),
                                            Some(alias_def.clone()),
                                            None,
                                        );
                                    }
                                }
                            }
                        }
                        AliasName::Enum(enum_name) => {
                            // Check if property_name is a valid enum value
                            if let Some(enum_def) = self.cwt_analyzer.get_enum(enum_name) {
                                if enum_def.values.contains(property_name) {
                                    return (alias_def.to.clone(), Some(alias_def.clone()), None);
                                }
                            }
                        }
                    }
                }
            }
        }

        // If no matching alias was found, return the original AliasMatchLeft
        (
            Arc::new(CwtType::Reference(ReferenceType::AliasMatchLeft {
                key: category.to_string(),
            })),
            None,
            None,
        )
    }
}
