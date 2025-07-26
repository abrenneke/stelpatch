use std::sync::Arc;

use cw_model::{BlockType, PatternProperty, Property};
use lasso::Spur;

use crate::handlers::cache::PatternMatcher;

/// Get subtype-specific property from a block type
pub fn get_subtype_property<'b>(
    block_type: &'b BlockType,
    subtype_name: Spur,
    property_name: Spur,
) -> Option<&'b Property> {
    // Check if there's a subtype definition for this block type
    if let Some(subtype_def) = block_type.subtypes.get(&subtype_name) {
        return subtype_def.allowed_properties.get(&property_name);
    }

    None
}

/// Get ALL subtype-specific pattern properties that match from a block type
pub fn get_all_subtype_pattern_properties<'b>(
    pattern_matcher: Arc<PatternMatcher>,
    block_type: &'b BlockType,
    subtype_name: Spur,
    property_name: Spur,
) -> Vec<&'b PatternProperty> {
    let mut matches = Vec::new();

    // Check if there are pattern properties for this subtype
    if let Some(pattern_properties) = block_type.subtype_pattern_properties.get(&subtype_name) {
        // Find all pattern properties that match the property name
        for pattern_property in pattern_properties {
            if pattern_matcher
                .key_matches_pattern_type(property_name, &pattern_property.pattern_type)
            {
                matches.push(pattern_property);
            }
        }
    }

    matches
}
