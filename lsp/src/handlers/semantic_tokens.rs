use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tower_lsp::Client;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use crate::handlers::utils::log_message_sync;

use super::document_cache::DocumentCache;

pub fn semantic_tokens_full(
    client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    document_cache: &DocumentCache,
    params: SemanticTokensParams,
) -> Result<Option<SemanticTokensResult>> {
    let uri = params.text_document.uri.to_string();

    log_message_sync(
        client,
        MessageType::INFO,
        format!("Semantic tokens (full) requested for: {}", uri),
    );

    let documents_guard = documents.read().unwrap();
    if let Some(content) = documents_guard.get(&uri) {
        let start_time = Instant::now();

        // Use document cache for efficient token generation
        let token_data = document_cache.get_semantic_tokens(&uri, content, None, None);

        let duration = start_time.elapsed();
        log_message_sync(
            client,
            MessageType::INFO,
            format!(
                "Semantic tokens (full) generated {} tokens in {:.2}ms for: {}",
                token_data.len(),
                duration.as_secs_f64() * 1000.0,
                uri
            ),
        );

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: token_data,
        })))
    } else {
        eprintln!("No content found for URI: {}", uri);
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: vec![],
        })))
    }
}

pub fn semantic_tokens_range(
    client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    document_cache: &DocumentCache,
    params: SemanticTokensRangeParams,
) -> Result<Option<SemanticTokensRangeResult>> {
    let uri = params.text_document.uri.to_string();

    log_message_sync(
        client,
        MessageType::INFO,
        format!(
            "Semantic tokens (range) requested for: {} at {:?}",
            uri, params.range
        ),
    );

    let documents_guard = documents.read().unwrap();
    if let Some(content) = documents_guard.get(&uri) {
        let start_time = Instant::now();

        // Use document cache for efficient token generation with range filtering
        let token_data =
            document_cache.get_semantic_tokens(&uri, content, None, Some(params.range));

        let duration = start_time.elapsed();
        log_message_sync(
            client,
            MessageType::INFO,
            format!(
                "Semantic tokens (range) generated {} tokens in {:.2}ms for range {:?} in: {}",
                token_data.len(),
                duration.as_secs_f64() * 1000.0,
                params.range,
                uri
            ),
        );

        Ok(Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
            result_id: None,
            data: token_data,
        })))
    } else {
        Ok(Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
            result_id: None,
            data: vec![],
        })))
    }
}
