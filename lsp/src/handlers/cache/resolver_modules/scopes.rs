use crate::handlers::scope::ScopeStack;
use cw_model::types::{CwtAnalyzer, LinkDefinition};
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
        property_name: &str,
        scope_stack: &ScopeStack,
    ) -> Option<String> {
        // First, check if this property is a scope property (from, fromfrom, etc.)
        if let Some(scope_context) = scope_stack.get_scope_by_name(property_name) {
            return Some(format!("scope property ({})", scope_context.scope_type));
        }

        // Second, check if this property is a link property
        let current_scope = &scope_stack.current_scope().scope_type;
        if let Some(link_def) = self.is_link_property(property_name, current_scope) {
            return Some(format!("link property ({})", link_def.output_scope));
        }

        None
    }

    /// Get all available scope properties and link properties for the current scope
    pub fn get_available_scope_and_link_properties(&self, scope_stack: &ScopeStack) -> Vec<String> {
        let mut properties = Vec::new();

        // Add scope properties (from, fromfrom, etc.) based on the current scope stack
        let scope_properties = scope_stack.available_scope_names();
        properties.extend(scope_properties);

        // Add link properties based on the current scope
        let current_scope = &scope_stack.current_scope().scope_type;
        let link_properties = self.get_scope_link_properties(current_scope);
        properties.extend(link_properties);

        properties.sort();
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
    pub fn get_all_link_properties(&self) -> Vec<String> {
        self.cwt_analyzer.get_links().keys().cloned().collect()
    }

    /// Get all available link properties for the current scope
    pub fn get_scope_link_properties(&self, scope: &str) -> Vec<String> {
        let mut link_properties = Vec::new();

        for (link_name, link_def) in self.cwt_analyzer.get_links() {
            if link_def.can_be_used_from(scope, &self.cwt_analyzer) {
                link_properties.push(link_name.clone());
            }
        }

        link_properties
    }

    /// Check if a property name is a link property for the current scope
    pub fn is_link_property(&self, property_name: &str, scope: &str) -> Option<&LinkDefinition> {
        if let Some(link_def) = self.cwt_analyzer.get_link(property_name) {
            if link_def.can_be_used_from(scope, &self.cwt_analyzer) {
                return Some(link_def);
            }
        }
        None
    }
}
