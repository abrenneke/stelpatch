use cw_model::types::CwtAnalyzer;
use cw_model::{BlockType, CwtOptions, CwtType, Property, ReferenceType, SimpleType};
use std::collections::{HashMap, HashSet};

/// Result of type resolution that includes both the resolved type and display information
#[derive(Debug, Clone)]
pub struct ResolvedType {
    pub cwt_type: CwtType,
    pub display_info: Option<ResolvedDisplayInfo>,
}

/// Additional display information for resolved types
#[derive(Debug, Clone)]
pub struct ResolvedDisplayInfo {
    pub enum_values: Option<Vec<String>>,
    pub value_set: Option<Vec<String>>,
    pub alias_names: Option<Vec<String>>,
    pub alias_value_types: Option<Vec<String>>,
    pub is_resolved_reference: bool,
}

/// Expand patterns in a block type
/// This handles both enum patterns and alias patterns
fn expand_patterns_in_block(block_type: &mut BlockType, cwt_analyzer: &CwtAnalyzer) {
    let mut new_properties = HashMap::new();

    // Process each enum pattern
    for (enum_key, value_type) in &block_type.enum_patterns {
        if let Some(enum_def) = cwt_analyzer.get_enum(enum_key) {
            // Create a property for each enum value
            for enum_value in &enum_def.values {
                let new_property = Property {
                    property_type: value_type.clone(),
                    options: CwtOptions::default(),
                    documentation: Some(format!("Enum value from {}", enum_key)),
                };
                new_properties.insert(enum_value.clone(), new_property);
            }
        }
    }

    // Process each alias pattern
    for (alias_key, value_type) in &block_type.alias_patterns {
        // Get all aliases from this category and create properties for them
        for (alias_key_full, alias_def) in cwt_analyzer.get_aliases() {
            if let Some((cat, name)) = alias_key_full.split_once(':') {
                if cat == alias_key {
                    // For alias patterns, we need to resolve the value_type
                    let resolved_value_type = match value_type {
                        CwtType::Reference(ReferenceType::AliasMatchLeft { key })
                            if key == alias_key =>
                        {
                            // This is alias_name[X] = alias_match_left[X] - use the alias definition
                            alias_def.rules.clone()
                        }
                        _ => value_type.clone(),
                    };

                    let new_property = Property {
                        property_type: resolved_value_type,
                        options: CwtOptions::default(),
                        documentation: Some(format!("Alias from {} category", alias_key)),
                    };
                    new_properties.insert(name.to_string(), new_property);
                }
            }
        }
    }

    // Add all expanded properties
    block_type.properties.extend(new_properties);
}

