//! Core converter for CWT values to CwtType
//!
//! This module provides utilities for converting CWT AST values to our CwtType system.

use cw_parser::{
    AstCwtIdentifierOrString,
    cwt::{
        AstCwtBlock, AstCwtIdentifier, CwtReferenceType, CwtSimpleValue, CwtSimpleValueType,
        CwtValue,
    },
};
use std::collections::HashMap;

use crate::{BlockType, CwtOptions, CwtType, Property, ReferenceType, SimpleType, TypeFingerprint};

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
    pub fn convert_block(block: &AstCwtBlock) -> CwtType {
        let mut properties: HashMap<String, Property> = HashMap::new();
        let mut alias_patterns = HashMap::new();
        let mut enum_patterns = HashMap::new();
        let mut union_values = Vec::new();

        // Process all items in the block normally
        for item in &block.items {
            match item {
                cw_parser::cwt::AstCwtExpression::Rule(rule) => {
                    match &rule.key {
                        AstCwtIdentifierOrString::Identifier(key_id) => {
                            match key_id.identifier_type {
                                CwtReferenceType::Enum => {
                                    let enum_key = key_id.name.raw_value().to_string();
                                    let value_type = Self::convert_value(&rule.value);
                                    enum_patterns.insert(enum_key, value_type);
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
                                            let value_type = Self::convert_value(&rule.value);
                                            alias_patterns.insert(
                                                key_str.raw_value().to_string(),
                                                value_type,
                                            );
                                            continue;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }

                    let key = rule.key.name();
                    let value_type = Self::convert_value(&rule.value);
                    let property_def = Property {
                        property_type: value_type,
                        options: CwtOptions::default(),
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
                cw_parser::cwt::AstCwtExpression::Value(value) => {
                    let value_type = Self::convert_value(value);
                    union_values.push(value_type);
                }
                cw_parser::cwt::AstCwtExpression::Identifier(id) => {
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

        if !union_values.is_empty() {
            union_values.dedup_by(|a, b| a.fingerprint() == b.fingerprint());

            return CwtType::Union(union_values);
        }

        CwtType::Block(BlockType {
            properties,
            subtypes: HashMap::new(),
            alias_patterns,
            enum_patterns,
            pattern_properties: Vec::new(),
            localisation: None,
            modifiers: None,
        })
    }

    /// Convert a CWT value to our type system
    pub fn convert_value(value: &CwtValue) -> CwtType {
        match value {
            CwtValue::Simple(simple) => Self::convert_simple_value(simple),
            CwtValue::Identifier(identifier) => Self::convert_identifier(identifier),
            CwtValue::Block(block) => Self::convert_block(block),
            CwtValue::String(s) => CwtType::Literal(s.raw_value().to_string()),
        }
    }
}
