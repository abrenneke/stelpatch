use cw_model::CwtType;

use crate::handlers::cache::TypeCache;

/// Get the type of a property from the expected type structure
pub fn get_property_type_from_expected_type(
    expected_type: &CwtType,
    property_name: &str,
) -> CwtType {
    get_property_type_from_expected_type_with_depth(expected_type, property_name, 0)
}

/// Get the type of a property from the expected type structure with recursion depth limit
fn get_property_type_from_expected_type_with_depth(
    expected_type: &CwtType,
    property_name: &str,
    depth: usize,
) -> CwtType {
    // Prevent infinite recursion by limiting depth
    if depth > 10 {
        eprintln!(
            "DEBUG: Max recursion depth reached for property {}, returning Unknown",
            property_name
        );
        return CwtType::Unknown;
    }

    if !TypeCache::is_initialized() {
        return CwtType::Unknown;
    }

    let cache = TypeCache::get().unwrap();
    let resolved_type = cache.resolve_type(expected_type);

    match resolved_type {
        CwtType::Block(obj) => {
            if let Some(property_def) = obj.properties.get(property_name) {
                // Return the resolved type of this property
                cache.resolve_type(&property_def.property_type)
            } else {
                // Check if the property matches any pattern property
                if let Some(pattern_property) = cache
                    .get_resolver()
                    .key_matches_pattern(property_name, &obj)
                {
                    cache.resolve_type(&pattern_property.value_type)
                } else {
                    CwtType::Unknown
                }
            }
        }
        CwtType::Union(types) => {
            // For union types, try to find the property in any of the union members
            for union_type in types {
                let property_type = get_property_type_from_expected_type_with_depth(
                    &union_type,
                    property_name,
                    depth + 1,
                );
                if !matches!(property_type, CwtType::Unknown) {
                    return property_type;
                }
            }
            CwtType::Unknown
        }
        CwtType::Reference(_) => {
            // For references, resolve and try again
            let resolved_ref = cache.resolve_type(&resolved_type);
            if !matches!(resolved_ref, CwtType::Reference(_)) {
                get_property_type_from_expected_type_with_depth(
                    &resolved_ref,
                    property_name,
                    depth + 1,
                )
            } else {
                CwtType::Unknown
            }
        }
        _ => CwtType::Unknown,
    }
}

/// Check if a key is valid for the given type
pub fn is_key_valid(cwt_type: &CwtType, key_name: &str) -> bool {
    is_key_valid_with_depth(cwt_type, key_name, 0)
}

/// Check if a key is valid for the given type with recursion depth limit
fn is_key_valid_with_depth(cwt_type: &CwtType, key_name: &str, depth: usize) -> bool {
    // Prevent infinite recursion by limiting depth
    if depth > 10 {
        return false;
    }

    if !TypeCache::is_initialized() {
        return true;
    }

    let cache = TypeCache::get().unwrap();
    let cwt_type = cache.resolve_type(cwt_type);

    let result = match &cwt_type {
        CwtType::Block(obj) => {
            // Check if the key is in the known properties
            if obj.properties.contains_key(key_name) {
                return true;
            }

            // Check if the key matches any pattern property
            if cache
                .get_resolver()
                .key_matches_pattern(key_name, obj)
                .is_some()
            {
                return true;
            }

            false
        }
        CwtType::Union(types) => {
            // For union types, key is valid if it's valid in any of the union members
            types
                .iter()
                .any(|t| is_key_valid_with_depth(t, key_name, depth + 1))
        }
        CwtType::Reference(_) => {
            // For references, try to resolve them but don't get stuck in infinite loops
            is_key_valid_with_depth(&cwt_type, key_name, depth + 1)
        }
        CwtType::Unknown => false,
        CwtType::Simple(_) => false,
        CwtType::Array(_) => false,
        CwtType::Literal(_) => false,
        CwtType::LiteralSet(_) => false,
        CwtType::Comparable(_) => false,
    };

    result
}
