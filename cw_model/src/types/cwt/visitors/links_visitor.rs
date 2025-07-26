//! Specialized visitor for CWT links definitions
//!
//! This visitor handles the processing of CWT links definitions, which define
//! scope transitions and data references used in script validation.

use cw_parser::{AstCwtRule, CwtValue, CwtVisitor};
use lasso::{Spur, ThreadedRodeo};

use crate::{ConversionError, CwtAnalysisData, LinkDefinition, LinkType};

/// Specialized visitor for links definitions
pub struct LinksVisitor<'a, 'interner> {
    data: &'a mut CwtAnalysisData,
    interner: &'interner ThreadedRodeo,
}

impl<'a, 'interner> LinksVisitor<'a, 'interner> {
    /// Create a new links visitor
    pub fn new(data: &'a mut CwtAnalysisData, interner: &'interner ThreadedRodeo) -> Self {
        Self { data, interner }
    }

    /// Check if this visitor can handle the given rule
    fn can_handle_rule(&self, rule: &AstCwtRule) -> bool {
        // Check if this is a links section
        rule.key.name() == "links"
    }

    /// Process a links section
    fn process_links_section(&mut self, rule: &AstCwtRule) {
        if let CwtValue::Block(block) = &rule.value {
            for item in &block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(link_rule) = item {
                    self.process_link_definition(link_rule);
                }
            }
        }
    }

    /// Process a single link definition
    fn process_link_definition(&mut self, rule: &AstCwtRule) {
        let link_name = self.interner.get_or_intern(rule.key.name());

        if let CwtValue::Block(block) = &rule.value {
            let mut link_def =
                LinkDefinition::new(link_name, Vec::new(), self.interner.get_or_intern("Any"));

            // Parse the link properties
            for item in &block.items {
                if let cw_parser::cwt::AstCwtExpression::Rule(prop_rule) = item {
                    let prop_key = prop_rule.key.name();

                    match prop_key {
                        "input_scopes" => {
                            if let Some(scopes) = self.parse_scope_list(&prop_rule.value) {
                                link_def.input_scopes = scopes;
                            }
                        }
                        "output_scope" => {
                            if let Some(scope) = self.parse_single_scope(&prop_rule.value) {
                                link_def.output_scope = scope;
                            }
                        }
                        "desc" => {
                            if let Some(desc) = self.parse_string_value(&prop_rule.value) {
                                link_def.desc = Some(desc);
                            }
                        }
                        "from_data" => {
                            if let Some(from_data) = self.parse_bool_value(&prop_rule.value) {
                                link_def.from_data = from_data;
                            }
                        }
                        "type" => {
                            if let Some(type_str) = self.parse_string_value(&prop_rule.value) {
                                if let Ok(link_type) =
                                    self.interner.resolve(&type_str).parse::<LinkType>()
                                {
                                    link_def.link_type = link_type;
                                }
                            }
                        }
                        "data_source" => {
                            if let Some(data_source) = self.parse_string_value(&prop_rule.value) {
                                link_def.data_source = Some(data_source);
                            }
                        }
                        "prefix" => {
                            if let Some(prefix) = self.parse_string_value(&prop_rule.value) {
                                link_def.prefix = Some(prefix);
                            }
                        }
                        _ => {
                            // Unknown property, could log a warning
                        }
                    }
                }
            }

            // Store the link definition
            self.data.links.insert(link_name, link_def);
        } else {
            self.data
                .errors
                .push(ConversionError::InvalidLinkFormat(format!(
                    "Link '{}' must have a block value",
                    self.interner.resolve(&link_name)
                )));
        }
    }

    /// Parse a list of scopes from a CWT value
    fn parse_scope_list(&self, value: &CwtValue) -> Option<Vec<Spur>> {
        match value {
            CwtValue::Block(block) => {
                let mut scopes = Vec::new();
                for item in &block.items {
                    if let cw_parser::cwt::AstCwtExpression::Value(val) = item {
                        if let Some(scope) = self.parse_single_scope(val) {
                            scopes.push(scope);
                        }
                    }
                }
                Some(scopes)
            }
            _ => None,
        }
    }

    /// Parse a single scope from a CWT value
    fn parse_single_scope(&self, value: &CwtValue) -> Option<Spur> {
        match value {
            CwtValue::String(s) => Some(self.interner.get_or_intern(s.raw_value())),
            CwtValue::Simple(_simple) => {
                // For simple values, we don't have a raw string value available
                // This might be a design issue - we may need to handle this differently
                None
            }
            _ => None,
        }
    }

    /// Parse a string value from a CWT value
    fn parse_string_value(&self, value: &CwtValue) -> Option<Spur> {
        match value {
            CwtValue::String(s) => Some(self.interner.get_or_intern(s.raw_value())),
            CwtValue::Simple(_simple) => {
                // For simple values, we don't have a raw string value available
                // This might be a design issue - we may need to handle this differently
                None
            }
            _ => None,
        }
    }

    /// Parse a boolean value from a CWT value
    fn parse_bool_value(&self, value: &CwtValue) -> Option<bool> {
        match value {
            CwtValue::String(s) => match s.raw_value() {
                "yes" | "true" => Some(true),
                "no" | "false" => Some(false),
                _ => None,
            },
            CwtValue::Simple(simple) => {
                // For simple bool values, assume "true" if it's a Bool type
                match simple.value_type {
                    cw_parser::cwt::CwtSimpleValueType::Bool => Some(true),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl<'a, 'interner> CwtVisitor<'a> for LinksVisitor<'a, 'interner> {
    fn visit_rule(&mut self, rule: &AstCwtRule<'a>) {
        if self.can_handle_rule(rule) {
            self.process_links_section(rule);
        }

        // Continue walking for nested rules
        self.walk_rule(rule);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cw_parser::CwtModule;

    #[test]
    fn test_links_visitor() {
        let mut data = CwtAnalysisData::new();
        let interner = ThreadedRodeo::new();
        let mut visitor = LinksVisitor::new(&mut data, &interner);

        let cwt_text = r#"
links = {
    owner = {
        input_scopes = { planet country ship }
        output_scope = "Country"
    }
    ruler = {
        input_scopes = { country }
        output_scope = "Leader"
        desc = "The ruler of the country"
        from_data = "yes"
        type = "scope"
    }
}
        "#;

        let module = CwtModule::from_input(cwt_text).unwrap();
        let links_rule = module.find_rule("links").unwrap();

        visitor.visit_rule(links_rule);

        assert_eq!(data.links.len(), 2);

        let owner_link = data.links.get(&interner.get_or_intern("owner")).unwrap();
        assert_eq!(
            owner_link.input_scopes,
            vec![
                interner.get_or_intern("planet"),
                interner.get_or_intern("country"),
                interner.get_or_intern("ship")
            ]
        );
        assert_eq!(owner_link.output_scope, interner.get_or_intern("Country"));

        let ruler_link = data.links.get(&interner.get_or_intern("ruler")).unwrap();
        assert_eq!(
            ruler_link.input_scopes,
            vec![interner.get_or_intern("country")]
        );
        assert_eq!(ruler_link.output_scope, interner.get_or_intern("Leader"));
        assert_eq!(
            ruler_link.desc,
            Some(interner.get_or_intern("The ruler of the country"))
        );
        assert_eq!(ruler_link.from_data, true);
        assert_eq!(ruler_link.link_type, LinkType::Scope);
    }
}
