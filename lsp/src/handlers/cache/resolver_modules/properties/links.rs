use std::sync::Arc;

use cw_model::{CwtAnalyzer, LinkDefinition};
use lasso::Spur;

use crate::interner::get_interner;

/// Check if a property name is a link property for the current scope
pub fn is_link_property<'a>(
    cwt_analyzer: &'a CwtAnalyzer,
    property_name: Spur,
    scope: Spur,
) -> Option<&'a LinkDefinition> {
    let interner = get_interner();
    if let Some(link_def) = cwt_analyzer.get_link(property_name).or_else(|| {
        let property_name_str = interner.resolve(&property_name);
        if property_name_str.starts_with("hidden:") {
            cwt_analyzer.get_link(
                interner.get_or_intern(property_name_str.split("hidden:").nth(1).unwrap()),
            )
        } else {
            None
        }
    }) {
        // If current scope is "unknown", treat it as a fallback that can navigate anywhere
        if scope == interner.get_or_intern("unknown")
            || link_def.can_be_used_from(scope, &cwt_analyzer, interner)
        {
            return Some(link_def);
        }
    }
    None
}

/// Get all available link properties for the current scope
pub fn get_scope_link_properties(cwt_analyzer: Arc<CwtAnalyzer>, scope: Spur) -> Vec<Spur> {
    let interner = get_interner();
    let mut link_properties = Vec::new();

    // If current scope is "unknown", treat it as a fallback that can navigate anywhere
    let is_unknown_scope = scope == interner.get_or_intern("unknown");

    for (link_name, link_def) in cwt_analyzer.get_links() {
        // If scope is unknown, allow all links as fallback, otherwise use normal validation
        if is_unknown_scope || link_def.can_be_used_from(scope, &cwt_analyzer, interner) {
            link_properties.push(link_name.clone());
        }
    }

    link_properties
}
