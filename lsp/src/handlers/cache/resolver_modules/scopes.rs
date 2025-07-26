use crate::{handlers::scope::ScopeStack, interner::get_interner};
use cw_model::types::{CwtAnalyzer, LinkDefinition};
use lasso::Spur;
use std::sync::Arc;

pub struct ScopeHandler {
    pub cwt_analyzer: Arc<CwtAnalyzer>,
}

impl ScopeHandler {
    pub fn new(cwt_analyzer: Arc<CwtAnalyzer>) -> Self {
        Self { cwt_analyzer }
    }

    /// Check if a property name is a valid scope property or link property
    /// Returns Some(description) if valid, None if invalid
    pub fn is_valid_scope_or_link_property(
        &self,
        property_name: Spur,
        scope_stack: &ScopeStack,
    ) -> Option<String> {
        let interner = get_interner();
        // First, check if this property is a scope property (from, fromfrom, etc.)
        if let Some(scope_context) = scope_stack.get_scope_by_name(property_name) {
            return Some(format!(
                "scope property ({})",
                interner.resolve(&scope_context.scope_type)
            ));
        }

        // Second, check if this property is a link property
        let current_scope = &scope_stack.current_scope().scope_type;
        if let Some(link_def) = self.is_link_property(property_name, *current_scope) {
            return Some(format!(
                "link property ({})",
                interner.resolve(&link_def.output_scope)
            ));
        }

        None
    }

    /// Get all available scope properties and link properties for the current scope
    pub fn get_available_scope_and_link_properties(&self, scope_stack: &ScopeStack) -> Vec<Spur> {
        let mut properties = Vec::new();

        // Add scope properties (from, fromfrom, etc.) based on the current scope stack
        let scope_properties = scope_stack.available_scope_names();
        properties.extend(scope_properties);

        // Add link properties based on the current scope
        let current_scope = &scope_stack.current_scope().scope_type;
        let link_properties = self.get_scope_link_properties(*current_scope);
        properties.extend(link_properties);

        properties.dedup();
        properties
    }

    /// Get all scope properties
    pub fn get_all_scope_properties(&self) -> Vec<String> {
        ScopeStack::get_all_scope_properties()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get all link properties
    pub fn get_all_link_properties(&self) -> Vec<Spur> {
        self.cwt_analyzer.get_links().keys().cloned().collect()
    }

    /// Get all available link properties for the current scope
    pub fn get_scope_link_properties(&self, scope: Spur) -> Vec<Spur> {
        let mut link_properties = Vec::new();

        for (link_name, link_def) in self.cwt_analyzer.get_links() {
            if link_def.can_be_used_from(scope, &self.cwt_analyzer, get_interner()) {
                link_properties.push(*link_name);
            }
        }

        link_properties
    }

    /// Check if a property name is a link property for the current scope
    pub fn is_link_property(&self, property_name: Spur, scope: Spur) -> Option<&LinkDefinition> {
        if let Some(link_def) = self.cwt_analyzer.get_link(property_name) {
            if link_def.can_be_used_from(scope, &self.cwt_analyzer, get_interner()) {
                return Some(link_def);
            }
        }
        None
    }
}