/// Resolve a type to its actual concrete type
/// This handles references and other indirect types
pub fn resolve_type(cwt_type: &CwtType, cwt_analyzer: &CwtAnalyzer) -> CwtType {
    let result = match cwt_type {
        // For references, try to resolve to the actual type
        CwtType::Reference(ref_type) => {
            match ref_type {
                ReferenceType::Type { key } => {
                    // Try to find the referenced type in our analyzer
                    if let Some(resolved_type) = cwt_analyzer.get_type(key) {
                        resolve_type(&resolved_type.rules, cwt_analyzer)
                    } else if let Some(resolved_type) = cwt_analyzer.get_single_alias(key) {
                        resolve_type(resolved_type, cwt_analyzer)
                    } else {
                        // If we can't resolve it, return the original reference
                        cwt_type.clone()
                    }
                }
                ReferenceType::Alias { key } => {
                    // Try to resolve alias references
                    if let Some(alias) = cwt_analyzer.get_alias(key) {
                        resolve_type(&alias.rules, cwt_analyzer)
                    } else if let Some(resolved_type) = cwt_analyzer.get_single_alias(key) {
                        resolve_type(resolved_type, cwt_analyzer)
                    } else {
                        cwt_type.clone()
                    }
                }
                ReferenceType::AliasName { key } => {
                    // For alias_name, create a block type with all aliases from this category as properties
                    create_alias_category_block(cwt_analyzer, key)
                }
                ReferenceType::AliasMatchLeft { key } => {
                    // For alias_match_left, we want to represent the value types of aliases from this category
                    let union_types = get_alias_value_types_from_category(cwt_analyzer, key);
                    if !union_types.is_empty() {
                        if union_types.len() == 1 {
                            union_types.into_iter().next().unwrap()
                        } else {
                            CwtType::Union(union_types)
                        }
                    } else {
                        cwt_type.clone()
                    }
                }
                ReferenceType::SingleAlias { key } => {
                    // Try to resolve single alias references
                    if let Some(resolved_type) = cwt_analyzer.get_single_alias(key) {
                        resolve_type(resolved_type, cwt_analyzer)
                    } else {
                        cwt_type.clone()
                    }
                }
                ReferenceType::Enum { key } => {
                    // Try to get the enum type from our analyzer
                    if let Some(enum_def) = cwt_analyzer.get_enum(key) {
                        CwtType::LiteralSet(enum_def.values.clone())
                    } else {
                        cwt_type.clone()
                    }
                }
                ReferenceType::ValueSet { key } => {
                    // Try to get the value set type from our analyzer
                    if let Some(value_set) = cwt_analyzer.get_value_set(key) {
                        CwtType::LiteralSet(value_set.clone())
                    } else {
                        cwt_type.clone()
                    }
                }
                ReferenceType::Value { key } => {
                    // Try to resolve value references
                    if let Some(resolved_type) = cwt_analyzer.get_value_set(key) {
                        CwtType::LiteralSet(resolved_type.clone())
                    } else {
                        cwt_type.clone()
                    }
                }
                ReferenceType::ComplexEnum { key } => {
                    // Try to get the enum type from our analyzer
                    if let Some(enum_def) = cwt_analyzer.get_enum(key) {
                        CwtType::LiteralSet(enum_def.values.clone())
                    } else {
                        cwt_type.clone()
                    }
                }
                ReferenceType::AliasKeysField { key } => {
                    // Try to resolve alias keys field references
                    if let Some(resolved_type) = cwt_analyzer.get_single_alias(key) {
                        resolve_type(resolved_type, cwt_analyzer)
                    } else {
                        cwt_type.clone()
                    }
                }
                ReferenceType::Subtype { name } => {
                    // For subtypes, we can't resolve them without more context
                    // Return a descriptive type instead
                    CwtType::Literal(format!("subtype:{}", name))
                }
                // For primitive-like references, return appropriate simple types
                ReferenceType::Colour { .. } => CwtType::Simple(SimpleType::Color),
                ReferenceType::Icon { .. } => CwtType::Simple(SimpleType::Icon),
                ReferenceType::Filepath { .. } => CwtType::Simple(SimpleType::Filepath),
                ReferenceType::StellarisNameFormat { .. } => {
                    CwtType::Simple(SimpleType::Localisation)
                }
                ReferenceType::Scope { .. } => CwtType::Simple(SimpleType::ScopeField),
                ReferenceType::ScopeGroup { .. } => CwtType::Simple(SimpleType::ScopeField),
                // For any remaining unhandled reference types, return the original
                _ => cwt_type.clone(),
            }
        }
        // For comparables, unwrap to the base type
        CwtType::Comparable(base_type) => resolve_type(base_type, cwt_analyzer),
        // For blocks, resolve and expand patterns
        CwtType::Block(block_type) => {
            let mut resolved_block = block_type.clone();
            expand_patterns_in_block(&mut resolved_block, cwt_analyzer);
            CwtType::Block(resolved_block)
        }
        // For all other types, return as-is
        _ => cwt_type.clone(),
    };

    result
}

/// Create a block type with all aliases from a category as properties
fn create_alias_category_block(cwt_analyzer: &CwtAnalyzer, category: &str) -> CwtType {
    let mut properties = HashMap::new();

    // Get all aliases from this category and create properties for them
    for (alias_key, alias_def) in cwt_analyzer.get_aliases() {
        if let Some((cat, name)) = alias_key.split_once(':') {
            if cat == category {
                // DON'T recursively resolve - just use the alias definition directly
                // This prevents stack overflow when aliases contain alias_name[same_category]
                let property = Property {
                    property_type: alias_def.rules.clone(),
                    options: CwtOptions::default(),
                    documentation: Some(format!("Alias from {} category", category)),
                };
                properties.insert(name.to_string(), property);
            }
        }
    }

    if properties.is_empty() {
        // If no aliases found, fall back to literal set of names
        let alias_names = get_alias_names_from_category(cwt_analyzer, category);
        CwtType::LiteralSet(alias_names)
    } else {
        CwtType::Block(BlockType {
            properties,
            subtypes: HashMap::new(),
            alias_patterns: HashMap::new(),
            enum_patterns: HashMap::new(),
            localisation: None,
            modifiers: None,
        })
    }
}

/// Get all alias names from a specific category
fn get_alias_names_from_category(cwt_analyzer: &CwtAnalyzer, category: &str) -> HashSet<String> {
    let mut alias_names = HashSet::new();

    // Look for aliases that match the category (format: "category:name")
    for alias_key in cwt_analyzer.get_aliases().keys() {
        if let Some((cat, name)) = alias_key.split_once(':') {
            if cat == category {
                alias_names.insert(name.to_string());
            }
        }
    }

    alias_names
}

