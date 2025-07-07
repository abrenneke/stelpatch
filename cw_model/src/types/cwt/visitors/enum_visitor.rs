//! Specialized visitor for CWT enum definitions
//!
//! This visitor handles the processing of CWT enum definitions, including both
//! simple enums and complex enums with path-based value extraction.

use super::super::conversion::ConversionError;
use super::super::definitions::*;
use super::converter::CwtConverter;
use super::registry::CwtAnalysisData;
use cw_parser::cwt::{
    AstCwtBlock, AstCwtRule, CwtReferenceType, AstCwtRuleKey, CwtSimpleValueType, CwtValue, CwtVisitor,
};
use std::collections::HashSet;

/// Specialized visitor for enum definitions
pub struct EnumVisitor<'a> {
    data: &'a mut CwtAnalysisData,
    in_enums_section: bool,
}

impl<'a> EnumVisitor<'a> {
    /// Create a new enum visitor
    pub fn new(data: &'a mut CwtAnalysisData) -> Self {
        Self {
            data,
            in_enums_section: false,
        }
    }

    /// Set whether we're in an enums section
    pub fn set_in_enums_section(&mut self, in_section: bool) {
        self.in_enums_section = in_section;
    }

    /// Check if this visitor can handle the given rule
    fn can_handle_rule(&self, rule: &AstCwtRule) -> bool {
        let key = rule.key.name();

        // Check for typed identifiers
        if let AstCwtRuleKey::Identifier(identifier) = &rule.key {
            return matches!(
                identifier.identifier_type,
                CwtReferenceType::Enum | CwtReferenceType::ComplexEnum
            );
        }

        // Check for legacy string-based format
        if key.starts_with("enum[") && key.ends_with("]") {
            return true;
        }

        if key.starts_with("complex_enum[") && key.ends_with("]") {
            return true;
        }

        // If we're in an enums section, we can handle any rule
        self.in_enums_section
    }

    /// Process an enum definition rule
    fn process_enum_definition(&mut self, rule: &AstCwtRule) {
        let enum_name = self.extract_enum_name(rule);
        let is_complex = self.is_complex_enum(rule);

        if let Some(name) = enum_name {
            if is_complex {
                self.process_complex_enum(&name, rule);
            } else {
                self.process_simple_enum(&name, rule);
            }
        } else {
            self.data.errors.push(ConversionError::InvalidEnumFormat);
        }
    }

    /// Extract the enum name from a rule
    fn extract_enum_name(&self, rule: &AstCwtRule) -> Option<String> {
        if let AstCwtRuleKey::Identifier(identifier) = &rule.key {
            if matches!(
                identifier.identifier_type,
                CwtReferenceType::Enum | CwtReferenceType::ComplexEnum
            ) {
                return Some(identifier.name.raw_value().to_string());
            }
        }

        // Legacy string-based handling
        let key = rule.key.name();
        if key.starts_with("enum[") && key.ends_with("]") {
            Some(key[5..key.len() - 1].to_string())
        } else if key.starts_with("complex_enum[") && key.ends_with("]") {
            Some(key[13..key.len() - 1].to_string())
        } else if self.in_enums_section {
            Some(key.to_string())
        } else {
            None
        }
    }

    /// Check if this is a complex enum
    fn is_complex_enum(&self, rule: &AstCwtRule) -> bool {
        if let AstCwtRuleKey::Identifier(identifier) = &rule.key {
            matches!(identifier.identifier_type, CwtReferenceType::ComplexEnum)
        } else {
            let key = rule.key.name();
            key.starts_with("complex_enum[") && key.ends_with("]")
        }
    }

    /// Process a simple enum definition
    fn process_simple_enum(&mut self, enum_name: &str, rule: &AstCwtRule) {
        let mut enum_def = EnumDefinition {
            values: HashSet::new(),
            complex: None,
        };

        // Extract enum values from the block
        if let CwtValue::Block(block) = &rule.value {
            Self::extract_enum_values(&mut enum_def, block);
        }

        self.data.enums.insert(enum_name.to_string(), enum_def);
    }

    /// Process a complex enum definition
    fn process_complex_enum(&mut self, enum_name: &str, rule: &AstCwtRule) {
        let mut enum_def = EnumDefinition {
            values: HashSet::new(),
            complex: Some(ComplexEnumDefinition {
                path: String::new(),
                name_structure: CwtConverter::convert_value(&rule.value),
                start_from_root: false,
            }),
        };

        // Extract complex enum configuration
        if let CwtValue::Block(block) = &rule.value {
            Self::extract_complex_enum_config(&mut enum_def, block);
        }

        self.data.enums.insert(enum_name.to_string(), enum_def);
    }

    /// Extract enum values from an enum definition block
    fn extract_enum_values(enum_def: &mut EnumDefinition, block: &AstCwtBlock) {
        for item in &block.items {
            match item {
                cw_parser::cwt::AstCwtExpression::String(s) => {
                    enum_def.values.insert(s.raw_value().to_string());
                }
                cw_parser::cwt::AstCwtExpression::Identifier(id) => {
                    enum_def.values.insert(id.name.raw_value().to_string());
                }
                _ => {}
            }
        }
    }

    /// Extract complex enum configuration
    fn extract_complex_enum_config(enum_def: &mut EnumDefinition, block: &AstCwtBlock) {
        if let Some(ref mut complex) = enum_def.complex {
            for item in &block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(rule) = item {
                    let key = rule.key.name();
                    match key {
                        "path" => {
                            if let CwtValue::String(s) = &rule.value {
                                complex.path = s.raw_value().to_string();
                            }
                        }
                        "start_from_root" => {
                            if let CwtValue::Simple(simple) = &rule.value {
                                if simple.value_type == CwtSimpleValueType::Bool {
                                    complex.start_from_root = true;
                                }
                            }
                        }
                        "name" => {
                            complex.name_structure = CwtConverter::convert_value(&rule.value);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

impl<'a> CwtVisitor<'a> for EnumVisitor<'a> {
    fn visit_rule(&mut self, rule: &AstCwtRule<'a>) {
        if self.can_handle_rule(rule) {
            self.process_enum_definition(rule);
        }

        // Continue walking for nested rules
        self.walk_rule(rule);
    }
}
