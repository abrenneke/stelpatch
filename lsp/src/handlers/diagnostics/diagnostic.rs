use std::ops::Range;

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position};

use crate::handlers::diagnostics::util::span_to_lsp_range;

/// Create a diagnostic for type mismatches
pub fn create_type_mismatch_diagnostic(
    span: Range<usize>,
    message: &str,
    content: &str,
) -> Diagnostic {
    let range = span_to_lsp_range(span, content);

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("type-mismatch".to_string())),
        code_description: None,
        source: Some("cw-type-checker".to_string()),
        message: message.to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Create a diagnostic for value mismatches
pub fn create_value_mismatch_diagnostic(
    span: Range<usize>,
    message: &str,
    content: &str,
) -> Diagnostic {
    let range = span_to_lsp_range(span, content);

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String("value-mismatch".to_string())),
        code_description: None,
        source: Some("cw-type-checker".to_string()),
        message: message.to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Create a diagnostic for an unexpected key
pub fn create_unexpected_key_diagnostic(
    span: Range<usize>,
    key_name: &str,
    type_name: &str,
    content: &str,
) -> Diagnostic {
    let range = span_to_lsp_range(span, content);

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String("unexpected-key".to_string())),
        code_description: None,
        source: Some("cw-type-checker".to_string()),
        message: format!("Unexpected key '{}' in {} entity", key_name, type_name),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Create an LSP diagnostic from a parsing error
pub fn create_diagnostic_from_parse_error(
    error: &cw_parser::CwParseError,
    _content: &str,
) -> Diagnostic {
    // Extract position information from the structured error
    let (range, message) = match error {
        cw_parser::CwParseError::Parse(parse_error) => {
            // Use the position information from the structured error
            let start_line = parse_error.line as u32;
            let start_character = parse_error.column as u32;

            // Calculate end position based on the span
            let span_length = (parse_error.span.end - parse_error.span.start) as u32;
            let end_character = start_character + span_length.max(1);

            let range = tower_lsp::lsp_types::Range {
                start: Position {
                    line: start_line,
                    character: start_character,
                },
                end: Position {
                    line: start_line,
                    character: end_character,
                },
            };

            (range, parse_error.message.clone())
        }
        cw_parser::CwParseError::Other(msg) => {
            // For non-parse errors, default to start of document
            let range = tower_lsp::lsp_types::Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 1,
                },
            };

            (range, msg.clone())
        }
    };

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("cw-parser".to_string()),
        message,
        related_information: None,
        tags: None,
        data: None,
    }
}
