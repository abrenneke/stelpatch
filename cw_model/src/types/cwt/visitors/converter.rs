//! Core converter for CWT values to CwtType
//!
//! This module provides utilities for converting CWT AST values to our CwtType system.

use cw_parser::cwt::{
    AstCwtBlock, AstCwtIdentifier, CwtReferenceType, CwtSimpleValue, CwtSimpleValueType, CwtValue,
};
use std::collections::{HashMap, HashSet};

use crate::{ArrayType, BlockType, CwtOptions, CwtType, Property, ReferenceType, SimpleType};

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
            _ => ReferenceType::Type {
                key: identifier.name.raw_value().to_string(),
            },
        };

        CwtType::Reference(reference_type)
    }

    /// Convert a CWT block to our type system
    pub fn convert_block(block: &AstCwtBlock) -> CwtType {
        let mut properties = HashMap::new();
        let mut union_values = Vec::new();
        let mut is_alias_context = false;
        let mut alias_type_key = None;

        // First pass: check for alias patterns
        for item in &block.items {
            if let cw_parser::cwt::AstCwtExpression::Rule(rule) = item {
                if Self::is_alias_pattern(rule) {
                    // Extract the alias type from the pattern
                    if let Some(extracted_type) = Self::extract_alias_type(rule) {
                        is_alias_context = true;
                        alias_type_key = Some(extracted_type);
                        break;
                    }
                }
            }
        }

        // If this is an alias context, return a reference to the alias type directly
        if is_alias_context {
            if let Some(type_key) = alias_type_key {
                return CwtType::Reference(ReferenceType::AliasName { key: type_key });
            }
        }

        // Process all items in the block normally
        for item in &block.items {
            match item {
                cw_parser::cwt::AstCwtExpression::Rule(rule) => {
                    // Skip alias patterns since we've already handled them
                    if Self::is_alias_pattern(rule) {
                        continue;
                    }

                    let key = rule.key.name();
                    let value_type = Self::convert_value(&rule.value);
                    let property_def = Property {
                        property_type: value_type,
                        options: CwtOptions::default(),
                        documentation: None,
                    };
                    properties.insert(key.to_string(), property_def);
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
            return CwtType::Union(union_values);
        }

        CwtType::Block(BlockType {
            properties,
            subtypes: HashMap::new(),
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

    /// Check if a rule is an alias pattern (alias_name[X] = alias_match_left[X])
    fn is_alias_pattern(rule: &cw_parser::cwt::AstCwtRule) -> bool {
        // Check if the key is alias_name[something]
        if let cw_parser::cwt::AstCwtIdentifierOrString::Identifier(key_id) = &rule.key {
            if matches!(key_id.identifier_type, CwtReferenceType::AliasName) {
                // Check if the value is alias_match_left[something]
                if let CwtValue::Identifier(value_id) = &rule.value {
                    if matches!(value_id.identifier_type, CwtReferenceType::AliasMatchLeft) {
                        // Both key and value should reference the same type
                        return key_id.name.raw_value() == value_id.name.raw_value();
                    }
                }
            }
        }
        false
    }

    /// Extract the alias type from an alias pattern
    fn extract_alias_type(rule: &cw_parser::cwt::AstCwtRule) -> Option<String> {
        if let cw_parser::cwt::AstCwtIdentifierOrString::Identifier(key_id) = &rule.key {
            if matches!(key_id.identifier_type, CwtReferenceType::AliasName) {
                return Some(key_id.name.raw_value().to_string());
            }
        }
        None
    }
}
