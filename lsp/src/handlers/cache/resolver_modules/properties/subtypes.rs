use std::sync::Arc;

use cw_model::{BlockType, PatternProperty, Property};

use crate::handlers::cache::PatternMatcher;

/// Get subtype-specific property from a block type
pub fn get_subtype_property<'b>(
    block_type: &'b BlockType,
    subtype_name: &str,
    property_name: &str,
) -> Option<&'b Property> {
    // Check if there's a subtype definition for this block type
    if let Some(subtype_def) = block_type.subtypes.get(subtype_name) {
        return subtype_def.allowed_properties.get(property_name);
    }

    None
}

/// Get subtype-specific pattern property from a block type
pub fn get_subtype_pattern_property<'b>(
    pattern_matcher: Arc<PatternMatcher>,
    block_type: &'b BlockType,
    subtype_name: &str,
    property_name: &str,
) -> Option<&'b PatternProperty> {
    // Check if there are pattern properties for this subtype
    if let Some(pattern_properties) = block_type.subtype_pattern_properties.get(subtype_name) {
        // Find a pattern property that matches the property name
        for pattern_property in pattern_properties {
            if pattern_matcher
                .key_matches_pattern_type(property_name, &pattern_property.pattern_type)
            {
                return Some(pattern_property);
            }
        }
    }

    None
}
