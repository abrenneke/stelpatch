//! Core converter for CWT values to InferredType
//!
//! This module provides utilities for converting CWT AST values to our InferredType system.

use super::super::super::inference::*;
use cw_parser::cwt::{
    AstCwtBlock, AstCwtIdentifier, CwtRange, CwtReferenceType, CwtSimpleValue, CwtSimpleValueType,
    CwtValue,
};
use std::collections::HashMap;

/// Converter for CWT values to InferredType
pub struct CwtConverter;

impl CwtConverter {
    /// Convert a CWT simple value to our type system
    pub fn convert_simple_value(simple: &CwtSimpleValue) -> InferredType {
        let primitive_type = match simple.value_type {
            CwtSimpleValueType::Bool => PrimitiveType::Boolean,
            CwtSimpleValueType::Int => PrimitiveType::Integer,
            CwtSimpleValueType::Float => PrimitiveType::Float,
            CwtSimpleValueType::Scalar => PrimitiveType::Scalar,
            CwtSimpleValueType::PercentageField => PrimitiveType::PercentageField,
            CwtSimpleValueType::Localisation => PrimitiveType::Localisation,
            CwtSimpleValueType::LocalisationSynced => PrimitiveType::LocalisationSynced,
            CwtSimpleValueType::LocalisationInline => PrimitiveType::LocalisationInline,
            CwtSimpleValueType::DateField => PrimitiveType::DateField,
            CwtSimpleValueType::VariableField => PrimitiveType::VariableField,
            CwtSimpleValueType::IntVariableField => PrimitiveType::IntVariableField,
            CwtSimpleValueType::ValueField => PrimitiveType::ValueField,
            CwtSimpleValueType::IntValueField => PrimitiveType::IntValueField,
            CwtSimpleValueType::ScopeField => PrimitiveType::ScopeField,
            CwtSimpleValueType::Filepath => PrimitiveType::Filepath,
            CwtSimpleValueType::Icon => PrimitiveType::Icon,
        };

        let mut base_type = InferredType::Primitive(primitive_type);

        // Apply range constraints if present
        if let Some(range) = &simple.range {
            base_type = Self::apply_range_constraints(base_type, range);
        }

        base_type
    }

    /// Convert a CWT identifier to our type system
    pub fn convert_identifier(identifier: &AstCwtIdentifier) -> InferredType {
        let reference_type = match &identifier.identifier_type {
            CwtReferenceType::TypeRef => ReferenceType::TypeRef {
                type_key: identifier.name.raw_value().to_string(),
                prefix: None,
                suffix: None,
            },
            CwtReferenceType::TypeRefWithPrefixSuffix(prefix, suffix) => ReferenceType::TypeRef {
                type_key: identifier.name.raw_value().to_string(),
                prefix: Some(prefix.to_string()),
                suffix: Some(suffix.to_string()),
            },
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
            _ => ReferenceType::TypeRef {
                type_key: identifier.name.raw_value().to_string(),
                prefix: None,
                suffix: None,
            },
        };

        InferredType::Reference(reference_type)
    }

    /// Convert a CWT block to our type system
    pub fn convert_block(block: &AstCwtBlock) -> InferredType {
        let mut properties = HashMap::new();

        // Process all items in the block
        for item in &block.items {
            match item {
                cw_parser::cwt::AstCwtExpression::Rule(rule) => {
                    let key = rule.key.name();
                    let value_type = Self::convert_value(&rule.value);
                    let property_def = PropertyDefinition {
                        property_type: Box::new(value_type),
                        cardinality: None, // Will be set by visitor with rule options
                        options: Vec::new(),
                        documentation: None,
                    };
                    properties.insert(key.to_string(), property_def);
                }
                cw_parser::cwt::AstCwtExpression::String(s) => {
                    // Handle string literals in blocks
                    let value = s.raw_value().to_string();
                    let property_def = PropertyDefinition {
                        property_type: Box::new(InferredType::Literal(value.clone())),
                        cardinality: None,
                        options: Vec::new(),
                        documentation: None,
                    };
                    properties.insert(value, property_def);
                }
                cw_parser::cwt::AstCwtExpression::Identifier(id) => {
                    // Handle identifiers in blocks
                    let value = id.name.raw_value().to_string();
                    let property_def = PropertyDefinition {
                        property_type: Box::new(InferredType::Literal(value.clone())),
                        cardinality: None,
                        options: Vec::new(),
                        documentation: None,
                    };
                    properties.insert(value, property_def);
                }
                _ => {
                    // Handle other expression types as needed
                }
            }
        }

        InferredType::Object(ObjectType {
            properties,
            subtypes: HashMap::new(),
            extensible: true,
            localisation: None,
            modifiers: None,
        })
    }