/// Get all value types from aliases in a specific category
fn get_alias_value_types_from_category(cwt_analyzer: &CwtAnalyzer, category: &str) -> Vec<CwtType> {
    let mut value_types = Vec::new();

    // Look for aliases that match the category (format: "category:name")
    for (alias_key, alias_def) in cwt_analyzer.get_aliases() {
        if let Some((cat, _name)) = alias_key.split_once(':') {
            if cat == category {
                // DON'T recursively resolve - just use the alias definition directly
                // This prevents stack overflow when aliases contain alias_name[same_category]
                value_types.push(alias_def.rules.clone());
            }
        }
    }

    // Remove duplicates by converting to a set-like structure
    value_types.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
    value_types.dedup();

    value_types
}

/// Resolve a type with display information for formatting
pub fn resolve_type_with_display_info(
    cwt_type: &CwtType,
    cwt_analyzer: &CwtAnalyzer,
    property_name: Option<&str>,
) -> ResolvedType {
    match cwt_type {
        CwtType::Reference(ref_type) => {
            match ref_type {
                ReferenceType::Enum { key } => {
                    if let Some(enum_def) = cwt_analyzer.get_enum(key) {
                        ResolvedType {
                            cwt_type: CwtType::LiteralSet(enum_def.values.clone()),
                            display_info: Some(ResolvedDisplayInfo {
                                enum_values: Some(enum_def.values.iter().cloned().collect()),
                                value_set: None,
                                alias_names: None,
                                alias_value_types: None,
                                is_resolved_reference: true,
                            }),
                        }
                    } else {
                        ResolvedType {
                            cwt_type: cwt_type.clone(),
                            display_info: None,
                        }
                    }
                }
                ReferenceType::ComplexEnum { key } => {
                    if let Some(enum_def) = cwt_analyzer.get_enum(key) {
                        ResolvedType {
                            cwt_type: CwtType::LiteralSet(enum_def.values.clone()),
                            display_info: Some(ResolvedDisplayInfo {
                                enum_values: Some(enum_def.values.iter().cloned().collect()),
                                value_set: None,
                                alias_names: None,
                                alias_value_types: None,
                                is_resolved_reference: true,
                            }),
                        }
                    } else {
                        ResolvedType {
                            cwt_type: cwt_type.clone(),
                            display_info: None,
                        }
                    }
                }
                ReferenceType::ValueSet { key } => {
                    if let Some(value_set) = cwt_analyzer.get_value_set(key) {
                        ResolvedType {
                            cwt_type: CwtType::LiteralSet(value_set.clone()),
                            display_info: Some(ResolvedDisplayInfo {
                                enum_values: None,
                                value_set: Some(value_set.iter().cloned().collect()),
                                alias_names: None,
                                alias_value_types: None,
                                is_resolved_reference: true,
                            }),
                        }
                    } else {
                        ResolvedType {
                            cwt_type: cwt_type.clone(),
                            display_info: None,
                        }
                    }
                }
                ReferenceType::AliasName { key } => {
                    // For AliasName, create a block with all aliases from this category
                    let block_type = create_alias_category_block(cwt_analyzer, key);
                    ResolvedType {
                        cwt_type: block_type,
                        display_info: Some(ResolvedDisplayInfo {
                            enum_values: None,
                            value_set: None,
                            alias_names: Some(
                                get_alias_names_from_category(cwt_analyzer, key)
                                    .into_iter()
                                    .collect(),
                            ),
                            alias_value_types: None,
                            is_resolved_reference: true,
                        }),
                    }
                }
                ReferenceType::AliasMatchLeft { key } => {
                    resolve_alias_match_left_with_property_context(key, cwt_analyzer, property_name)
                }
                ReferenceType::Alias { key } => {
                    if let Some(alias) = cwt_analyzer.get_alias(key) {
                        resolve_type_with_display_info(&alias.rules, cwt_analyzer, property_name)
                    } else if let Some(resolved_type) = cwt_analyzer.get_single_alias(key) {
                        resolve_type_with_display_info(resolved_type, cwt_analyzer, property_name)
                    } else {
                        ResolvedType {
                            cwt_type: cwt_type.clone(),
                            display_info: None,
                        }
                    }
                }
                ReferenceType::SingleAlias { key } => {
                    if let Some(resolved_type) = cwt_analyzer.get_single_alias(key) {
                        resolve_type_with_display_info(resolved_type, cwt_analyzer, property_name)
                    } else {
                        ResolvedType {
                            cwt_type: cwt_type.clone(),
                            display_info: None,
                        }
                    }
                }
                ReferenceType::AliasKeysField { key } => {
                    if let Some(resolved_type) = cwt_analyzer.get_single_alias(key) {
                        resolve_type_with_display_info(resolved_type, cwt_analyzer, property_name)
                    } else {
                        ResolvedType {
                            cwt_type: cwt_type.clone(),
                            display_info: None,
                        }
                    }
                }
                _ => {
                    // For other reference types, use basic resolution
                    ResolvedType {
                        cwt_type: resolve_type(cwt_type, cwt_analyzer),
                        display_info: Some(ResolvedDisplayInfo {
                            enum_values: None,
                            value_set: None,
                            alias_names: None,
                            alias_value_types: None,
                            is_resolved_reference: true,
                        }),
                    }
                }
            }
        }
        _ => {
            // For non-reference types, handle blocks specially to expand patterns
            match cwt_type {
                CwtType::Block(block_type) => {
                    let mut resolved_block = block_type.clone();
                    expand_patterns_in_block(&mut resolved_block, cwt_analyzer);
                    ResolvedType {
                        cwt_type: CwtType::Block(resolved_block),
                        display_info: None,
                    }
                }
                _ => {
                    // For other non-reference types, just return as-is
                    ResolvedType {
                        cwt_type: cwt_type.clone(),
                        display_info: None,
                    }
                }
            }
        }
    }
}

