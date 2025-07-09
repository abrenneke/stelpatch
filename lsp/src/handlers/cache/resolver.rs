use super::core::GameDataCache;
use cw_model::types::CwtAnalyzer;
use cw_model::{
    AliasName, AliasPattern, BlockType, CwtOptions, CwtType, Property, ReferenceType, SimpleType,
};
use std::collections::HashMap;

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

    if !GameDataCache::is_initialized() {
        return;
    }

    let game_data = GameDataCache::get().unwrap();

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
    for (alias_pattern, value_type) in &block_type.alias_patterns {
        // Get all aliases from this category and create properties for them
        for (alias_key_full, _) in cwt_analyzer.get_aliases() {
            if alias_key_full.category == *alias_pattern {
                match &alias_key_full.name {
                    // For alias[foo:x] = bar, we create a single property for each alias
                    AliasName::Static(name) => {
                        let new_property = Property {
                            property_type: resolve_type(value_type, cwt_analyzer),
                            options: CwtOptions::default(),
                            documentation: Some(format!("Alias from {} category", alias_pattern)),
                        };
                        new_properties.insert(name.to_string(), new_property);
                    }
                    // For alias[foo:<type_name>] = bar, we expand <type_name> to all types in the namespace
                    AliasName::TypeRef(name) => {
                        let all_types = game_data.get_namespace_keys(name);
                        if let Some(all_types) = all_types {
                            for type_key in all_types {
                                let new_property = Property {
                                    property_type: resolve_type(value_type, cwt_analyzer),
                                    options: CwtOptions::default(),
                                    documentation: Some(format!(
                                        "Alias from {} category",
                                        alias_pattern
                                    )),
                                };
                                new_properties.insert(type_key.clone(), new_property);
                            }
                        }
                    }
                    AliasName::Enum(name) => {
                        let all_enums = cwt_analyzer.get_enum(name);
                        if let Some(all_enums) = all_enums {
                            for enum_value in &all_enums.values {
                                let new_property = Property {
                                    property_type: value_type.clone(),
                                    options: CwtOptions::default(),
                                    documentation: Some(format!("Enum value from {}", name)),
                                };

                                new_properties.insert(enum_value.clone(), new_property);
                            }
                        }
                    }
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
                    let type_def = cwt_analyzer.get_type(key);

                    if let Some(type_def) = type_def {
                        if let Some(path) = type_def.path.as_ref() {
                            // CWT paths are prefixed with "game/"
                            let path = path.trim_start_matches("game/");

                            // For Type references, we want the union of all keys in that namespace
                            // This is what the user expects when they hover over "resource" - they want to see
                            // all the possible resource keys like "energy", "minerals", etc.
                            if let Some(game_data) = GameDataCache::get() {
                                if let Some(namespace_keys) = game_data.get_namespace_keys(&path) {
                                    return CwtType::LiteralSet(
                                        namespace_keys.iter().cloned().collect(),
                                    );
                                } else {
                                    eprintln!("Failed to resolve namespace: {}", path);
                                }

                                // Also try the key directly in case it's already a full path
                                if let Some(namespace_keys) = game_data.get_namespace_keys(key) {
                                    return CwtType::LiteralSet(
                                        namespace_keys.iter().cloned().collect(),
                                    );
                                }
                            } else {
                                eprintln!("Failed to resolve type: {}, no game data", key);
                            }
                        } else {
                            eprintln!("Failed to resolve type: {}, no path", key);
                        }
                    } else {
                        eprintln!("Failed to resolve type: {}, no type definition", key);
                    }

                    // If game data isn't available or namespace not found, return the original reference
                    cwt_type.clone()
                }
                ReferenceType::Alias { .. } => {
                    // Invalid alias[] on RHS
                    cwt_type.clone()
                }
                ReferenceType::AliasName { .. } => {
                    // Invalid alias_name on RHS
                    cwt_type.clone()
                }
                ReferenceType::AliasMatchLeft { key } => {
                    // For alias_match_left, we want to represent the value types of aliases from this category
                    let mut union_types = Vec::new();

                    // Look for aliases that match the category (format: "category:name")
                    for (alias_key, alias_def) in cwt_analyzer.get_aliases() {
                        if alias_key.category == *key {
                            // DON'T recursively resolve - just use the alias definition directly
                            union_types.push(alias_def.to.clone());
                        }
                    }

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
                ReferenceType::SingleAlias { .. } => {
                    // Invalid single_alias_name on RHS
                    cwt_type.clone()
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

/// Resolve a type with display information for formatting, with namespace context
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
                    // TODO
                    ResolvedType {
                        cwt_type: cwt_type.clone(),
                        display_info: None,
                    }
                }
                ReferenceType::AliasMatchLeft { key } => {
                    // TODO
                    ResolvedType {
                        cwt_type: cwt_type.clone(),
                        display_info: None,
                    }
                }
                ReferenceType::Alias { key } => {
                    // TODO
                    ResolvedType {
                        cwt_type: cwt_type.clone(),
                        display_info: None,
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
