use crate::handlers::cache::resolver::TypeResolver;

use super::core::TypeCache;
use super::formatter::format_type_description_with_property_context;
use super::types::TypeInfo;

/// Get type information for a namespace entity (top-level entity structure)
pub async fn get_namespace_entity_type(namespace: &str) -> Option<TypeInfo> {
    if !TypeCache::is_initialized() {
        return Some(TypeInfo {
            property_path: "entity".to_string(),
            type_description: "Loading type information...".to_string(),
            cwt_type: None,
            documentation: None,
            source_info: Some("Type system initializing".to_string()),
        });
    }

    let cache = TypeCache::get().unwrap();
    let mut resolver = TypeResolver::new(cache.get_cwt_analyzer().clone());
    if let Some(namespace_type) = cache.get_namespace_type(namespace) {
        let scoped_type = namespace_type.clone();
        Some(TypeInfo {
            property_path: "entity".to_string(),
            type_description: format_type_description_with_property_context(
                &scoped_type,
                0,
                30,
                cache.get_cwt_analyzer(),
                &mut resolver,
                None, // No specific property name for top-level entity types
            ),
            cwt_type: Some(scoped_type.cwt_type().clone()),
            documentation: None,
            source_info: Some(format!("Entity structure for {} namespace", namespace)),
        })
    } else {
        Some(TypeInfo {
            property_path: "entity".to_string(),
            type_description: "No type information available for this namespace".to_string(),
            cwt_type: None,
            documentation: None,
            source_info: Some(format!("Namespace {} not found in type system", namespace)),
        })
    }
}

/// Get type information for a property within a namespace entity
/// The property_path should be just the property path without the entity name
pub async fn get_entity_property_type(namespace: &str, property_path: &str) -> Option<TypeInfo> {
    if !TypeCache::is_initialized() {
        return Some(TypeInfo {
            property_path: property_path.to_string(),
            type_description: "Loading type information...".to_string(),
            cwt_type: None,
            documentation: None,
            source_info: Some("Type system initializing".to_string()),
        });
    }

    let cache = TypeCache::get().unwrap();
    cache.get_property_type(namespace, property_path)
}
