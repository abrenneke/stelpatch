//! Core converter for CWT values to CwtType
//!
//! This module provides utilities for converting CWT AST values to our CwtType system.

use cw_parser::{
    AstCwtExpression, AstCwtIdentifierOrString,
    cwt::{
        AstCwtBlock, AstCwtIdentifier, CwtReferenceType, CwtSimpleValue, CwtSimpleValueType,
        CwtValue,
    },
};
use std::collections::HashMap;

use crate::{
    BlockType, CwtOptions, CwtType, PatternProperty, PatternType, Property, ReferenceType,
    SimpleType,
};

/// Converter for CWT values to CwtType
pub struct CwtConverter;

impl CwtConverter {
    /// Convert a CWT simple value to our type system
    pub fn convert_simple_value(simple: &CwtSimpleValue) -> CwtType {
        let primitive_type = match simple.value_type {
            CwtSimpleValueType::Bool => SimpleType::Bool,
            CwtSimpleValueType::Int => SimpleType::Int,
            CwtSimpleValueType::Float => SimpleType::Float,
            CwtSimpleValueType::Scalar => SimpleType::Scalar,
            CwtSimpleValueType::PercentageField => SimpleType::PercentageField,
            CwtSimpleValueType::Localisation => SimpleType::Localisation,
            CwtSimpleValueType::LocalisationSynced => SimpleType::LocalisationSynced,
            CwtSimpleValueType::LocalisationInline => SimpleType::LocalisationInline,
            CwtSimpleValueType::DateField => SimpleType::DateField,
            CwtSimpleValueType::VariableField => SimpleType::VariableField,
            CwtSimpleValueType::IntVariableField => SimpleType::IntVariableField,
            CwtSimpleValueType::ValueField => SimpleType::ValueField,
            CwtSimpleValueType::IntValueField => SimpleType::IntValueField,
            CwtSimpleValueType::ScopeField => SimpleType::ScopeField,
            CwtSimpleValueType::Filepath => SimpleType::Filepath,
            CwtSimpleValueType::Icon => SimpleType::Icon,
        };

        CwtType::Simple(primitive_type)
    }

