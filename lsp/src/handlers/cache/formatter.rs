use cw_model::types::CwtAnalyzer;
use cw_model::{CwtType, ReferenceType, SimpleType};

use crate::handlers::cache::resolver::TypeResolver;
use crate::handlers::scoped_type::{CwtTypeOrSpecial, PropertyNavigationResult, ScopedType};

const MAX_UNION_MEMBERS: usize = 8;

/// Format a type description with depth control, max lines, optional CWT context, and property name context
pub fn format_type_description_with_property_context(
    scoped_type: &ScopedType,
    depth: usize,
    max_lines: usize,
    cwt_context: &CwtAnalyzer,
    resolver: &TypeResolver,
    property_name: Option<&str>,
) -> String {
    if depth > 5 {
        return "...".to_string();
    }

    let scoped_type = resolver.resolve_type(scoped_type);

    match scoped_type.cwt_type() {
        CwtTypeOrSpecial::CwtType(CwtType::Literal(lit)) => format!("\"{}\"", lit),
        CwtTypeOrSpecial::CwtType(CwtType::LiteralSet(literals)) => {
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
        CwtTypeOrSpecial::CwtType(CwtType::Simple(simple)) => match simple {
            SimpleType::Bool => "boolean".to_string(),
            SimpleType::Int => "integer".to_string(),
            SimpleType::Float => "float".to_string(),
            SimpleType::Scalar => "scalar".to_string(),
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
        CwtTypeOrSpecial::CwtType(CwtType::Reference(ref_type)) => {
            // Special handling for alias_match_left - expand it like a block
            if let ReferenceType::AliasMatchLeft { .. } = ref_type {
                let available_properties = resolver.get_available_properties(&scoped_type);

                let mut lines = vec![];

                for property_name in available_properties {
                    let property_type = resolver.navigate_to_property(&scoped_type, &property_name);

                    if let PropertyNavigationResult::Success(property_type) = property_type {
                        let formatted_value = format_type_description_with_property_context(
                            &property_type,
                            depth + 1,
                            max_lines,
                            cwt_context,
                            resolver,
                            Some(&property_name),
                        );

                        lines.push(format!("{} = {}", property_name, formatted_value));
                    }
                }

                lines.join("\n")
            } else {
                format!("reference {:?}", ref_type)
            }
        }
        CwtTypeOrSpecial::CwtType(CwtType::Comparable(comparable)) => {
            format!(
                "comparable[{}]",
                format_type_description_with_property_context(
                    &ScopedType::new_cwt(*comparable.clone(), scoped_type.scope_stack().clone()),
                    depth + 1,
                    max_lines,
                    cwt_context,
                    resolver,
                    property_name,
                )
            )
        }
        CwtTypeOrSpecial::CwtType(CwtType::Block(block)) => {
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
                    &ScopedType::new_cwt(
                        property_def.property_type.clone(),
                        scoped_type.scope_stack().clone(),
                    ),
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

                let formatted_value = format_type_description_with_property_context(
                    &ScopedType::new_cwt(
                        pattern_property.value_type.clone(),
                        scoped_type.scope_stack().clone(),
                    ),
                    depth + 1,
                    max_lines - line_count,
                    cwt_context,
                    resolver,
                    None, // Pattern properties don't have specific property names
                );

                // Handle multi-line types (nested blocks)
                if formatted_value.contains('\n') {
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
                    lines.push(format!("{}", formatted_value));
                    line_count += 1;
                }
                properties_shown += 1;
            }

            lines.push("}".to_string());

            lines.join("\n")
        }
        CwtTypeOrSpecial::CwtType(CwtType::Array(array_type)) => {
            let element_desc = format_type_description_with_property_context(
                &ScopedType::new_cwt(
                    *array_type.element_type.clone(),
                    scoped_type.scope_stack().clone(),
                ),
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
        CwtTypeOrSpecial::CwtType(CwtType::Union(types)) => {
            if types.len() <= MAX_UNION_MEMBERS {
                types
                    .iter()
                    .map(|t| {
                        format_type_description_with_property_context(
                            &ScopedType::new_cwt(t.clone(), scoped_type.scope_stack().clone()),
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
                        .take(MAX_UNION_MEMBERS)
                        .map(|t| format_type_description_with_property_context(
                            &ScopedType::new_cwt(t.clone(), scoped_type.scope_stack().clone()),
                            depth + 1,
                            max_lines,
                            cwt_context,
                            resolver,
                            property_name,
                        ))
                        .collect::<Vec<_>>()
                        .join(" | "),
                    types.len() - MAX_UNION_MEMBERS
                )
            }
        }
        CwtTypeOrSpecial::CwtType(CwtType::Unknown) => "unknown".to_string(),
        CwtTypeOrSpecial::ScopedUnion(types) => {
            if types.len() <= MAX_UNION_MEMBERS {
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
                        .take(MAX_UNION_MEMBERS)
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
                    types.len() - MAX_UNION_MEMBERS
                )
            }
        }
    }
}
