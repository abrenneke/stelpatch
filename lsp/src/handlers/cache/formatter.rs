use cw_model::types::CwtAnalyzer;
use cw_model::{CwtType, ReferenceType, SimpleType};

use crate::handlers::cache::resolver::TypeResolver;

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
                let total_properties = block.properties.len() + block.pattern_properties.len();
                if total_properties == 0 {
                    return "{}".to_string();
                } else {
                    return format!("{{ /* ... +{} properties */ }}", total_properties);
                }
            }

            let total_properties = block.properties.len() + block.pattern_properties.len();
            if total_properties == 0 {
                return "{}".to_string();
            }

            // Collect regular properties
            let mut properties: Vec<_> = block.properties.iter().collect();
            properties.sort_by_key(|(k, _)| k.as_str());

            let mut lines = vec!["{".to_string()];
            let mut line_count = 1;
            let mut properties_shown = 0;

            // Show regular properties first
            for (key, property_def) in properties {
                if line_count >= max_lines {
                    lines.push(format!(
                        "  # ... +{} more properties",
                        total_properties - properties_shown
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

            // Show pattern properties
            for pattern_property in &block.pattern_properties {
                if line_count >= max_lines {
                    lines.push(format!(
                        "  # ... +{} more properties",
                        total_properties - properties_shown
                    ));
                    break;
                }

                let pattern_description = match &pattern_property.pattern_type {
                    cw_model::types::PatternType::AliasName { category } => {
                        format!("({})", category)
                    }
                    cw_model::types::PatternType::Enum { key } => {
                        format!("({})", key)
                    }
                };

                let formatted_value = format_type_description_with_property_context(
                    &pattern_property.value_type,
                    depth + 1,
                    max_lines - line_count,
                    cwt_context,
                    resolver,
                    None, // Pattern properties don't have specific property names
                );

                // Handle multi-line types (nested blocks)
                if formatted_value.contains('\n') {
                    lines.push(format!("  {}:", pattern_description));
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
                    lines.push(format!("  {} = {}", pattern_description, formatted_value));
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
