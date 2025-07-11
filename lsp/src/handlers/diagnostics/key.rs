use cw_model::CwtType;

use crate::handlers::{
    cache::TypeCache,
    scoped_type::{CwtTypeOrSpecial, PropertyNavigationResult, ScopedType},
};

/// Get the type of a property from the expected type structure
pub fn get_property_type_from_expected_type(
    expected_type: &ScopedType,
    property_name: &str,
) -> ScopedType {
    get_property_type_from_expected_type_with_depth(expected_type, property_name, 0)
}

/// Get the type of a property from the expected type structure with recursion depth limit
fn get_property_type_from_expected_type_with_depth(
    expected_type: &ScopedType,
    property_name: &str,
    depth: usize,
) -> ScopedType {
    // Prevent infinite recursion by limiting depth
    if depth > 10 {
        eprintln!(
            "DEBUG: Max recursion depth reached for property {}, returning Unknown",
            property_name
        );
        return ScopedType::new_cwt(CwtType::Unknown, expected_type.scope_stack().clone());
    }

    if !TypeCache::is_initialized() {
        return ScopedType::new_cwt(CwtType::Unknown, expected_type.scope_stack().clone());
    }

    let cache = TypeCache::get().unwrap();
    let resolved_type = cache.resolve_type(expected_type);

    match &resolved_type.cwt_type() {
        CwtTypeOrSpecial::CwtType(CwtType::Block(obj)) => {
            if let Some(property_def) = obj.properties.get(property_name) {
                // Return the resolved type of this property
                cache.resolve_type(&ScopedType::new_cwt(
                    property_def.property_type.clone(),
                    expected_type.scope_stack().clone(),
                ))
            } else {
                // Check if the property matches any pattern property
                if let Some(pattern_property) = cache
                    .get_resolver()
                    .key_matches_pattern(property_name, &obj)
                {
                    cache.resolve_type(&ScopedType::new_cwt(
                        pattern_property.value_type.clone(),
                        expected_type.scope_stack().clone(),
                    ))
                } else {
                    ScopedType::new_cwt(CwtType::Unknown, expected_type.scope_stack().clone())
                }
            }
        }
        CwtTypeOrSpecial::CwtType(CwtType::Union(types)) => {
            // For union types, try to find the property in any of the union members
            for union_type in types {
                let property_type = get_property_type_from_expected_type_with_depth(
                    &ScopedType::new_cwt(union_type.clone(), expected_type.scope_stack().clone()),
                    property_name,
                    depth + 1,
                );
                if !matches!(
                    property_type.cwt_type(),
                    CwtTypeOrSpecial::CwtType(CwtType::Unknown)
                ) {
                    return property_type;
                }
            }
            ScopedType::new_cwt(CwtType::Unknown, expected_type.scope_stack().clone())
        }
        CwtTypeOrSpecial::CwtType(CwtType::Reference(_)) => {
            // For references, resolve and try again
            let resolved_ref = cache.resolve_type(&resolved_type);
            if !matches!(
                resolved_ref.cwt_type(),
                CwtTypeOrSpecial::CwtType(CwtType::Reference(_))
            ) {
                get_property_type_from_expected_type_with_depth(
                    &resolved_ref,
                    property_name,
                    depth + 1,
                )
            } else {
                ScopedType::new_cwt(CwtType::Unknown, expected_type.scope_stack().clone())
            }
        }
        _ => ScopedType::new_cwt(CwtType::Unknown, expected_type.scope_stack().clone()),
    }
}

/// Check if a key is valid for the given type
pub fn is_key_valid(cwt_type: &ScopedType, key_name: &str) -> bool {
    if !TypeCache::is_initialized() {
        return true;
    }

    let cache = TypeCache::get().unwrap();

    let result = cache
        .get_resolver()
        .navigate_to_property(cwt_type, key_name);

    matches!(result, PropertyNavigationResult::Success(_))
}
