use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tower_lsp::Client;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use crate::handlers::document_cache::DocumentCache;

pub fn document_formatting(
    _client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    _document_cache: &DocumentCache,
    params: DocumentFormattingParams,
) -> Result<Option<Vec<TextEdit>>> {
    let uri = params.text_document.uri.to_string();
    let documents = documents.read().expect("Failed to read documents");

    let Some(content) = documents.get(&uri) else {
        return Ok(None);
    };

    let formatted_content = cw_format::format_module(content.clone());

    // If content is already formatted, return None
    if *content == formatted_content {
        return Ok(None);
    }

    // Create a text edit that replaces the entire document
    let edit = TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: get_document_end_position(content),
        },
        new_text: formatted_content,
    };

    Ok(Some(vec![edit]))
}

pub fn document_range_formatting(
    _client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    _document_cache: &DocumentCache,
    params: DocumentRangeFormattingParams,
) -> Result<Option<Vec<TextEdit>>> {
    let uri = params.text_document.uri.to_string();
    let documents = documents.read().expect("Failed to read documents");

    let Some(content) = documents.get(&uri) else {
        return Ok(None);
    };

    // Extract the range content
    let range_content = extract_range_content(content, &params.range);
    let formatted_range_content = cw_format::format_module(range_content.clone());

    // If range content is already formatted, return None
    if range_content == formatted_range_content {
        return Ok(None);
    }

    // Create a text edit that replaces the range
    let edit = TextEdit {
        range: params.range,
        new_text: formatted_range_content,
    };

    Ok(Some(vec![edit]))
}

fn get_document_end_position(content: &str) -> Position {
    let lines: Vec<&str> = content.lines().collect();
    let last_line_index = lines.len().saturating_sub(1);
    let last_line_length = lines
        .get(last_line_index)
        .map(|line| line.len())
        .unwrap_or(0);

    Position {
        line: last_line_index as u32,
        character: last_line_length as u32,
    }
}

fn extract_range_content(content: &str, range: &Range) -> String {
    let lines: Vec<&str> = content.lines().collect();

    let start_line = range.start.line as usize;
    let end_line = range.end.line as usize;
    let start_char = range.start.character as usize;
    let end_char = range.end.character as usize;

    if start_line >= lines.len() {
        return String::new();
    }

    if start_line == end_line {
        // Single line range
        if let Some(line) = lines.get(start_line) {
            let start_idx = start_char.min(line.len());
            let end_idx = end_char.min(line.len());
            return line[start_idx..end_idx].to_string();
        }
    } else {
        // Multi-line range
        let mut result = String::new();

        for (i, line) in lines
            .iter()
            .enumerate()
            .skip(start_line)
            .take(end_line - start_line + 1)
        {
            if i == start_line {
                // First line: start from start_char
                let start_idx = start_char.min(line.len());
                result.push_str(&line[start_idx..]);
            } else if i == end_line {
                // Last line: end at end_char
                let end_idx = end_char.min(line.len());
                result.push_str(&line[..end_idx]);
            } else {
                // Middle lines: include entire line
                result.push_str(line);
            }

            if i < end_line {
                result.push('\n');
            }
        }

        return result;
    }

    String::new()
}