    /// Convert a CWT value to our type system
    pub fn convert_value(value: &CwtValue) -> InferredType {
        match value {
            CwtValue::Simple(simple) => Self::convert_simple_value(simple),
            CwtValue::Identifier(identifier) => Self::convert_identifier(identifier),
            CwtValue::Block(block) => Self::convert_block(block),
            CwtValue::String(s) => InferredType::Literal(s.raw_value().to_string()),
        }
    }

    /// Apply range constraints to a type
    fn apply_range_constraints(base_type: InferredType, range: &CwtRange) -> InferredType {
        let inference_range = Range {
            min: match &range.min {
                cw_parser::cwt::CwtRangeBound::Int(s) => {
                    RangeBound::Integer(s.parse().unwrap_or(0))
                }
                cw_parser::cwt::CwtRangeBound::Float(s) => {
                    RangeBound::Float(s.parse().unwrap_or(0.0))
                }
                cw_parser::cwt::CwtRangeBound::Infinity(false) => RangeBound::NegInfinity,
                cw_parser::cwt::CwtRangeBound::Infinity(true) => RangeBound::PosInfinity,
            },
            max: match &range.max {
                cw_parser::cwt::CwtRangeBound::Int(s) => {
                    RangeBound::Integer(s.parse().unwrap_or(0))
                }
                cw_parser::cwt::CwtRangeBound::Float(s) => {
                    RangeBound::Float(s.parse().unwrap_or(0.0))
                }
                cw_parser::cwt::CwtRangeBound::Infinity(false) => RangeBound::NegInfinity,
                cw_parser::cwt::CwtRangeBound::Infinity(true) => RangeBound::PosInfinity,
            },
        };

        match base_type {
            InferredType::Primitive(PrimitiveType::Integer) => {
                InferredType::Constrained(ConstrainedType {
                    base_type: Box::new(base_type),
                    range: Some(inference_range),
                    cardinality: None,
                    options: Vec::new(),
                })
            }
            InferredType::Primitive(PrimitiveType::Float) => {
                InferredType::Constrained(ConstrainedType {
                    base_type: Box::new(base_type),
                    range: Some(inference_range),
                    cardinality: None,
                    options: Vec::new(),
                })
            }
            _ => base_type,
        }
    }

    /// Apply cardinality constraints to a type
    pub fn apply_cardinality_constraints(
        base_type: InferredType,
        cardinality: &super::super::options::CardinalityConstraint,
    ) -> InferredType {
        if cardinality.max == Some(1) && cardinality.min == 0 {
            // Optional type - use constrained type with cardinality
            InferredType::Constrained(ConstrainedType {
                base_type: Box::new(base_type),
                cardinality: Some(Cardinality::optional()),
                range: None,
                options: Vec::new(),
            })
        } else if cardinality.max.is_none() || cardinality.max.unwrap() > 1 {
            // Array type
            InferredType::Array(ArrayType {
                element_type: Box::new(base_type),
                cardinality: Cardinality::new(cardinality.min, cardinality.max),
            })
        } else {
            base_type
        }
    }
}