    /// Convert a CWT identifier to our type system
    pub fn convert_identifier(identifier: &AstCwtIdentifier) -> CwtType {
        let reference_type = match &identifier.identifier_type {
            CwtReferenceType::TypeRef => ReferenceType::Type {
                key: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::TypeRefWithPrefixSuffix(prefix, suffix) => {
                ReferenceType::TypeWithAffix {
                    key: identifier.name.raw_value().to_string(),
                    prefix: Some(prefix.to_string()),
                    suffix: Some(suffix.to_string()),
                }
            }
            CwtReferenceType::Enum => ReferenceType::Enum {
                key: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::Scope => ReferenceType::Scope {
                key: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::Value => ReferenceType::Value {
                key: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::ValueSet => ReferenceType::ValueSet {
                key: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::ScopeGroup => ReferenceType::ScopeGroup {
                key: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::Alias => ReferenceType::Alias {
                key: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::AliasName => ReferenceType::AliasName {
                key: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::AliasMatchLeft => ReferenceType::AliasMatchLeft {
                key: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::SingleAlias => ReferenceType::SingleAlias {
                key: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::AliasKeysField => ReferenceType::AliasKeysField {
                key: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::Icon => ReferenceType::Icon {
                path: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::Filepath => ReferenceType::Filepath {
                path: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::Colour => ReferenceType::Colour {
                format: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::StellarisNameFormat => ReferenceType::StellarisNameFormat {
                key: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::Subtype => ReferenceType::Subtype {
                name: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::Type => ReferenceType::Type {
                key: identifier.name.raw_value().to_string(),
            },
            CwtReferenceType::ComplexEnum => ReferenceType::ComplexEnum {
                key: identifier.name.raw_value().to_string(),
            },
        };

        CwtType::Reference(reference_type)
    }

    /// Convert a CWT block to our type system
    pub fn convert_block(block: &AstCwtBlock, type_name: Option<String>) -> CwtType {
        let mut properties: HashMap<String, Property> = HashMap::new();
        let mut subtype_properties: HashMap<String, HashMap<String, Property>> = HashMap::new();
        let mut subtype_pattern_properties: HashMap<String, Vec<PatternProperty>> = HashMap::new();
        let mut pattern_properties = Vec::new();
        let mut union_values = Vec::new();

        // Process all items in the block normally
        for item in &block.items {
            match item {
                AstCwtExpression::Rule(rule) => {
                    match &rule.key {
                        AstCwtIdentifierOrString::Identifier(key_id) => {
                            match key_id.identifier_type {
                                CwtReferenceType::Enum => {
                                    let enum_key = key_id.name.raw_value().to_string();
                                    let value_type = Self::convert_value(&rule.value, None);

                                    pattern_properties.push(PatternProperty {
                                        pattern_type: PatternType::Enum {
                                            key: enum_key.clone(),
                                        },
                                        value_type: value_type.clone(),
                                        options: Default::default(),
                                        documentation: None,
                                    });

                                    continue;
                                }
                                CwtReferenceType::TypeRef => {
                                    let type_name = key_id.name.raw_value().to_string();
                                    let value_type = Self::convert_value(&rule.value, None);

                                    pattern_properties.push(PatternProperty {
                                        pattern_type: PatternType::Type {
                                            key: type_name.clone(),
                                        },
                                        value_type: value_type.clone(),
                                        options: Default::default(),
                                        documentation: None,
                                    });

                                    continue;
                                }
                                CwtReferenceType::AliasName => {
                                    match &key_id.name.key {
                                        // Handle alias_name[foo:<type_name>] = bar
                                        AstCwtIdentifierOrString::Identifier(_) => {
                                            panic!(
                                                "alias_name[foo:<type_name>] = bar is not supported"
                                            );
                                        }
                                        // Handle alias[foo:x] = bar
                                        AstCwtIdentifierOrString::String(key_str) => {
                                            let value_type = Self::convert_value(&rule.value, None);
                                            pattern_properties.push(PatternProperty {
                                                pattern_type: PatternType::AliasName {
                                                    category: key_str.raw_value().to_string(),
                                                },
                                                value_type: value_type.clone(),

                                                // TODO!
                                                options: Default::default(),
                                                documentation: None,
                                            });
                                            continue;
                                        }
                                    }
                                }
                                CwtReferenceType::Subtype => {
                                    let value_type = Self::convert_value(&rule.value, None);

                                    let subtype_name = if key_id.is_not {
                                        format!("!{}", key_id.name.raw_value().to_string())
                                    } else {
                                        key_id.name.raw_value().to_string()
                                    };

                                    let subtype_map =
                                        subtype_properties.entry(subtype_name.clone()).or_default();
                                    let subtype_patterns = subtype_pattern_properties
                                        .entry(subtype_name.clone())
                                        .or_default();

                                    if let CwtType::Block(block) = value_type {
                                        // Extract regular properties
                                        for (key, value) in block.properties.iter() {
                                            subtype_map.insert(
                                                key.clone(),
                                                Property {
                                                    property_type: value.property_type.clone(),
                                                    options: value.options.clone(),
                                                    documentation: value.documentation.clone(),
                                                },
                                            );
                                        }

                                        // Extract pattern properties
                                        for pattern_prop in block.pattern_properties.iter() {
                                            subtype_patterns.push(PatternProperty {
                                                pattern_type: pattern_prop.pattern_type.clone(),
                                                value_type: pattern_prop.value_type.clone(),
                                                options: pattern_prop.options.clone(),
                                                documentation: pattern_prop.documentation.clone(),
                                            });
                                        }
                                    } else {
                                        eprintln!("Expected block type, got {:?}", value_type);
                                    }

                                    continue;
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }

                    let options = CwtOptions::from_rule(rule);

                    let key = rule.key.name();
                    let value_type = Self::convert_value(&rule.value, None);
                    let property_def = Property {
                        property_type: value_type,
                        options,
                        documentation: None,
                    };

                    // Handle duplicate keys by creating unions
                    let key_string = key.to_string();
                    if let Some(existing_property) = properties.get(&key_string) {
                        // Key already exists, create a union
                        let union_type = match &existing_property.property_type {
                            CwtType::Union(existing_types) => {
                                // Already a union, add the new type to it
                                let mut new_types = existing_types.clone();
                                new_types.push(property_def.property_type);
                                CwtType::Union(new_types)
                            }
                            existing_type => {
                                // Not a union yet, create one with both types
                                CwtType::Union(vec![
                                    existing_type.clone(),
                                    property_def.property_type,
                                ])
                            }
                        };

                        let unified_property = Property {
                            property_type: union_type,
                            options: CwtOptions::default(),
                            documentation: None,
                        };
                        properties.insert(key_string, unified_property);
                    } else {
                        // Key doesn't exist yet, insert normally
                        properties.insert(key_string, property_def);
                    }
                }
                AstCwtExpression::Value(value) => {
                    let value_type = Self::convert_value(value, None);
                    union_values.push(value_type);
                }
                AstCwtExpression::Identifier(id) => {
                    // Handle identifiers in blocks
                    let value = id.name.raw_value().to_string();
                    let property_def = Property {
                        property_type: CwtType::Literal(value.clone()),
                        options: CwtOptions::default(),
                        documentation: None,
                    };
                    properties.insert(value, property_def);
                }
                _ => {
                    // Handle other expression types as needed
                }
            }
        }

        CwtType::Block(BlockType {
            type_name: type_name.unwrap_or_default(),
            properties,
            subtypes: HashMap::new(),
            subtype_properties,
            subtype_pattern_properties,
            pattern_properties,
            localisation: None,
            modifiers: None,
            additional_flags: union_values,
        })
    }

    /// Convert a CWT value to our type system
    pub fn convert_value(value: &CwtValue, type_name: Option<String>) -> CwtType {
        match value {
            CwtValue::Simple(simple) => Self::convert_simple_value(simple),
            CwtValue::Identifier(identifier) => Self::convert_identifier(identifier),
            CwtValue::Block(block) => Self::convert_block(block, type_name),
            CwtValue::String(s) => CwtType::Literal(s.raw_value().to_string()),
        }
    }
}
