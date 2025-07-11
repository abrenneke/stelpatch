use cw_model::CwtType;

use crate::handlers::{
    cache::TypeCache,
    scoped_type::{CwtTypeOrSpecial, ScopedType},
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
        return ScopedType::new_cwt(CwtType::Unknown, expected_type.scope_context().clone());
    }

    if !TypeCache::is_initialized() {
        return ScopedType::new_cwt(CwtType::Unknown, expected_type.scope_context().clone());
    }

    let cache = TypeCache::get().unwrap();
    let resolved_type = cache.resolve_type(expected_type);

    match &resolved_type.cwt_type() {
        CwtTypeOrSpecial::CwtType(CwtType::Block(obj)) => {
            if let Some(property_def) = obj.properties.get(property_name) {
                // Return the resolved type of this property
                cache.resolve_type(&ScopedType::new_cwt(
                    property_def.property_type.clone(),
                    expected_type.scope_context().clone(),
                ))
            } else {
                // Check if the property matches any pattern property
                if let Some(pattern_property) = cache
                    .get_resolver()
                    .key_matches_pattern(property_name, &obj)
                {
                    cache.resolve_type(&ScopedType::new_cwt(
                        pattern_property.value_type.clone(),
                        expected_type.scope_context().clone(),
                    ))
                } else {
                    ScopedType::new_cwt(CwtType::Unknown, expected_type.scope_context().clone())
                }
            }
        }
        CwtTypeOrSpecial::CwtType(CwtType::Union(types)) => {
            // For union types, try to find the property in any of the union members
            for union_type in types {
                let property_type = get_property_type_from_expected_type_with_depth(
                    &ScopedType::new_cwt(union_type.clone(), expected_type.scope_context().clone()),
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
            ScopedType::new_cwt(CwtType::Unknown, expected_type.scope_context().clone())
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
                ScopedType::new_cwt(CwtType::Unknown, expected_type.scope_context().clone())
            }
        }
        _ => ScopedType::new_cwt(CwtType::Unknown, expected_type.scope_context().clone()),
    }
}

/// Check if a key is valid for the given type
pub fn is_key_valid(cwt_type: &ScopedType, key_name: &str) -> bool {
    is_key_valid_with_depth(cwt_type, key_name, 0)
}

/// Check if a key is valid for the given type with recursion depth limit
fn is_key_valid_with_depth(cwt_type: &ScopedType, key_name: &str, depth: usize) -> bool {
    // Prevent infinite recursion by limiting depth
    if depth > 10 {
        return false;
    }

    if !TypeCache::is_initialized() {
        return true;
    }

    let cache = TypeCache::get().unwrap();
    let scoped_type = cache.resolve_type(cwt_type);

    let result = match &scoped_type.cwt_type() {
        CwtTypeOrSpecial::CwtType(CwtType::Block(obj)) => {
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
        CwtTypeOrSpecial::CwtType(CwtType::Union(types)) => {
            // For union types, key is valid if it's valid in any of the union members
            types.iter().any(|t| {
                is_key_valid_with_depth(
                    &ScopedType::new_cwt(t.clone(), scoped_type.scope_context().clone()),
                    key_name,
                    depth + 1,
                )
            })
        }
        CwtTypeOrSpecial::CwtType(CwtType::Reference(_)) => {
            // For references, try to resolve them but don't get stuck in infinite loops
            is_key_valid_with_depth(&scoped_type, key_name, depth + 1)
        }
        CwtTypeOrSpecial::CwtType(CwtType::Unknown) => false,
        CwtTypeOrSpecial::CwtType(CwtType::Simple(_)) => false,
        CwtTypeOrSpecial::CwtType(CwtType::Array(_)) => false,
        CwtTypeOrSpecial::CwtType(CwtType::Literal(_)) => false,
        CwtTypeOrSpecial::CwtType(CwtType::LiteralSet(_)) => false,
        CwtTypeOrSpecial::CwtType(CwtType::Comparable(_)) => false,
        CwtTypeOrSpecial::ScopedUnion(_) => todo!(),
    };

    result
}
