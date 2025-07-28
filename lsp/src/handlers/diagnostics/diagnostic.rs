use std::ops::Range;

use lasso::Spur;
use tower_lsp::lsp_types::{DiagnosticSeverity, NumberOrString};

use crate::{handlers::diagnostics::util::span_to_lsp_range, interner::get_interner};

#[derive(Debug, Clone)]
pub struct UnresolvedDiagnostic<'a> {
    pub span: Range<usize>,
    pub message: String,
    pub content: &'a str,
    pub severity: DiagnosticSeverity,
    pub code: Option<NumberOrString>,
}

impl<'a> From<UnresolvedDiagnostic<'a>> for tower_lsp::lsp_types::Diagnostic {
    fn from(diagnostic: UnresolvedDiagnostic<'a>) -> Self {
        let range = span_to_lsp_range(diagnostic.span, &diagnostic.content);
        tower_lsp::lsp_types::Diagnostic {
            range,
            severity: Some(diagnostic.severity),
            message: diagnostic.message,
            code: diagnostic.code,
            code_description: None,
            source: None,
            related_information: None,
            tags: None,
            data: None,
        }
    }
}

/// Create a diagnostic for type mismatches
pub fn create_type_mismatch_diagnostic<'a>(
    span: Range<usize>,
    message: &str,
    content: &'a str,
) -> UnresolvedDiagnostic<'a> {
    UnresolvedDiagnostic {
        span,
        message: message.to_string(),
        content,
        severity: DiagnosticSeverity::ERROR,
        code: Some(NumberOrString::String("type-mismatch".to_string())),
    }
}

/// Create a diagnostic for value mismatches
pub fn create_value_mismatch_diagnostic<'a>(
    span: Range<usize>,
    message: &str,
    content: &'a str,
) -> UnresolvedDiagnostic<'a> {
    UnresolvedDiagnostic {
        span,
        message: message.to_string(),
        content,
        severity: DiagnosticSeverity::WARNING,
        code: Some(NumberOrString::String("value-mismatch".to_string())),
    }
}

/// Create a diagnostic for an unexpected key
pub fn create_unexpected_key_diagnostic<'a>(
    span: Range<usize>,
    key_name: Spur,
    type_name: &str,
    content: &'a str,
) -> UnresolvedDiagnostic<'a> {
    UnresolvedDiagnostic {
        span,
        message: format!(
            "Unexpected key '{}' in {} entity",
            get_interner().resolve(&key_name),
            type_name
        ),
        content,
        severity: DiagnosticSeverity::WARNING,
        code: Some(NumberOrString::String("unexpected-key".to_string())),
    }
}

/// Create an LSP diagnostic from a parsing error
pub fn create_diagnostic_from_parse_error<'a>(
    error: &cw_parser::CwParseError,
    content: &'a str,
) -> UnresolvedDiagnostic<'a> {
    // Extract position information from the structured error
    let (range, message) = match error {
        cw_parser::CwParseError::Parse(parse_error) => {
            (parse_error.span.clone(), parse_error.message.clone())
        }
        cw_parser::CwParseError::Other(msg) => {
            // For non-parse errors, default to start of document
            let range = 0..1;

            (range, msg.clone())
        }
    };

    UnresolvedDiagnostic {
        span: range,
        message: message.clone(),
        content,
        severity: DiagnosticSeverity::ERROR,
        code: None,
    }
}
