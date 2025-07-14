use std::ops::Range;

use cw_model::CwtType;
use tower_lsp::lsp_types::Position;

/// Get a human-readable name for a CWT type
pub fn get_type_name(cwt_type: &CwtType) -> String {
    match cwt_type {
        CwtType::Simple(simple_type) => format!("{:?}", simple_type),
        CwtType::Block(_) => "block".to_string(),
        CwtType::Literal(value) => format!("'{}'", value),
        CwtType::LiteralSet(values) => {
            let value_list: Vec<_> = values.iter().take(3).collect();
            if values.len() > 3 {
                format!("one of {:?}...", value_list)
            } else {
                format!("one of {:?}", value_list)
            }
        }
        CwtType::Array(_) => "array".to_string(),
        CwtType::Union(_) => "union".to_string(),
        CwtType::Comparable(base_type) => {
            format!("comparable {}", get_type_name(base_type))
        }
        CwtType::Reference(ref_type) => match ref_type {
            cw_model::ReferenceType::Type { key } => format!("<{}>", key),
            cw_model::ReferenceType::Enum { key } => format!("enum {}", key),
            cw_model::ReferenceType::ComplexEnum { key } => format!("complex_enum {}", key),
            cw_model::ReferenceType::ValueSet { key } => format!("value_set {}", key),
            cw_model::ReferenceType::Value { key } => format!("value {}", key),
            cw_model::ReferenceType::Scope { key } => format!("scope {}", key),
            cw_model::ReferenceType::ScopeGroup { key } => format!("scope_group {}", key),
            cw_model::ReferenceType::Alias { key } => format!("alias {}", key),
            cw_model::ReferenceType::AliasName { key } => format!("alias_name {}", key),
            cw_model::ReferenceType::AliasMatchLeft { key } => {
                format!("alias_match_left {}", key)
            }
            cw_model::ReferenceType::SingleAlias { key } => format!("single_alias {}", key),
            cw_model::ReferenceType::AliasKeysField { key } => {
                format!("alias_keys_field {}", key)
            }
            cw_model::ReferenceType::Colour { format } => format!("colour ({})", format),
            cw_model::ReferenceType::Icon { path } => format!("icon ({})", path),
            cw_model::ReferenceType::Filepath { path } => format!("filepath ({})", path),
            cw_model::ReferenceType::Subtype { name } => format!("subtype {}", name),
            cw_model::ReferenceType::StellarisNameFormat { key } => {
                format!("name_format {}", key)
            }
            _ => format!("reference {:?}", ref_type),
        },
        CwtType::Unknown => "unknown".to_string(),
        CwtType::Any => "any".to_string(),
    }
}

/// Convert a byte span to an LSP range
pub fn span_to_lsp_range(span: Range<usize>, content: &str) -> tower_lsp::lsp_types::Range {
    let start_position = offset_to_position(content, span.start);
    let end_position = offset_to_position(content, span.end);

    tower_lsp::lsp_types::Range {
        start: start_position,
        end: end_position,
    }
}

/// Convert byte offset to LSP position
fn offset_to_position(content: &str, offset: usize) -> Position {
    let mut line = 0;
    let mut character = 0;

    for (i, ch) in content.char_indices() {
        if i >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }

    Position {
        line: line as u32,
        character: character as u32,
    }
}
