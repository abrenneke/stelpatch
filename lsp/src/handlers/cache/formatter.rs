use cw_model::types::CwtAnalyzer;
use cw_model::{CwtType, ReferenceType, SimpleType};

use crate::handlers::cache::resolver::TypeResolver;

/// Helper function to format enum values
fn format_enum_values(ref_type: &ReferenceType, enum_values: &[String]) -> String {
    let type_name = match ref_type {
        ReferenceType::Enum { key } => format!("enum {}", key),
        ReferenceType::ComplexEnum { key } => format!("complex_enum {}", key),
        _ => "enum".to_string(),
    };

    let values: Vec<_> = enum_values.iter().take(5).collect();
    if values.len() < enum_values.len() {
        format!(
            "{} ({}... +{} more)",
            type_name,
            values
                .iter()
                .map(|v| format!("\"{}\"", v))
                .collect::<Vec<_>>()
                .join(" | "),
            enum_values.len() - values.len()
        )
    } else {
        format!(
            "{} ({})",
            type_name,
            values
                .iter()
                .map(|v| format!("\"{}\"", v))
                .collect::<Vec<_>>()
                .join(" | ")
        )
    }
}

/// Helper function to format value sets
fn format_value_set(ref_type: &ReferenceType, value_set: &[String]) -> String {
    let type_name = match ref_type {
        ReferenceType::ValueSet { key } => format!("value_set {}", key),
        ReferenceType::Value { key } => format!("value {}", key),
        _ => "value_set".to_string(),
    };

    let values: Vec<_> = value_set.iter().take(5).collect();
    if values.len() < value_set.len() {
        format!(
            "{} ({}... +{} more)",
            type_name,
            values
                .iter()
                .map(|v| format!("\"{}\"", v))
                .collect::<Vec<_>>()
                .join(" | "),
            value_set.len() - values.len()
        )
    } else {
        format!(
            "{} ({})",
            type_name,
            values
                .iter()
                .map(|v| format!("\"{}\"", v))
                .collect::<Vec<_>>()
                .join(" | ")
        )
    }
}

/// Helper function to format alias names
fn format_alias_names(ref_type: &ReferenceType, alias_names: &[String]) -> String {
    let type_name = match ref_type {
        ReferenceType::AliasName { key } => format!("alias_name {}", key),
        _ => "alias_name".to_string(),
    };

    let values: Vec<_> = alias_names.iter().take(5).collect();
    if values.len() < alias_names.len() {
        format!(
            "{} ({}... +{} more)",
            type_name,
            values
                .iter()
                .map(|v| format!("\"{}\"", v))
                .collect::<Vec<_>>()
                .join(" | "),
            alias_names.len() - values.len()
        )
    } else {
        format!(
            "{} ({})",
            type_name,
            values
                .iter()
                .map(|v| format!("\"{}\"", v))
                .collect::<Vec<_>>()
                .join(" | ")
        )
    }
}

/// Helper function to format alias value types
fn format_alias_value_types(ref_type: &ReferenceType, alias_value_types: &[String]) -> String {
    let type_name = match ref_type {
        ReferenceType::AliasMatchLeft { key } => format!("alias_match_left {}", key),
        _ => "alias_match_left".to_string(),
    };

    let values: Vec<_> = alias_value_types.iter().take(3).collect(); // Show fewer for complex types
    if values.len() < alias_value_types.len() {
        format!(
            "{} ({}... +{} more)",
            type_name,
            values
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" | "),
            alias_value_types.len() - values.len()
        )
    } else {
        format!(
            "{} ({})",
            type_name,
            values
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" | ")
        )
    }
}

