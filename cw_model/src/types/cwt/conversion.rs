//! Conversion utilities and error types for CWT analysis
//!
//! This module contains utilities for converting between CWT AST types and our
//! CwtType system, as well as error types for conversion failures.

use crate::{CwtType, SimpleType};

use cw_parser::cwt::{CwtSimpleValue, CwtSimpleValueType};

/// Errors that can occur during CWT conversion
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionError {
    InvalidRange,
    InvalidCardinality(String),
    InvalidSubtypeFormat,
    InvalidEnumFormat,
    InvalidAliasFormat,
    InvalidLinkFormat(String),
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
            ConversionError::InvalidLinkFormat(link_error) => {
                write!(f, "Invalid link format: {}", link_error)
            }
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
}
