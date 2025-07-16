use std::sync::Arc;

use cw_model::{CwtType, ReferenceType, SimpleType};

use crate::handlers::cache::resolver::TypeResolver;
use crate::handlers::scoped_type::{CwtTypeOrSpecial, PropertyNavigationResult, ScopedType};

const MAX_UNION_MEMBERS: usize = 8;
const MAX_LITERAL_SET_MEMBERS: usize = 30;

/// A formatter for CWT types that holds references to context and configuration
pub struct TypeFormatter<'a> {
    resolver: &'a TypeResolver,
    max_lines: usize,
}

impl<'a> TypeFormatter<'a> {
    pub fn new(resolver: &'a TypeResolver, max_lines: usize) -> Self {
        Self {
            resolver,
            max_lines,
        }
    }

    pub fn format_type(&self, scoped_type: Arc<ScopedType>, property_name: Option<&str>) -> String {
        self.format_type_with_depth(scoped_type, 0, property_name)
    }

    /// Format a type description with depth control and optional property name context
    fn format_type_with_depth(
        &self,
        scoped_type: Arc<ScopedType>,
        depth: usize,
        property_name: Option<&str>,
    ) -> String {
        if depth > 3 {
            return "...".to_string();
        }

        let scoped_type = self.resolver.resolve_type(scoped_type);

        match scoped_type.cwt_type() {
            CwtTypeOrSpecial::CwtType(CwtType::Literal(lit)) => format!("\"{}\"", lit),
            CwtTypeOrSpecial::CwtType(CwtType::LiteralSet(literals)) => {
                let mut sorted: Vec<_> = literals.iter().collect();
                sorted.sort();
                if sorted.len() <= MAX_LITERAL_SET_MEMBERS {
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
                        literals.len() - MAX_LITERAL_SET_MEMBERS
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
                format!("reference {:?}", ref_type)
            }
            CwtTypeOrSpecial::CwtType(CwtType::Comparable(comparable)) => {
                format!(
                    "comparable[{}]",
                    self.format_type_with_depth(
                        Arc::new(ScopedType::new_cwt(
                            *comparable.clone(),
                            scoped_type.scope_stack().clone(),
                            scoped_type.in_scripted_effect_block().cloned(),
                        )),
                        depth + 1,
                        property_name,
                    )
                )
            }
            CwtTypeOrSpecial::CwtType(CwtType::Block(block)) => {
                // Show:
                // - The root obj
                // - The properties of the root obj
                // - The properties of the properties of the root obj
                let available_properties =
                    self.resolver.get_available_properties(scoped_type.clone());

                if depth >= 1 {
                    if available_properties.is_empty() {
                        return format!("{} {{}}", block.type_name);
                    } else {
                        return format!(
                            "{} {{ /* ... +{} properties */ }}",
                            block.type_name,
                            available_properties.len()
                        );
                    }
                }

                if available_properties.is_empty() {
                    return format!("{} {{}}", block.type_name);
                }

                let mut sorted_properties: Vec<_> = available_properties.iter().collect();
                sorted_properties.sort();

                let mut lines = vec![format!("{} {{", block.type_name)];
                let mut line_count = 1;
                let mut properties_shown = 0;

                for property_name in sorted_properties {
                    if line_count >= self.max_lines {
                        lines.push(format!(
                            "  # ... +{} more properties",
                            available_properties.len() - properties_shown
                        ));
                        break;
                    }

                    let property_type = self
                        .resolver
                        .navigate_to_property(scoped_type.clone(), property_name);

                    if let PropertyNavigationResult::Success(property_type) = property_type {
                        if matches!(
                            property_type.cwt_type(),
                            CwtTypeOrSpecial::CwtType(CwtType::Reference(
                                ReferenceType::AliasMatchLeft { .. }
                            ))
                        ) {
                            eprintln!(
                                "navigate_to_property '{}' did not resolve the alias_match_left, coming from {:?}",
                                property_name, scoped_type
                            );
                        }

                        let formatted_value = self.format_type_with_depth(
                            property_type,
                            depth + 1,
                            Some(property_name),
                        );

                        // Handle multi-line types (nested blocks)
                        if formatted_value.contains('\n') {
                            lines.push(format!("  {}:", property_name));
                            let nested_lines: Vec<&str> = formatted_value.lines().collect();
                            let mut lines_added = 1;

                            for line in nested_lines {
                                if line.starts_with("{") {
                                    continue;
                                }
                                if line_count + lines_added >= self.max_lines {
                                    lines.push("    # ... (truncated)".to_string());
                                    break;
                                }
                                lines.push(format!("    {}", line));
                                lines_added += 1;
                            }
                            line_count += lines_added;
                        } else {
                            lines.push(format!("  {} = {}", property_name, formatted_value));
                            line_count += 1;
                        }
                        properties_shown += 1;
                    }
                }

                lines.push("}".to_string());

                lines.join("\n")
            }
            CwtTypeOrSpecial::CwtType(CwtType::Array(array_type)) => {
                let element_desc = self.format_type_with_depth(
                    Arc::new(ScopedType::new_cwt(
                        *array_type.element_type.clone(),
                        scoped_type.scope_stack().clone(),
                        scoped_type.in_scripted_effect_block().cloned(),
                    )),
                    depth + 1,
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
                            self.format_type_with_depth(
                                Arc::new(ScopedType::new_cwt(
                                    t.clone(),
                                    scoped_type.scope_stack().clone(),
                                    scoped_type.in_scripted_effect_block().cloned(),
                                )),
                                depth + 1,
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
                            .map(|t| self.format_type_with_depth(
                                Arc::new(ScopedType::new_cwt(
                                    t.clone(),
                                    scoped_type.scope_stack().clone(),
                                    scoped_type.in_scripted_effect_block().cloned(),
                                )),
                                depth + 1,
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
                            self.format_type_with_depth(
                                Arc::new(t.clone()),
                                depth + 1,
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
                            .map(|t| {
                                self.format_type_with_depth(
                                    Arc::new(t.clone()),
                                    depth + 1,
                                    property_name,
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(" | "),
                        types.len() - MAX_UNION_MEMBERS
                    )
                }
            }

            CwtTypeOrSpecial::CwtType(CwtType::Any) => "any".to_string(),
        }
    }
}