/// Format a type description with depth control, max lines, optional CWT context, and property name context
pub fn format_type_description_with_property_context(
    cwt_type: &CwtType,
    depth: usize,
    max_lines: usize,
    cwt_context: &CwtAnalyzer,
    resolver: &TypeResolver,
    property_name: Option<&str>,
) -> String {
    if depth > 5 {
        return "...".to_string();
    }

    let cwt_type = resolver.resolve_type(cwt_type);

    match cwt_type {
        CwtType::Literal(lit) => format!("\"{}\"", lit),
        CwtType::LiteralSet(literals) => {
            let mut sorted: Vec<_> = literals.iter().collect();
            sorted.sort();
            if sorted.len() <= 6 {
                sorted
                    .iter()
                    .map(|s| format!("\"{}\"", s))
                    .collect::<Vec<_>>()
                    .join(" | ")
            } else {
                format!(
                    "{} | /* ... +{} more */",
                    sorted
                        .iter()
                        .take(4)
                        .map(|s| format!("\"{}\"", s))
                        .collect::<Vec<_>>()
                        .join(" | "),
                    literals.len() - 4
                )
            }
        }
        CwtType::Simple(simple) => match simple {
            SimpleType::Bool => "boolean".to_string(),
            SimpleType::Int => "integer".to_string(),
            SimpleType::Float => "float".to_string(),
            SimpleType::Scalar => "scalar (0.0-1.0)".to_string(),
            SimpleType::PercentageField => "percentage".to_string(),
            SimpleType::Localisation => "localisation key".to_string(),
            SimpleType::LocalisationSynced => "synced localisation key".to_string(),
            SimpleType::LocalisationInline => "inline localisation".to_string(),
            SimpleType::DateField => "date (YYYY.MM.DD)".to_string(),
            SimpleType::VariableField => "variable reference".to_string(),
            SimpleType::IntVariableField => "integer variable reference".to_string(),
            SimpleType::ValueField => "value reference".to_string(),
            SimpleType::IntValueField => "integer value reference".to_string(),
            SimpleType::ScopeField => "scope reference".to_string(),
            SimpleType::Filepath => "file path".to_string(),
            SimpleType::Icon => "icon reference".to_string(),
            SimpleType::Color => "color (rgb/hsv/hex)".to_string(),
            SimpleType::Maths => "mathematical expression".to_string(),
        },
        CwtType::Reference(ref_type) => {
            // Use the enhanced resolver for most reference types
            let resolved = resolver.resolve_type_with_display_info(
                &CwtType::Reference(ref_type.clone()),
                property_name,
            );

            // If we got a resolved type different from the original, format it recursively
            if !matches!(resolved.cwt_type, CwtType::Reference(_)) {
                return format_type_description_with_property_context(
                    &resolved.cwt_type,
                    depth + 1,
                    max_lines,
                    cwt_context,
                    resolver,
                    property_name,
                );
            }

            // If we have display info, use it to format the reference nicely
            if let Some(display_info) = &resolved.display_info {
                if let Some(enum_values) = &display_info.enum_values {
                    return format_enum_values(&ref_type, enum_values);
                }
                if let Some(value_set) = &display_info.value_set {
                    return format_value_set(&ref_type, value_set);
                }
                if let Some(alias_names) = &display_info.alias_names {
                    return format_alias_names(&ref_type, alias_names);
                }
                if let Some(alias_value_types) = &display_info.alias_value_types {
                    return format_alias_value_types(&ref_type, alias_value_types);
                }
            }

            // Fallback to basic reference type formatting
            match ref_type {
                ReferenceType::Type { key } => format!("<{}>", key),
                ReferenceType::Enum { key } => format!("enum {}", key),
                ReferenceType::ComplexEnum { key } => format!("complex_enum {}", key),
                ReferenceType::ValueSet { key } => format!("value_set {}", key),
                ReferenceType::Value { key } => format!("value {}", key),
                ReferenceType::Scope { key } => format!("scope {}", key),
                ReferenceType::ScopeGroup { key } => format!("scope_group {}", key),
                ReferenceType::Alias { key } => format!("alias {}", key),
                ReferenceType::AliasName { key } => format!("alias_name {}", key),
                ReferenceType::AliasMatchLeft { key } => format!("alias_match_left {}", key),
                ReferenceType::SingleAlias { key } => format!("single_alias {}", key),
                ReferenceType::AliasKeysField { key } => format!("alias_keys_field {}", key),
                ReferenceType::Colour { format } => format!("colour ({})", format),
                ReferenceType::Icon { path } => format!("icon ({})", path),
                ReferenceType::Filepath { path } => format!("filepath ({})", path),
                ReferenceType::Subtype { name } => format!("subtype {}", name),
                ReferenceType::StellarisNameFormat { key } => format!("name_format {}", key),
                _ => format!("reference {:?}", ref_type),
            }
        }
        CwtType::Comparable(comparable) => {
            format!(
                "comparable[{}]",
                format_type_description_with_property_context(
                    &comparable,
                    depth + 1,
                    max_lines,
                    cwt_context,
                    resolver,
                    property_name,
                )
            )
        }
        CwtType::Block(block) => {
            // Show:
            // - The root obj
            // - The properties of the root obj
            // - The properties of the properties of the root obj
            if depth >= 1 {
                if block.properties.is_empty() {
                    return "{}".to_string();
                } else {
                    return format!("{{ /* ... +{} properties */ }}", block.properties.len());
                }
            }

            if block.properties.is_empty() {
                return "{}".to_string();
            }

            let mut properties: Vec<_> = block.properties.iter().collect();
            properties.sort_by_key(|(k, _)| k.as_str());

            let mut lines = vec!["{".to_string()];
            let mut line_count = 1;
            let mut properties_shown = 0;

            for (key, property_def) in properties {
                if line_count >= max_lines {
                    lines.push(format!(
                        "  # ... +{} more properties",
                        block.properties.len() - properties_shown
                    ));
                    break;
                }

                let formatted_value = format_type_description_with_property_context(
                    &property_def.property_type,
                    depth + 1,
                    max_lines - line_count,
                    cwt_context,
                    resolver,
                    Some(key), // Pass the property name for alias resolution
                );

                // Handle multi-line types (nested blocks)
                if formatted_value.contains('\n') {
                    lines.push(format!("  {}:", key));
                    let nested_lines: Vec<&str> = formatted_value.lines().collect();
                    let mut lines_added = 1;

                    for line in nested_lines {
                        if line.starts_with("{") {
                            continue;
                        }
                        if line_count + lines_added >= max_lines {
                            lines.push("    # ... (truncated)".to_string());
                            break;
                        }
                        lines.push(format!("    {}", line));
                        lines_added += 1;
                    }
                    line_count += lines_added;
                } else {
                    lines.push(format!("  {} = {}", key, formatted_value));
                    line_count += 1;
                }
                properties_shown += 1;
            }

            lines.push("}".to_string());

            lines.join("\n")
        }
        CwtType::Array(array_type) => {
            let element_desc = format_type_description_with_property_context(
                &array_type.element_type,
                depth + 1,
                max_lines,
                cwt_context,
                resolver,
                property_name,
            );
            if element_desc.contains('\n') {
                format!(
                    "array[{}]",
                    if let CwtType::Block(_) = array_type.element_type.as_ref() {
                        "Entity"
                    } else {
                        "object"
                    }
                )
            } else {
                format!("array[{}]", element_desc)
            }
        }
        CwtType::Union(types) => {
            if types.len() <= 8 {
                types
                    .iter()
                    .map(|t| {
                        format_type_description_with_property_context(
                            t,
                            depth + 1,
                            max_lines,
                            cwt_context,
                            resolver,
                            property_name,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" | ")
            } else {
                format!(
                    "{} | /* ... +{} more types */",
                    types
                        .iter()
                        .take(8)
                        .map(|t| format_type_description_with_property_context(
                            t,
                            depth + 1,
                            max_lines,
                            cwt_context,
                            resolver,
                            property_name,
                        ))
                        .collect::<Vec<_>>()
                        .join(" | "),
                    types.len() - 8
                )
            }
        }
        CwtType::Unknown => "unknown".to_string(),
    }
}
