use crate::handlers::cache::resolver::TypeResolver;

use super::core::TypeCache;
use super::formatter::TypeFormatter;
use super::types::TypeInfo;

/// Get type information for a namespace entity (top-level entity structure)
pub fn get_namespace_entity_type(namespace: &str) -> Option<TypeInfo> {
    if !TypeCache::is_initialized() {
        return Some(TypeInfo {
            property_path: "entity".to_string(),
            type_description: "Loading type information...".to_string(),
            scoped_type: None,
            documentation: None,
            source_info: Some("Type system initializing".to_string()),
        });
    }

    let cache = TypeCache::get().unwrap();
    let resolver = TypeResolver::new(cache.get_cwt_analyzer().clone());
    let formatter = TypeFormatter::new(&resolver, 30);

    if let Some(namespace_type) = cache.get_namespace_type(namespace) {
        let scoped_type = namespace_type.clone();
        Some(TypeInfo {
            property_path: "entity".to_string(),
            type_description: formatter.format_type(
                scoped_type.clone(),
                None, // No specific property name for top-level entity types
            ),
            scoped_type: Some(scoped_type),
            documentation: None,
            source_info: Some(format!("Entity structure for {} namespace", namespace)),
        })
    } else {
        Some(TypeInfo {
            property_path: "entity".to_string(),
            type_description: "No type information available for this namespace".to_string(),
            scoped_type: None,
            documentation: None,
            source_info: Some(format!("Namespace {} not found in type system", namespace)),
        })
    }
}

/// Get type information for a property within a namespace entity
/// The property_path should be just the property path without the entity name
pub fn get_entity_property_type(namespace: &str, property_path: &str) -> Option<TypeInfo> {
    if !TypeCache::is_initialized() {
        return Some(TypeInfo {
            property_path: property_path.to_string(),
            type_description: "Loading type information...".to_string(),
            scoped_type: None,
            documentation: None,
            source_info: Some("Type system initializing".to_string()),
        });
    }

    let cache = TypeCache::get().unwrap();
    cache.get_property_type(namespace, property_path)
}
