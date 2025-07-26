//! Specialized visitor for CWT enum definitions
//!
//! This visitor handles the processing of CWT enum definitions, including both
//! simple enums and complex enums with path-based value extraction.

use super::super::conversion::ConversionError;
use super::super::definitions::*;
use super::converter::CwtConverter;
use super::registry::CwtAnalysisData;
use cw_parser::{
    AstCwtExpression,
    cwt::{
        AstCwtBlock, AstCwtIdentifierOrString, AstCwtRule, CwtReferenceType, CwtSimpleValueType,
        CwtValue, CwtVisitor,
    },
};
use lasso::{Spur, ThreadedRodeo};
use std::collections::HashSet;

/// Specialized visitor for enum definitions
pub struct EnumVisitor<'a, 'interner> {
    data: &'a mut CwtAnalysisData,
    interner: &'interner ThreadedRodeo,
    in_enums_section: bool,
}

impl<'a, 'interner> EnumVisitor<'a, 'interner> {
    /// Create a new enum visitor
    pub fn new(data: &'a mut CwtAnalysisData, interner: &'interner ThreadedRodeo) -> Self {
        Self {
            data,
            interner,
            in_enums_section: false,
        }
    }

    /// Set whether we're in an enums section
    pub fn set_in_enums_section(&mut self, in_section: bool) {
        self.in_enums_section = in_section;
    }

    /// Check if this visitor can handle the given rule
    fn can_handle_rule(&self, rule: &AstCwtRule) -> bool {
        // Only handle typed identifiers - no legacy string parsing
        if let AstCwtIdentifierOrString::Identifier(identifier) = &rule.key {
            let can_handle = matches!(
                identifier.identifier_type,
                CwtReferenceType::Enum | CwtReferenceType::ComplexEnum
            );
            return can_handle;
        }

        // If we're in an enums section, explicitly reject non-typed identifiers
        // This prevents nested structure rules (like "tradition_swap = { name = enum_name }")
        // from being processed as separate enum definitions
        false
    }

    /// Process an enum definition rule
    fn process_enum_definition(&mut self, rule: &AstCwtRule) {
        let enum_name = self.extract_enum_name(rule);
        let is_complex = self.is_complex_enum(rule);

        if let Some(name) = enum_name {
            if is_complex {
                self.process_complex_enum(name, rule);
            } else {
                self.process_simple_enum(name, rule);
            }
        } else {
            self.data.errors.push(ConversionError::InvalidEnumFormat);
        }
    }

    /// Extract the enum name from a rule
    fn extract_enum_name(&self, rule: &AstCwtRule) -> Option<Spur> {
        if let AstCwtIdentifierOrString::Identifier(identifier) = &rule.key {
            if matches!(
                identifier.identifier_type,
                CwtReferenceType::Enum | CwtReferenceType::ComplexEnum
            ) {
                return Some(self.interner.get_or_intern(identifier.name.raw_value()));
            }
        }

        None
    }

    /// Check if this is a complex enum
    fn is_complex_enum(&self, rule: &AstCwtRule) -> bool {
        if let AstCwtIdentifierOrString::Identifier(identifier) = &rule.key {
            matches!(identifier.identifier_type, CwtReferenceType::ComplexEnum)
        } else {
            false
        }
    }

    /// Process a simple enum definition
    fn process_simple_enum(&mut self, enum_name: Spur, rule: &AstCwtRule) {
        let mut enum_def = EnumDefinition {
            values: HashSet::new(),
            complex: None,
        };

        // Extract enum values from the block
        if let CwtValue::Block(block) = &rule.value {
            Self::extract_enum_values(&mut enum_def, block, self.interner);
        }

        self.data.enums.insert(enum_name, enum_def);
    }

    /// Process a complex enum definition
    fn process_complex_enum(&mut self, enum_name: Spur, rule: &AstCwtRule) {
        let mut enum_def = EnumDefinition {
            values: HashSet::new(),
            complex: Some(ComplexEnumDefinition {
                path: self.interner.get_or_intern(""),
                name_structure: CwtConverter::convert_value(&rule.value, None, self.interner),
                start_from_root: false,
            }),
        };

        // Extract complex enum configuration
        if let CwtValue::Block(block) = &rule.value {
            Self::extract_complex_enum_config(&mut enum_def, block, self.interner);
        }

        self.data.enums.insert(enum_name, enum_def);
    }

    /// Extract enum values from an enum definition block
    fn extract_enum_values(
        enum_def: &mut EnumDefinition,
        block: &AstCwtBlock,
        interner: &ThreadedRodeo,
    ) {
        for item in &block.items {
            match item {
                AstCwtExpression::Value(s) => match s {
                    CwtValue::String(s) => {
                        enum_def
                            .values
                            .insert(interner.get_or_intern(s.raw_value()));
                    }
                    CwtValue::Identifier(id) => {
                        enum_def
                            .values
                            .insert(interner.get_or_intern(id.name.raw_value()));
                    }
                    _ => {}
                },
                AstCwtExpression::Identifier(id) => {
                    enum_def
                        .values
                        .insert(interner.get_or_intern(id.name.raw_value()));
                }
                _ => {}
            }
        }
    }

    /// Extract complex enum configuration
    fn extract_complex_enum_config(
        enum_def: &mut EnumDefinition,
        block: &AstCwtBlock,
        interner: &ThreadedRodeo,
    ) {
        if let Some(ref mut complex) = enum_def.complex {
            for item in &block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(rule) = item {
                    let key = rule.key.name();
                    match key {
                        "path" => {
                            if let CwtValue::String(s) = &rule.value {
                                complex.path = interner.get_or_intern(s.raw_value());
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
                            complex.name_structure =
                                CwtConverter::convert_value(&rule.value, None, interner);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

impl<'a, 'interner> CwtVisitor<'a> for EnumVisitor<'a, 'interner> {
    fn visit_rule(&mut self, rule: &AstCwtRule<'a>) {
        if self.can_handle_rule(rule) {
            self.process_enum_definition(rule);
        }

        // Continue walking for nested rules
        self.walk_rule(rule);
    }
}
