//! Conversion utilities and error types for CWT analysis
//!
//! This module contains utilities for converting between CWT AST types and our
//! InferredType system, as well as error types for conversion failures.

use super::super::inference::*;
use super::options::CardinalityConstraint;
use cw_parser::cwt::{CwtRange, CwtSimpleValue, CwtSimpleValueType};

/// Errors that can occur during CWT conversion
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionError {
    InvalidRange,
    InvalidCardinality(String),
    InvalidSubtypeFormat,
    InvalidEnumFormat,
    InvalidAliasFormat,
    UnsupportedFeature(String),
    MissingReference(String),
    InvalidTypeDefinition(String),
    InvalidComplexEnum(String),
    InvalidRuleDefinition(String),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::InvalidRange => write!(f, "Invalid range specification"),
            ConversionError::InvalidCardinality(card) => write!(f, "Invalid cardinality: {}", card),
            ConversionError::InvalidSubtypeFormat => write!(f, "Invalid subtype format"),
            ConversionError::InvalidEnumFormat => write!(f, "Invalid enum format"),
            ConversionError::InvalidAliasFormat => write!(f, "Invalid alias format"),
            ConversionError::UnsupportedFeature(feature) => {
                write!(f, "Unsupported CWT feature: {}", feature)
            }
            ConversionError::MissingReference(ref_name) => {
                write!(f, "Missing reference: {}", ref_name)
            }
            ConversionError::InvalidTypeDefinition(type_name) => {
                write!(f, "Invalid type definition: {}", type_name)
            }
            ConversionError::InvalidComplexEnum(enum_name) => {
                write!(f, "Invalid complex enum: {}", enum_name)
            }
            ConversionError::InvalidRuleDefinition(rule_name) => {
                write!(f, "Invalid rule definition: {}", rule_name)
            }
        }
    }
}

impl std::error::Error for ConversionError {}

/// Conversion utilities for CWT types
pub struct ConversionUtils;

impl ConversionUtils {
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

    /// Apply range constraints to a type
    pub fn apply_range_constraints(base_type: InferredType, range: &CwtRange) -> InferredType {
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
        cardinality: &CardinalityConstraint,
    ) -> InferredType {
        if cardinality.max == Some(1) && cardinality.min == Some(0) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_constraints() {
        let range = cw_parser::cwt::CwtRange {
            min: cw_parser::cwt::CwtRangeBound::Int("0"),
            max: cw_parser::cwt::CwtRangeBound::Int("10"),
            span: 0..10,
        };
        let base_type = InferredType::Primitive(PrimitiveType::Integer);

        let constrained = ConversionUtils::apply_range_constraints(base_type, &range);

        match constrained {
            InferredType::Constrained(constraint) => {
                assert!(constraint.range.is_some());
                let range_constraint = constraint.range.as_ref().unwrap();
                assert_eq!(range_constraint.min, RangeBound::Integer(0));
                assert_eq!(range_constraint.max, RangeBound::Integer(10));
            }
            _ => panic!("Expected constrained type"),
        }
    }

    #[test]
    fn test_cardinality_constraints() {
        let cardinality = CardinalityConstraint {
            min: Some(0),
            max: Some(1),
            is_warning: false,
        };
        let base_type = InferredType::Primitive(PrimitiveType::Integer);

        let constrained = ConversionUtils::apply_cardinality_constraints(base_type, &cardinality);

        match constrained {
            InferredType::Constrained(constraint) => {
                assert!(constraint.cardinality.is_some());
                let card = constraint.cardinality.as_ref().unwrap();
                assert_eq!(card.min, Some(0));
                assert_eq!(card.max, Some(1));
            }
            _ => panic!("Expected constrained type"),
        }
    }
}