/// Resolve alias_name with property context (handles namespaced aliases)
fn resolve_alias_name_with_property_context(
    key: &str,
    cwt_analyzer: &CwtAnalyzer,
    property_name: Option<&str>,
) -> ResolvedType {
    // If we have a property name, try the namespaced version first
    if let Some(prop_name) = property_name {
        let namespaced_key = format!("{}:{}", key, prop_name);

        if let Some(alias) = cwt_analyzer.get_alias(&namespaced_key) {
            return resolve_type_with_display_info(&alias.rules, cwt_analyzer, property_name);
        } else if let Some(resolved_type) = cwt_analyzer.get_single_alias(&namespaced_key) {
            return resolve_type_with_display_info(resolved_type, cwt_analyzer, property_name);
        }
    }

    // Create a block with all aliases from this category
    let block_type = create_alias_category_block(cwt_analyzer, key);
    ResolvedType {
        cwt_type: block_type,
        display_info: Some(ResolvedDisplayInfo {
            enum_values: None,
            value_set: None,
            alias_names: Some(
                get_alias_names_from_category(cwt_analyzer, key)
                    .into_iter()
                    .collect(),
            ),
            alias_value_types: None,
            is_resolved_reference: true,
        }),
    }
}

/// Resolve alias_match_left with property context
fn resolve_alias_match_left_with_property_context(
    key: &str,
    cwt_analyzer: &CwtAnalyzer,
    property_name: Option<&str>,
) -> ResolvedType {
    // If we have a property name, try the namespaced version first
    if let Some(prop_name) = property_name {
        let namespaced_key = format!("{}:{}", key, prop_name);

        if let Some(alias) = cwt_analyzer.get_alias(&namespaced_key) {
            return resolve_type_with_display_info(&alias.rules, cwt_analyzer, property_name);
        } else if let Some(resolved_type) = cwt_analyzer.get_single_alias(&namespaced_key) {
            return resolve_type_with_display_info(resolved_type, cwt_analyzer, property_name);
        }
    }

    // Get all value types from aliases in this category
    let value_types = get_alias_value_types_from_category(cwt_analyzer, key);

    if !value_types.is_empty() {
        let resolved_type = if value_types.len() == 1 {
            value_types.clone().into_iter().next().unwrap()
        } else {
            CwtType::Union(value_types.clone())
        };

        // Convert types to string representations for display
        let type_strings: Vec<String> = value_types
            .iter()
            .map(|t| format!("{:?}", t)) // Simple debug representation for now
            .collect();

        ResolvedType {
            cwt_type: resolved_type,
            display_info: Some(ResolvedDisplayInfo {
                enum_values: None,
                value_set: None,
                alias_names: None,
                alias_value_types: Some(type_strings),
                is_resolved_reference: true,
            }),
        }
    } else {
        // Fallback to original behavior
        if let Some(resolved_type) = cwt_analyzer.get_single_alias(key) {
            resolve_type_with_display_info(resolved_type, cwt_analyzer, property_name)
        } else if let Some(alias) = cwt_analyzer.get_alias(key) {
            resolve_type_with_display_info(&alias.rules, cwt_analyzer, property_name)
        } else {
            ResolvedType {
                cwt_type: CwtType::Reference(ReferenceType::AliasMatchLeft {
                    key: key.to_string(),
                }),
                display_info: None,
            }
        }
    }
}
