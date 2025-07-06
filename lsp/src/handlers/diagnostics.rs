use cw_parser::AstModule;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::Client;
use tower_lsp::lsp_types::*;

use super::document_cache::DocumentCache;

/// Generate diagnostics for a document by attempting to parse it
pub async fn generate_diagnostics(
    client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    _document_cache: &DocumentCache,
    uri: &str,
) {
    let documents_guard = documents.read().await;
    if let Some(content) = documents_guard.get(uri) {
        let diagnostics = parse_and_generate_diagnostics(content).await;

        // Publish diagnostics to the client
        client
            .publish_diagnostics(Url::parse(uri).unwrap(), diagnostics, None)
            .await;
    }
}

/// Parse content and generate diagnostics for any parsing errors
async fn parse_and_generate_diagnostics(content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Try to parse the content
    let mut module = AstModule::new();
    if let Err(error) = module.parse_input(content) {
        // Convert parsing error to LSP diagnostic
        let diagnostic = create_diagnostic_from_parse_error(&error, content);
        diagnostics.push(diagnostic);
    }

    diagnostics
}

/// Create an LSP diagnostic from a parsing error
fn create_diagnostic_from_parse_error(
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

            let range = Range {
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
            let range = Range {
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
